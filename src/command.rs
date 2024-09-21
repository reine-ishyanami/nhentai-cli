use dialoguer::theme::ColorfulTheme;
use dialoguer::Select;
use image::{ColorType, GenericImageView, ImageFormat};

use miniz_oxide::deflate::{compress_to_vec_zlib, CompressionLevel};
use walkdir::{DirEntry, WalkDir};
use zip::AesMode;
use zip::{result::ZipError, write::SimpleFileOptions};

use std::io::{Read, Seek};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::{env, fs};
use std::{fs::File, io::Write};
use tokio::task::JoinSet;

use pdf_writer::{Content, Filter, Finish, Name, Pdf, Rect, Ref};

use clap::{Parser, Subcommand};

use crate::config::{CompressConfig, Config, Language, PdfConfig};
use crate::error::{CustomError, EResult};
use crate::model::{HentaiDetail, HentaiHref, HentaiStore};
use crate::parse::{get_hentai_detail, get_hentai_list};
use crate::request::{download_image, navigate};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct App {
    #[command(subcommand)]
    pub cmd: Commands,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// generate config file
    Generate {
        /// generate global config file
        #[arg(short)]
        global: bool,
    },
    /// download hentai by name
    Download {
        /// hentai name
        #[arg(short, long)]
        name: String,
        /// interaction
        #[arg(short, long)]
        interaction: Option<bool>,
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
            Commands::Generate { global } => generate(config, file, *global),
            Commands::Download { name, interaction } => download(name, config, interaction).await,
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
/// * `global` - 是否为全局配置文件
fn generate(config: Config, file: &str, global: bool) {
    let config_str = serde_yaml::to_string(&config).unwrap();
    if global {
        let exe_path = env::current_exe().unwrap();
        let config_path = exe_path.parent().unwrap().join(file);
        let mut file = File::create(config_path).unwrap();
        file.write_all(config_str.as_bytes()).unwrap();
        log::info!("generate global config file success");
    } else {
        let mut file = File::create(file).unwrap();
        file.write_all(config_str.as_bytes()).unwrap();
        log::info!("generate config file success");
    }
}

///
/// 根据 hentai 名称搜索 hentai
///
/// # Arguments
///
/// * `name` - hentai 名称
/// * `language` - 语言
/// * `interaction` - 是否交互模式
///
/// # Returns
///
/// * `HentaiDetail` - 搜索到的 hentai 详情
///
/// # Errors
///
/// * `Box<dyn Error>` - 搜索失败
///
async fn search(name: &str, language: &Language, interaction: bool) -> EResult<HentaiDetail> {
    let base_url = format!("https://nhentai.net/search/?q={}", name);
    // 第一次请求，获取 hentai 列表
    let html = navigate(base_url.as_str()).await.expect("navigate failed");
    let hentai_list = get_hentai_list(html.as_str()).await;
    let hentai_arr: &[HentaiHref] = &hentai_list;

    let single_hentai_href = if interaction {
        // 如果是交互模式，则让终端用户选择一个 hentai
        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Pick Hentai You Want ")
            .default(0)
            .items(hentai_arr)
            .interact()
            .unwrap();
        Some(&hentai_arr[selection])
    } else {
        // 非交互模式下，选择符合对应语言的第一个 hentai
        hentai_list.iter().find(|hentai| hentai.language == *language)
    };
    if let Some(hentai) = single_hentai_href {
        log::debug!("found: {}", hentai.title);
        // 第二次请求，获取指定 hentai 主页
        let html = navigate(hentai.href.as_str()).await.expect("navigate failed");
        Ok(get_hentai_detail(html.as_str()).await)
    } else {
        log::error!("{} hentai not found", language);
        Err(CustomError::NotFoundError {
            language: language.to_string(),
        })
    }
}

/// 根据 hentai 名称下载 hentai
///
/// # Arguments
///
/// * `name` - hentai 名称
/// * `config` - 配置文件
/// * `interaction` - 是否开启交互模式
async fn download(name: &str, config: Config, interaction: &Option<bool>) {
    let base_url = "https://i3.nhentai.net/galleries";
    // 如果传入了是否开启交互式的参数
    let interaction = if let Some(interaction) = interaction {
        *interaction
    } else {
        config.interaction
    };
    // 是否所有文件下载成功
    let mut all_success = true;
    if let Ok(hentai_detail) = search(name, &config.language, interaction).await {
        let path = format!("{}/{}", config.root_dir, name);
        // 创建目录
        if let Err(e) = fs::create_dir_all(path) {
            log::warn!("create dir failed: {}", e);
        }
        // 并发任务集合
        let mut set = JoinSet::new();
        log::info!("fetch {} pages", hentai_detail.res_list.len());
        for ele in hentai_detail.res_list {
            let hentai_store = HentaiStore {
                url: format!("{}/{}/{}", base_url, &hentai_detail.gallery, ele),
                path: PathBuf::from(format!("{}/{}/{}", config.root_dir, name, ele)),
            };
            set.spawn(download_image(hentai_store, config.retry_count, config.replace));
        }
        log::info!("download start");
        // 当任务全部执行完毕
        while let Some(res) = set.join_next().await {
            // 下载任务发生异常时进行提示
            match res.unwrap() {
                Ok((time, filename)) => {
                    log::debug!("use {} time to download {}", time, filename);
                }
                Err(error) => {
                    all_success = false;
                    log::error!("download failed: {}", error);
                }
            }
        }
        log::info!("download finished");
    }
    if config.compress.enable && (config.compress.all_success && all_success || !config.compress.all_success) {
        compress(name, name, &None, &None, config.compress);
    }
    if config.pdf.enable && (config.pdf.all_success && all_success || !config.pdf.all_success)  {
        convert(name, name, &None, config.pdf);
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
fn convert(path: &str, name: &str, dir: &Option<String>, pdf_config: PdfConfig) {
    let pdf_dir = match dir {
        Some(dir) => dir.clone(),
        None => pdf_config.dir,
    };
    if let Err(e) = fs::create_dir_all(pdf_dir.as_str()) {
        log::warn!("create dir failed: {}", e);
    }
    log::info!("convert to pdf start");
    // 创建 pdf
    let mut pdf = Pdf::new();

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

    // 定义图片文件的扩展名
    let image_extensions = vec!["jpg", "jpeg", "png", "gif", "bmp", "tiff", "svg", "webp"];

    for (index, entry) in entries.enumerate() {
        // 判断项目是否是图片类型文件
        let path = entry.unwrap().path();
        if let Some(ext) = path.extension() {
            let is_image = image_extensions.contains(&ext.to_string_lossy().to_lowercase().as_str());
            if is_image {
                page_ids.push(Ref::new(index as i32 * 4 + 20));
                image_ids.push(Ref::new(index as i32 * 4 + 20 + 1));
                s_mask_ids.push(Ref::new(index as i32 * 4 + 20 + 2));
                content_ids.push(Ref::new(index as i32 * 4 + 20 + 3));
                paths.push(path);
            } else {
                log::warn!("file {} is not a image, skip", path.display());
            }
        } else {
            log::error!("invalid file extension name: {:?}", path);
        }
    }
    // 设置 pdf 参数
    let page_ids_clone = page_ids.clone();
    let len = page_ids_clone.len();
    pdf.pages(page_tree_id).kids(page_ids_clone).count(len as i32);

    paths.sort_by(|a, b| {
        let a: u16 = a
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .parse()
            .expect("please use number to name image");
        let b: u16 = b
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .parse()
            .expect("please use number to name image");
        a.cmp(&b)
    });

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
            convert_image_into_pdf(
                &mut pdf,
                path,
                page_tree_id,
                page_id,
                image_id,
                content_id,
                s_mask_id,
                file_name,
            );
        }
    }
    // 保存PDF文件
    match std::fs::write(format!("{}/{}.pdf", pdf_dir, name), pdf.finish()) {
        Ok(_) => log::info!("save pdf {:?}", name),
        Err(e) => log::error!("{:?}", e),
    }
}

///
/// 将图片转换为 PDF
///
/// # Arguments
/// * `pdf` - PDF 文件
/// * `path` - 图片路径
/// * `page_tree_id` - 页面树 ID
/// * `page_id` - 页面 ID
/// * `image_id` - 图片 ID
/// * `content_id` - 内容 ID
/// * `s_mask_id` - s_mask ID
/// * `file_name` - 文件名
///
fn convert_image_into_pdf(
    pdf: &mut Pdf,
    path: &PathBuf,
    page_tree_id: Ref,
    page_id: Ref,
    image_id: Ref,
    content_id: Ref,
    s_mask_id: Ref,
    file_name: &str,
) {
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
        ImageFormat::Jpeg => {
            log::debug!("image {} format: jpeg , color {:?}", path.display(), dynamic.color());
            match dynamic.color() {
                ColorType::L8 => handle_png_l8(),
                ColorType::Rgb8 => (Filter::DctDecode, data, None),
                _ => {
                    log::error!("unsupported color type: {:?}", dynamic.color());
                    return;
                }
            }
        }
        ImageFormat::Png => {
            log::debug!("image {} format: png , color {:?}", path.display(), dynamic.color());
            handle_png_l8()
        }
        _ => {
            log::error!("unsupported image format");
            return;
        }
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

/// 将 hentai 打包为 zip
///
/// # Arguments
///
/// * `path` - hentai 图片路径
/// * `name` - hentai zip 名称
/// * `secret` - zip 密码
/// * `dir` - zip 存储路径
/// * `compress_config` - 打包配置
fn compress(path: &str, name: &str, secret: &Option<String>, dir: &Option<String>, compress_config: CompressConfig) {
    let password = match secret {
        Some(secret) => secret.clone(),
        None => compress_config.password,
    };
    let cpr_dir = match dir {
        Some(dir) => dir.clone(),
        None => compress_config.dir,
    };
    // 创建目录
    if let Err(e) = fs::create_dir_all(&cpr_dir) {
        log::warn!("create dir failed: {}", e);
    }
    let dst_file = PathBuf::from_str(format!("{}/{}.zip", cpr_dir, name).as_str()).unwrap();
    let method = zip::CompressionMethod::Stored;
    let src_dir = PathBuf::from_str(path).unwrap();
    log::info!("compress {:?} to {:?}", src_dir.display(), dst_file.display());
    match doit(&src_dir, &dst_file, method, password) {
        Ok(_) => log::info!("done: {:?} written to {:?}", path, dst_file),
        Err(e) => log::error!("Error: {:?}", e),
    }
}

/// 将指定目录树内容写入 zip 文件
///
/// # Arguments
///
/// * `it` - 源目录树
/// * `prefix` - 源目录
/// * `writer` - zip 文件
/// * `method` - 压缩方法
/// * `password` - zip 密码
fn zip_dir<T>(
    it: &mut dyn Iterator<Item = DirEntry>,
    prefix: &Path,
    writer: T,
    method: zip::CompressionMethod,
    password: String,
) -> EResult<()>
where
    T: Write + Seek,
{
    let mut zip = zip::ZipWriter::new(writer);
    // 密码为空则不设置密码
    let options = if password.is_empty() {
        SimpleFileOptions::default()
            .compression_method(method)
            .unix_permissions(0o755)
    } else {
        SimpleFileOptions::default()
            .compression_method(method)
            .with_aes_encryption(AesMode::Aes256, &password)
            .unix_permissions(0o755)
    };

    let prefix = Path::new(prefix);
    let mut buffer = Vec::new();
    for entry in it {
        let path = entry.path();
        let name = path.strip_prefix(prefix).unwrap();
        let path_as_string = name.to_str().map(str::to_owned).unwrap();

        // 写入文件
        if path.is_file() {
            log::debug!("adding file {:?} as {:?} ...", path, name);
            zip.start_file(path_as_string, options)?;
            let mut f = File::open(path)?;
            f.read_to_end(&mut buffer)?;
            zip.write_all(&buffer)?;
            buffer.clear();
        } else if !name.as_os_str().is_empty() {
            log::debug!("adding dir {:?} as {:?} ...", path_as_string, name);
            zip.add_directory(path_as_string, options)?;
        }
    }
    zip.finish()?;
    Ok(())
}

/// 遍历文件夹文件，压缩成 zip 文件
///
/// # Arguments
///
/// * `src_dir` - 文件夹路径
/// * `dst_file` - 压缩文件名
/// * `method` - 压缩方式
/// * `password` - 密码
fn doit(src_dir: &Path, dst_file: &Path, method: zip::CompressionMethod, password: String) -> EResult<()> {
    if !Path::new(src_dir).is_dir() {
        return Err(ZipError::FileNotFound.into());
    }
    let path = Path::new(dst_file);
    let file = File::create(path).unwrap();
    let walkdir = WalkDir::new(src_dir);
    let it = walkdir.into_iter();
    zip_dir(&mut it.filter_map(|e| e.ok()), src_dir, file, method, password)?;
    Ok(())
}
