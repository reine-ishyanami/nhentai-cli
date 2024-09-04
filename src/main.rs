mod command;
mod config;
mod error;
mod model;
mod parse;
mod request;

use std::{env, fs::File, io::Read, path::Path, thread};

use crate::command::App;
use crate::config::Config;
use crate::error::EResult;
use chrono::Local;
use clap::Parser;
use env_logger::Builder;
use std::io::Write;

const CONFIG_FILE_PATH: &str = "nhentai.yaml";

#[tokio::main]
async fn main() -> EResult<()> {
    let args = App::parse();
    let config: Config = match load_config(CONFIG_FILE_PATH) {
        Ok(config) => config,
        Err(e) => panic!("profile format error: {}, please regenerate the profile file", e),
    };
    Ok(args.cmd.run(config, CONFIG_FILE_PATH).await)
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
    // 读取程序目录配置文件
    let exe_path = env::current_exe()?;
    let config_path = exe_path.parent().unwrap().join(CONFIG_FILE_PATH);
    // 读取程序默认配置(3)
    let mut config: Config = Config::default(); // (3)

    // 读取当前目录下配置文件(1)
    if let Ok(mut file) = File::open(Path::new(file_name)) {
        // (1)
        // 读取文件内容到字符串
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        config = serde_yaml::from_str(&contents)?;
    } else if let Ok(mut file) = File::open(Path::new(&config_path)) {
        // (2)
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
