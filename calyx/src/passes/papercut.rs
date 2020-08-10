use crate::errors::Error;
use crate::lang::component::Component;
use crate::lang::{context::Context, structure_iter::NodeType};
use crate::passes::visitor::{Action, Named, VisResult, Visitor};

#[derive(Default)]
pub struct Papercut {}

impl Named for Papercut {
    fn name() -> &'static str {
        "papercut"
    }

    fn description() -> &'static str {
        "Detect various common made mistakes"
    }
}

impl Visitor for Papercut {
    fn start(&mut self, comp: &mut Component, _c: &Context) -> VisResult {
        let st = &comp.structure;
        // For each group, check if there is at least one write to the done
        // signal of that group.
        for maybe_group in st.groups.keys() {
            if let Some(group) = maybe_group {
                let done_writes = st
                    .edge_idx()
                    .with_node(st.get_node_by_name(group)?)
                    .with_node_type(NodeType::Hole)
                    .with_port("done".to_string())
                    .detach()
                    .count();
                if done_writes == 0 {
                    return Err(Error::Papercut(
                        "No writes to done hole".to_string(),
                        group.clone(),
                    ));
                }
            }
        }

        Ok(Action::Continue)
    }
}
