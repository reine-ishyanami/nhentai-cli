mod command;
mod config;
mod error;
mod model;
mod parse;
mod request;

use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
    thread,
};

use crate::command::Args;
use crate::config::Config;
use crate::error::EResult;
use chrono::Local;
use clap::Parser;
use env_logger::Builder;

const CONFIG_FILE_PATH: &str = "config.yaml";

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let config: Config = match load_config(CONFIG_FILE_PATH) {
        Ok(config) => config,
        Err(e) => panic!("程序启动异常 {}", e),
    };
    args.cmd.run(config, CONFIG_FILE_PATH).await
}

/// 读取配置文件
///
/// # Arguments
///
/// * `file_name` - 文件名称
///
/// # Returns
///
/// 配置文件内容
///
/// # Errors
///
/// 可能发生的异常
///
fn load_config(file_name: &str) -> EResult<Config> {
    let mut config: Config = Config::default();
    if let Ok(f) = File::open(Path::new(file_name)) {
        let mut file = f;
        // 读取文件内容到字符串
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        config = serde_yaml::from_str(&contents)?;
    }
    Builder::new()
        .parse_filters(config.log.level.to_string().as_str())
        .format(|buf, record| {
            writeln!(
                buf,
                "{} [{:5}] [{:20}] [{:20}] {}",
                Local::now().format("%Y-%m-%dT%H:%M:%S"),
                record.level(),
                record.target(),
                thread::current().name().unwrap_or("unknown"),
                record.args()
            )
        })
        .init();
    Ok(config)
}
