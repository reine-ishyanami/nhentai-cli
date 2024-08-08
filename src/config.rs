use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub log: Log,
    pub language: String,
    pub retry_count: u8,
    pub root_dir: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            log: Log::default(),
            language: "Chinese".to_owned(),
            retry_count: 5u8,
            root_dir: ".".to_owned()
        }
    }
}


#[derive(Debug, Serialize, Deserialize)]
pub struct Log {
    pub level: String
}


impl Default for Log {
    fn default() -> Self {
        Self { level: "info".to_owned() }
    }
}