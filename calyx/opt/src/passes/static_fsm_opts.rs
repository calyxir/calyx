use crate::traversal::{
    Action, ConstructVisitor, Named, ParseVal, PassOpt, VisResult, Visitor,
};
use calyx_ir::{self as ir, LibrarySignatures};
use calyx_utils::CalyxResult;

/// Adds assignments from a components `clk` port to every
/// component that contains an input `clk` port.
pub struct StaticFSMOpts {
    one_hot_cutoff: u64,
}

impl Named for StaticFSMOpts {
    fn name() -> &'static str {
        "static-fsm-opts"
    }

    fn description() -> &'static str {
        "Inserts attributes to optimize Static FSMs"
    }

    fn opts() -> Vec<PassOpt> {
        vec![PassOpt::new(
            "one-hot-cutoff",
            "The upper limit on the number of states the static FSM must have before we pick binary \
            encoding over one-hot. Defaults to 0 (i.e., always choose binary encoding)",
            ParseVal::Num(0),
            PassOpt::parse_num,
        )]
    }
}

impl ConstructVisitor for StaticFSMOpts {
    fn from(ctx: &ir::Context) -> CalyxResult<Self> {
        let opts = Self::get_opts(ctx);

        Ok(StaticFSMOpts {
            one_hot_cutoff: opts["one-hot-cutoff"].pos_num().unwrap(),
        })
    }

    fn clear_data(&mut self) {}
}

impl Visitor for StaticFSMOpts {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        comp.get_static_groups_mut().iter_mut().for_each(|sgroup| {
            let sgroup_latency = sgroup.borrow().get_latency();
            // If static group's latency is less than the cutoff, encode as a
            // one-hot FSM.
            if sgroup_latency < self.one_hot_cutoff {
                sgroup
                    .borrow_mut()
                    .attributes
                    .insert(ir::BoolAttr::OneHot, 1);
            }
        });

        // we don't need to traverse control
        Ok(Action::Stop)
    }
}
