//! Define the PassManager structure that is used to construct and run pass
//! passes.
use crate::traversal;
use calyx_ir as ir;
use calyx_utils::{Error, MultiError};
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::time::Instant;

pub type PassResult<T> = std::result::Result<T, MultiError>;

/// Top-level type for all passes that transform an [ir::Context]
pub type PassClosure = Box<dyn Fn(&mut ir::Context) -> PassResult<()>>;

/// Structure that tracks all registered passes for the compiler.
#[derive(Default)]
pub struct PassManager {
    /// All registered passes
    passes: HashMap<String, PassClosure>,
    /// Tracks alias for groups of passes that run together.
    aliases: HashMap<String, Vec<String>>,
    // Track the help information for passes
    help: HashMap<String, String>,
}

impl PassManager {
    /// Register a new Calyx pass and return an error if another pass with the
    /// same name has already been registered.
    ///
    /// ## Example
    /// ```rust
    /// let pm = PassManager::default();
    /// pm.register_pass::<WellFormed>()?;
    /// ```
    pub fn register_pass<Pass>(&mut self) -> PassResult<()>
    where
        Pass:
            traversal::Visitor + traversal::ConstructVisitor + traversal::Named,
    {
        self.register_generic_pass::<Pass>(Box::new(|ir| {
            Pass::do_pass_default(ir)?;
            Ok(())
        }))
    }

    /// Registers a diagnostic pass as a normal pass. If there is an error,
    /// this will report the first error gathered by the pass.
    pub fn register_diagnostic<Pass>(&mut self) -> PassResult<()>
    where
        Pass: traversal::Visitor
            + traversal::ConstructVisitor
            + traversal::Named
            + traversal::DiagnosticPass,
    {
        self.register_generic_pass::<Pass>(Box::new(|ir| {
            let mut visitor = Pass::from(ir)?;
            visitor.do_pass(ir)?;

            let errors: Vec<_> =
                visitor.diagnostics().errors_iter().cloned().collect();
            if !errors.is_empty() {
                Err(MultiError::from(errors))
            } else {
                // only show warnings, if there are no errors
                visitor.diagnostics().warning_iter().for_each(
                    |warning| log::warn!(target: Pass::name(), "{warning:?}"),
                );
                Ok(())
            }
        }))
    }

    fn register_generic_pass<Pass>(
        &mut self,
        pass_closure: PassClosure,
    ) -> PassResult<()>
    where
        Pass:
            traversal::Visitor + traversal::ConstructVisitor + traversal::Named,
    {
        let name = Pass::name().to_string();
        if self.passes.contains_key(&name) {
            return Err(Error::misc(format!(
                "Pass with name '{}' is already registered.",
                name
            ))
            .into());
        }
        self.passes.insert(name.clone(), pass_closure);
        let mut help = format!("- {}: {}", name, Pass::description());
        for opt in Pass::opts() {
            write!(
                &mut help,
                "\n  * {}: {} (default: {})",
                opt.name(),
                opt.description(),
                opt.default()
            )
            .unwrap();
        }
        self.help.insert(name, help);
        Ok(())
    }

