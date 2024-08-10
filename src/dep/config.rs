use std::{collections::BTreeMap, fmt};

use serde::{Deserialize, Deserializer, Serialize};

/// 配置文件
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub language: String,
    pub retry_count: u8,
    pub root_dir: String,
    pub replace: bool,
    pub compress: Compress,
    pub pdf: Pdf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            language: "Chinese".to_owned(),
            retry_count: 5u8,
            root_dir: ".".to_owned(),
            replace: false,
            compress: Compress::default(),
            pdf: Pdf::default(),
        }
    }
}

/// 压缩配置
#[derive(Debug, Serialize, Deserialize)]
pub struct Compress {
    pub enable: bool,
    pub password: String,
    pub dir: String,
}

impl Default for Compress {
    fn default() -> Self {
        Self {
            enable: false,
            password: "".to_owned(),
            dir: "cpr".to_owned(),
        }
    }
}

/// PDF配置
#[derive(Debug, Serialize, Deserialize)]
pub struct Pdf {
    pub enable: bool,
    pub dir: String,
}

impl Default for Pdf {
    fn default() -> Self {
        Self {
            enable: false,
            dir: "pdf".to_owned(),
        }
    }
}
