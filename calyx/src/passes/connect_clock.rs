use crate::lang::{
    component::Component, context::Context, structure::NodeData,
};
use crate::passes::visitor::{Action, Named, VisResult, Visitor};

/// Inserts wires from the `clk` input of each component
/// to all subcomponents that have a `clk` input port.
#[derive(Default)]
pub struct ConnectClock {}

impl Named for ConnectClock {
    fn name() -> &'static str {
        "connect-clocks"
    }

    fn description() -> &'static str {
        "Connect the top-level clock to subcomponent clocks"
    }
}

impl Visitor for ConnectClock {
    fn start(&mut self, comp: &mut Component, _c: &Context) -> VisResult {
        // let clk_idx = comp.structure.get_io_index("clk")?;
        // let nodes: Vec<_> = comp
        //     .structure
        //     .instances()
        //     .filter_map(|(idx, data)| {
        //         if let NodeData::Instance { signature, .. } = data {
        //             if signature.has_input("clk") {
        //                 Some(idx)
        //             } else {
        //                 None
        //             }
        //         } else {
        //             None
        //         }
        //     })
        //     .collect();
        // nodes
        //     .iter()
        //     .map(|idx| comp.structure.insert_edge(clk_idx, "clk", *idx, "clk"))
        //     .collect::<Result<Vec<_>, _>>()?;

        Ok(Action::Stop)
    }
}
