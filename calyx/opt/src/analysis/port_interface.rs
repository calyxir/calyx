use calyx_ir as ir;
use calyx_utils::{CalyxResult, Error};
use itertools::Itertools;
use std::collections::{HashMap, HashSet};

/// Tuple containing (port, set of ports).
/// When the first port is read from, all of the ports in the set must be written to.
type ReadTogether = (ir::Id, HashSet<ir::Id>);
/// Read together specs map the name of a primitive to its [ReadTogether] specs
type ReadTogetherSpecs = HashMap<ir::Id, Vec<ReadTogether>>;

/// Set of ports that need to be driven together.
type WriteTogether = HashSet<ir::Id>;
// Write together specs map the name of a primitive to the set of ports that need
// to be driven together.
type WriteTogetherSpecs = HashMap<ir::Id, Vec<WriteTogether>>;

/// Helper methods to parse `@read_together` and `@write_together` specifications
pub struct PortInterface;

impl PortInterface {
    /// Construct @write_together specs from the primitive definitions.
    pub fn write_together_specs<'a>(
        primitives: impl Iterator<Item = &'a ir::Primitive>,
    ) -> WriteTogetherSpecs {
        let mut write_together = HashMap::new();
        for prim in primitives {
            let writes: Vec<HashSet<ir::Id>> = prim
                .find_all_with_attr(ir::NumAttr::WriteTogether)
                .map(|pd| {
                    (
                        pd.attributes.get(ir::NumAttr::WriteTogether).unwrap(),
                        pd.name(),
                    )
                })
                .into_group_map()
                .into_values()
                .map(|writes| writes.into_iter().collect::<HashSet<_>>())
                .collect();
            if !writes.is_empty() {
                write_together.insert(prim.name, writes);
            }
        }
        write_together
    }

    /// Construct `@read_together` spec from the definition of a primitive.
    /// Each spec is allowed to have exactly one output port along with one
    /// or more input ports.
    /// The specification dictates that before reading the output port, the
    /// input ports must be driven, i.e., the output port is combinationally
    /// related to the input ports and only those ports.
    pub fn comb_path_spec(
        prim: &ir::Primitive,
    ) -> CalyxResult<Vec<ReadTogether>> {
        prim
                .find_all_with_attr(ir::NumAttr::ReadTogether)
                .map(|pd| (pd.attributes.get(ir::NumAttr::ReadTogether).unwrap(), pd))
                .into_group_map()
                .into_values()
                .map(|ports| {
                    let (outputs, inputs): (Vec<_>, Vec<_>) =
                        ports.into_iter().partition(|&port| {
                            matches!(port.direction, ir::Direction::Output)
                        });
                    // There should only be one port in the read_together specification.
                    if outputs.len() != 1 {
                        return Err(Error::papercut(format!("Invalid @read_together specification for primitive `{}`. Each specification group is only allowed to have one output port specified.", prim.name)))
                    }
                    assert!(outputs.len() == 1);
                    Ok((
                        outputs[0].name(),
                        inputs
                            .into_iter()
                            .map(|port| port.name())
                            .collect::<HashSet<_>>(),
                    ))
                })
                .collect::<CalyxResult<_>>()
    }

    /// Construct @read_together specs from the primitive definitions.
    pub fn comb_path_specs<'a>(
        primitives: impl Iterator<Item = &'a ir::Primitive>,
    ) -> CalyxResult<ReadTogetherSpecs> {
        let mut read_together = HashMap::new();
        for prim in primitives {
            let reads = Self::comb_path_spec(prim)?;
            if !reads.is_empty() {
                read_together.insert(prim.name, reads);
            }
        }
        Ok(read_together)
    }
}
