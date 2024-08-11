use std::path::PathBuf;
use crate::config::Language;

#[derive(Debug, Clone)]
pub struct HentaiHref {
    pub href: String,
    pub title: String,
    pub language: Language,
}

pub struct HentaiDetail {
    pub gallery: String,
    pub res_list: Vec<String>,
}

pub struct HentaiStore {
    pub url: String,
    pub path: PathBuf,
}

