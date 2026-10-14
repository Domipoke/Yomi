use std::{format, fs, vec};

use std::result::Result::Ok;
use serde::Deserialize;

use crate::components::Library;
use crate::config::getConfig;
use crate::library::LibraryItem;
use crate::source::mangago::Mangago;
use crate::source::{AnimeSource, Episode, MangaSource, SourceType};
use crate::source::animeworld::Animeworld;
use crate::source::mangaworld::Mangaworld;

#[derive(Deserialize, Clone)]
pub struct Extension {
    pub Type: String,
    pub Info: ExtensionInfo,
    pub Sitemanager: ExtensionSitemanager
}
#[derive(Deserialize, Clone)]
pub struct ExtensionInfo {
    pub Name: String,
    pub Description: String,
    pub Language: String
}

#[derive(Deserialize, Clone)]
pub struct ExtensionSitemanager {
    pub Hostname: String,
    pub Url: String,

}

impl Extension {
    pub fn buildfromName(kind: SourceType,name: &str) -> Result<Extension, &str> {
        let f = format!("extensions/{}/{}.toml",kind.to_string(), name.to_lowercase());
        if let Ok(exists)= fs::exists(&f) {
            if exists {
                if let Ok(content)= fs::read_to_string(&f) {
                    let parsed:Extension = toml::from_str(&content).unwrap();
                    return Ok(parsed);
                }
            }
        }
        return Err(name); 
    }
    pub fn lsSource(kind: SourceType) -> Result<Vec<String>, String> {
        let f = format!("extensions/{}",kind.to_string());
        if let Ok(list) = fs::read_dir(&f) {
            return Ok(list.filter_map(Result::ok) 
            .filter(|entry| {
                entry.file_type().map(|ft| ft.is_file()).unwrap_or(false)
            })
            .filter_map(|entry| {
                let name = entry.file_name().into_string().ok()?;
                if name.ends_with(".toml") {
                    Some(name.strip_suffix(".toml")?.to_string())
                } else {
                    None
                }
            })
            .collect());
        }
        return Err("Ls not possible".to_owned());
    }
    pub fn toAnimeSource(self) -> Box<dyn AnimeSource+ Send + Sync> {
        match self.Info.Name.trim().to_lowercase().as_str() {
            // "animeworld" => Animeworld::new(self.Sitemanager.Hostname, self.Sitemanager.Url),
            _ => Box::new(Animeworld::new(self.Sitemanager.Hostname, self.Sitemanager.Url))
        }
    }
    pub fn toMangaSource(self) -> Box<dyn MangaSource+ Send + Sync> {
        match self.Info.Name.trim().to_lowercase().as_str() {
            // "animeworld" => Animeworld::new(self.Sitemanager.Hostname, self.Sitemanager.Url),
            "mangago" => Box::new(Mangago::new(self.Sitemanager.Hostname, self.Sitemanager.Url)),
            _ => Box::new(Mangaworld::new(self.Sitemanager.Hostname, self.Sitemanager.Url)),
        }
    }
    pub fn defaultAnimeSource() -> Result<Box<dyn AnimeSource>, String> {
        let def = &getConfig().Anime.source;
        if let Ok(ext) = Extension::buildfromName(SourceType::Anime, def) {
            return Ok(ext.toAnimeSource());
        }
        return Err(format!("No default file for {}",def));
    }
    pub fn defaultMangaSource() -> Result<Box<dyn MangaSource>, String> {
        let def = &getConfig().Manga.source;
        if let Ok(ext) = Extension::buildfromName(SourceType::Manga, def) {
            return Ok(ext.toMangaSource());
        }
        return Err(format!("No default file for {}",def));
    }
}

