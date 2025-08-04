use calyx_ir::{self as ir};
use calyx_ir::{Nothing, build_assignments};
use calyx_ir::{guard, structure};
use calyx_utils::math::bits_needed_for;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug, Clone, Copy, Default)]
// Define an FSMEncoding Enum
pub enum FSMEncoding {
    #[default]
    Binary,
    OneHot,
}

#[derive(Debug)]
/// Represents a static FSM (i.e., the actual register in hardware that counts)
pub struct StaticFSM {
    /// The actual register cell
    fsm_cell: ir::RRC<ir::Cell>,
    /// Type of encoding (binary or one-hot)
    encoding: FSMEncoding,
    /// The fsm's bitwidth (this redundant information bc  we have `cell`)
    /// but makes it easier if we easily have access to this.
    bitwidth: u64,
    /// Mapping of queries: (u64, u64) -> Port
    queries: HashMap<(u64, u64), ir::RRC<ir::Port>>,
}
impl StaticFSM {
    // Builds a static_fsm from: num_states and encoding type.
    pub fn from_basic_info(
        num_states: u64,
        encoding: FSMEncoding,
        builder: &mut ir::Builder,
    ) -> Self {
        // Determine number of bits needed in the register.
        let fsm_size = match encoding {
            /* represent 0..latency */
            FSMEncoding::Binary => bits_needed_for(num_states + 1),
            FSMEncoding::OneHot => num_states,
        };
        // OHE needs an initial value of 1.
        let register = match encoding {
            FSMEncoding::Binary => {
                builder.add_primitive("fsm", "std_reg", &[fsm_size])
            }
            FSMEncoding::OneHot => {
                builder.add_primitive("fsm", "init_one_reg", &[fsm_size])
            }
        };

        StaticFSM {
            encoding,
            fsm_cell: register,
            bitwidth: fsm_size,
            queries: HashMap::new(),
        }
    }

    // Builds an incrementer, and returns the assignments and incrementer cell itself.
    // assignments are:
    // adder.left = fsm.out; adder.right = 1;
    // Returns tuple: (assignments, adder)
    pub fn build_incrementer(
        &self,
        builder: &mut ir::Builder,
    ) -> (Vec<ir::Assignment<Nothing>>, ir::RRC<ir::Cell>) {
        let fsm_cell = Rc::clone(&self.fsm_cell);
        // For OHE, the "adder" can just be a shifter.
        // For OHE the first_state = 1 rather than 0.
        // Final state is encoded differently for OHE vs. Binary
        let adder = match self.encoding {
            FSMEncoding::Binary => {
                builder.add_primitive("adder", "std_add", &[self.bitwidth])
            }
            FSMEncoding::OneHot => {
                builder.add_primitive("lsh", "std_lsh", &[self.bitwidth])
            }
        };
        let const_one = builder.add_constant(1, self.bitwidth);
        let incr_assigns = build_assignments!(
          builder;
          // increments the fsm
          adder["left"] = ? fsm_cell["out"];
          adder["right"] = ? const_one["out"];
        )
        .to_vec();
        (incr_assigns, adder)
    }

    // Returns the assignments that conditionally increment the fsm,
    // based on guard.
    // The assignments are:
    // fsm.in = guard ? adder.out;
    // fsm.write_en = guard ? 1'd1;
    // Returns a vec of these assignments.
    pub fn conditional_increment(
        &self,
        guard: ir::Guard<Nothing>,
        adder: ir::RRC<ir::Cell>,
        builder: &mut ir::Builder,
    ) -> Vec<ir::Assignment<Nothing>> {
        let fsm_cell = Rc::clone(&self.fsm_cell);
        let signal_on = builder.add_constant(1, 1);
        let my_assigns = build_assignments!(
          builder;
          // increments the fsm
          fsm_cell["in"] = guard ? adder["out"];
          fsm_cell["write_en"] = guard ? signal_on["out"];
        );
        my_assigns.to_vec()
    }

    // Returns the assignments that conditionally resets the fsm to 0,
    // but only if guard is true.
    // The assignments are:
    // fsm.in = guard ? 0;
    // fsm.write_en = guard ? 1'd1;
    // Returns a vec of these assignments.
    pub fn conditional_reset(
        &self,
        guard: ir::Guard<Nothing>,
        builder: &mut ir::Builder,
    ) -> Vec<ir::Assignment<Nothing>> {
        let fsm_cell = Rc::clone(&self.fsm_cell);
        let signal_on = builder.add_constant(1, 1);
        let const_0 = match self.encoding {
            FSMEncoding::Binary => builder.add_constant(0, self.bitwidth),
            FSMEncoding::OneHot => builder.add_constant(1, self.bitwidth),
        };
        let assigns = build_assignments!(
          builder;
          fsm_cell["in"] = guard ? const_0["out"];
          fsm_cell["write_en"] = guard ? signal_on["out"];
        );
        assigns.to_vec()
    }

