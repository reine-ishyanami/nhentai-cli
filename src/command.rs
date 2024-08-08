use std::fs;
use std::path::PathBuf;
use std::{error::Error, fs::File, io::Write};
use tokio::{runtime::Runtime, spawn};
use tokio::task::JoinSet;


use clap::{Parser, Subcommand};

use crate::error::EResult;
use crate::config::Config;
use crate::model::{HentaiDetail, HentaiStore};
use crate::parse::{get_hentai_detail, get_hentai_list};
use crate::request::{download_image, navigate};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub cmd: Commands,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// generate config file
    Generate,
    /// download hentai by name
    Download {
        /// hentai name
        #[arg(short, long)]
        name: String,
    },
    /// convert hentai to pdf
    Convert {
        /// hentai images path
        #[arg(short, long)]
        path: String,
        /// hentai pdf name
        #[arg(short, long)]
        name: String,
    },
    /// compress hentai to zip
    Compress {
        /// hentai images path
        #[arg(short, long)]
        path: String,
        /// hentai zip name
        #[arg(short, long)]
        name: String,
    },
}

impl Commands {
    pub async fn run(&self, config: Config, file: &str) -> Result<(), Box<dyn Error>> {
        match self {
            Commands::Generate => {
                let config_str = serde_yaml::to_string(&config).unwrap();
                let mut file = File::create(file).unwrap();
                file.write_all(config_str.as_bytes()).unwrap();
                Ok(())
            }
            Commands::Download { name } => Ok(download(name, config).await),
            Commands::Convert { path, name } => Ok(convert(path, name, config).await),
            Commands::Compress { path, name } => Ok(compress(path, name, config).await),
        }
    }
}

///
/// 根据 hentai 名称搜索 hentai
///
/// # Arguments
///
/// * `name` - hentai 名称
/// * `language` - 语言
///
/// # Returns
///
/// * `HentaiDetail` - 搜索到的 hentai 详情
///
/// # Errors
///
/// * `Box<dyn Error>` - 搜索失败
///
async fn search(name: &String, language: &str) -> Result<HentaiDetail, Box<dyn Error>> {
    let base_url = format!("https://nhentai.net/search/?q={}", name);
    // 第一次请求，获取 hentai 列表
    let html = navigate(base_url.as_str()).await.unwrap();
    let hentai_list = get_hentai_list(html.as_str()).await;
    // 根据所选语言匹配 hentai
    if let Some(target) = hentai_list.iter().filter(|hentai| hentai.title.contains(language)).next() {
        // 第二次请求，获取指定 hentai 主页
        let html = navigate(target.href.as_str()).await.unwrap();
        Ok(get_hentai_detail(html.as_str()).await)
    } else {
        log::error!("not found");
        Err("language not found".into())
    }
}

/// 根据 hentai 名称下载 hentai
///
/// # Arguments
///
/// * `name` - hentai 名称
/// * `config` - 配置文件
async fn download(name: &String, config: Config) {
    let base_url = "https://i3.nhentai.net/galleries";
    if let Ok(hentai_detail) = search(name, config.language.as_str()).await {
        let mut path = PathBuf::new();
        path.push(config.root_dir.as_str());
        path.push(name);
        if let Err(e) = fs::create_dir_all(path) {
            log::warn!("create dir failed: {}", e);
        }
        let mut set = JoinSet::new();
        for ele in hentai_detail.res_list {
            let mut path = PathBuf::new();
            path.push(config.root_dir.as_str());
            path.push(name);
            path.push(ele.as_str());
            let hentai_store = HentaiStore {
                url: format!("{}/{}/{}", base_url, &hentai_detail.gallery, ele),
                path: path,
            };
            set.spawn(download_image(hentai_store, config.retry_count));
            
        }
        while let Some(_) = set.join_next().await {}
        log::info!("download finished");
    } else {
        log::error!("search nothing of {}", config.language.as_str());
    }
}

/// 将 hentai 转换为 pdf
///
/// # Arguments
///
/// * `path` - hentai 图片路径
/// * `name` - hentai pdf 名称
/// * `config` - 配置文件
async fn convert(path: &String, name: &String, config: Config) {
    todo!()
}

/// 将 hentai 压缩为 zip
///
/// # Arguments
///
/// * `path` - hentai 图片路径
/// * `name` - hentai zip 名称
/// * `config` - 配置文件
async fn compress(path: &String, name: &String, config: Config) {
    todo!()
}
