//! Port of `src/utils/logger.ts`.
//!
//! The TS logger writes through the `debug` package and is enabled only when the `DEBUG`
//! environment variable selects the `composition` namespace. The port is a no-op logger with
//! the same API and call sites (see `crates/PORTING.md`): `is_enabled` is always false, and the
//! closures that build messages are never called.

/// `LoggerContext`: shared indentation and timing state of a family of loggers.
#[derive(Clone, Debug, Default)]
pub struct LoggerContext {}

impl LoggerContext {
    pub fn new() -> Self {
        LoggerContext::default()
    }

    pub fn down(&mut self, _time: f64) {}

    pub fn up(&mut self, _time: f64) -> Option<f64> {
        None
    }

    pub fn get_time(&mut self) -> f64 {
        0.0
    }
}

/// A message: a string, or a closure building it lazily (`string | (() => string)`).
pub trait LogMessage {
    fn into_message(self) -> String;
}

impl LogMessage for &str {
    fn into_message(self) -> String {
        self.to_string()
    }
}

impl LogMessage for String {
    fn into_message(self) -> String {
        self
    }
}

impl<F: FnOnce() -> String> LogMessage for F {
    fn into_message(self) -> String {
        self()
    }
}

/// `Logger`. Cheap to clone; `create` makes a child logger as in TS.
#[derive(Clone, Debug)]
pub struct Logger {
    pub is_enabled: bool,
}

impl Logger {
    /// `new Logger(id, context)`.
    pub fn new(_id: &str, _context: &mut LoggerContext) -> Self {
        Logger { is_enabled: false }
    }

    /// `log(msg, prefix = "- ")`. No-op: `msg` is not evaluated.
    pub fn log(&self, _msg: impl LogMessage) {}

    /// `group(msg)`. No-op.
    pub fn group(&self, _msg: impl LogMessage) {}

    /// `groupEnd(msg?)`. No-op.
    pub fn group_end(&self, _msg: impl LogMessage) {}

    /// `create(id)`: a logger sharing this logger's context.
    pub fn create(&self, _id: &str) -> Logger {
        Logger { is_enabled: false }
    }
}
