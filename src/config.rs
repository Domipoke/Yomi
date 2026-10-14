use std::{default, fs, sync::{LazyLock, Mutex, RwLock}};
use ratatui::style::palette::tailwind;
use serde::{Deserialize, Deserializer};
use std::sync::OnceLock;

use crate::{log, player::Player::{AnimePlayer, Vlc}, utils::utils::ColorPickerOption};

#[cfg(feature = "remote")]
use crate::player::Player::Vlcr;



#[derive(Deserialize, Clone)]
pub struct Config {
    pub Anime: AnimeSettings,
    pub Manga: MangaSettings,
    pub Theme: ThemeSettings,
    pub General: GeneralSettings,
    pub Device: Vec<DeviceSettings>
}
#[derive(Deserialize, Clone)]
pub struct DeviceSettings {
    pub name: String,
    pub user: String,
    pub host: String,
    pub key: String,
}


#[derive(Deserialize, Clone)]
pub struct AnimeSettings {
    pub source: String,
    #[serde(deserialize_with = "anime_player_deserialize")]
    pub player: AnimePlayerEnum
}
#[derive(Deserialize, Clone)]
pub struct GeneralSettings {
    pub askToQuit: bool,
    pub defaultDevice: String
}

#[derive(Deserialize, Clone)]
pub struct MangaSettings {
    #[serde(deserialize_with = "manga_reader_deserialize")]
    pub reader: MangaReader,
    pub source: String,
    pub programCmd: String,
    pub programArg: String
}
#[derive(Deserialize, Clone)]
pub enum MangaReader {
    Sumatra, PDF, CBZ, CbzReader, Calibre
}

#[derive(Deserialize, Clone)]
pub enum AnimePlayerEnum {
    VLC, 
    #[cfg(feature = "remote")]
    VLCR
}
impl MangaReader {
    pub fn to_str(&self) -> &str {
        match self {
            MangaReader::CBZ => "CBZ",
            MangaReader::Calibre => "Calibre",
            MangaReader::CbzReader => "CbzReader",
            MangaReader::PDF => "PDF",
            MangaReader::Sumatra => "Sumatra"
        }
    }
}
impl AnimePlayerEnum {
    pub fn to_str(&self) -> &str {
        match self {
            AnimePlayerEnum::VLC => "vlc",
            #[cfg(feature = "remote")]
            AnimePlayerEnum::VLCR => "vlcr",
        }
    }
    pub fn buildPlayer(&self, selectedDevice: Option<DeviceSettings>) -> Box<dyn AnimePlayer + Send> {
        match self {
            AnimePlayerEnum::VLC => Box::new(Vlc {cmd: "vlc".to_owned(), args: vec![]}),
            #[cfg(feature = "remote")]
            AnimePlayerEnum::VLCR => {
                return Box::new(Vlcr {
                    dev: selectedDevice,
                    tx: None
                });
            },
        }
    }
}
fn manga_reader_deserialize<'de,D>(deserializer: D) -> Result<MangaReader, D::Error> where D: Deserializer<'de> {
    let s = String::deserialize(deserializer)?.to_lowercase();
    match s.as_str() {
        "sumatra" => Ok(MangaReader::Sumatra),
        "pdf" => Ok(MangaReader::PDF),
        "cbz" => Ok(MangaReader::CBZ),
        "calibre" => Ok(MangaReader::Calibre),
        "cbzreader" => Ok(MangaReader::CbzReader),
        _ => {
            log!("Invalid manga reader, using default option: PDF");
            return Ok(MangaReader::PDF);
        }
    }
}
fn anime_player_deserialize<'de,D>(deserializer: D) -> Result<AnimePlayerEnum, D::Error> where D: Deserializer<'de> {
    let s = String::deserialize(deserializer)?.to_lowercase();
    match s.as_str() {
        "vlc" => Ok(AnimePlayerEnum::VLC),
        #[cfg(feature = "remote")]
        "vlcr" => Ok(AnimePlayerEnum::VLCR),
        _ => {
            log!("Invalid manga reader, using default option: PDF");
            return Ok(AnimePlayerEnum::VLC);
        }
    }
}



static CONFIG: LazyLock<RwLock<Config>> = LazyLock::new(|| {
    RwLock::new(load_config())
});
static DEVICE: LazyLock<Mutex<Option<DeviceSettings>>> = LazyLock::new(|| Mutex::new(
    None
));
pub fn getDevice()-> Option<DeviceSettings> {
    match DEVICE.lock() {
        Ok(x) => x.clone(),
        _ => None,
    }
}
pub fn setDevice(dev: Option<DeviceSettings>) {
    match DEVICE.lock() {
        Ok(mut x) => {
            *x = dev;
        },
        Err(_) => return,
    }
}
pub fn getConfig() -> Config {
    let lock = CONFIG.read().unwrap();
    lock.clone()
}


