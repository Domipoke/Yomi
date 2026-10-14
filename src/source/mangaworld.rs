use std::{collections::HashMap, f64::consts::E, format, io::Read, sync::Arc};
use crc32fast::Hasher;
use reqwest::get;
use scraper::{ElementRef, Html, Selector};
use zip::write::SimpleFileOptions;

use crate::{log, source::{Episode, EpisodeListItem, MangaMetadata, MangaSource, SourceType, openManga}, utils::utils::{AsyncError, MangaCbz}};

pub struct Mangaworld {
    url: String,
    hostname:String,
    sourcetype: SourceType
}

#[async_trait::async_trait]
impl MangaSource  for Mangaworld {
    fn get_hostname(&self)->String {
        self.hostname.clone()
    }
    fn get_url(&self) -> String {
        self.url.clone()
    }
    fn has_update_page(&self) -> bool {true}
    fn new(hostname: String, url: String) -> Self {
        Self {
            url,
            hostname,
            sourcetype: crate::source::SourceType::Manga
        } 
    }
    async fn listupdated(&self, page: usize) -> Result<Vec<crate::source::Episode>, AsyncError> {
        let body = self.fetch_updated_page(page).await?;
        self.extract_episode_from_updated(body)
    }
    //https://www.mangaworld.mx/archive?keyword=
    fn extract_episode_from_search(&self, body: String) -> Result<Vec<crate::source::Episode>, AsyncError> {
        let document = Html::parse_document(&body);
        let sel_inner = Selector::parse("div.entry").unwrap();
        // let sel_episode_number = Selector::parse("a.xanh").unwrap();
        let sel_anime_name = Selector::parse("a.manga-title").unwrap();
        let cards = document.select(&sel_inner)
        .filter_map(|card| {
            // let carde = card.select(&sel_episode_number).next()?;
            // let index = carde.inner_html().trim().split(" ").last().unwrap().parse::<usize>().unwrap_or(0);
                // log!("index: {}", index);
            let carda = card.select(&sel_anime_name).next()?;

            // log!("Card a found");
            let url = carda.value().attr("href").unwrap_or("").trim().to_owned();
            // log!("Card url: {}",url);
            let title = carda.inner_html().trim().to_owned();
            // log!("Card title: {}", title);
            Some(Episode {
                title,
                index: 0,
                url,
                section: 0
            })
        });
        Ok(cards.collect::<Vec<Episode>>())
    }
    //https://www.mangaworld.ac/
    fn extract_episode_from_updated(&self, body: String) -> Result<Vec<crate::source::Episode>, AsyncError> {
        let document = Html::parse_document(&body);
        let sel_inner = Selector::parse("div.entry").unwrap();
        let sel_episode_number = Selector::parse("a.xanh").unwrap();
        let sel_anime_name = Selector::parse("a.manga-title").unwrap();
        let cards = document.select(&sel_inner)
        .filter_map(|card| {
            let carde = card.select(&sel_episode_number).next()?;
            let index = carde.inner_html().trim().split(" ").last().unwrap().parse::<usize>().unwrap_or(0);
            let url = carde.value().attr("href").unwrap_or("").trim().to_owned();
                // log!("index: {}", index);
            let carda = card.select(&sel_anime_name).next()?;

            // log!("Card a found");
            // log!("{}\t{}\t{}\t{}");
            // log!("Card url: {}",url);
            let title = carda.inner_html().trim().to_owned();
            // log!("Card title: {}", title);
            Some(Episode {
                title,
                index,
                url,
                section: 0
            })
        });
        Ok(cards.collect::<Vec<Episode>>())
    }

    async fn fetch_search_page(&self, query: String, page: usize) -> Result<String, AsyncError> {
        let u = format!("{}/archive?keyword={}&page={}",self.get_url(), query, page);
        let r = reqwest::get(u).await?.error_for_status()?.text().await?;
        return Ok(r);
    }
    
    

