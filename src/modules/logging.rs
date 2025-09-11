use log::{LevelFilter, error};
use log4rs::append::console::ConsoleAppender;
use log4rs::append::file::FileAppender;
use log4rs::encode::pattern::PatternEncoder;
use log4rs::config::{Appender, Config, Logger, Root};

use std::panic::set_hook;
use chrono::{DateTime, Local};

/// Initializes the logger.
/// 
/// Logs to both stdout and timestamped file.
/// # Panic
/// This function will panic if *log4rs::init_logger()* fails for any reason, or if it fails to build the logger.
/// 
/// This is intended functionality as we never want taskmaster to run if logging is dead.
pub fn init_logging() {
    let time: DateTime<Local> = Local::now();
    let timestamp = time.format("%Y-%m-%d_%H-%M-%S").to_string();

    let stdout = ConsoleAppender::builder().build();
    let stdout_append = Appender::builder().build("stdout", Box::new(stdout));

    let logfile = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{date(%Y-%m-%d %H:%M:%S)} - {file}:{module}:{line} - {highlight({level})} - {message}\n")))
        .build(format!(".logs/taskmaster_{}.log", timestamp))
        .unwrap();
    let logfile_append = Appender::builder().build("logfile", Box::new(logfile));

    let logger = Logger::builder().build("taskmaster::main", LevelFilter::Info);

    let root = Root::builder().appender("stdout").appender("logfile").build(LevelFilter::Info);

    let config = Config::builder()
        .appender(stdout_append)
        .appender(logfile_append)
        .logger(logger)
        .build(root)
        .unwrap();

    let _ = log4rs::init_config(config).unwrap();

    set_hook(Box::new(|panic_info| {
        error!("Taskmaster panicked with: \"{:?}\"", panic_info);
    }));
}