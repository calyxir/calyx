use std::rc::Rc;

use crate::{
    guard,
    ir::{self, RRC},
    passes::math_utilities::get_bit_width_from,
    structure,
};

/// Name of the attributes added by this pass.
pub const ID: &str = "ST_ID";
pub const LOOP: &str = "LOOP";
pub const START: &str = "START";
pub const END: &str = "END";

/// Computes the states associated with control nodes in a static program.
/// Each enable statement gets a number corresponding to the FSM state when it
/// can be scheduled.
///
/// While loops instantiate an indexor is used by
/// [crate::passes::TopDownStaticTiming] to implement the compilation logic. We
/// allocate these in this pass so that [ComputeStates] can correctly compute the
/// exit edges for a given control program.
pub struct ComputeStates {
    /// Current state
    cur_st: u64,
    /// Mapping for loop indices
    indices: Vec<RRC<ir::Cell>>,
}
impl Default for ComputeStates {
    fn default() -> Self {
        Self {
            /// 0 is a special start state allocated to the start of the
            /// program so we start with the state 1.
            cur_st: 1,
            indices: vec![],
        }
    }
}

impl ComputeStates {
    /// Compute the states associated with the control program.
    pub fn new(con: &mut ir::Control, builder: &mut ir::Builder) -> Self {
        let mut cs = Self::default();
        cs.recur(con, builder);
        cs
    }

    fn recur(&mut self, con: &mut ir::Control, builder: &mut ir::Builder) {
        match con {
            ir::Control::Enable(en) => {
                // debug_assert!(en.attributes.get("static").is_none());
                en.attributes[ID] = self.cur_st;
                let time = en.attributes["static"];
                self.cur_st += time;
            }
            ir::Control::Seq(seq) => {
                for stmt in &mut seq.stmts {
                    self.recur(stmt, builder);
                }
            }
            ir::Control::If(ir::If {
                tbranch, fbranch, ..
            }) => {
                self.recur(tbranch, builder);
                self.recur(fbranch, builder);
            }
            ir::Control::While(wh) => self.compute_while(wh, builder),
            ir::Control::Par(par) => {
                par.attributes[ID] = self.cur_st;
                // All statements should only contain enables and get the same
                // start state as the `par` block.
                for stmt in &mut par.stmts {
                    if let ir::Control::Enable(en) = stmt {
                        en.attributes[ID] = self.cur_st;
                    } else {
                        unreachable!("Par should only contain enables")
                    }
                }
                let time = par.attributes["static"];
                self.cur_st += time;
            }
            ir::Control::Invoke(_) => unreachable!(
                "Invoke statements should have been compiled away."
            ),
            ir::Control::Empty(_) => {
                unreachable!("Empty blocks should have been compiled away")
            }
        }
    }

    fn compute_while(&mut self, wh: &mut ir::While, builder: &mut ir::Builder) {
        // Compute START, END, and LOOP index attributes
        wh.attributes[START] = self.cur_st;
        let body_time = wh.attributes["static"];
        // Instantiate the indexing variable for this while loop
        let size = get_bit_width_from(body_time + 1);
        structure!(builder;
            let idx = prim std_reg(size);
        );
        self.indices.push(idx);
        let idx_pos = self.indices.len() - 1;
        // Add attribute to track the loop counter
        wh.attributes[LOOP] = idx_pos as u64;
        self.recur(&mut wh.body, builder);
        // Mark the end state of the body
        wh.attributes[END] = self.cur_st;
    }

    /// Computes the outgoing edges from the control programs.
    /// **Requires**: `con` is a sub-program of the control program used to
    /// construct this [States] instance.
    pub fn control_exits(
        &self,
        con: &ir::Control,
        builder: &mut ir::Builder,
        exits: &mut Vec<(u64, ir::Guard)>,
    ) {
        match con {
            ir::Control::Enable(en) => {
                let st = en.attributes[ID] + en.attributes["static"] - 1;
                exits.push((st, ir::Guard::True));
            }
            ir::Control::Par(par) => {
                let st = par.attributes[ID] + par.attributes["static"] - 1;
                exits.push((st, ir::Guard::True))
            }
            ir::Control::Seq(s) => {
                if let Some(stmt) = s.stmts.last() {
                    self.control_exits(stmt, builder, exits);
                }
            }
            ir::Control::If(if_) => {
                let ir::If {
                    tbranch, fbranch, ..
                } = if_;
                self.control_exits(tbranch, builder, exits);
                self.control_exits(fbranch, builder, exits);
            }
            ir::Control::While(wh) => {
                let ir::While { body, .. } = wh;
                // Compute the exit conditions for the loop body
                let mut loop_exits = Vec::new();
                self.control_exits(body, builder, &mut loop_exits);

                // Guard the exit edges for the body with the loop exit condition
                let (idx, bound) = self.loop_bounds(wh, builder);
                let guard = guard!(idx["out"]).eq(guard!(bound["out"]));
                exits.extend(
                    loop_exits
                        .into_iter()
                        .map(|(st, g)| (st, g & guard.clone())),
                );
            }
            ir::Control::Invoke(_) => {
                unreachable!("Invoke should have been compiled away")
            }
            ir::Control::Empty(_) => {
                unreachable!("Empty block in control_exits")
            }
        }
    }

    /// Generate the guard condition for exiting the given loop.
    /// **Requires**: The loop is a sub-program of the control program used to
    /// generate this [States] instance.
    pub fn loop_bounds(
        &self,
        wh: &ir::While,
        builder: &mut ir::Builder,
    ) -> (RRC<ir::Cell>, RRC<ir::Cell>) {
        let max_count = wh.attributes["static"];
        let size = get_bit_width_from(max_count + 1);
        structure!(builder;
            let max = constant(max_count, size);
        );
        let idx_pos = wh.attributes[LOOP] as usize;
        let idx = Rc::clone(&self.indices[idx_pos]);
        (idx, max)
    }
}
