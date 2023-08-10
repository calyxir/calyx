use calyx_ir as ir;

const NODE_ID: ir::Attribute =
    ir::Attribute::Internal(ir::InternalAttr::NODE_ID);
const BEGIN_ID: ir::Attribute =
    ir::Attribute::Internal(ir::InternalAttr::BEGIN_ID);
const END_ID: ir::Attribute = ir::Attribute::Internal(ir::InternalAttr::END_ID);

/// Adding "NODE_ID", "BEGIN_ID", and "END_ID" attribute to control statement
pub struct ControlId;

impl ControlId {
    fn compute_unique_ids_static(
        scon: &mut ir::StaticControl,
        mut cur_state: u64,
        two_if_ids: bool,
    ) -> u64 {
        match scon {
            ir::StaticControl::Empty(_) => cur_state,
            ir::StaticControl::Enable(ir::StaticEnable {
                attributes, ..
            })
            | ir::StaticControl::Invoke(ir::StaticInvoke {
                attributes, ..
            }) => {
                attributes.insert(NODE_ID, cur_state);
                cur_state + 1
            }
            ir::StaticControl::Repeat(ir::StaticRepeat {
                attributes,
                body,
                ..
            }) => {
                attributes.insert(NODE_ID, cur_state);
                cur_state += 1;
                Self::compute_unique_ids_static(body, cur_state, two_if_ids)
            }
            ir::StaticControl::Par(ir::StaticPar {
                stmts, attributes, ..
            })
            | ir::StaticControl::Seq(ir::StaticSeq {
                stmts, attributes, ..
            }) => {
                attributes.insert(NODE_ID, cur_state);
                cur_state += 1;
                stmts.iter_mut().for_each(|stmt| {
                    let new_state = Self::compute_unique_ids_static(
                        stmt, cur_state, two_if_ids,
                    );
                    cur_state = new_state;
                });
                cur_state
            }
            ir::StaticControl::If(ir::StaticIf {
                tbranch,
                fbranch,
                attributes,
                ..
            }) => {
                if two_if_ids {
                    attributes.insert(BEGIN_ID, cur_state);
                    cur_state += 1;
                    cur_state = Self::compute_unique_ids_static(
                        tbranch, cur_state, two_if_ids,
                    );
                    cur_state = Self::compute_unique_ids_static(
                        fbranch, cur_state, two_if_ids,
                    );
                    attributes.insert(END_ID, cur_state);
                    cur_state + 1
                } else {
                    attributes.insert(NODE_ID, cur_state);
                    cur_state += 1;
                    cur_state = Self::compute_unique_ids_static(
                        tbranch, cur_state, two_if_ids,
                    );
                    cur_state = Self::compute_unique_ids_static(
                        fbranch, cur_state, two_if_ids,
                    );
                    cur_state + 1
                }
            }
        }
    }

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
            })
            | ir::Control::Repeat(ir::Repeat {
                body, attributes, ..
            }) => {
                attributes.insert(NODE_ID, cur_state);
                cur_state += 1;
                Self::compute_unique_ids(body, cur_state, two_if_ids)
            }
            ir::Control::Static(s) => {
                Self::compute_unique_ids_static(s, cur_state, two_if_ids)
            }
            ir::Control::Empty(_) => cur_state,
        }
    }

    // Gets attribute s from c, panics otherwise. Should be used when you know
    // that c has attribute s.
    pub fn get_guaranteed_attribute<A>(c: &ir::Control, attr: A) -> u64
    where
        A: Into<ir::Attribute>,
    {
        c.get_attribute(attr.into()).unwrap_or_else(||unreachable!(
          "called get_guaranteed_attribute, meaning we had to be sure it had the attribute"
      ))
    }

    // Gets attribute s from c, panics otherwise. Should be used when you know
    // that c has attribute s.
    pub fn get_guaranteed_attribute_static<A>(
        sc: &ir::StaticControl,
        attr: A,
    ) -> u64
    where
        A: Into<ir::Attribute>,
    {
        sc.get_attribute(attr.into()).unwrap_or_else(||unreachable!(
          "called get_guaranteed_attribute_static, meaning we had to be sure it had the attribute"
      ))
    }

    // Gets attribute NODE_ID from c
    pub fn get_guaranteed_id(c: &ir::Control) -> u64 {
        Self::get_guaranteed_attribute(c, NODE_ID)
    }

    // Gets attribute NODE_ID from c
    pub fn get_guaranteed_id_static(sc: &ir::StaticControl) -> u64 {
        Self::get_guaranteed_attribute_static(sc, NODE_ID)
    }

    // takes in a static control scon, and adds unique id to each static enable.
    // Returns cur_state, i.e., what the next enable should be labeled as
    pub fn add_static_enable_ids_static(
        scon: &mut ir::StaticControl,
        mut cur_state: u64,
    ) -> u64 {
        match scon {
            ir::StaticControl::Enable(se) => {
                se.attributes.insert(NODE_ID, cur_state);
                cur_state + 1
            }
            ir::StaticControl::Invoke(_) | ir::StaticControl::Empty(_) => {
                cur_state
            }
            ir::StaticControl::Par(ir::StaticPar { stmts, .. })
            | ir::StaticControl::Seq(ir::StaticSeq { stmts, .. }) => {
                for stmt in stmts {
                    let new_state =
                        Self::add_static_enable_ids_static(stmt, cur_state);
                    cur_state = new_state
                }
                cur_state
            }
            ir::StaticControl::If(ir::StaticIf {
                tbranch, fbranch, ..
            }) => {
                let mut new_state =
                    Self::add_static_enable_ids_static(tbranch, cur_state);
                cur_state = new_state;
                new_state =
                    Self::add_static_enable_ids_static(fbranch, cur_state);
                new_state
            }
            ir::StaticControl::Repeat(ir::StaticRepeat { body, .. }) => {
                Self::add_static_enable_ids_static(body, cur_state)
            }
        }
    }

    // takes in ir::Control `con`, and adds unique id to every static enable within it.
    // returns u64 `cur_state` that says what the next staticenable should be labeled as.
    pub fn add_static_enable_ids(
        con: &mut ir::Control,
        mut cur_state: u64,
    ) -> u64 {
        match con {
            ir::Control::Enable(_)
            | ir::Control::Invoke(_)
            | ir::Control::Empty(_) => cur_state,
            ir::Control::Par(ir::Par { stmts, .. })
            | ir::Control::Seq(ir::Seq { stmts, .. }) => {
                for stmt in stmts {
                    let new_state =
                        Self::add_static_enable_ids(stmt, cur_state);
                    cur_state = new_state
                }
                cur_state
            }
            ir::Control::If(ir::If {
                tbranch, fbranch, ..
            }) => {
                let mut new_state =
                    Self::add_static_enable_ids(tbranch, cur_state);
                cur_state = new_state;
                new_state = Self::add_static_enable_ids(fbranch, cur_state);
                new_state
            }
            ir::Control::While(ir::While { body, .. })
            | ir::Control::Repeat(ir::Repeat { body, .. }) => {
                Self::add_static_enable_ids(body, cur_state)
            }
            ir::Control::Static(s) => {
                Self::add_static_enable_ids_static(s, cur_state)
            }
        }
    }

    // Gets NODE_ID from StaticEnable se, panics otherwise. Should be used when you know
    // that se has attributes NODE_ID.
    pub fn get_guaranteed_enable_id(se: &ir::StaticEnable) -> u64 {
        se.get_attribute(NODE_ID).unwrap_or_else(||unreachable!(
          "called get_guaranteed_enable_id, meaning we had to be sure it had a NODE_ID attribute"
      ))
    }
}
