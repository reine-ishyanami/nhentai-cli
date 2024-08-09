use std::{collections::BTreeMap, fmt};

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub log: Log,
    pub language: String,
    pub retry_count: u8,
    pub root_dir: String,
    pub replace: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            log: Log::default(),
            language: "Chinese".to_owned(),
            retry_count: 5u8,
            root_dir: ".".to_owned(),
            replace: false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Log {
    pub level: LogLevelMap,
}
impl Default for Log {
    fn default() -> Self {
        Self {
            level: LogLevelMap::default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LogLevelMap(BTreeMap<String, LogLevel>);

impl Default for LogLevelMap {
    fn default() -> Self {
        let mut map = BTreeMap::new();
        map.insert("nhentai_rs".to_owned(), LogLevel::Info);
        Self(map)
    }
}

impl fmt::Display for LogLevelMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut pairs: Vec<String> = Vec::new();
        for (key, value) in &self.0 {
            pairs.push(format!("{}={}", key, value));
        }
        write!(f, "{}", pairs.join(","))
    }
}

#[derive(Debug, Serialize, Deserialize)]
enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Trace => write!(f, "trace"),
            LogLevel::Debug => write!(f, "debug"),
            LogLevel::Info => write!(f, "info"),
            LogLevel::Warn => write!(f, "warn"),
            LogLevel::Error => write!(f, "error"),
        }
    }
}
