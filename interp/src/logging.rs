use once_cell::sync::OnceCell;

// re-export for convenience
pub use slog::Logger;
#[allow(unused_imports)]
pub(crate) use slog::{debug, error, info, o, trace, warn};

use slog::{Drain, Level};

static ROOT_LOGGER: OnceCell<Logger> = OnceCell::new();

pub fn initialize_default_logger() {
    initialize_logger(true);
}

pub fn initialize_logger(quiet: bool) {
    let decorator = slog_term::TermDecorator::new().stderr().build();
    let drain = slog_term::FullFormat::new(decorator).build();
    let filter_level = if quiet { Level::Error } else { Level::Trace };
    let drain = drain.filter_level(filter_level).fuse();

    let drain = slog_async::Async::new(drain).build().fuse();

    let logger = slog::Logger::root(drain, o!());

    #[allow(unused_must_use)]
    {
        ROOT_LOGGER.set(logger);
    }
}

pub fn root() -> &'static Logger {
    ROOT_LOGGER.get().unwrap_or_else(|| {
        initialize_default_logger();
        ROOT_LOGGER.get().unwrap()
    })
}

/// Utility method for creating subloggers for components/primitives/etc. This
/// is the preferred method for getting a logger. Initializes the source key with
/// the supplied name.
pub fn new_sublogger<S: AsRef<str>>(source_name: S) -> Logger {
    root().new(o!("source" => String::from(source_name.as_ref())))
}
