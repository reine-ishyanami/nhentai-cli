extern crate image;

use image::{ColorType, GenericImageView, ImageFormat};

use miniz_oxide::deflate::{compress_to_vec_zlib, CompressionLevel};

use std::fs;
use std::path::PathBuf;
use std::{error::Error, fs::File, io::Write};
use tokio::task::JoinSet;

use pdf_writer::{Content, Filter, Finish, Name, Pdf as PdfObject, Rect, Ref};

use clap::{Parser, Subcommand};

use crate::config::{Compress, Config, Pdf};
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
        /// hentai pdf store dir
        #[arg(short, long)]
        dir: Option<String>,
    },
    /// compress hentai to zip
    Compress {
        /// hentai images path
        #[arg(short, long)]
        path: String,
        /// hentai zip name
        #[arg(short, long)]
        name: String,
        /// hentai zip password
        #[arg(short, long)]
        secret: Option<String>,
        /// hentai zip store dir
        #[arg(short, long)]
        dir: Option<String>,
    },
}

impl Commands {
    pub async fn run(&self, config: Config, file: &str) {
        match self {
            Commands::Generate => generate(config, file),
            Commands::Download { name } => download(name, config).await,
            Commands::Convert { path, name, dir } => convert(path, name, dir, config.pdf),
            Commands::Compress {
                path,
                name,
                secret,
                dir,
            } => compress(path, name, secret, dir, config.compress),
        }
    }
}

/// 生成默认配置文件
///
/// # Arguments
///
/// * `config` - 配置文件
/// * `file` - 配置文件路径
fn generate(config: Config, file: &str) {
    let config_str = serde_yaml::to_string(&config).unwrap();
    let mut file = File::create(file).unwrap();
    file.write_all(config_str.as_bytes()).unwrap();
    log::info!("generate config file success");
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
    if let Some(target) = hentai_list
        .iter()
        .filter(|hentai| hentai.title.contains(language))
        .next()
    {
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
        // 创建目录
        if let Err(e) = fs::create_dir_all(path) {
            log::warn!("create dir failed: {}", e);
        }
        // 并发任务集合
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
            set.spawn(download_image(hentai_store, config.retry_count, config.replace));
        }
        log::info!("download start");
        // 当任务全部执行完毕
        while let Some(_) = set.join_next().await {}
        log::info!("download finished");
    } else {
        log::error!("search nothing of {}", config.language.as_str());
    }
    if config.compress.enable {
        compress(&name, &name, &None, &None, config.compress);
    }
    if config.pdf.enable {
        convert(&name, &name, &None, config.pdf);
    }
}