    // Returns a guard that takes a (beg, end) `query`, and returns the equivalent
    // guard to `beg <= fsm.out < end`.
    pub fn query_between(
        &mut self,
        builder: &mut ir::Builder,
        query: (u64, u64),
    ) -> Box<ir::Guard<Nothing>> {
        let (beg, end) = query;
        // Querying OHE is easy, since we already have `self.get_one_hot_query()`
        let fsm_cell = Rc::clone(&self.fsm_cell);
        if matches!(self.encoding, FSMEncoding::OneHot) {
            let g = self.get_one_hot_query(fsm_cell, (beg, end), builder);
            return Box::new(g);
        }

        if beg + 1 == end {
            // if beg + 1 == end then we only need to check if fsm == beg
            let interval_const = builder.add_constant(beg, self.bitwidth);
            let g = guard!(fsm_cell["out"] == interval_const["out"]);
            Box::new(g)
        } else if beg == 0 {
            // if beg == 0, then we only need to check if fsm < end
            let end_const = builder.add_constant(end, self.bitwidth);
            let lt: ir::Guard<Nothing> =
                guard!(fsm_cell["out"] < end_const["out"]);
            Box::new(lt)
        } else {
            // otherwise, check if fsm >= beg & fsm < end
            let beg_const = builder.add_constant(beg, self.bitwidth);
            let end_const = builder.add_constant(end, self.bitwidth);
            let beg_guard: ir::Guard<Nothing> =
                guard!(fsm_cell["out"] >= beg_const["out"]);
            let end_guard: ir::Guard<Nothing> =
                guard!(fsm_cell["out"] < end_const["out"]);
            Box::new(ir::Guard::And(Box::new(beg_guard), Box::new(end_guard)))
        }
    }

    // Given a one-hot query, it will return a guard corresponding to that query.
    // If it has already built the query (i.e., added the wires/continuous assigments),
    // it just uses the same port.
    // Otherwise it will build the query.
    fn get_one_hot_query(
        &mut self,
        fsm_cell: ir::RRC<ir::Cell>,
        (lb, ub): (u64, u64),
        builder: &mut ir::Builder,
    ) -> ir::Guard<Nothing> {
        match self.queries.get(&(lb, ub)) {
            None => {
                let port = Self::build_one_hot_query(
                    Rc::clone(&fsm_cell),
                    self.bitwidth,
                    (lb, ub),
                    builder,
                );
                self.queries.insert((lb, ub), Rc::clone(&port));
                ir::Guard::port(port)
            }
            Some(port) => ir::Guard::port(Rc::clone(port)),
        }
    }

    // Given a (lb, ub) query, and an fsm (and for convenience, a bitwidth),
    // Returns a `port`: port is a `wire.out`, where `wire` holds
    // whether or not the query is true, i.e., whether the FSM really is
    // between [lb, ub).
    fn build_one_hot_query(
        fsm_cell: ir::RRC<ir::Cell>,
        fsm_bitwidth: u64,
        (lb, ub): (u64, u64),
        builder: &mut ir::Builder,
    ) -> ir::RRC<ir::Port> {
        // The wire that holds the query
        let formatted_name = format!("bw_{lb}_{ub}");
        let wire: ir::RRC<ir::Cell> =
            builder.add_primitive(formatted_name, "std_wire", &[1]);
        let wire_out = wire.borrow().get("out");

        // Continuous assignments to check the FSM
        let assigns = {
            let in_width = fsm_bitwidth;
            // Since 00...00 is the initial state, we need to check lb-1.
            let start_index = lb;
            // Since verilog slices are inclusive.
            let end_index = ub - 1;
            let out_width = ub - lb; // == (end_index - start_index + 1)
            structure!(builder;
                let slicer = prim std_bit_slice(in_width, start_index, end_index, out_width);
                let const_slice_0 = constant(0, out_width);
                let signal_on = constant(1,1);
            );
            let slicer_neq_0 = guard!(slicer["out"] != const_slice_0["out"]);
            // Extend the continuous assignmments to include this particular query for FSM state;
            let my_assigns = build_assignments!(builder;
                slicer["in"] = ? fsm_cell["out"];
                wire["in"] = slicer_neq_0 ? signal_on["out"];
            );
            my_assigns.to_vec()
        };
        builder.add_continuous_assignments(assigns);
        wire_out
    }

    // Return a unique id (i.e., get_unique_id for each FSM in the same component
    // will be different).
    pub fn get_unique_id(&self) -> ir::Id {
        self.fsm_cell.borrow().name()
    }

    // Return the bitwidth of an FSM object
    pub fn get_bitwidth(&self) -> u64 {
        self.bitwidth
    }
}
