use std::collections::{HashMap, HashSet};
use std::{format, fs};

use std::result::Result::Ok;
use serde::{Deserialize, Serialize};

use crate::extension::Extension;
use crate::source::{Episode, SourceType};



#[derive(Deserialize,Clone, Serialize, PartialEq, Eq)]
pub struct LibraryItem {
    pub name: String, 
    pub source: String,
    pub ext: String
}


impl LibraryItem {
    pub fn buildfromName(name: &str) -> Result<HashMap<String, Vec<LibraryItem>>, &str> {
        let f = format!("library/{}", name);
        if let Ok(exists)= fs::exists(&f) {
            if exists {
                if let Ok(content)= fs::read_to_string(&f) {
                    let mut data: HashMap<String, Vec<LibraryItem>> = toml::from_str(&content).unwrap();
                    for items in data.values_mut() {
                        let mut hs = HashSet::new();
                        items.retain(|item| {
                            hs.insert(item.name.clone())
                        });
                        items.sort_by(|a,b| {
                            a.name.cmp(&b.name)
                        });
                    }
                    return Ok(data);
                }
            }
        }
        return Err(name); 
    }
}

pub fn addToFavourite(kind: SourceType, what:&Episode, source: &String) {
    let toml = match kind {
        SourceType::Anime => "animelibrary.toml",
        SourceType::Manga => "mangalibrary.toml",
    };
    if let Ok(mut data) = LibraryItem::buildfromName(toml) {
        if !data.values().any(|x| x.iter().any(|y| y.source == what.url)) {
            // add
            if let Ok(ext) = Extension::buildfromName(kind, source) {
                let libitem = LibraryItem { name: what.title.clone(), source: what.url.clone(), ext: source.clone() };
                if let Some(v) = data.get_mut(&ext.Info.Name) {
                    v.push(libitem);
                } else {
                    data.insert("addToFavourite".to_string(), vec![libitem]);
                }
                if let Ok(s) = toml::to_string_pretty(&data) {
                    fs::write(format!("library/{}", toml), s);
                }
            }
        }
    }
}