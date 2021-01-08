use crate::ir;
use std::collections::HashMap;

/// Simple HashMap-based name generator that generates new names for each
/// prefix.
/// **Note**: The name generator is *not* hygienic!
/// For example:
/// ```
/// namegen.gen_name("seq");  // Generates "seq0"
/// ... // 8 more calls.
/// namegen.gen_name("seq");  // Generates "seq10"
/// namegen.gen_name("seq1"); // CONFLICT! Generates "seq10".
/// ```
#[derive(Clone, Debug, Default)]
pub struct NameGenerator {
    name_hash: HashMap<String, i64>,
}

impl NameGenerator {
    /// Returns a new String that starts with `prefix`.
    /// For example:
    /// ```
    /// namegen.gen_name("seq");  // Generates "seq0"
    /// namegen.gen_name("seq");  // Generates "seq1"
    /// ```
    pub fn gen_name<S>(&mut self, prefix: S) -> ir::Id
    where
        S: Into<ir::Id> + ToString,
    {
        // Insert default value for this prefix if there is no entry.
        let count = self
            .name_hash
            .entry(prefix.to_string())
            .and_modify(|v| *v += 1)
            .or_insert(-1);

        // If the count is -1, don't create a suffix
        if *count == -1 {
            prefix.into()
        } else {
            ir::Id::from(prefix.to_string() + &count.to_string())
        }
    }
}