    /// Adds a new alias for groups of passes. An alias is a list of strings
    /// that represent valid pass names OR an alias.
    /// The passes and aliases are executed in the order of specification.
    pub fn add_alias(
        &mut self,
        name: String,
        passes: Vec<String>,
    ) -> PassResult<()> {
        if self.aliases.contains_key(&name) {
            return Err(Error::misc(format!(
                "Alias with name '{}'  already registered.",
                name
            ))
            .into());
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

    /// Return the help string for a specific pass.
    pub fn specific_help(&self, pass: &str) -> Option<String> {
        self.help.get(pass).cloned().or_else(|| {
            self.aliases.get(pass).map(|passes| {
                let pass_str = passes
                    .iter()
                    .map(|p| format!("- {p}"))
                    .collect::<Vec<String>>()
                    .join("\n");
                format!("`{pass}' is an alias for pass pipeline:\n{}", pass_str)
            })
        })
    }

    /// Return a string representation to show all available passes and aliases.
    /// Appropriate for help text.
    pub fn complete_help(&self) -> String {
        let mut ret = String::with_capacity(1000);

        // Push all passes.
        let mut pass_names = self.passes.keys().collect::<Vec<_>>();
        pass_names.sort();
        ret.push_str("Passes:\n");
        pass_names.iter().for_each(|&pass| {
            writeln!(ret, "{}", self.help[pass]).unwrap();
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
            writeln!(ret, "- {}: {}", alias, pass_str).unwrap();
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
        insns: &[String],
    ) -> PassResult<(Vec<String>, HashSet<String>)> {
        let mut insertions = insns
            .iter()
            .filter_map(|str| match str.split_once(':') {
                Some((before, after)) => {
                    Some((before.to_string(), after.to_string()))
                }
                None => {
                    log::warn!("No ':' in {str}. Ignoring this option.");
                    None
                }
            })
            .collect::<Vec<_>>();
        // Incls and excls can have aliases in them. Resolve them.
        let mut passes = incls
            .iter()
            .flat_map(|maybe_alias| self.resolve_alias(maybe_alias))
            .collect::<Vec<_>>();

        let excl_set = excls
            .iter()
            .flat_map(|maybe_alias| self.resolve_alias(maybe_alias))
            .collect::<HashSet<String>>();

        // Validate that names of passes in incl and excl sets are known
        passes.iter().chain(excl_set.iter().chain(insertions.iter().flat_map(|(pass1, pass2)| vec![pass1, pass2]))).try_for_each(|pass| {
            if !self.passes.contains_key(pass) {
                Err(Error::misc(format!(
                    "Unknown pass: {pass}. Run compiler with pass-help subcommand to view registered passes."
                )))
            } else {
                Ok(())
            }
        })?;

        // Remove passes from `insertions` that are not slated to run.
        insertions.retain(|(pass1, pass2)|
            if !passes.contains(pass1) || excl_set.contains(pass1) {
                log::warn!("Pass {pass1} is not slated to run. Reordering will have no effect.");
                false
            }
            else if !passes.contains(pass2) || excl_set.contains(pass2) {
                log::warn!("Pass {pass2} is not slated to run. Reordering will have no effect.");
                false
            }
            else {
                true
            }
        );

        // Perform re-insertion.
        // Insert `after` right after `before`. If `after` already appears after
        // before, do nothing.
        for (before, after) in insertions {
            let before_idx =
                passes.iter().position(|pass| *pass == before).unwrap();
            let after_idx =
                passes.iter().position(|pass| *pass == after).unwrap();
            // Only need to perform re-insertion if it is actually out of order.
            if before_idx > after_idx {
                passes.insert(before_idx + 1, after);
                passes.remove(after_idx);
            }
        }

        Ok((passes, excl_set))
    }

    /// Executes a given "plan" constructed using the incl and excl lists.
    /// ord is a relative ordering that should be enforced.
    pub fn execute_plan(
        &self,
        ctx: &mut ir::Context,
        incl: &[String],
        excl: &[String],
        insn: &[String],
        dump_ir: bool,
    ) -> PassResult<()> {
        let (passes, excl_set) = self.create_plan(incl, excl, insn)?;

        for name in passes {
            // Pass is known to exist because create_plan validates the
            // names of passes.
            let pass = &self.passes[&name];

            // Conditional compilation for WASM target because Instant::now
            // is not supported.
            if cfg!(not(target_family = "wasm")) {
                if !excl_set.contains(&name) {
                    let start = Instant::now();
                    pass(ctx)?;
                    if dump_ir {
                        ir::Printer::write_context(
                            ctx,
                            true,
                            &mut std::io::stdout(),
                        )?;
                    }
                    let elapsed = start.elapsed();
                    // Warn if pass takes more than 3 seconds.
                    if elapsed.as_secs() > 5 {
                        log::warn!("{name}: {}ms", elapsed.as_millis());
                    } else {
                        log::info!("{name}: {}ms", start.elapsed().as_millis());
                    }
                } else {
                    log::info!("{name}: Ignored")
                }
            } else if !excl_set.contains(&name) {
                pass(ctx)?;
            }
        }

        Ok(())
    }
}

/// Simple macro to register an alias with a pass manager.
///
/// ## Example
/// ```
/// let pm = PassManager::default();
/// // Register passes WellFormed, Papercut, and Canonicalize.
/// register_alias!(pm, "validate", [WellFormed, Papercut, Canonicalize]);
/// ```
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
