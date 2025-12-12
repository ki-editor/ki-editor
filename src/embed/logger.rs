use log::{LevelFilter, Log, Metadata, Record};
use std::io::{self, Write};

/// Forwards log messages to host app
pub(crate) struct HostLogger {
    min_level: LevelFilter,
}

impl HostLogger {
    pub(crate) fn new(min_level: LevelFilter) -> Self {
        Self { min_level }
    }
}

impl Log for HostLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.min_level
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let level_str = match record.level() {
                log::Level::Error => "error",
                log::Level::Warn => "warn",
                log::Level::Info => "info",
                log::Level::Debug => "debug",
                log::Level::Trace => "trace",
            };

            let message = format!("{}", record.args());

            // Just output to stderr for debugging
            // This appears in the Host debug console already
            let _ = writeln!(io::stderr(), "[{level_str}] {message}");
        }
    }

    fn flush(&self) {
        // Nothing to do, messages are flushed immediately
    }
}
