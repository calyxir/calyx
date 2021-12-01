use lazy_static::*;

// re-export for convenience
pub use slog::{debug, error, info, trace, warn};
use slog::{o, Drain, Level, Logger};

lazy_static! {
    /// Global root logger. Note should be initialized after SETTINGS
    pub static ref ROOT_LOGGER: Logger = {
        let decorator = slog_term::TermDecorator::new().stderr().build();
        let drain = slog_term::FullFormat::new(decorator).build();
        let filter_level = if crate::configuration::SETTINGS.read().unwrap().quiet {
            Level::Error
        } else {
            Level::Trace
        };
        let drain = drain.filter_level(filter_level).fuse();

        let drain = slog_async::Async::new(drain).build().fuse();

        slog::Logger::root(drain, o!())
    };
}
