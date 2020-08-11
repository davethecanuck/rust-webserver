use std::path::PathBuf;
use flexi_logger;

#[derive(Debug)]
pub struct AppLogConfig {
    pub path: Option<PathBuf>,
    pub level: log::Level,
}

impl AppLogConfig {
    pub fn new(verbosity: String, path: Option<PathBuf>) -> AppLogConfig {
        let level = match verbosity.to_lowercase().as_str() {
            "error" => log::Level::Error,
            "warn" => log::Level::Warn,
            "info" => log::Level::Info,
            "debug" => log::Level::Debug,
            "trace" => log::Level::Trace,
            _ => log::Level::Info,
        };
        AppLogConfig { path, level }
    }

    // Initializing with flexi_logger
    pub fn init_flexi_logger(&self) {
        let level = self.level.to_string().to_lowercase();
        match &self.path {
            None => {
               flexi_logger::Logger::with_str(level)
                  .format(flexi_logger::colored_opt_format)
                  .start()
                  .unwrap();
            },
            Some(path) => {
                // Log to the given path as a directory
                flexi_logger::Logger::with_str(level)
                    .format(flexi_logger::colored_opt_format)
                    .log_to_file()
                    .directory(path)
                    .start()
                    .unwrap();
            },
        };
    }
}

#[test]
fn test_level() {
    // Test for upper and lower case 
    let inputs = vec![
        ("error", log::Level::Error),
        ("ERROR", log::Level::Error),
        ("warn", log::Level::Warn),
        ("WARN", log::Level::Warn),
        ("info", log::Level::Info),
        ("INFO", log::Level::Info),
        ("debug", log::Level::Debug),
        ("DEBUG", log::Level::Debug),
        ("trace", log::Level::Trace),
        ("TRACE", log::Level::Trace),
        ("INVALID_USE_INPUT", log::Level::Info),
    ];

    for (level_str, level) in inputs {
        let config = AppLogConfig::new(String::from(level_str), None);
        assert_eq!(config.level, level);
    }
}

#[test]
fn test_path() {
    let filename = "foo.log";
    let path = PathBuf::from(filename);
    let config = AppLogConfig::new(String::from("error"), Some(path));
    assert_eq!(config.path.unwrap(), PathBuf::from(filename))
}