/// 将 hentai 转换为 pdf
///
/// # Arguments
///
/// * `path` - hentai 图片路径
/// * `name` - hentai pdf 名称
/// * `dir` - pdf 存储路径
/// * `pdf_config` - pdf 配置
fn convert(path: &String, name: &String, dir: &Option<String>, pdf_config: Pdf) {
    let pdf_dir = match dir {
        Some(dir) => dir.clone(),
        None => pdf_config.dir,
    };
    if let Err(e) = fs::create_dir_all(pdf_dir.as_str()) {
        log::warn!("create dir failed: {}", e);
    }
    log::info!("convert to pdf start");
    // 创建 pdf
    let mut pdf = PdfObject::new();

    // 定义 pdf 参数
    let catalog_id = Ref::new(1);
    let page_tree_id = Ref::new(2);
    pdf.catalog(catalog_id).pages(page_tree_id);

    // 获取目录中的所有文件
    let entries = fs::read_dir(path).unwrap();

    let mut page_ids = Vec::new();
    let mut image_ids = Vec::new();
    let mut s_mask_ids = Vec::new();
    let mut content_ids = Vec::new();
    let mut paths = Vec::new();

    for (index, entry) in entries.enumerate() {
        page_ids.push(Ref::new(index as i32 * 4 + 20));
        image_ids.push(Ref::new(index as i32 * 4 + 20 + 1));
        s_mask_ids.push(Ref::new(index as i32 * 4 + 20 + 2));
        content_ids.push(Ref::new(index as i32 * 4 + 20 + 3));
        paths.push(entry.unwrap().path());
    }
    // 设置 pdf 参数
    let page_ids_clone = page_ids.clone();
    let len = page_ids_clone.len();
    pdf.pages(page_tree_id).kids(page_ids_clone).count(len as i32);

    paths.sort_by(|a, b| {
        let a: u16 = a
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .split(".")
            .next()
            .unwrap()
            .parse()
            .expect("请不要在资源文件夹中添加非图片文件");
        let b: u16 = b
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .split(".")
            .next()
            .unwrap()
            .parse()
            .expect("请不要在资源文件夹中添加非图片文件");
        a.cmp(&b)
    });

    // let entries = fs::read_dir(path).unwrap();

    // 遍历目录中的每个文件
    for (index, path) in paths.iter().enumerate() {
        // 只处理文件，忽略目录
        if path.is_file() {
            log::debug!("processing file: {}", path.display());
            let path_clone = &path.clone();
            let file_name = path_clone.file_name().unwrap().to_str().unwrap();
            let page_id = *page_ids.get(index).unwrap();
            let image_id = *image_ids.get(index).unwrap();
            let content_id = *content_ids.get(index).unwrap();
            let s_mask_id = *s_mask_ids.get(index).unwrap();
            let image_name = Name(file_name.as_bytes());
            let mut page = pdf.page(page_id);

            // 解析图片
            let data = std::fs::read(path).unwrap();
            let format = image::guess_format(&data).unwrap();
            let dynamic = image::load_from_memory(&data).unwrap();

            // 处理 png 文件或者 l8 色域图片
            let handle_png_l8 = || {
                let level = CompressionLevel::DefaultLevel as u8;
                let encoded = compress_to_vec_zlib(dynamic.to_rgb8().as_raw(), level);
                let mask = dynamic.color().has_alpha().then(|| {
                    let alphas: Vec<_> = dynamic.pixels().map(|p| (p.2).0[3]).collect();
                    compress_to_vec_zlib(&alphas, level)
                });
                (Filter::FlateDecode, encoded, mask)
            };

            let (filter, encoded, mask) = match format {
                // A JPEG is already valid DCT-encoded data.
                ImageFormat::Jpeg => {
                    log::debug!("image {} format: jpeg , color {:?}", path.display(), dynamic.color());
                    match dynamic.color() {
                        ColorType::L8 => handle_png_l8(),
                        ColorType::Rgb8 => (Filter::DctDecode, data, None),
                        _ => panic!("unsupported color type: {:?}", dynamic.color())
                    }
                }
                ImageFormat::Png => {
                    log::debug!("image {} format: png , color {:?}", path.display(), dynamic.color());
                    handle_png_l8()
                }
                _ => panic!("unsupported image format"),
            };

            // 页面大小
            let rect = Rect::new(0.0, 0.0, dynamic.width() as f32, dynamic.height() as f32);

            page.media_box(rect);
            page.parent(page_tree_id);
            page.contents(content_id);
            page.resources().x_objects().pair(image_name, image_id);
            page.finish();

            // 写入图片到 PDF 文件
            let mut image = pdf.image_xobject(image_id, &encoded);
            image.filter(filter);
            image.width(dynamic.width() as i32);
            image.height(dynamic.height() as i32);
            image.color_space().device_rgb();
            image.bits_per_component(8);
            if mask.is_some() {
                image.s_mask(s_mask_id);
            }
            image.finish();

            // 追加透明通道
            if let Some(encoded) = &mask {
                let mut s_mask = pdf.image_xobject(s_mask_id, encoded);
                s_mask.filter(filter);
                s_mask.width(dynamic.width() as i32);
                s_mask.height(dynamic.height() as i32);
                s_mask.color_space().device_gray();
                s_mask.bits_per_component(8);
            }

            log::debug!(
                "image {} size: {}x{}",
                path.display(),
                dynamic.width(),
                dynamic.height()
            );

            let w = dynamic.width() as f32;
            let h = dynamic.height() as f32;

            let x = rect.x2 - w;
            let y = rect.y2 - h;

            let mut content = Content::new();
            content.save_state();
            content.transform([w, 0.0, 0.0, h, x, y]);
            content.x_object(image_name);
            content.restore_state();
            pdf.stream(content_id, &content.finish());
        }
    }
    // 保存PDF文件
    match std::fs::write(format!("{}/{}.pdf", pdf_dir, name), pdf.finish()) {
        Ok(_) => log::info!("save pdf {:?}", name),
        Err(e) => panic!("{:?}", e),
    }
}

/// 将 hentai 打包为 zip
///
/// # Arguments
///
/// * `path` - hentai 图片路径
/// * `name` - hentai zip 名称
/// * `secret` - zip 密码
/// * `dir` - zip 存储路径
/// * `compress_config` - 打包配置
fn compress(path: &String, name: &String, secret: &Option<String>, dir: &Option<String>, compress_config: Compress) {
    let password = match secret {
        Some(secret) => secret.clone(),
        None => compress_config.password,
    };
    let cpr_dir = match dir {
        Some(dir) => dir.clone(),
        None => compress_config.dir,
    };
    todo!()
}
