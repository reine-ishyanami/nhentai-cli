extern crate image;

use image::{ColorType, GenericImageView, ImageFormat};

use miniz_oxide::deflate::{compress_to_vec_zlib, CompressionLevel};
use walkdir::{DirEntry, WalkDir};
use zip::AesMode;
use zip::{result::ZipError, write::SimpleFileOptions};

use std::io::{Read, Seek};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::{ fs::File, io::Write};
use std::fs;
use anyhow::bail;
use tokio::task::JoinSet;

use pdf_writer::{Content, Filter, Finish, Name, Pdf as PdfObject, Rect, Ref};

use clap::{Parser, Subcommand};

use crate::dep::config::{Compress, Config, Pdf};
use crate::dep::error::EResult;
use crate::dep::model::{HentaiDetail, HentaiStore};
use crate::dep::parse::{get_hentai_detail, get_hentai_list};
use crate::dep::request::{download_image, navigate};

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
pub(crate) async fn search(name: &str, language: &str) -> anyhow::Result<HentaiDetail> {
    let base_url = format!("https://nhentai.net/search/?q={}", String::from(name));
    // 第一次请求，获取 hentai 列表
    let html = navigate(base_url.as_str()).await?;
    println!("{}", html);
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
        bail!("language not found")
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
        // let mut path = PathBuf::new();
        // path.push(config.root_dir.as_str());
        // path.push(name);
        let path = format!("{}/{}", config.root_dir, name);
        // 创建目录
        if let Err(e) = fs::create_dir_all(path) {
        }
        // 并发任务集合
        let mut set = JoinSet::new();
        for ele in hentai_detail.res_list {
            // let mut path = PathBuf::new();
            // path.push(config.root_dir.as_str());
            // path.push(name);
            // path.push(ele.as_str());
            let hentai_store = HentaiStore {
                url: format!("{}/{}/{}", base_url, &hentai_detail.gallery, ele),
                path: PathBuf::from(format!("{}/{}/{}", config.root_dir, name, ele)),
            };
            set.spawn(download_image(hentai_store, config.retry_count, config.replace));
        }
        // 当任务全部执行完毕
        while let Some(_) = set.join_next().await {}
    } else {
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
    }
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
                ImageFormat::Jpeg => {
                    match dynamic.color() {
                        ColorType::L8 => handle_png_l8(),
                        ColorType::Rgb8 => (Filter::DctDecode, data, None),
                        _ => panic!("unsupported color type: {:?}", dynamic.color()),
                    }
                }
                ImageFormat::Png => {
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
        Ok(_) => {},
        Err(e) => {
            // @TODO 返回错误
        },
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
    // 创建目录
    if let Err(e) = fs::create_dir_all(&cpr_dir) {

    }
    let dst_file = PathBuf::from_str(format!("{}/{}.zip", cpr_dir, name).as_str()).unwrap();
    let method = zip::CompressionMethod::Stored;
    let src_dir = PathBuf::from_str(path).unwrap();

    match doit(&src_dir, &dst_file, method, password) {
        Ok(_) => {},
        Err(e) => {
            // @TODO 返回错误
        },
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
    if password.is_empty() {}
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
            zip.start_file(path_as_string, options)?;
            let mut f = File::open(path)?;

            f.read_to_end(&mut buffer)?;
            zip.write_all(&buffer)?;
            buffer.clear();
        } else if !name.as_os_str().is_empty() {
            // Only if not root! Avoids path spec / warning
            // and mapname conversion failed error on unzip
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
