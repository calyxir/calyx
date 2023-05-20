use crate::traversal::{Action, ConstructVisitor, Named, Visitor};
use calyx_ir as ir;
use ir::RRC;
use itertools::Itertools;
use linked_hash_map::LinkedHashMap;
use smallvec::smallvec;
use std::collections::{HashMap, HashSet};

const DEFAULT: u64 = 32;

#[derive(Default)]
/// A pass to detect cells that have been inlined into the top-level component
/// and turn them into real cells marked with [ir::BoolAttr::External].
pub struct DiscoverExternal {
    default: u64,
}

impl Named for DiscoverExternal {
    fn name() -> &'static str {
        "discover-external"
    }

    fn description() -> &'static str {
        "Detect cells that have been inlined into a component's interface and turn them into @external cells"
    }

    fn opts() -> &'static [(&'static str, &'static str)] {
        &[(
            "default",
            "Default width for external cells. Defaults to 32 bits.",
        )]
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

        // Walk over the ports and detect ports that have the same prefixes.
        // Build a map from prefix to ports
        let names = comp
            .signature
            .borrow()
            .ports()
            .iter()
            .map(|p| p.borrow().name)
            .collect::<Vec<_>>();

        // Group ports by longest common prefix
        // NOTE(rachit): This is an awfully inefficient representation. We really
        // want a TrieMap here.
        let mut prefix_map: HashMap<String, HashSet<ir::Id>> = HashMap::new();
        for name in names {
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
        let mut prim_ports: HashMap<ir::Id, HashSet<ir::Id>> = HashMap::new();
        for prim in sigs.signatures() {
            // Ignore clk and reset cells
            let hs = prim
                .signature
                .iter()
                .filter(|p| {
                    !p.attributes.has(ir::BoolAttr::Clk)
                        && !p.attributes.has(ir::BoolAttr::Reset)
                })
                .map(|p| p.name)
                .collect::<HashSet<_>>();
            prim_ports.insert(prim.name, hs);
        }

        // For all prefixes, check if there is a primitive that matches the
        // prefix. If there is, then we have an external cell.
        let mut matching_pre: Vec<(String, ir::Id)> = vec![];
        for (prefix, ports) in prefix_map.iter() {
            for (&prim, prim_ports) in prim_ports.iter() {
                if prim_ports == ports {
                    matching_pre.push((prefix.clone(), prim));
                }
            }
        }

        // Remove all ports that have a matching prefix and collect them into a
        // map from prefixes
        let mut port_map: HashMap<String, Vec<RRC<ir::Port>>> = HashMap::new();
        let mut new_ports = smallvec![];
        'outer: for port in comp.signature.borrow_mut().ports.drain(..) {
            // If this matches a prefix, add it to the corresponding port map
            for (pre, _) in &matching_pre {
                if port.borrow().name.as_ref().starts_with(pre) {
                    port_map.entry(pre.clone()).or_default().push(port.clone());
                    continue 'outer;
                }
            }
            new_ports.push(port);
        }
        comp.signature.borrow_mut().ports = new_ports;

        // Add external cells for all matching prefixes
        for (pre, prim) in matching_pre {
            log::info!("Prefix {} matches primitive {}", pre, prim);
            // Attempt to infer the parameters for the external cell
            let prim_sig = sigs.get_primitive(prim);
            let ports = port_map.remove(&pre).unwrap();
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
                                .ends_with(abs.name.as_ref())
                        })
                        .unwrap_or_else(|| {
                            panic!("No port found for {}", abs.name)
                        });
                    // Update the value of the parameter
                    *params.get_mut(&value).unwrap() =
                        Some(port.borrow().width);
                }
            }

            let param_values = params
                .into_iter()
                .map(|(_, v)| {
                    if let Some(v) = v {
                        v
                    } else {
                        log::warn!("Unable to infer parameter value for {} in {}, defaulting to {}", pre, prim, DEFAULT);
                        DEFAULT
                    }
                })
                .collect_vec();

            let mut builder = ir::Builder::new(comp, sigs);
            let cell = builder.add_primitive(pre, prim, &param_values);
            cell.borrow_mut()
                .attributes
                .insert(ir::BoolAttr::External, 1);
        }

        // Purely structural pass
        Ok(Action::Stop)
    }
}
