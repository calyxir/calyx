//! Define the PassManager structure that is used to construct and run pass
//! passes.
use crate::{
    errors::{Error, FutilResult},
    ir,
};
use std::collections::{HashMap, HashSet};

/// Top-level type for all passes that transform an [ir::Context]
pub type PassClosure = Box<dyn Fn(&mut ir::Context) -> FutilResult<()>>;

/// Structure that tracks all registered passes for the compiler.
#[derive(Default)]
pub struct PassManager {
    /// All registered passes
    passes: HashMap<String, PassClosure>,

    /// Tracks alias for groups of passes that run together.
    aliases: HashMap<String, Vec<String>>,
}

impl PassManager {
    /// Registers a new pass with the pass manager. Return `Err` if there is
    /// already a pass with the same name.
    pub fn add_pass(
        &mut self,
        name: String,
        pass_func: PassClosure,
    ) -> FutilResult<()> {
        if self.passes.contains_key(&name) {
            return Err(Error::Misc(format!(
                "Pass with name '{}' is already registered.",
                name
            )));
        }
        self.passes.insert(name, pass_func);
        Ok(())
    }

    /// Adds a new alias for groups of passes. An alias is a list of strings
    /// that represent valid pass names OR an alias.
    /// The passes and aliases are executed in the order of specification.
    pub fn add_alias(
        &mut self,
        name: String,
        passes: Vec<String>,
    ) -> FutilResult<()> {
        if self.aliases.contains_key(&name) {
            return Err(Error::Misc(format!(
                "Alias with name '{}'  already registered.",
                name
            )));
        }
        // Expand any aliases used in defining this alias.
        let all_passes = passes
            .into_iter()
            .flat_map(|pass| {
                if self.aliases.contains_key(&pass) {
                    self.aliases[&pass].clone()
                } else if self.passes.contains_key(&pass) {
                    vec![pass]
                } else {
                    panic!("No pass or alias named: {}", pass)
                }
            })
            .collect();
        self.aliases.insert(name, all_passes);
        Ok(())
    }

    /// Return a string representation to show all available passes and aliases.
    /// Appropriate for help text.
    pub fn show_names(&self) -> String {
        let mut ret = String::with_capacity(100);

        // Push all passes.
        let mut pass_names = self.passes.keys().collect::<Vec<_>>();
        pass_names.sort();
        ret.push_str("Passes:\n");
        pass_names.iter().for_each(|pass| {
            ret.push_str(&format!("- {}", pass));
            ret.push('\n');
        });

        // Push all aliases
        let mut aliases = self.aliases.iter().collect::<Vec<_>>();
        aliases.sort_by(|kv1, kv2| kv1.0.cmp(kv2.0));
        ret.push_str("\nAliases:\n");
        aliases.iter().for_each(|(alias, passes)| {
            let pass_str = passes
                .iter()
                .map(|p| p.to_string())
                .collect::<Vec<String>>()
                .join(", ");
            ret.push_str(&format!("- {}: {}", alias, pass_str));
            ret.push('\n');
        });
        ret
    }

    /// Attempts to resolve the alias name. If there is no alias with this name,
    /// assumes that this is a pass instead.
    fn resolve_alias(&self, maybe_alias: &str) -> Vec<String> {
        self.aliases
            .get(maybe_alias)
            .cloned()
            .unwrap_or_else(|| vec![maybe_alias.to_string()])
    }

    /// Creates a plan using an inclusion and exclusion list which might contain
    /// aliases.
    fn create_plan(
        &self,
        incls: &[String],
        excls: &[String],
    ) -> FutilResult<(Vec<String>, HashSet<String>)> {
        // Incls and excls can both have aliases in them. Resolve them.
        let passes = incls
            .iter()
            .flat_map(|maybe_alias| self.resolve_alias(maybe_alias))
            .collect::<Vec<_>>();

        let excl_set = excls
            .iter()
            .flat_map(|maybe_alias| self.resolve_alias(maybe_alias))
            .collect::<HashSet<String>>();

        // Validate that names of passes in incl and excl sets are known
        passes.iter().chain(excl_set.iter()).try_for_each(|pass| {
            if !self.passes.contains_key(pass) {
                Err(Error::UnknownPass(pass.to_string()))
            } else {
                Ok(())
            }
        })?;

        Ok((passes, excl_set))
    }

    /// Executes a given "plan" constructed using the incl and excl lists.
    pub fn execute_plan(
        &self,
        ctx: &mut ir::Context,
        incl: &[String],
        excl: &[String],
    ) -> FutilResult<()> {
        let (passes, excl_set) = self.create_plan(incl, excl)?;
        for name in passes {
            // Pass is known to exist because create_plan validates the
            // names of passes.
            let pass = &self.passes[&name];
            if !excl_set.contains(&name) {
                pass(ctx)?;
            }
        }

        Ok(())
    }
}

/// Simple macro to register a pass with a pass manager.
#[macro_export]
macro_rules! register_pass {
    ($manager:expr, $pass:ident) => {
        let name = $pass::name().to_string();
        let pass_closure: crate::pass_manager::PassClosure = Box::new(|ir| {
            $pass::do_pass_default(ir)?;
            Ok(())
        });
        $manager.add_pass(name, pass_closure)?;
    };
}

/// Simple macro to register an alias with a pass manager.
#[macro_export]
macro_rules! register_alias {
    (@unwrap_name $pass:ident) => {
        $pass::name().to_string()
    };

    (@unwrap_name $pass:literal) => {
        $pass.to_string()
    };

    ($manager:expr, $alias:literal, [ $($pass:tt),* $(,)? ]) => {
        $manager.add_alias($alias.to_string(), vec![
            $(register_alias!(@unwrap_name $pass)),*
        ])?;
    };
}