pub fn load_config() -> Config {
    let content = fs::read_to_string("config.toml").expect("Can't read config file");
    let parsed:Config = toml::from_str(&content).unwrap();
    if let Some(x) = parsed.Device.iter().find(|x| x.name == parsed.General.defaultDevice) {
        setDevice(Some(x.clone()));
    }
    parsed
}


pub fn tailwindPaletteFromString(c: &str, def: tailwind::Palette) -> tailwind::Palette {
    let res = match c {
        "AMBER" => tailwind::AMBER,
        "BLUE" => tailwind::BLUE,
        "CYAN" => tailwind::CYAN,
        "EMERALD" => tailwind::EMERALD,
        "FUCHSIA" => tailwind::FUCHSIA,
        "GRAY" => tailwind::GRAY,
        "GREEN" => tailwind::GREEN,
        "INDIGO" => tailwind::INDIGO,
        "LIME" => tailwind::LIME,
        "NEUTRAL" => tailwind::NEUTRAL,
        "ORANGE" => tailwind::ORANGE,
        "PINK" => tailwind::PINK,
        "PURPLE" => tailwind::PURPLE,
        "RED" => tailwind::RED,
        "ROSE" => tailwind::ROSE,
        "SKY" => tailwind::SKY,
        "SLATE" => tailwind::SLATE,
        "STONE" => tailwind::STONE,
        "TEAL" => tailwind::TEAL,
        "VIOLET" => tailwind::VIOLET,
        "YELLOW" => tailwind::YELLOW,
        "ZINC" => tailwind::ZINC,
        _ => def
    };
    res
}

pub fn setMangaSource(v:String) {
    let mut conf = CONFIG.write().unwrap();
    conf.Manga.source = v
}

pub fn setAnimeSource(v:String) {
    let mut conf = CONFIG.write().unwrap();
    conf.Anime.source = v
}

#[derive(Deserialize, Clone)]
pub struct Color {
    red: u8,
    green: u8,
    blue: u8,
    opacity: u8
}
impl Color {
    pub fn to_u32(&self) -> u32 {
        ((self.opacity as u32) << 24) |
        ((self.red     as u32) << 16) |
        ((self.green   as u32) << 8)  |
        (self.blue    as u32)
    }
    pub fn to_ratatui(&self) -> ratatui::style::Color {
        // log!("{}", self.to_u32());
        let x = ratatui::style::Color::from_u32(self.to_u32());
        x
    }
}
pub fn color_deserialize<'de,D>(deserializer: D) -> Result<Color, D::Error> where D: Deserializer<'de> {
    let s = String::deserialize(deserializer)?.to_lowercase();
    // log!("{:?}",s.get(1..3));

    let hexred = match s.get(1..3) {
        Some(res)=>match u8::from_str_radix(res, 16){
            Ok(r) => r,
            Err(_) => {
                log!("Invalid hex color");
                0
            },
        },
        None => {
            log!("Invalid hex color");
            0
        }
    };
    let hexgre = match s.get(3..5) {
        Some(res)=>match u8::from_str_radix(res, 16){
            Ok(r) => r,
            Err(_) => {
                log!("Invalid hex color");
                0
            },
        },
        None => {
            log!("Invalid hex color");
            0
        }
    };
    let hexblu = match s.get(5..7) {
        Some(res)=>match u8::from_str_radix(res, 16){
            Ok(r) => r,
            Err(_) => {
                log!("Invalid hex color");
                0
            },
        },
        None => {
            log!("Invalid hex color");
            0
        }
    };
    // log!("{} {} {}", hexred, hexgre, hexblu);
    return Ok(Color {
        red: hexred,
        green: hexgre,
        blue: hexblu,
        opacity: 255
    })
}

#[derive(Deserialize, Clone)]
pub struct ThemeSettings {
    #[serde(deserialize_with = "color_deserialize")]
    pub base_border: Color,
    #[serde(deserialize_with = "color_deserialize")]
    pub anime_border: Color,
    #[serde(deserialize_with = "color_deserialize")]
    pub manga_border: Color,
    #[serde(deserialize_with = "color_deserialize")]
    pub menu_text: Color,
    #[serde(deserialize_with = "color_deserialize")]
    pub menu_selection: Color,
    #[serde(deserialize_with = "color_deserialize")]
    pub res_border: Color,
    #[serde(deserialize_with = "color_deserialize")]
    pub res_text: Color,
    #[serde(deserialize_with = "color_deserialize")]
    pub res_selection: Color,
    #[serde(deserialize_with = "color_deserialize")]
    pub cmd_text: Color,
    #[serde(deserialize_with = "color_deserialize")]
    pub status_loading: Color,
    #[serde(deserialize_with = "color_deserialize")]
    pub cmd_keys: Color,
    #[serde(deserialize_with = "color_deserialize")]
    pub list_border: Color,
}


