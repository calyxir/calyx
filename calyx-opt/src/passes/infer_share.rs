use crate::analysis::{DominatorMap, ShareSet};
use crate::traversal::{
    Action, ConstructVisitor, Named, Order, ParseVal, PassOpt, VisResult,
    Visitor,
};
use calyx_ir as ir;
use calyx_utils::{CalyxResult, OutputFile};

/// This pass checks if components are (state) shareable. Here is the process it
/// goes through: if a component uses any ref cells, or non-shareable cells then it
/// is automatically not shareable. Otherwise, check if each read of a stateful
/// cell is guaranteed to be dominated by a write to the same cell-- we check this
/// by building a domination map. If so, component is state shareable.
pub struct InferShare {
    print_dmap: Option<OutputFile>,
    print_static_analysis: Option<OutputFile>,
    state_shareable: ShareSet,
    shareable: ShareSet,
    //name of main (so we can skip it)
    main: ir::Id,
}

impl Named for InferShare {
    fn name() -> &'static str {
        "infer-share"
    }

    fn description() -> &'static str {
        "Infer User Defined Components as Shareable"
    }

    fn opts() -> Vec<PassOpt> {
        vec![
            PassOpt::new(
                "print-dmap",
                "Print the domination map",
                ParseVal::OutStream(OutputFile::Null),
                PassOpt::parse_outstream,
            ),
            PassOpt::new(
                "print-static-analysis",
                "Prints the domination analysis for static dmaps",
                ParseVal::OutStream(OutputFile::Null),
                PassOpt::parse_outstream,
            ),
        ]
    }
}

impl ConstructVisitor for InferShare {
    fn from(ctx: &ir::Context) -> CalyxResult<Self>
    where
        Self: Sized + Named,
    {
        let opts = Self::get_opts(ctx);

        let state_shareable = ShareSet::from_context::<true>(ctx);
        let shareable = ShareSet::from_context::<false>(ctx);

        Ok(InferShare {
            print_dmap: opts[&"print-dmap"].not_null_outstream(),
            print_static_analysis: opts[&"print-static-analysis"]
                .not_null_outstream(),
            state_shareable,
            shareable,
            main: ctx.entrypoint,
        })
    }

    fn clear_data(&mut self) {}
}

impl Visitor for InferShare {
    fn iteration_order() -> Order {
        Order::Post
    }
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        //if the component is main, then we can stop checking
        if comp.name == self.main {
            return Ok(Action::Stop);
        }

        // closure to determine if cell is type ThisComponent or Constant
        let const_or_this = |cell: &ir::RRC<ir::Cell>| -> bool {
            matches!(
                cell.borrow().prototype,
                ir::CellType::ThisComponent | ir::CellType::Constant { .. }
            )
        };

        // returns true if cell is shareble, state_shareable, Constant, or This component
        let type_is_shareable = |cell: &ir::RRC<ir::Cell>| -> bool {
            const_or_this(cell)
                || self.shareable.is_shareable_component(cell)
                || self.state_shareable.is_shareable_component(cell)
        };

        // cannot contain any external cells, or any cells of a "non-shareable" type
        // (i.e. not shareable, state_shareable, const or This component)
        if comp.cells.iter().any(|cell| {
            !type_is_shareable(cell) && !cell.borrow().is_reference()
        }) {
            return Ok(Action::Stop);
        }

        // build the domination map
        let mut dmap =
            DominatorMap::new(&mut comp.control.borrow_mut(), comp.name);

        // print the domination map if command line argument says so
        if let Some(s) = &self.print_dmap {
            write!(s.get_write(), "{dmap:?}").unwrap();
        }
        if let Some(s) = &self.print_static_analysis {
            write!(s.get_write(), "{:?}", dmap.static_par_domination).unwrap();
        }

        for (node, dominators) in dmap.map.iter_mut() {
            //get the reads
            let reads =
                DominatorMap::get_node_reads(node, comp, &self.state_shareable);

            //if read and write occur in same group/invoke, then we cannot label it
            //shareable. So we remove node from its dominators
            dominators.remove(node);
            for cell_name in reads {
                if !DominatorMap::key_written_guaranteed(
                    cell_name, dominators, comp,
                ) {
                    return Ok(Action::Stop);
                }
            }
        }
        comp.attributes.insert(ir::BoolAttr::StateShare, 1);
        self.state_shareable.add(comp.name);
        Ok(Action::Stop)
    }
}
