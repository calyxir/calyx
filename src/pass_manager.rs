use calyx::{
    errors::{Error, FutilResult},
    ir,
};
use std::collections::{HashMap, HashSet};

pub type PassClosure = Box<dyn Fn(&mut ir::Context) -> FutilResult<()>>;

/// Structure that tracks all registered passes for the compiler.
pub struct PassManager {
    /// All registered passes
    passes: HashMap<String, PassClosure>,

    /// Tracks alias for groups of passes that run together.
    aliases: HashMap<String, Vec<String>>,
}

impl PassManager {
    pub fn new() -> Self {
        PassManager {
            passes: HashMap::new(),
            aliases: HashMap::new(),
        }
    }
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
    /// that represent valid pass names to be executed for the alias. The
    /// order of execution of passes is the same as the order of specification.
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
        self.aliases.insert(name, passes);
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

    /// Creates a plan using an inclusion and exclusion list which might contain
    /// aliases.
    fn create_plan(
        &self,
        incls: &[String],
        excls: &[String],
    ) -> (Vec<String>, HashSet<String>) {
        // Incls and excls can both have aliases in them. Resolve them.
        let passes = incls
            .iter()
            .flat_map(|maybe_alias| {
                self.aliases
                    .get(maybe_alias)
                    .cloned()
                    .unwrap_or_else(|| vec![maybe_alias.clone()])
            })
            .collect::<Vec<_>>();

        let excl_set = excls
            .iter()
            .flat_map(|maybe_alias| {
                self.aliases
                    .get(maybe_alias)
                    .cloned()
                    .unwrap_or_else(|| vec![maybe_alias.clone()])
            })
            .collect::<HashSet<String>>();

        (passes, excl_set)
    }

    /// Executes a given "plan" constructed using the incl and excl lists.
    pub fn execute_plan(
        &self,
        ctx: &mut ir::Context,
        incl: &[String],
        excl: &[String],
    ) -> FutilResult<()> {
        let (passes, excl_set) = self.create_plan(incl, excl);
        for name in passes {
            if let Some(pass) = self.passes.get(&name) {
                if !excl_set.contains(&name) {
                    pass(ctx)?;
                }
            } else {
                return Err(Error::UnknownPass(
                    name.to_string(),
                    self.show_names(),
                ));
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
    ($manager:expr, $alias:literal, [ $($pass:ident),* $(,)? ]) => {
        $manager.add_alias($alias.to_string(), vec![
            $($pass::name().to_string()),*
        ])?;
    };
}
