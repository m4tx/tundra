use chrono::Local;
use lazy_static::lazy_static;
use log::{LevelFilter, SetLoggerError};
use log::{Metadata, Record};
use std::sync::Mutex;

struct Logger;

const MAX_LEVEL_FILTER: LevelFilter = if cfg!(debug_assertions) {
    LevelFilter::Debug
} else {
    LevelFilter::Info
};

impl log::Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= MAX_LEVEL_FILTER.to_level().expect("Invalid level filter")
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let log_line = format!(
                "{} [{}] [{}] {}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.target(),
                record.args()
            );

            println!("{}", &log_line);
            let mut logs = LOGS.lock().expect("Could not lock logs store");
            logs.push(log_line);
        }
    }

    fn flush(&self) {}
}

static LOGGER: Logger = Logger;
lazy_static! {
    static ref LOGS: Mutex<Vec<String>> = Mutex::new(Vec::new());
}

pub fn init_logging() -> Result<(), SetLoggerError> {
    log::set_logger(&LOGGER)?;
    log::set_max_level(MAX_LEVEL_FILTER);

    Ok(())
}

pub fn get_logs() -> &'static Mutex<Vec<String>> {
    &*LOGS
}
