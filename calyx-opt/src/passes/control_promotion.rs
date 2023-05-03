use crate::traversal::Named;
use crate::traversal::Visitor;
use crate::traversal::{Action, VisResult};
use calyx_ir as ir;
use ir::Attributes;

/// Expands the static island by aligning the separate static islands in
/// `static seq` and `static par`
/// For example:
/// ```
/// wires {
///  static group g1<1> {...}
///  static group g2<1> {...}
///  static group g3<1> {...}
/// }
///
/// control { seq {g1; g2; g3;} }
/// ```
/// gets turned into
/// ```
/// wires {...}
/// control {static seq {g1; g2; g3;} }
/// ```
///
/// while
///
/// ```
/// wires {
///  static group g1<1> {...}
///  static group g2<1> {...}
///  group g3 {...}
/// }
///
/// control {seq {g1; g2; g3;} }
/// ```
/// gets turned into
/// ```
/// wires {...}
/// control {seq {static seq {g1; g2;} g3;}}
/// ```
/// similarly for `static par`
#[derive(Default)]
pub struct ControlPromotion;

impl Named for ControlPromotion {
    fn name() -> &'static str {
        "control-promotion"
    }

    fn description() -> &'static str {
        "Merge static control statements into bigger static islands"
    }
}

impl Visitor for ControlPromotion {
    fn finish_seq(
        &mut self,
        s: &mut ir::Seq,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let mut stmt_new: Vec<ir::Control> = Vec::new();
        let mut static_seq: Vec<ir::StaticControl> = Vec::new();
        let mut latency = 0;
        let mut all_static = true;
        for stmt in s.stmts.drain(..) {
            if let ir::Control::Static(sc) = stmt {
                // if control is static then store it in static_seq
                latency += sc.get_latency();
                static_seq.push(sc);
            } else {
                if !static_seq.is_empty() {
                    // if static_seq is not empty then create a new `static seq`
                    // to wrap all static control inside `static_seq`
                    let mut new_vec: Vec<ir::StaticControl> = Vec::new();
                    std::mem::swap(&mut static_seq, &mut new_vec);
                    let st_s = ir::StaticSeq {
                        stmts: new_vec,
                        attributes: Attributes::default(),
                        latency: latency,
                    };
                    stmt_new.push(ir::Control::Static(ir::StaticControl::Seq(
                        st_s,
                    )));
                }
                all_static = false;
                latency = 0;
                stmt_new.push(stmt);
            }
        }
        if !static_seq.is_empty() {
            let st_s = ir::StaticSeq {
                stmts: static_seq,
                attributes: Attributes::default(),
                latency: latency,
            };
            let static_island =
                ir::Control::Static(ir::StaticControl::Seq(st_s));
            if all_static {
                return Ok(Action::change(static_island));
            } else {
                stmt_new.push(static_island);
            }
        }

        Ok(Action::change(ir::Control::Seq(ir::Seq {
            stmts: stmt_new,
            attributes: Attributes::default(),
        })))
    }

    fn finish_par(
        &mut self,
        s: &mut ir::Par,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let (s_stmt, d_stmt): (Vec<ir::Control>, Vec<ir::Control>) =
            std::mem::take(&mut s.stmts)
                .into_iter()
                .partition(|s| matches!(s, ir::Control::Static(_)));
        let mut latency = 0;
        let mut static_new: Vec<ir::StaticControl> = Vec::new();
        for stmt in s_stmt.into_iter() {
            let sc = match stmt {
                ir::Control::Static(sc) => sc,
                _ => unreachable!(
                    "all stmts contained in s_stmt should be static"
                ),
            };
            latency = std::cmp::max(latency, sc.get_latency());
            static_new.push(sc);
        }
        let mut stmt_new: Vec<ir::Control> = Vec::new();
        let static_p = ir::StaticPar {
            stmts: static_new,
            attributes: Attributes::default(),
            latency,
        };
        let static_c = ir::Control::Static(ir::StaticControl::Par(static_p));
        if d_stmt.is_empty() {
            return Ok(Action::change(static_c));
        } else {
            stmt_new.push(static_c);
            stmt_new.extend(d_stmt);
            return Ok(Action::change(ir::Control::par(stmt_new)));
        }
    }
}
