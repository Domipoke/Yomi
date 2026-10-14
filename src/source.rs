use std::{collections::HashMap, fs, path::Path, process::{Command, Stdio}, sync::Arc};

use async_trait::async_trait;

use crate::{config::getConfig, utils::utils::AsyncError};

#[cfg(feature = "debug")]
use crate::log;

pub mod animeworld;
pub mod mangaworld;
pub mod mangago;

#[derive(Clone, Copy)]
pub enum SourceType {
    Anime, Manga
}
impl SourceType {
    pub fn to_string(self) -> String {
        match self {
            SourceType::Anime => "anime".to_owned(),
            SourceType::Manga => "manga".to_owned()
        }
    }
}
pub enum SourceLanguages {
    It, Jp, Eng
}

pub struct Series<S: AnimeSource> {
    episodes: Vec<Episode>,
    title: String,
    lang: SourceLanguages,
    url: String,
    source: S
}
pub struct Episode {
    pub title: String,
    pub index: usize,
    pub section: usize,
    pub url: String
}
#[derive(Clone)]
pub struct EpisodeListItem {
    pub title: String,
    pub url: String
}
pub struct Comic<S: MangaSource> {
    episodes: Vec<Episode>,
    title: String,
    lang: SourceLanguages,
    url: String,
    source: S
}




#[async_trait]
pub trait AnimeSource: Send + Sync { 
    fn has_update_page(&self) -> bool;
    /*let mut items = hostname.split(".").collect::<Vec<&str>>();
    if let Some(f) = items.first() {
        if *f != "www" {
            items.insert(0, "www");
            } 
        }
        Self {
            url: items.join("."),
            host: items[1..items.len() - 1].join("."),
            hostname: "https://".to_owned()+&items[1..].join("."),
            sourcetype
            } */
    fn new(hostname: String, url: String) -> Self where Self: Sized;

    fn get_hostname(&self)->String;
    fn get_url(&self)->String;

    /** {
        let p = std::cmp::max(1, page);
        let u = format!("{}/updated?page={}",self.url,p);
        let a = get(u).await?.error_for_status()?.text().await?;
        return Html::parse_document(&a);
    }*/
    async fn fetch_updated_page(&self, page: usize)-> Result<String, AsyncError>;
    fn extract_episode_from_updated(&self, document: String) -> Result<Vec<Episode>, AsyncError>;
    
    async fn fetch_search_page(&self, query: String, page: usize) -> Result<String, AsyncError>;
    fn extract_episode_from_search(&self, document: String) -> Result<Vec<Episode>, AsyncError>;

    async fn listupdated(&self, page: usize) -> Result<Vec<Episode>, AsyncError> {
        let body = self.fetch_updated_page(page).await?;
        self.extract_episode_from_updated(body)
    }
    async fn search(&self, query: String, page: usize) -> Result<Vec<Episode>, AsyncError>{
        let body = self.fetch_search_page(query, page).await?;
        self.extract_episode_from_search(body)
    }

    async fn episode_openable_link(&self, card: Episode) -> Result<String, Box<dyn std::error::Error>>;
    async fn episode_play_url(&self, url: &str) -> Result<String, Box<dyn std::error::Error>>;
    async fn lib_url_parse(&self, url: String) -> Result<HashMap<String, EpisodeListItem>,  AsyncError>;
    async fn lib_ep_select(&self, eli: EpisodeListItem) -> Result<(),  AsyncError>;
}

#[async_trait]
pub trait MangaSource: Send + Sync {
    fn new(hostname: String, url: String) -> Self where Self: Sized;

    fn get_hostname(&self)->String;
    fn get_url(&self)->String;
    fn has_update_page(&self) -> bool;
    async fn fetch_updated_page(&self, page: usize)-> Result<String, AsyncError>;
    fn extract_episode_from_updated(&self, document: String) -> Result<Vec<Episode>, AsyncError>;
    
    async fn fetch_search_page(&self, query: String, page: usize) -> Result<String, AsyncError>;
    fn extract_episode_from_search(&self, document: String) -> Result<Vec<Episode>, AsyncError>;

    async fn listupdated(&self, page: usize) -> Result<Vec<Episode>, AsyncError> {
        let body = self.fetch_updated_page(page).await?;
        self.extract_episode_from_updated(body)
    }
    async fn search(&self, query: String, page: usize) -> Result<Vec<Episode>, AsyncError>{
        let body = self.fetch_search_page(query, page).await?;
        self.extract_episode_from_search(body)
    }
    async fn lib_url_parse(&self, url: String) -> Result<HashMap<String, EpisodeListItem>,  AsyncError>;
    async fn extract_pages_url(&self, chapter_url: String) -> Result<Vec<String>, AsyncError>;

    async fn to_pdf(&self, metadata: MangaMetadata, img:Vec<String>)  -> Result<String, AsyncError>;
    async fn to_cbz(&self, metadata: MangaMetadata, img:Vec<String>)  -> Result<String, AsyncError>;
}

pub fn openManga(so: Arc<Box<dyn MangaSource + Send + Sync>>, metadata: MangaMetadata) {
    let c = getConfig().Manga.reader;
    tokio::spawn(async move {
        if let Ok(imgs) = so.extract_pages_url(metadata.url.clone()).await {
            let path = match c {
                crate::config::MangaReader::CBZ | crate::config::MangaReader::Sumatra | crate::config::MangaReader::CbzReader => {
                    so.to_cbz(metadata, imgs).await
                },
                crate::config::MangaReader::PDF | crate::config::MangaReader::Calibre => {
                    so.to_pdf(metadata, imgs).await
                },
            };
            match path {
                Err(k) => {
                    // ex.send(k); // TODO Send error string to a tx<string> 
                },
                Ok(s) => {
                    let command = getConfig().Manga.programCmd.clone();
                    let pargs = getConfig().Manga.programArg;
                    let args = pargs.split(" ").filter(|x| !x.is_empty());
                    let cmd = match c {
                        crate::config::MangaReader::Sumatra => {
                            let mut a = Command::new(command);
                            a.args(args).arg(s.clone());
                            Some(a)
                        },
                        crate::config::MangaReader::CbzReader => {
                            let mut a = Command::new(command);
                            a.args(args).arg(s.clone());
                            Some(a)
                        },
                        crate::config::MangaReader::Calibre => {
                            let mut a = Command::new(command);
                            a.args(args).arg(s.clone());
                            Some(a)
                        }
                        crate::config::MangaReader::PDF => None,
                        crate::config::MangaReader::CBZ => None,
                    };
                    match cmd {
                        Some(mut c) => {
                            let _ = c.stdin(Stdio::null())
                                .stdout(Stdio::null())
                                .stderr(Stdio::null())
                                .spawn();
                        }
                        None => {},
                    };
                }
            };
        }
    });
}

pub struct MangaMetadata {
    pub  title: Option<String>,
    pub  url: String,
    pub  chapter: Option<String>,
    pub  volume: Option<String>
}

pub fn getSources(folder: &str) -> Vec<String> {
    let mut p = Path::new("./extensions").join(folder);
    let mut res = vec![];
    for files in fs::read_dir(p) {
        for rf in files {
            if let Ok(f) = rf {
                if let Some(name)  = f.path().file_stem() {
                    if let Some(s) = name.to_str() {
                        res.push(s.to_owned());
                    }
                }
            }
        }
    }
    res
}