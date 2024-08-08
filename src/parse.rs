use crate::model::{HentaiDetail, HentaiHref};
use log;
use scraper::{selectable::Selectable, Html, Selector};

///
/// 获取 html 中的 hentai 列表
///
/// # Arguments
///
/// * `html` - html 字符串
///
/// # Returns
///
/// * `Vec<HentaiHref>` - hentai 列表
///
pub async fn get_hentai_list(html: &str) -> Vec<HentaiHref> {
    let fragment = Html::parse_document(html);
    let gallery_class = Selector::parse(".gallery").unwrap(); // 选择所有的 gallery 类
    let a_tag = Selector::parse("a").unwrap(); // 选择所有的 a 标签
    let caption_class = Selector::parse(".caption").unwrap(); // 选择所有的 caption 类
    let mut hentai_list = vec![];
    for element in fragment.select(&gallery_class) {
        let href = element.select(&a_tag).next().unwrap().value().attr("href").unwrap();
        let title = element.select(&caption_class).next().unwrap().inner_html();
        let hentai_href = HentaiHref {
            href: format!("https://nhentai.net{}", href),
            title: title,
        };
        hentai_list.push(hentai_href);
    }
    hentai_list
}

///
/// 获取 html 中的 hentai 详情
///
/// # Arguments
///
/// * `html` - html 字符串
///
/// # Returns
///
/// * `HentaiDetail` - hentai 详情
pub async fn get_hentai_detail(html: &str) -> HentaiDetail {
    let fragment = Html::parse_document(html);
    let cover_id_img_tag = Selector::parse("#cover img").unwrap(); // 选择所有的 cover img 标签
    let src_url = fragment.select(&cover_id_img_tag).next().unwrap().value().attr("data-src").unwrap();
    let split: Vec<&str> = src_url.split('/').collect();
    let gallery = split[split.len() - 2];
    let container_id = Selector::parse("#thumbnail-container .thumbs").unwrap();
    let container_class = Selector::parse(".thumb-container").unwrap();
    let img_tag = Selector::parse("img").unwrap();
    let mut res_list: Vec<String> = Vec::new();
    log::debug!("collect images url");
    for element in fragment.select(&container_id).next().unwrap().select(&container_class) {
        let src_url = element.select(&img_tag).next().unwrap().value().attr("data-src").unwrap();
        let split: Vec<&str> = src_url.split('/').collect();
        let last_segment = split[split.len() - 1];
        let img: Vec<&str> = last_segment.split('.').collect();
        if img.len() >= 2 {
            let first = img[0].replace("t", "");
            res_list.push(format!("{}.{}", first, img[1]));
        }
    }
    HentaiDetail {
        gallery: gallery.to_owned(),
        res_list: res_list,
    }
}