    async fn fetch_updated_page(&self, page: usize)-> Result<String, AsyncError> {
        let p = std::cmp::max(1, page);
        let u = format!("{}?page={}",self.get_url(), p);
        let a = get(u).await?.error_for_status()?.text().await?;
        return Ok(a);
    }
    async fn lib_url_parse(&self, url: String) -> Result<HashMap<String, EpisodeListItem>,  AsyncError> {
        let mut hs = HashMap::<String, EpisodeListItem>::new();
        let u = if url.starts_with(&self.get_url()) {url} else {format!("{}{}",self.get_url(), url)};
        if let Ok(a) = get(u).await?.error_for_status()?.text().await {
            let b = Html::parse_document(&a);
            let select_volume_element = Selector::parse(".chapters-wrapper .volume-element").unwrap();
            let select_volume_name = Selector::parse(".volume .volume-name").unwrap();
            let select_episodes = Selector::parse("div.chapter a.chap").unwrap();
            let select_span = Selector::parse("span").unwrap();
            
            for vol in b.select(&select_volume_element) {
                if let Some(volname) = vol.select(&select_volume_name).next() {
                    let vv:Vec<&str> = volname.text().collect();
                    let volume = vv.join(" ");
                    for ep in vol.select(&select_episodes) {
                        if let Some(span) = ep.select(&select_span).next() {
                            let alink = ep.value();
                            let chapter = span.inner_html();
                            let format_part = |input: &str, prefix: &str|-> String {
                                input.split_whitespace().nth(1)
                                    .map(|num_str| {
                                        match num_str.split_once('.') {
                                            Some((a, b)) if !b.is_empty() => {
                                                format!("{} {:>4}.{}", prefix, a, b)
                                            },
                                            Some((_, _)) | None => {
                                                let m = num_str.trim_end_matches('.');
                                                format!("{} {:>4}", prefix, m)
                                            }
                                        }
                                    })
                                    .unwrap_or_default()
                            };
                            let title_vol = format_part(&volume, "Volume");
                            let title_cha = format_part(&chapter, "Capitolo");
                            let title = vec![title_vol, title_cha]
                            .into_iter()
                            .filter(|s| !s.is_empty())
                            .collect::<Vec<_>>()
                            .join(" ");
                            hs.insert(title.clone(), EpisodeListItem {
                                title,
                                url:  alink.attr("href").unwrap_or("").trim().to_owned()
                            });
                        }
                    }
                }
            }
        }
        return Ok(hs);
    }

    
    async fn extract_pages_url(&self, chapter_url: String) -> Result<Vec<String>, AsyncError>  {
        let u = if chapter_url.starts_with(&self.get_url()) {chapter_url} else {format!("{}{}",self.get_url(), chapter_url)};
        let url = format!("{}/1?style=list", u);
        let a = get(url).await?.error_for_status()?.text().await?;
        let b = Html::parse_document(&a);
        let sel = Selector::parse("#page img").unwrap();
        let pages =b.select(&sel).filter_map(|img| img.value().attr("src")).map(|x| x.to_owned());
        Ok(pages.collect())
    }

    async fn to_pdf(&self, metadata: MangaMetadata, images:Vec<String>)  -> Result<String, AsyncError> {
        Ok("".to_owned())
    }
    async fn to_cbz(&self, metadata: MangaMetadata, images:Vec<String>)  -> Result<String, AsyncError> {
        let bp = format!("./downloads/manga/{}/{}", metadata.title.unwrap_or("manga".to_owned()).replace(".", "_").replace("/", "_").to_ascii_lowercase(),metadata.chapter.unwrap_or("none".to_owned()));
        let cbz = MangaCbz::new(bp.clone(), false)?;
        let mut i:usize = 0;
        std::fs::create_dir_all(&bp)?;
        for img in images {
            let mut h = Hasher::new();
            let png = get(img).await?.bytes().await?;
            h.update(&png); // TODO
            let hs= h.finalize();
            let b = cbz.has_page(i,hs);
            log!("has page i={} b={}",i,b);
            if !b {
                let image_path = format!("{}/{}.png",bp,i);
                std::fs::File::create(&image_path)?;
                std::fs::write(&image_path, png)?;
                cbz.add_page_path(image_path.clone())?;
                std::fs::remove_file(image_path)?;
            }
            i=i+1;
        }
        std::fs::remove_dir_all(&bp)?;
        Ok(cbz.path)
    }
    // async fn chapterMetadata(&self, url: String) -> MangaMetadata {
    //     //url = https://www.mangaworld.mx/manga/1637/chainsaw-man/read/5f9cc1d81db7241f07086b78
    //     if let Ok(res) = get(&url).await {
    //         if let Ok(body) = res.text().await {
    //             let html = Html::parse_document(&body);
    //             let reader_a = Selector::parse("#reader a").unwrap();
    //             let volopt = Selector::parse(".volume option").unwrap();
    //             let chaopt = Selector::parse(".chapter option").unwrap();
    //             let title = match html.select(&reader_a).next() {
    //                 Some(t) => {
    //                     let a:Vec<&str> = t.text().collect();
    //                     let b = a.join(" ");
    //                     let c = b.clone();
    //                     let prefix = "Ritorna a";
    //                     Some(b.clone().strip_prefix(prefix).map(|f| f.to_owned()).unwrap_or(c))
    //                 },
    //                 None => None,
    //             };
    //             let cv_num = |a: ElementRef| {
    //                 let v:Vec<&str> = a.text().collect();
    //                 let vs = v.join(" ");
    //                 let last = vs.split(" ").last();
    //                 last.map(|n| n.trim_start_matches("0").to_owned())
    //             };
    //             let volume = html.select(&volopt).find(|e| e.attr("selected").is_some()).and_then(cv_num);
    //             let chapter = html.select(&chaopt).find(|e| e.attr("selected").is_some()).and_then(cv_num);
    //             return MangaMetadata {
    //                 title,
    //                 url: Some(url),
    //                 chapter: chapter,
    //                 volume: volume
    //             };
    //         }
    //     }
        
    //     MangaMetadata { title: None, url: Some(url), chapter: None, volume: None }
    // }
}

