use std::{ffi::OsString, fs::{create_dir_all, read_dir}};
use serde::Deserialize;

// const DOWNLOADS:    &str = "/downloads";
const ANIME_FOLDER: &str = "./downloads/anime";
const MANGA_FOLDER: &str = "./downloads/manga";

pub fn init_folders() {
    let _ = create_dir_all(ANIME_FOLDER);
    let _ = create_dir_all(MANGA_FOLDER);    
}


pub fn list_folder(folder: &str) -> Result<Vec<OsString>, Box<dyn std::error::Error>> {
    let r = read_dir(folder)?
        .filter_map(|entry| entry.ok())        
        .filter(|entry| entry.path().is_dir())    
        .map(|entry| entry.file_name())           
        .collect::<Vec<OsString>>();
    Ok(r)
}
pub fn read_anime_folder() -> Result<Vec<OsString>, Box<dyn std::error::Error>> {
    list_folder(ANIME_FOLDER)
}
pub fn read_manga_folder() -> Result<Vec<OsString>, Box<dyn std::error::Error>> {
    list_folder(MANGA_FOLDER)
}

#[derive(Deserialize)]
pub struct Config {
    pub Settings: Settings,
    pub Extension: Extensions,
}


#[derive(Deserialize)]
pub struct Settings {
    pub Folders: Folders
}
#[derive(Deserialize)]
pub struct Extensions {
    pub defaultanime: String,
    pub defaultmanga: String
}
#[derive(Deserialize)]
pub struct Folders {
    pub basedir: String,
    pub extensions: String,
    pub anime: String,
    pub manga: String
}
#[derive(Deserialize)]
pub struct ColorPalette {
    pub anime: String,
    pub manga: String,
    pub source: String,
    pub settings: String
}




// impl Config  {
//     pub fn defaultAnimeSource() -> Source {
//         Animeworld::new(hostname, url, sourcetype)
//     }
// }