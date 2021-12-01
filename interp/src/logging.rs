use lazy_static::*;

// re-export for convenience
#[allow(unused_imports)]
pub(crate) use slog::{debug, error, info, o, trace, warn, Logger};

use slog::{Drain, Level};

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

pub fn new_sublogger<S: AsRef<str>>(source_name: S) -> Logger {
    ROOT_LOGGER.new(o!("source" => String::from(source_name.as_ref())))
}
