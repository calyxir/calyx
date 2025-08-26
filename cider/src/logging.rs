// re-export for convenience
pub use slog::Logger;
#[allow(unused_imports)]
pub use slog::{debug, error, info, o, trace, warn};

use slog::{Drain, Level};

use crate::configuration::LoggingConfig;

pub fn initialize_logger(conf: LoggingConfig) -> Logger {
    let decorator = slog_term::TermDecorator::new().stderr();
    let decorator = match conf.color_config {
        crate::configuration::ColorConfig::On => {
            decorator.force_color().build()
        }
        crate::configuration::ColorConfig::Off => {
            decorator.force_plain().build()
        }
        crate::configuration::ColorConfig::Auto => decorator.build(),
    };

    let drain = slog_term::FullFormat::new(decorator).build();
    let filter_level = if conf.quiet && !conf.debug_logging {
        Level::Error
    } else {
        Level::Trace
    };
    let drain = drain.filter_level(filter_level).fuse();

    // TODO griffin: make this configurable
    let drain = slog_async::Async::new(drain)
        .chan_size(1024)
        .overflow_strategy(slog_async::OverflowStrategy::Block)
        .build()
        .fuse();

    let logger = slog::Logger::root(drain, o!());

    if conf.quiet && conf.debug_logging {
        warn!(
            logger,
            "Quiet mode ignored because debug logging is enabled"
        )
    }
    logger
}
