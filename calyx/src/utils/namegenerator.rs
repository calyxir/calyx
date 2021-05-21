use crate::ir;
use std::collections::{HashMap, HashSet};

/// Simple HashMap-based name generator that generates new names for each
/// prefix.
/// ```
#[derive(Clone, Debug, Default)]
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
            ..NameGenerator::default()
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
        // Insert default value for this prefix if there is no entry.
        let count = self
            .name_hash
            .entry(prefix.to_string())
            .and_modify(|v| *v += 1)
            .or_insert(-1);

        // If the count is -1, don't create a suffix
        let name = if *count == -1 {
            prefix.clone().into()
        } else {
            ir::Id::from(prefix.to_string() + &count.to_string())
        };

        // check to see if we've generated this name before, if we have, generate a new one
        if self.generated_names.contains(&name.id) {
            self.gen_name(prefix)
        } else {
            self.generated_names.insert(name.to_string());
            name
        }
    }
}
