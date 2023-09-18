use crate::traversal::{Action, ConstructVisitor, Named, Visitor};
use calyx_ir as ir;
use calyx_utils::CalyxResult;
use ir::RRC;
use itertools::Itertools;
use linked_hash_map::LinkedHashMap;
use std::collections::{HashMap, HashSet};

/// A pass to detect cells that have been inlined into the top-level component
/// and turn them into real cells marked with [ir::BoolAttr::External].
pub struct DiscoverExternal {
    /// The default value used for parameters that cannot be inferred.
    default: u64,
    /// The suffix to be remove from the inferred names
    suffix: Option<String>,
}

impl Named for DiscoverExternal {
    fn name() -> &'static str {
        "discover-external"
    }

    fn description() -> &'static str {
        "Detect cells that have been inlined into a component's interface and turn them into @external cells"
    }
}

impl ConstructVisitor for DiscoverExternal {
    fn from(ctx: &ir::Context) -> CalyxResult<Self>
    where
        Self: Sized,
    {
        // Manual parsing because our options are not flags
        let n = Self::name();
        let given_opts: HashSet<_> = ctx
            .extra_opts
            .iter()
            .filter_map(|opt| {
                let mut splits = opt.split(':');
                if splits.next() == Some(n) {
                    splits.next()
                } else {
                    None
                }
            })
            .collect();

        let mut default = None;
        let mut suffix = None;
        for opt in given_opts {
            let mut splits = opt.split('=');
            let spl = splits.next();
            // Search for the "default=<n>" option
            if spl == Some("default") {
                let Some(val) = splits.next().and_then(|v| v.parse().ok())
                else {
                    log::warn!("Failed to parse default value. Please specify using -x {}:default=<n>", n);
                    continue;
                };
                log::info!("Setting default value to {}", val);

                default = Some(val);
            }
            // Search for "strip-suffix=<str>" option
            else if spl == Some("strip-suffix") {
                let Some(suff) = splits.next() else {
                    log::warn!("Failed to parse suffix. Please specify using -x {}:strip-suffix=<str>", n);
                    continue;
                };
                log::info!("Setting suffix to {}", suff);

                suffix = Some(suff.to_string());
            }
        }

        Ok(Self {
            default: default.unwrap_or(32),
            suffix,
        })
    }

    fn clear_data(&mut self) {
        /* All data is shared */
    }
}

impl Visitor for DiscoverExternal {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> crate::traversal::VisResult {
        // Ignore non-toplevel components
        if !comp.attributes.has(ir::BoolAttr::TopLevel) {
            return Ok(Action::Stop);
        }

        // Group ports by longest common prefix
        // NOTE(rachit): This is an awfully inefficient representation. We really
        // want a TrieMap here.
        let mut prefix_map: LinkedHashMap<String, HashSet<ir::Id>> =
            LinkedHashMap::new();
        for port in comp.signature.borrow().ports() {
            let name = port.borrow().name;
            let mut prefix = String::new();
            // Walk over the port name and add it to the prefix map
            for c in name.as_ref().chars() {
                prefix.push(c);
                if prefix == name.as_ref() {
                    // We have reached the end of the name
                    break;
                }
                // Remove prefix from name
                let name = name.as_ref().strip_prefix(&prefix).unwrap();
                prefix_map
                    .entry(prefix.clone())
                    .or_default()
                    .insert(name.into());
            }
        }

        // For all cells in the library, build a set of port names.
        let mut prim_ports: LinkedHashMap<ir::Id, HashSet<ir::Id>> =
            LinkedHashMap::new();
        for prim in sigs.signatures() {
            let hs = prim
                .signature
                .iter()
                .filter(|p| {
                    // Ignore clk and reset cells
                    !p.attributes.has(ir::BoolAttr::Clk)
                        && !p.attributes.has(ir::BoolAttr::Reset)
                })
                .map(|p| p.name())
                .collect::<HashSet<_>>();
            prim_ports.insert(prim.name, hs);
        }

        // For all prefixes, check if there is a primitive that matches the
        // prefix. If there is, then we have an external cell.
        let mut pre_to_prim: LinkedHashMap<String, ir::Id> =
            LinkedHashMap::new();
        for (prefix, ports) in prefix_map.iter() {
            for (&prim, prim_ports) in prim_ports.iter() {
                if prim_ports == ports {
                    pre_to_prim.insert(prefix.clone(), prim);
                }
            }
        }

