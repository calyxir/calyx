use std::sync::OnceLock;

use crate::Driver;

/// Fn type representing the redact_arg function required for implementing `argh::FromArgs`
pub type RedactArgFn =
    fn(&[&str], &[&str]) -> Result<Vec<String>, argh::EarlyExit>;

/// Fn type representing the from_arg function required for implementing `argh::FromArgs`
pub type FromArgFn<T> = fn(&[&str], &[&str]) -> Result<T, argh::EarlyExit>;

/// Trait for extending the cli provided by `fud_core::cli`.
///
/// Below is an example of how to use this trait to add a subcommand named `test` to
/// the `fud_core` cli.
///
/// ```rust
/// /// some test command
/// #[derive(FromArgs)]
/// #[argh(subcommand, name = "test")]
/// pub struct TestCommand {
///     /// some arg
///     #[argh(positional)]
///     arg: String,
/// }
///
/// pub enum TestExt {
///     Test(TestCommand)
/// }
///
/// impl CliExt for Fud2CliExt {
///     fn run(&self, driver: &fud_core::Driver) -> anyhow::Result<()> {
///         match &self {
///             Fud2CliExt::Test(cmd) => {
///                 println!("hi there: {}", cmd.arg);
///                 Ok(())
///             }
///         }
///     }
///
///     fn inner_command_info() -> Vec<CommandInfo> {
///         vec![CommandInfo {
///             name: "test",
///             description: "test command",
///         }]
///     }
///
///     fn inner_redact_arg_values() -> Vec<(&'static str, RedactArgFn)> {
///         vec![("test", TestCommand::redact_arg_values)]
///     }
///
///     fn inner_from_args() -> Vec<(&'static str, FromArgFn<Self>)> {
///         vec![("test", |cmd_name, args| {
///             TestCommand::from_args(cmd_name, args).map(Self::Test)
///         })]
///     }
/// }
/// ```
pub trait CliExt: Sized {
    /// Action to execute when this subcommand is provided to the cli
    fn run(&self, driver: &Driver) -> anyhow::Result<()>;

    /// Provides the command names and descriptions for all subcommands in the cli
    /// extension.
    fn inner_command_info() -> Vec<argh::CommandInfo>;

    /// Forward `redact_arg_values` parsing of subcommands to `fud_core::cli` parsing.
    fn inner_redact_arg_values() -> Vec<(&'static str, RedactArgFn)>;

    /// Forward `from_args` parsing of subcommands to `fud_core::cli` parsing.
    fn inner_from_args() -> Vec<(&'static str, FromArgFn<Self>)>;
}

/// Wrapper type over types that implement `CliExt`. This is needed so that we can
/// implement the foreign trait `argh::DynamicSubCommand` on a user provided `CliExt`.
pub struct FakeCli<T: CliExt>(pub T);

impl<T: CliExt> argh::DynamicSubCommand for FakeCli<T> {
    fn commands() -> &'static [&'static argh::CommandInfo] {
        static RET: OnceLock<Vec<&'static argh::CommandInfo>> = OnceLock::new();
        RET.get_or_init(|| {
            T::inner_command_info()
                .into_iter()
                .map(|cmd_info| &*Box::leak(Box::new(cmd_info)))
                .collect()
        })
    }

    fn try_redact_arg_values(
        command_name: &[&str],
        args: &[&str],
    ) -> Option<Result<Vec<String>, argh::EarlyExit>> {
        for (reg_name, f) in T::inner_redact_arg_values() {
            if let Some(&name) = command_name.last() {
                if name == reg_name {
                    return Some(f(command_name, args));
                }
            }
        }
        None
    }

    fn try_from_args(
        command_name: &[&str],
        args: &[&str],
    ) -> Option<Result<Self, argh::EarlyExit>> {
        for (reg_name, f) in T::inner_from_args() {
            if let Some(&name) = command_name.last() {
                if name == reg_name {
                    return Some(f(command_name, args).map(FakeCli));
                }
            }
        }
        None
    }
}

/// The default CliExt used if none is provided. This doesn't define any new commands.
impl CliExt for () {
    fn inner_command_info() -> Vec<argh::CommandInfo> {
        vec![]
    }

    fn inner_redact_arg_values(
    ) -> Vec<(&'static str, crate::cli_ext::RedactArgFn)> {
        vec![]
    }

    fn inner_from_args() -> Vec<(&'static str, crate::cli_ext::FromArgFn<Self>)>
    {
        vec![]
    }

    fn run(&self, _driver: &Driver) -> anyhow::Result<()> {
        Ok(())
    }
}
