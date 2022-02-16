use once_cell::sync::OnceCell;

// re-export for convenience
pub use slog::Logger;
#[allow(unused_imports)]
pub(crate) use slog::{debug, error, info, o, trace, warn};

use crate::configuration::Config;
use slog::{Drain, Level};

static ROOT_LOGGER: OnceCell<Logger> = OnceCell::new();

pub fn initialze_logger(config: &Config) {
    let decorator = slog_term::TermDecorator::new().stderr().build();
    let drain = slog_term::FullFormat::new(decorator).build();
    let filter_level = if config.quiet {
        Level::Error
    } else {
        Level::Trace
    };
    let drain = drain.filter_level(filter_level).fuse();

    let drain = slog_async::Async::new(drain).build().fuse();

    let logger = slog::Logger::root(drain, o!());

    ROOT_LOGGER
        .set(logger)
        .expect("Failed to set logger, perhaps it is already initialized");
}

pub fn root() -> &'static Logger {
    ROOT_LOGGER.get().expect("logger not initialized")
}

/// Utility method for creating subloggers for components/primitives/etc. This
/// is the prefered method for getting a logger. Initializes the source key with
/// the supplied name.
pub fn new_sublogger<S: AsRef<str>>(source_name: S) -> Logger {
    root().new(o!("source" => String::from(source_name.as_ref())))
}
