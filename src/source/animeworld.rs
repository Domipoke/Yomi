
use std::{collections::HashMap, format, process::{Command, Stdio}};

use reqwest::get;
use scraper::{Html, Selector};

use crate::{config::{getConfig, getDevice}, source::{AnimeSource, Episode, EpisodeListItem, SourceType}, utils::utils::AsyncError};

pub struct Animeworld {
    url: String,
    hostname:String,
    sourcetype: SourceType
}

impl Animeworld {
    fn id_extract( s:&str) -> Option<&str> {
        s.split("/").last()
    }
}
#[async_trait::async_trait]
impl AnimeSource for Animeworld {
    fn has_update_page(&self) -> bool {true}
    fn get_hostname(&self)->String {
        self.hostname.clone()
    }
    fn get_url(&self) -> String {
        self.url.clone()
    }
    
    fn new(hostname: String, url: String) -> Self {
        Self {
            url,
            hostname,
            sourcetype: crate::source::SourceType::Anime
        } 
    }
    async fn listupdated(&self, page: usize) -> Result<Vec<crate::source::Episode>, AsyncError> {
        let body = self.fetch_updated_page(page).await?;
        self.extract_episode_from_updated(body)
    }
    fn extract_episode_from_search(&self, body: String) -> Result<Vec<crate::source::Episode>, AsyncError> {
        let document = Html::parse_document(&body);
        let sel_filmlist = Selector::parse("div.film-list").unwrap();
        let sel_index = Selector::parse("div.film-list").unwrap();
        let sel_inner = Selector::parse("div.inner").unwrap();
        let sel_episode_number = Selector::parse("div.ep").unwrap();
        let sel_anime_name = Selector::parse("a.name").unwrap();
        if let Some(filmls) = document.select(&sel_filmlist).nth(0) {
            let cards = filmls.select(&sel_inner)
            .filter_map(|card| {
                // log!("index: {}", index);
                let carda = card.select(&sel_anime_name).next()?;
                let index = card.select(&sel_episode_number).next();
                // log!("Card a found");
                let url = carda.value().attr("href").unwrap_or("").trim().to_owned();
                // log!("Card url: {}",url);
                let title = carda.inner_html().trim().to_owned();
                // log!("Card title: {}", title);
                Some(Episode {
                    title,
                    index: index.and_then(|f| Some(f.inner_html().trim().split(" ").last().unwrap().parse::<usize>().unwrap_or(0))).unwrap_or(0),
                    url,
                    section: 0
                })
            });
            Ok(cards.collect::<Vec<Episode>>())
        } else {
            Ok(Vec::<Episode>::new())
        }
    }
    fn extract_episode_from_updated(&self, body: String) -> Result<Vec<crate::source::Episode>, AsyncError> {
        self.extract_episode_from_search(body)
    }

    
    async fn episode_openable_link(&self, card: Episode) -> Result<String, Box<dyn std::error::Error>> {
        let id= Self::id_extract(&card.url).unwrap();
        let u = format!("{}/api/episode/serverPlayerAnimeWorld?id={}", self.url, id);
        let r = get(u).await?.error_for_status()?.text().await?;
        let b = Html::parse_document(&r);
        let select_source = Selector::parse("source").unwrap();
        Ok(b.select(&select_source).next().unwrap().value().attr("src").unwrap_or("").to_owned())
    }
    async fn episode_play_url(&self, url: &str) -> Result<String, Box<dyn std::error::Error>> {
        if let Some(id) = Self::id_extract(url) {
            let u = format!("{}/api/episode/serverPlayerAnimeWorld?id={}", self.url, id);
            match Command::new("curl")
                .arg("-L")
                .arg(u).output() {
                Ok(output) => {
                if  let Ok(html) = String::from_utf8(output.stdout) {
                        let b = Html::parse_document(&html);
                        let select_source = Selector::parse("source").unwrap();
                        if let Some(c) = b.select(&select_source).next() {
                            return Ok(c.attr("src").unwrap_or("").to_owned());
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(String::new())

    }
    async fn fetch_search_page(&self, query: String, page: usize) -> Result<String, AsyncError> {
        let u = format!("{}/search?keyword={}&page={}",self.get_url(), query, page);
        let r = reqwest::get(u).await?.error_for_status()?.text().await?;
        return Ok(r);
    }
    
    async fn search(&self, query: String, page: usize) -> Result<Vec<crate::source::Episode>, AsyncError>{
        let body = self.fetch_search_page(query, page).await?;
        self.extract_episode_from_search(body)
    }

    async fn fetch_updated_page(&self, page: usize)-> Result<String, AsyncError> {
        let p = std::cmp::max(1, page);
        let u = format!("{}/updated?page={}",self.get_url(),p);
        let a = get(u).await?.error_for_status()?.text().await?;
        return Ok(a);
    }

    async fn lib_url_parse(&self, url: String) -> Result<HashMap<String, EpisodeListItem>,  AsyncError>{
        let mut hs = HashMap::<String, EpisodeListItem>::new();
        let u = if url.starts_with(&self.get_url()) {url} else {format!("{}{}",self.get_url(), url)};
        if let Ok(a) = get(u).await?.error_for_status()?.text().await {
            let b = Html::parse_document(&a);
            let select_episodes = Selector::parse("li.episode a").unwrap();
            for ep in b.select(&select_episodes) {
                let alink = ep.value();
                hs.insert(ep.inner_html(), EpisodeListItem { 
                    title: format!("Ep. {:>4}",ep.inner_html()),
                    url: alink.attr("href").unwrap_or("").trim().to_owned()
                });
            }
        }
        return Ok(hs);
    }
    async fn lib_ep_select(&self, eli: EpisodeListItem) -> Result<(),  AsyncError> {
        if let Some(url) = Self::id_extract(&eli.url) {
            let mut player = getConfig().Anime.player.buildPlayer(getDevice());
            if let Ok(vlc_url) =self.episode_play_url(url).await {
                if !vlc_url.is_empty() {
                    let _ = player.play(&vlc_url);
                }
            }
        }
        Ok(())
    }
}