use crate::dep::error::CustomError;
use crate::dep::error::EResult;
use crate::dep::model::HentaiStore;
use reqwest::Client;
use std::{fs::File, io::Write};

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/58.0.3029.110 Safari/537.3";

/// 发送 HTTP Get 请求
///
/// # Arguments
///
/// * `url` - 请求的 URL
///
/// # Returns
///
/// * `Result<String, reqwest::Error>` - 请求结果
pub async fn navigate(url: &str) -> EResult<String> {
    // 创建一个客户端实例
    let client = Client::new();
    // 发送 GET 请求，并添加自定义头部
    let response = client.get(url).header("User-Agent", USER_AGENT).send().await?;
    // 检查响应的状态码
    if response.status().is_success() {
        // 读取响应体为字符串
        let body = response.text().await?;
        Ok(body)
    } else {
        // 如果响应状态码不是成功，返回错误
        Err(CustomError::RequestError {
            message: "Request failed".to_owned(),
            code: response.status().as_u16(),
        })
    }
}

///
/// 下载单个图片
///
/// # Arguments
///
/// * `hentaiStore` - HentaiStore 实例
/// * `retryCount` - 重试次数
///
pub async fn download_image(hentai_store: HentaiStore, max_count: u8, replace: bool) -> EResult<()> {
    let mut retry_count = 0u8;
    while retry_count < max_count {
        // 发送GET请求获取图片
        let response = reqwest::get(hentai_store.url.as_str()).await?;
        // 检查响应状态码是否为200（OK）
        if response.status().is_success() {
            // 获取响应体
            let bytes = response.bytes().await?;
            // 判断文件是否存在
            if hentai_store.path.exists() {
                // 如果不允许替换已有文件
                if !replace {
                    return Err(CustomError::FileError {
                        message: format!("{:?} 文件已存在", hentai_store.path.file_name()),
                    });
                }
            }
            // 打开文件准备写入
            if let Ok(mut file) = File::create(hentai_store.path.clone()) {
                // 将图片数据写入文件
                file.write_all(&bytes)?;
            }
            return Ok(());
        } else {
            retry_count += 1;
        }
    }
    Err(CustomError::RequestError {
        message: "Too many retries".to_owned(),
        code: 400u16,
    })
}
