use std::{collections::BTreeMap, fmt};

use serde::{Deserialize, Deserializer, Serialize};

/// 配置文件
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub log: LogConfig,
    pub language: Language,
    #[serde(rename = "retry-count")]
    pub retry_count: u8,
    #[serde(rename = "root-dir")]
    pub root_dir: String,
    pub replace: bool,
    pub interaction: bool,
    pub compress: CompressConfig,
    pub pdf: PdfConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            log: LogConfig::default(),
            language: Language::default(),
            retry_count: 5u8,
            root_dir: ".".to_owned(),
            replace: false,
            interaction: false,
            compress: CompressConfig::default(),
            pdf: PdfConfig::default(),
        }
    }
}

/// 模块日志等级配置
#[derive(Debug, Serialize, Deserialize)]
pub struct LogConfig {
    pub level: LogLevelMap,
}
impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: LogLevelMap::default(),
        }
    }
}

/// 模块日志等级映射
#[derive(Debug, Serialize, Deserialize)]
pub struct LogLevelMap(BTreeMap<String, LogLevel>);

impl Default for LogLevelMap {
    fn default() -> Self {
        let mut map = BTreeMap::new();
        map.insert("nhentai".to_owned(), LogLevel::Info);
        Self(map)
    }
}

impl ToString for LogLevelMap {
    fn to_string(&self) -> String {
        let mut pairs: Vec<String> = Vec::new();
        if let Some(log) = self.0.get("root") {
            format!("{}", log)
        } else {
            for (key, value) in &self.0 {
                pairs.push(format!("{}={}", key.replace("-", "_"), value));
            }
            format!("{}", pairs.join(","))
        }
    }
}

/// 日志等级
#[derive(Debug, Serialize)]
enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// 自定义日志等级反序列化，使其大小写不敏感
impl<'de> Deserialize<'de> for LogLevel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let level = String::deserialize(deserializer)?;
        Ok(match level.to_uppercase().as_str() {
            "TRACE" => LogLevel::Trace,
            "DEBUG" => LogLevel::Debug,
            "INFO" => LogLevel::Info,
            "WARN" => LogLevel::Warn,
            "ERROR" => LogLevel::Error,
            _ => return Err(serde::de::Error::custom("Invalid log level")),
        })
    }
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

/// 压缩配置
#[derive(Debug, Serialize, Deserialize)]
pub struct CompressConfig {
    pub enable: bool,
    pub password: String,
    pub dir: String,
    #[serde(rename = "all-success")]
    pub all_success: bool,
}

impl Default for CompressConfig {
    fn default() -> Self {
        Self {
            enable: false,
            password: "".to_owned(),
            dir: "cpr".to_owned(),
            all_success: true,
        }
    }
}

/// PDF配置
#[derive(Debug, Serialize, Deserialize)]
pub struct PdfConfig {
    pub enable: bool,
    pub dir: String,
    #[serde(rename = "all-success")]
    pub all_success: bool,
}

impl Default for PdfConfig {
    fn default() -> Self {
        Self {
            enable: false,
            dir: "pdf".to_owned(),
            all_success: true,
        }
    }
}

/// 语言配置
#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
pub enum Language {
    Chinese,  // 中文
    English,  // 英文
    Japanese, // 日文
}

impl Default for Language {
    fn default() -> Self {
        Self::Chinese
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Language::Chinese => write!(f, "Chinese"),
            Language::English => write!(f, "English"),
            Language::Japanese => write!(f, "Japanese"),
        }
    }
}

impl<'de> Deserialize<'de> for Language {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let level = String::deserialize(deserializer)?;
        Ok(match level.to_uppercase().as_str() {
            "CHINESE" => Language::Chinese,
            "ENGLISH" => Language::English,
            "JAPANESE" => Language::Japanese,
            _ => return Err(serde::de::Error::custom("Invalid language")),
        })
    }
}

impl Language {
    // 这个方法将 data-tag 转换为对应的Language枚举
    pub fn from_data_tag(data_tag: &str) -> Option<Self> {
        match data_tag {
            "29963" => Some(Language::Chinese),
            "12227" => Some(Language::English),
            "6346" => Some(Language::Japanese),
            _ => None,
        }
    }
    // 获取图标
    pub fn get_icon(&self) -> &str {
        match self {
            Language::Chinese => "🇨🇳",
            Language::English => "🇬🇧",
            Language::Japanese => "🇯🇵",
        }
    }
}
