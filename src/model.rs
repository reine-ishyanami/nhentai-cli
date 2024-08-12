use std::path::PathBuf;
use crate::config::Language;

#[derive(Debug, Clone)]
pub struct HentaiHref {
    pub href: String,
    pub title: String,
    pub language: Language,
}


impl ToString for HentaiHref {
    fn to_string(&self) -> String {
        format!("{} {} ", self.language.get_icon(), self.title)
    }
}

pub struct HentaiDetail {
    pub gallery: String,
    pub res_list: Vec<String>,
}

pub struct HentaiStore {
    pub url: String,
    pub path: PathBuf,
}

