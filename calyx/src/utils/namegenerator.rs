use crate::ir;
use std::collections::{HashMap, HashSet};

/// Simple HashMap-based name generator that generates new names for each
/// prefix.
/// ```
#[derive(Clone, Debug)]
pub struct NameGenerator {
    name_hash: HashMap<String, i64>,
    generated_names: HashSet<String>,
}

impl NameGenerator {
    /// Create a NameGenerator where `names` are already defined so that this generator
    /// will never generate those names.
    pub fn with_prev_defined_names(names: HashSet<String>) -> Self {
        NameGenerator {
            generated_names: names,
            name_hash: HashMap::default(),
        }
    }

    /// Returns a new String that starts with `prefix`.
    /// For example:
    /// ```
    /// namegen.gen_name("seq");  // Generates "seq0"
    /// namegen.gen_name("seq");  // Generates "seq1"
    /// ```
    pub fn gen_name<S>(&mut self, prefix: S) -> ir::Id
    where
        S: Into<ir::Id> + ToString + Clone,
    {
        let mut cur_prefix: ir::Id = prefix.into();
        loop {
            // Insert default value for this prefix if there is no entry.
            let count = self
                .name_hash
                .entry(cur_prefix.to_string())
                .and_modify(|v| *v += 1)
                .or_insert(-1);

            let name = if *count == -1 {
                cur_prefix.clone().into()
            } else {
                ir::Id::from(cur_prefix.to_string() + &count.to_string())
            };

            // If we've not generated this name before, return it.
            if !self.generated_names.contains(&name.id) {
                self.generated_names.insert(name.to_string());
                return ir::Id::from(name);
            }

            // If the name was generated before, use the current name as the prefix.
            cur_prefix = name;
        }
    }
}
