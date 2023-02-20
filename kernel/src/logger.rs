use log::{Log, Metadata};

use crate::println;

pub static LOGGER: Logger = Logger::new();

#[cfg(debug_assertions)]
macro_rules! fmt_record {
    ($record:ident) => {
        format_args!(
            "[{}\t{}\t{}:{}]\t{}",
            crate::time::boot_elapsed(),
            $record.level(),
            $record.file().unwrap(),
            $record.line().unwrap(),
            $record.args(),
        )
    };
}

#[cfg(not(debug_assertions))]
macro_rules! fmt_record {
    ($record:ident) => {
        format_args!(
            "[{}\t{}]\t{}",
            $record.level(),
            $record.target(),
            $record.args()
        )
    };
}

pub struct Logger {
    _private: (),
}
impl Logger {
    const fn new() -> Self {
        Self { _private: () }
    }
}

impl Log for Logger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        println!("{}", fmt_record!(record));
    }

    fn flush(&self) {}
}

unsafe impl Send for Logger {}
unsafe impl Sync for Logger {}
