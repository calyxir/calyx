use crate::{config::Config, error::LocalResult};
use semver::Version;
use tempdir::TempDir;

pub trait Plugin: Send + Sync {
    /// A unique name for this plugin.
    fn name(&self) -> &'static str;

    /// The version of tb this plugin was built for.
    fn version(&self) -> Version;

    /// Declares the configuration for this plugin.
    fn setup(&self, config: &mut Config) -> LocalResult<()>;

    /// Runs this plugin's testbench.
    /// - `input` is a relative path to the input file in `work_dir`.
    /// - `tests` are a relative paths to the testing harnesses in `work_dir`.
    fn run(
        &self,
        input: String,
        tests: &[String],
        work_dir: TempDir,
        config: &Config,
    ) -> LocalResult<()>;
}

pub type PluginRef = Box<dyn Plugin>;

// https://www.michaelfbryan.com/rust-ffi-guide/dynamic_loading.html
pub type PluginCreate = unsafe fn() -> *mut dyn Plugin;

/// `declare_plugin!(MyPlugin, MyPlugin::constructor)` exposes `MyPlugin` to the
/// world as constructable by the zero-arity `MyPlugin::constructor`.
#[macro_export]
macro_rules! declare_plugin {
    ($plugin_type:ty, $constructor:path) => {
        #[no_mangle]
        pub extern "C" fn _plugin_create() -> *mut dyn $crate::plugin::Plugin {
            let boxed: $crate::plugin::PluginRef = Box::new($constructor());
            Box::into_raw(boxed)
        }
    };
}
