use crate::ir::{self};

const NODE_ID: &str = "NODE_ID";
const BEGIN_ID: &str = "BEGIN_ID";
const END_ID: &str = "END_ID";

/// Adding "NODE_ID", "BEGIN_ID", and "END_ID" attribute to control statement
pub struct ControlId;

impl ControlId {
    /// Adds the @NODE_ID attribute to all control stmts except emtpy ones.
    /// If two_if_ids is true, then if statements get a BEGIN_ID and END_ID instead
    /// of a NODE_ID
    ///
    /// ## Example:
    /// ```
    /// seq { A; if cond {X} else{Y}; par { C; D; }; E }
    /// ```
    ///
    /// gets the labels (if two_if_ids is):
    ///
    /// ```
    /// @NODE_ID(0)seq {
    ///   @NODE_ID(1) A;
    ///   @BEGIN_ID(2) @END_ID(5) if cond {
    ///     @NODE_ID(3) X
    ///   }
    ///   else{
    ///     @NODE_ID(4) Y
    ///   }
    ///   @NODE_ID(6) par {
    ///     @NODE_ID(7) C;
    ///     @NODE_ID(8) D;
    ///   }
    ///   @NODE_ID(9) E;
    /// }
    /// ```
    /// if two_if_ids were false, the if statement would just get a single NODE_ID
    pub fn compute_unique_ids(
        con: &mut ir::Control,
        mut cur_state: u64,
        two_if_ids: bool,
    ) -> u64 {
        match con {
            ir::Control::Enable(ir::Enable { attributes, .. })
            | ir::Control::Invoke(ir::Invoke { attributes, .. }) => {
                attributes.insert(NODE_ID, cur_state);
                cur_state + 1
            }
            ir::Control::Par(ir::Par {
                stmts, attributes, ..
            })
            | ir::Control::Seq(ir::Seq {
                stmts, attributes, ..
            }) => {
                attributes.insert(NODE_ID, cur_state);
                cur_state += 1;
                stmts.iter_mut().for_each(|stmt| {
                    let new_state =
                        Self::compute_unique_ids(stmt, cur_state, two_if_ids);
                    cur_state = new_state;
                });
                cur_state
            }
            ir::Control::If(ir::If {
                tbranch,
                fbranch,
                attributes,
                ..
            }) => {
                if two_if_ids {
                    attributes.insert(BEGIN_ID, cur_state);
                    cur_state += 1;
                    cur_state = Self::compute_unique_ids(
                        tbranch, cur_state, two_if_ids,
                    );
                    cur_state = Self::compute_unique_ids(
                        fbranch, cur_state, two_if_ids,
                    );
                    attributes.insert(END_ID, cur_state);
                    cur_state + 1
                } else {
                    attributes.insert(NODE_ID, cur_state);
                    cur_state += 1;
                    cur_state = Self::compute_unique_ids(
                        tbranch, cur_state, two_if_ids,
                    );
                    cur_state = Self::compute_unique_ids(
                        fbranch, cur_state, two_if_ids,
                    );
                    cur_state + 1
                }
            }
            ir::Control::While(ir::While {
                body, attributes, ..
            }) => {
                attributes.insert(NODE_ID, cur_state);
                cur_state += 1;
                Self::compute_unique_ids(body, cur_state, two_if_ids)
            }
            ir::Control::Empty(_) => cur_state,
        }
    }

    // Gets attribute s from c, panics otherwise. Should be used when you know
    // that c has attribute s.
    pub fn get_guaranteed_attribute(c: &ir::Control, s: &str) -> u64 {
        *c.get_attribute(s).unwrap_or_else(||unreachable!(
          "called get_guaranteed_attribute, meaning we had to be sure it had the id"
      ))
    }

    // Gets attribute NODE_ID from c
    pub fn get_guaranteed_id(c: &ir::Control) -> u64 {
        Self::get_guaranteed_attribute(c, NODE_ID)
    }
}
