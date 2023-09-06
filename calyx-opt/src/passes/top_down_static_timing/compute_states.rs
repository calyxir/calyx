use crate::passes::math_utilities::get_bit_width_from;
use calyx_ir::{self as ir, guard, structure, RRC};
use ir::Nothing;
use std::rc::Rc;

/// Name of the attributes added by this pass.
pub const ID: ir::Attribute = ir::Attribute::Internal(ir::InternalAttr::ST_ID);
pub const LOOP: ir::Attribute = ir::Attribute::Internal(ir::InternalAttr::LOOP);
pub const START: ir::Attribute =
    ir::Attribute::Internal(ir::InternalAttr::START);
pub const END: ir::Attribute = ir::Attribute::Internal(ir::InternalAttr::END);

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
                debug_assert!(en.attributes.get(ID).is_none());
                en.attributes.insert(ID, self.cur_st);
                let time = en.attributes.get(ir::NumAttr::Static).unwrap();
                self.cur_st += time;
            }
            ir::Control::Static(_) => {
                panic!("Static behavior on tdst TBD")
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
                par.attributes.insert(ID, self.cur_st);
                // All statements should only contain enables and get the same
                // start state as the `par` block.
                for stmt in &mut par.stmts {
                    if let ir::Control::Enable(en) = stmt {
                        en.attributes.insert(ID, self.cur_st);
                    } else {
                        unreachable!("Par should only contain enables")
                    }
                }
                let time = par.attributes.get(ir::NumAttr::Static).unwrap();
                self.cur_st += time;
            }
            ir::Control::Invoke(_) => unreachable!(
                "Invoke statements should have been compiled away."
            ),
            ir::Control::Repeat(_) => {
                unreachable!("Repeats should've been compiled away.")
            }
            ir::Control::Empty(_) => {
                unreachable!("Empty blocks should have been compiled away")
            }
        }
    }

    fn compute_while(&mut self, wh: &mut ir::While, builder: &mut ir::Builder) {
        // Compute START, END, and LOOP index attributes
        wh.attributes.insert(START, self.cur_st);
        let body_time = wh.attributes.get(ir::NumAttr::Static).unwrap();
        // Instantiate the indexing variable for this while loop
        let size = get_bit_width_from(body_time + 1);
        structure!(builder;
            let idx = prim std_reg(size);
        );
        self.indices.push(idx);
        let idx_pos = self.indices.len() - 1;
        // Add attribute to track the loop counter
        wh.attributes.insert(LOOP, idx_pos as u64);
        self.recur(&mut wh.body, builder);
        // Mark the end state of the body
        wh.attributes.insert(END, self.cur_st);
    }

    /// Computes the outgoing edges from the control programs.
    /// **Requires**: `con` is a sub-program of the control program used to
    /// construct this [States] instance.
    pub fn control_exits(
        &self,
        con: &ir::Control,
        builder: &mut ir::Builder,
        exits: &mut Vec<(u64, ir::Guard<Nothing>)>,
    ) {
        match con {
            ir::Control::Enable(en) => {
                let st = en.attributes.get(ID).unwrap()
                    + en.attributes.get(ir::NumAttr::Static).unwrap()
                    - 1;
                exits.push((st, ir::Guard::True));
            }
            ir::Control::Static(_) => {
                panic!("Static behavior on tdst TBD")
            }
            ir::Control::Par(par) => {
                let st = par.attributes.get(ID).unwrap()
                    + par.attributes.get(ir::NumAttr::Static).unwrap()
                    - 1;
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
                let guard = guard!(idx["out"] == bound["out"]);
                exits.extend(
                    loop_exits
                        .into_iter()
                        .map(|(st, g)| (st, g & guard.clone())),
                );
            }
            ir::Control::Invoke(_) => {
                unreachable!("Invoke should have been compiled away")
            }
            ir::Control::Repeat(_) => {
                unreachable!("Repeat should have been compiled away")
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
        let max_count = wh.attributes.get(ir::NumAttr::Static).unwrap();
        let size = get_bit_width_from(max_count + 1);
        structure!(builder;
            let max = constant(max_count, size);
        );
        let idx_pos = wh.attributes.get(LOOP).unwrap() as usize;
        let idx = Rc::clone(&self.indices[idx_pos]);
        (idx, max)
    }

    /// Return iterator over all defined loop indexing cells
    pub fn indices(self) -> impl Iterator<Item = RRC<ir::Cell>> {
        self.indices.into_iter()
    }
}
