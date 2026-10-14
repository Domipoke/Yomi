#[cfg(test)]
mod tests {
    use tokio::runtime::Runtime;

use super::*;
    use crate::{config::{Config, getConfig}, extension::Extension, source::{MangaSource, SourceType, mangago::Mangago}};
    
    #[test]
    fn mangago() {
        let r:Runtime = tokio::runtime::Runtime::new().expect("No runtime?");
        let a:Extension = Extension::buildfromName(SourceType::Manga, "mangago").expect("Can't build");
        let b:Box<dyn MangaSource + Send + Sync> = a.toMangaSource();
        let x:String = r.block_on(b.fetch_updated_page(0)).expect("fetch Update");
        let y = b.extract_episode_from_updated(x).expect("Error");
        println!("{}",y.len());
        assert!(y.len()>0, "No elements found");
    }
}