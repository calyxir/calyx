//! Statically infers the number of cycles for groups where the `done`
//! signal relies only on other `done` signals, and then inserts "static"
//! annotations with those inferred values. If there is an existing
//! annotation in a group that differs from an inferred value, this
//! pass will throw an error. If a group's `done` signal relies on signals
//! that are not only `done` signals, this pass will ignore that group.
use std::collections::HashMap;

use crate::analysis::GraphAnalysis;
use crate::errors::Error;
use crate::frontend::library::ast as lib;
use crate::ir;
use crate::ir::traversal::{Action, Named, VisResult, Visitor};

pub struct InferStaticTiming<'a> {
    /// primitive name -> (go signal, done signal, latency)
    prim_latency_data: HashMap<&'a str, (&'a str, &'a str, u64)>,
}

impl Named for InferStaticTiming<'_> {
    fn name() -> &'static str {
        "infer-static-timing"
    }

    fn description() -> &'static str {
        "infers and annotates static timing for groups when possible"
    }
}

impl Default for InferStaticTiming<'_> {
    fn default() -> Self {
        let prim_latency_data = [("std_reg", ("write_en", "done", 1))]
            .iter()
            .cloned()
            .collect();
        InferStaticTiming { prim_latency_data }
    }
}

/// Attempts to infer the number of cycles starting when
/// group[go] is high, and port is high. If inference is
/// not possible, returns None.
fn infer_latency<'a>(
    port: &ir::Port,
    group: &ir::Group,
    analysis: &GraphAnalysis,
    latency_data: &HashMap<&'a str, (&'a str, &'a str, u64)>,
) -> Option<u64> {
    if let ir::PortParent::Cell(cell) = &port.parent {
        match &cell.upgrade().unwrap().borrow().prototype {
            ir::CellType::Primitive { name, .. } => {
                let data = latency_data.get(name.as_ref());
                if let Some((go, done, latency)) = data {
                    if port.name == *done {
                        let go_port: &ir::Port = &group
                            .assignments
                            .iter()
                            .find(|a| {
                                // XXX(rachit): What is this searching for?
                                let a_dst = a.dst.borrow();
                                let a_dst_name = a_dst.name.to_string();
                                let a_prnt_name = a_dst.get_parent_name();
                                let b_prnt_name = port.get_parent_name();
                                a_dst_name == *go && a_prnt_name == b_prnt_name
                            })
                            .unwrap()
                            .dst
                            .borrow();

                        if let Some(write) = analysis.writes_to(go_port).next()
                        {
                            return infer_latency(
                                &write.borrow(),
                                group,
                                analysis,
                                latency_data,
                            )
                            .map(|write_latency| write_latency + latency);
                        }
                    } else if port.name == *go {
                        // Right now, we're just assuming there's 1 write.
                        if let Some(write) = analysis.writes_to(port).next() {
                            return infer_latency(
                                &write.borrow(),
                                group,
                                analysis,
                                latency_data,
                            );
                        }
                    }
                }
            }

            ir::CellType::Constant { .. } => return Some(0),
            ir::CellType::Component { .. } => return None,
            ir::CellType::ThisComponent => return None,
        }
    }
    None
}

impl Visitor<()> for InferStaticTiming<'_> {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _c: &lib::LibrarySignatures,
    ) -> VisResult<()> {
        let analysis = GraphAnalysis::from(&*comp);

        let mut latency_result: Option<u64> = None;
        for group in &comp.groups {
            for asgn in &group.borrow().assignments {
                let asgn_dst = asgn.dst.borrow();
                let asgn_src = asgn.src.borrow();
                if asgn_dst.name == "done"
                    && asgn_dst.get_parent_name() == group.borrow().name
                {
                    if let Some(latency) = infer_latency(
                        &asgn_src,
                        &group.borrow(),
                        &analysis,
                        &self.prim_latency_data,
                    ) {
                        let grp = group.borrow();
                        if let Some(curr_lat) = grp.attributes.get("static") {
                            if *curr_lat != latency {
                                return Err(
                                    Error::ImpossibleLatencyAnnotation(
                                        grp.name.to_string(),
                                        *curr_lat,
                                        latency,
                                    ),
                                );
                            }
                        }
                        latency_result = Some(latency);
                    } else {
                        latency_result = None;
                    }
                }
            }

            match latency_result {
                Some(res) => {
                    group
                        .borrow_mut()
                        .attributes
                        .insert("static".to_string(), res);
                }
                None => continue,
            }
        }
        Ok(Action::stop_default())
    }
}
