use std::collections::HashMap;
use crate::errors;
use crate::lang::component::Component;
use crate::lang::{
    ast, ast::Control, context::Context, structure::StructureGraph,
};
use crate::passes::visitor::{Action, Named, VisResult, Visitor};

/// Pass that collapses
/// ```
/// (par (enable A B)
///      (enable C D))
///      ..)
/// ```
/// into
/// ```
/// (par (enable A B C D)
///      ..)
/// ```
///
/// when the sub-graphs induced by (enable A B) and (enable C D) have no common
/// edges (i.e. cannot observe each other's computation).
///
/// For example, suppose that this were the structure graph of your component:
/// ```
/// ╭─╮    ╭─╮
/// │A│    │C│
/// ╰┬╯    ╰┬╯
///  │      │
///  │      │
///  v      v
/// ╭─╮    ╭─╮
/// │B│    │D│
/// ╰─╯    ╰─╯
/// ```
/// In this case, the program
/// ```
/// (par (enable A B) (enable C D))
/// ```
/// is equivalent to
/// ```
/// (par (enable A B C D))
/// ```
/// because there are no edges between the sub-graph induced by `A` and `B`
/// and the sub-graph induced by `C` and `D`.
///
/// If instead this were your component graph:
/// ```
/// ╭─╮    ╭─╮
/// │A│───>│C│
/// ╰┬╯    ╰┬╯
///  │      │
///  │      │
///  v      v
/// ╭─╮    ╭─╮
/// │B│    │D│
/// ╰─╯    ╰─╯
/// ```
/// then `par` should be collapsed to:
/// ```
/// ╭─╮   ╭──╮   ╭─╮
/// │A│──>│id│──>│C│
/// ╰┬╯   ╰──╯   ╰┬╯
///  │            │
///  │            │
///  v            v
/// ╭─╮          ╭─╮
/// │B│          │D│
/// ╰─╯          ╰─╯
/// ```
/// which we can represented as 
/// ```
/// (par (enable A B C D))
/// ```
/// and replace the control of `enable A C` with
/// ```
/// (enable A id C)


pub struct EdgeRemove {
    pub edge_clear: HashMap<ast::Id, Vec<ast::Id>>,
}

impl Default for EdgeRemove {
    fn default() -> Self {
        EdgeRemove {
            edge_clear: HashMap::new()
        }
    }
}


impl Named for EdgeRemove {
    fn name() -> &'static str {
        "edge-remove-hashmap"
    }

    fn description() -> &'static str {
        "generate empty hashmap for edge remove"
    }
}