        // Collect all ports associated with a specific prefix
        let mut port_map: LinkedHashMap<String, Vec<RRC<ir::Port>>> =
            LinkedHashMap::new();
        'outer: for port in &comp.signature.borrow().ports {
            // If this matches a prefix, add it to the corresponding port map
            for pre in pre_to_prim.keys() {
                if port.borrow().name.as_ref().starts_with(pre) {
                    port_map.entry(pre.clone()).or_default().push(port.clone());
                    continue 'outer;
                }
            }
        }

        // Add external cells for all matching prefixes
        let mut pre_to_cells = LinkedHashMap::new();
        for (pre, &prim) in &pre_to_prim {
            log::info!("Prefix {} matches primitive {}", pre, prim);
            // Attempt to infer the parameters for the external cell
            let prim_sig = sigs.get_primitive(prim);
            let ports = &port_map[pre];
            let mut params: LinkedHashMap<_, Option<u64>> = prim_sig
                .params
                .clone()
                .into_iter()
                .map(|p| (p, None))
                .collect();

            // Walk over the abstract port definition and attempt to match the bitwidths
            for abs in &prim_sig.signature {
                if let ir::Width::Param { value } = abs.width {
                    // Find the corresponding port
                    let port = ports
                        .iter()
                        .find(|p| {
                            p.borrow()
                                .name
                                .as_ref()
                                .ends_with(abs.name().as_ref())
                        })
                        .unwrap_or_else(|| {
                            panic!("No port found for {}", abs.name())
                        });
                    // Update the value of the parameter
                    let v = params.get_mut(&value).unwrap();
                    if let Some(v) = v {
                        if *v != port.borrow().width {
                            log::warn!(
                                "Mismatched bitwidths for {} in {}, defaulting to {}",
                                pre,
                                prim,
                                self.default
                            );
                            *v = self.default;
                        }
                    } else {
                        *v = Some(port.borrow().width);
                    }
                }
            }

            let param_values = params
                .into_iter()
                .map(|(_, v)| {
                    if let Some(v) = v {
                        v
                    } else {
                        log::warn!(
                            "Unable to infer parameter value for {} in {}, defaulting to {}",
                            pre,
                            prim,
                            self.default
                        );
                        self.default
                    }
                })
                .collect_vec();

            let mut builder = ir::Builder::new(comp, sigs);
            // Remove the suffix from the cell name
            let name = if let Some(suf) = &self.suffix {
                pre.strip_suffix(suf).unwrap_or(pre)
            } else {
                pre
            };
            let cell = builder.add_primitive(name, prim, &param_values);
            cell.borrow_mut()
                .attributes
                .insert(ir::BoolAttr::External, 1);
            pre_to_cells.insert(pre.clone(), cell);
        }

        // Rewrite the ports mentioned in the component signature and remove them
        let mut rewrites: ir::rewriter::PortRewriteMap = HashMap::new();
        for (pre, ports) in port_map {
            // let prim = sigs.get_primitive(pre_to_prim[&pre]);
            let cr = pre_to_cells[&pre].clone();
            let cell = cr.borrow();
            let cell_ports = cell.ports();
            // Iterate over ports with the same names.
            for pr in ports {
                let port = pr.borrow();
                let cp = cell_ports
                    .iter()
                    .find(|p| {
                        port.name.as_ref().ends_with(p.borrow().name.as_ref())
                    })
                    .unwrap_or_else(|| {
                        panic!("No port found for {}", port.name)
                    });
                rewrites.insert(port.canonical(), cp.clone());
            }
        }

        comp.for_each_assignment(|assign| {
            assign.for_each_port(|port| {
                rewrites.get(&port.borrow().canonical()).cloned()
            })
        });
        comp.for_each_static_assignment(|assign| {
            assign.for_each_port(|port| {
                rewrites.get(&port.borrow().canonical()).cloned()
            })
        });

        // Remove all ports from the signature that match a prefix
        comp.signature.borrow_mut().ports.retain(|p| {
            !pre_to_prim
                .keys()
                .any(|pre| p.borrow().name.as_ref().starts_with(pre))
        });

        // Purely structural pass
        Ok(Action::Stop)
    }
}
