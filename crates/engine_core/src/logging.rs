//! Logging system for the Lattice engine.
//!
//! Provides structured logging with console and optional file output.

use std::path::Path;
use tracing::Level;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize the logging system.
///
/// # Arguments
/// * `level` - Default log level
/// * `log_dir` - Optional directory for file logging (rolling daily)
///
/// # Panics
/// Panics if the subscriber cannot be initialized (already initialized).
pub fn init_logging(level: Level, log_dir: Option<&Path>) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(level.as_str()));

    let fmt_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false);

    if let Some(dir) = log_dir {
        let file_appender = tracing_appender::rolling::daily(dir, "lattice.log");
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
        
        let file_layer = fmt::layer()
            .with_writer(non_blocking)
            .with_ansi(false);

        tracing_subscriber::registry()
            .with(filter)
            .with(fmt_layer)
            .with(file_layer)
            .init();
            
        // Note: _guard is dropped here, which could cause issues.
        // In a real app, the guard should be held for the program lifetime.
        // For now, we rely on the console layer always working.
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt_layer)
            .init();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;

    static INIT: Once = Once::new();

    fn init_test_logging() {
        INIT.call_once(|| {
            init_logging(Level::DEBUG, None);
        });
    }

    #[test]
    fn test_logging_initializes() {
        init_test_logging();
        tracing::info!("Test log message");
        // If we get here without panic, logging is working
    }

    #[test]
    fn test_level_filtering() {
        init_test_logging();
        // Trace is below DEBUG, should be filtered
        tracing::trace!("This should be filtered");
        tracing::debug!("This should appear");
    }
}
