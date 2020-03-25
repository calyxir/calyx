use crate::lang::{
    component::Component, context::Context, structure::NodeData,
};
use crate::passes::visitor::{Action, VisResult, Visitor};

#[derive(Default)]
pub struct ConnectClock {}

impl Visitor for ConnectClock {
    fn name(&self) -> String {
        "Latency Insenstive".to_string()
    }

    fn start(&mut self, comp: &mut Component, _c: &Context) -> VisResult {
        let clk_idx = comp.structure.get_io_index("clk")?;
        let nodes: Vec<_> = comp
            .structure
            .instances()
            .filter_map(|(idx, data)| {
                if let NodeData::Instance { signature, .. } = data {
                    if signature.has_input("clk") {
                        Some(idx)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();
        nodes
            .iter()
            .map(|idx| comp.structure.insert_edge(clk_idx, "clk", *idx, "clk"))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Action::Stop)
    }
}
