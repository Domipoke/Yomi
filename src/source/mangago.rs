use std::{collections::HashMap, f64::consts::E, fmt::format, format, io::Read, iter::Once, sync::OnceLock};
use base64::{Engine, engine::general_purpose};
use crc32fast::Hasher;
use hex::decode;
use regex::Regex;
use reqwest::{Client, Response, get, header::{HeaderMap, HeaderValue}};
use scraper::{Element, Html, Selector};
use anyhow::{Context, Result};
use zip::write::SimpleFileOptions;

use crate::{log, source::{Episode, EpisodeListItem, MangaMetadata, MangaSource, SourceType}, utils::utils::{AsyncError, MangaCbz}};
const UA: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/146.0.0.0 Safari/537.36";

pub static MANGAGO_CLIENT: OnceLock<Client> = OnceLock::new();
fn get_client() -> Client {
    MANGAGO_CLIENT.get_or_init(|| {
        Client::builder()
            .pool_max_idle_per_host(1)
            .cookie_store(true)
            .user_agent(UA)
        .build().unwrap_or(Client::new())
    }).clone()
}
async fn fetch(url: String) -> Result<String, reqwest::Error> {
    let cl = get_client();
    cl.get(url).send().await?.error_for_status()?.text().await
}

pub struct IndexUrl {
    index: usize,
    url: String
}
pub struct Mangago {
    url: String,
    hostname:String,
    sourcetype: SourceType
}


#[async_trait::async_trait]
impl MangaSource for Mangago {
    // mangago.me
    // https://www.mangago.me
    fn new(hostname:String,url:String) -> Self {
        Self { url, hostname, sourcetype: crate::source::SourceType::Manga }
    }

    fn get_hostname(&self) -> String {
        self.hostname.clone()
    }

    fn get_url(&self) -> String {
        self.url.clone()
    }
    fn has_update_page(&self) -> bool {
        true
    }
    async fn listupdated(&self, page: usize) -> Result<Vec<crate::source::Episode>, AsyncError> {
        let body = self.fetch_updated_page(page).await?;
        self.extract_episode_from_updated(body)
    }
    // https://www.mangago.me/r/l_search/?name=query
    fn extract_episode_from_search(&self, body: String) -> Result<Vec<crate::source::Episode>, AsyncError> {
        let document = Html::parse_document(&body);
        let sel_inner = Selector::parse("#search_list li").unwrap();
        // let sel_episode_number = Selector::parse("a.xanh").unwrap();
        let sel_anime_name = Selector::parse(".tit a").unwrap();
        let cards = document.select(&sel_inner)
        .filter_map(|card| {
            // let carde = card.select(&sel_episode_number).next()?;
            // let index = carde.inner_html().trim().split(" ").last().unwrap().parse::<usize>().unwrap_or(0);
                // log!("index: {}", index);
            let carda = card.select(&sel_anime_name).last()?;

            // log!("Card a found");
            let url = carda.value().attr("href").unwrap_or("").trim().to_owned();
            // log!("Card url: {}",url);
            let title = carda.text().filter_map(|e| {
                match e.is_empty() {
                    true => None,
                    false => Some(e.trim().to_owned()),
                }
            }).collect::<Vec<String>>().join(" ");
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
    // https://www.mangago.me/list/latest/all/p/
    fn extract_episode_from_updated(&self, body: String) -> Result<Vec<crate::source::Episode>, AsyncError> {
        let document = Html::parse_document(&body);
        let sel_inner = Selector::parse("#search_list li").unwrap();
        let sel_anime_name = Selector::parse("a").unwrap();
        let sel_a_chapt = Selector::parse("div.row-5 a").unwrap();
        let chico = Selector::parse(".chicho").unwrap();
        let mut section = 0;
        let cards = document.select(&sel_inner)
        .filter_map(|card| {
            // let carde = card.select(&sel_episode_number).next()?;
            // let index = carde.inner_html().trim().split(" ").last().unwrap().parse::<usize>().unwrap_or(0);
                // log!("index: {}", index);
            let mut carda = card.select(&sel_anime_name);
            let info = carda.next()?;
            let title = info.value().attr("title").unwrap_or("").trim();
            let mut url = "";
            let episodes = match card.select(&sel_a_chapt).next() {
                Some(ele) => {
                    url=ele.value().attr("href").unwrap_or("");
                    let inner = ele.inner_html();
                    let ret = inner.rsplitn(2, ".");
                    let retu:Vec<usize> = ret.filter_map(|f| {
                        let temp = f.trim();
                        temp.parse::<usize>().ok()
                    }).collect();
                    if (retu.len()> 2) {
                        section = retu[2];
                        Some(retu[1])
                    } else if (retu.len() == 2) {
                        Some(retu[1])
                    } else if (retu.len() > 0) {
                        Some(retu[0])
                    } else {
                        None
                    }
                }
                _ => None
            };
            
            // let mut url = "";
            // let mut index = 0;
            // match card.select(&chico).next() {
            //     Some(lastindex) => {
            //         url = lastindex.value().attr("href").unwrap_or("").trim();
            //         match lastindex.inner_html().to_owned().rsplit_once(".") {
            //             Some(a) => {
            //                 let indexstr = a.1;
            //                 match indexstr.trim().parse::<usize>() {
            //                     Ok(b) => {index = b;},
            //                     _ => {}
            //                 }
            //             },
            //             _ => {},
            //         }
            //     },
            //     _=>{}
            // };
            // println!("url: {}, index: {}", url, index);
            // let title = carda.text().filter_map(|e| {
            //     match e.is_empty() {
            //         true => None,
            //         false => Some(e.trim().to_owned()),
            //     }
            // }).collect::<Vec<String>>().join(" ");
            // log!("Card title: {}", title);
            // println!("title: {}", title);
            Some(Episode {
                title: title.to_string(),
                index: episodes.unwrap_or(0),
                url: url.to_string(),
                section: section
            })
        });
        Ok(cards.collect::<Vec<Episode>>())
    }
    async fn fetch_search_page(&self, query: String, page: usize) -> Result<String, AsyncError> {
        let cl = reqwest::Client::new();
        let p = std::cmp::max(1, page);
        let u = format!("{}/r/l_search/?name={}&page={}",self.get_url(), query, p);
        let r = cl.get(u).header("user-agent", UA).send().await?.error_for_status()?.text().await?;
        return Ok(r);
    }
    async fn fetch_updated_page(&self, page: usize)-> Result<String, AsyncError> {
        let cl = reqwest::Client::new();
        let p = std::cmp::max(1, page);
        let u = format!("{}/list/latest/all/{}",self.get_url(), p);
        let a = cl.get(u).header("user-agent", UA).send().await?.error_for_status()?.text().await?;
        return Ok(a);
    }
    // https://www.mangago.me/read-manga/tongari_boushi_no_atelier/
    async fn lib_url_parse(&self, arg_url: String) -> Result<HashMap<String, EpisodeListItem>,  AsyncError> {
        let cl = reqwest::Client::new();
        let mut hs = HashMap::<String, EpisodeListItem>::new();
        let u = if arg_url.starts_with(&self.get_url()) {arg_url} else {format!("{}{}",self.get_url(), arg_url)};
        if let Ok(a) = cl.get(u).header("user-agent", UA).send().await?.error_for_status()?.text().await {
            let b = Html::parse_document(&a);
            let table_row_sel = Selector::parse("#chapter_table tr").unwrap();
            let chico = Selector::parse(".chico").unwrap();
            let table_row = b.select(&table_row_sel);
            for tr in table_row {
                if let Some(a) = tr.select(&chico).next() {
                    let url = a.value().attr("href").unwrap_or("").trim().to_owned();
                    let title: String = a.text().collect();
                    
                    let k = title.split_whitespace().find(|ch| ch.to_lowercase().starts_with("ch."))
                        .and_then(|m| m.split_once(".")).and_then(|s| Some(s.1.to_owned()));

                    hs.insert(k.unwrap_or("af".to_string()), EpisodeListItem {
                        title,
                        url
                    });
                }
            }
        }
        return Ok(hs);
    }
    // TODO: https://claude.ai/chat/a3cece13-03d9-4992-b3cb-df73c3bb875b
    async fn extract_pages_url(&self, chapter_url: String) -> Result<Vec<String>, AsyncError>  {
        let u:String = if chapter_url.starts_with(&self.get_url()) {chapter_url} else {format!("{}{}",self.get_url(), chapter_url)};
        let doc = get_client().get(&u).send().await?.text().await?;
        let cl = get_client();
        claude_refactor_mangago_tachiyomi_extension::get_chapter_image_urls(&doc, &u,  &cl).await.map_err(Into::into)
    }
    async fn to_pdf(&self, metadata: MangaMetadata, images:Vec<String>)  -> Result<String, AsyncError> {    
        
        Ok("".to_owned())
    }
    async fn to_cbz(&self, metadata: MangaMetadata, images:Vec<String>)  -> Result<String, AsyncError> {
        let bp = format!("./downloads/manga/{}/{}", metadata.title.unwrap_or("manga".to_owned()).replace(".", "_").replace("/", "_").to_ascii_lowercase(),metadata.chapter.unwrap_or("none".to_owned()));
        let cbz = MangaCbz::new(bp.clone(), false)?;
        let mut i:usize = 0;
        std::fs::create_dir_all(&bp)?;
        log!("{}", images.join("\n"));
        for img in images {
            let mut h = Hasher::new();
            let png = get_client().get(img).send().await?.bytes().await?;
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
}


struct SoJsonV4Deobfuscator {}
impl SoJsonV4Deobfuscator {
    fn decode(jsf: String) -> Result<String, String> {
        if !jsf.starts_with("['sojson.v4']")  {
            return Err("Obfuscated code is not sojson.v4".to_owned());//throw IllegalArgumentException("Obfuscated code is not sojson.v4")
        }
        //let regex = Regex::new(r"[a-zA-Z]+").unwrap();
        let args= jsf[240..(jsf.len()- 59)].split(|c: char| c.is_alphabetic());//.split(&regex);

        return Ok(args.filter_map(|s| s.parse::<u32>().ok())
            .filter_map(std::char::from_u32)
            .collect());
    }
}



// mod MangagoAniyomi {
//     use once_cell::sync::Lazy;
//     use regex::Regex;
//     static RE_DESCKEY: Lazy<Regex> = Lazy::new(|| Regex::new(r"desckey=([^&]+)").unwrap());
//     static RE_COLS: Lazy<Regex> = Lazy::new(|| Regex::new(r"cols=(\d+)").unwrap());
//     static RE_TOTAL_PAGES: Lazy<Regex> = Lazy::new(|| Regex::new(r"total_pages\s*=\s*(\d+)").unwrap());
//     static RE_IMG_SRCS: Lazy<Regex> = Lazy::new(|| Regex::new(r#"var imgsrcs\s*=\s*['"]([a-zA-Z0-9+=/]+)['"]"#).unwrap());
//     static RE_IMG_SRCS: Lazy<Regex> = Lazy::new(|| {Regex::new(r#"var imgsrcs\s*=\s*['"]([a-zA-Z0-9+=/]+)['"]"#).unwrap()});

//     static RE_COLS: Lazy<Regex> = Lazy::new(|| {Regex::new(r#"var\s*widthnum\s*=\s*heightnum\s*=\s*(\d+);"#).unwrap()});
//     pub struct Page {
//         pub index: usize,
//         pub url: String,
//         pub image_url: Option<String>,
//     }
//     fn get_and_unscramble(client: &Client, url_str: &str) -> anyhow::Result<Vec<u8>> {
//         let response = client.get(url_str).send()?;
        
//         let fragment = url_str.split_once('#').map(|(_, f)| f).unwrap_or("");

//         if !fragment.contains("desckey=") {
//             return Ok(response.bytes()?.to_vec());
//         }

//         let key = RE_DESCKEY.captures(fragment)
//             .and_then(|cap| cap.get(1))
//             .map(|m| m.as_str())
//             .ok_or_else(|| anyhow::anyhow!("Missing desckey"))?;

//         let cols = RE_COLS.captures(fragment)
//             .and_then(|cap| cap.get(1))
//             .and_then(|m| m.as_str().parse::<usize>().ok())
//             .ok_or_else(|| anyhow::anyhow!("Missing or invalid cols"))?;

//         let body_bytes = response.bytes()?.to_vec();
//         let unscrambled = unscramble_image(body_bytes, key, cols);

//         Ok(unscrambled)
//     }
//     fn page_list_parse(html_content: &str, base_url: &str, domain: &str) -> Result<Vec<Page>, Box<dyn std::error::Error>> {
//         let document = Html::parse_document(html_content);
        
//         let available_images = get_chapter_image_urls(&document);

//         if !available_images.is_empty() && available_images.iter().all(|s| !s.trim().is_empty()) {
//             return Ok(available_images.into_iter().enumerate().map(|(idx, img)| {
//                 Page { index: idx, url: String::new(), image_url: Some(img) }
//             }).collect());
//         }

//         let script_selector = Selector::parse("script").unwrap();
//         let total_pages = document.select(&script_selector)
//             .find_map(|element| {
//                 let data = element.inner_html();
//                 RE_TOTAL_PAGES.captures(&data)
//                     .and_then(|cap| cap[1].parse::<usize>().ok())
//             })
//             .ok_or("Total page count not found")?;

//         let input_selector = Selector::parse("input#curl").unwrap();
//         let url_template = document.select(&input_selector)
//             .next()
//             .and_then(|el| el.value().attr("value"))
//             .map(|v| v.trim().trim_start_matches('/'))
//             .ok_or("Input #curl not found")?;

//         if !url_template.contains("{page}") {
//             return Err("No replaceable string in url template".into());
//         }

//         let current_url = Url::parse(base_url)?; 
//         let path_segments: Vec<&str> = current_url.path_segments().map(|s| s.collect()).unwrap_or_default();
//         let host = current_url.host_str().unwrap_or("");
//         let template_first_segment = url_template.split('/').next().unwrap_or("");

//         let prefix = if host.ends_with(domain) 
//             && path_segments.len() > 3 
//             && path_segments[0] == "read-manga" 
//             && path_segments[2] == template_first_segment 
//         {
//             let slug = path_segments[1];
//             format!("{}/read-manga/{}", base_url.trim_end_matches('/'), slug)
//         } else if !host.ends_with(domain) && path_segments.get(0) == Some(&template_first_segment) {
//             format!("https://{}", host)
//         } else {
//             return Err("Unexpected Url structure".into());
//         };

//         let mut pages: Vec<Page> = (1..=total_pages).map(|p| {
//             let mut page_url = Url::parse(&prefix).unwrap();
//             let specific_path = url_template.replace("{page}", &p.to_string());
            
//             page_url.set_path(&format!("{}/{}", page_url.path().trim_end_matches('/'), specific_path));
//             page_url.set_fragment(Some(&p.to_string()));

//             Page {
//                 index: p,
//                 url: page_url.to_string(),
//                 image_url: None,
//             }
//         }).collect();

//         for (i, page) in pages.iter_mut().enumerate() {
//             if let Some(img) = available_images.get(i) {
//                 if !img.trim().is_empty() {
//                     page.image_url = Some(img.clone());
//                 }
//             }
//         }

//         Ok(pages)
//     }
//     fn get_chapter_image_urls(html: &str, client: &reqwest::blocking::Client) -> Result<Vec<String>, Box<dyn std::error::Error>> {
//         let document = Html::parse_document(html);

//         // 1. Extract and Base64 decode images
//         let script_sel = Selector::parse("script").unwrap();
//         let b64_data = document.select(&script_sel)
//             .find_map(|el| {
//                 let data = el.inner_html();
//                 RE_IMG_SRCS.captures(&data).map(|cap| cap[1].to_string())
//             })
//             .ok_or("Could not find 'imgsrcs' script")?;
        
//         let encrypted_images = general_purpose::STANDARD.decode(b64_data)?;

//         // 2. Fetch and De-obfuscate chapter.js
//         let js_sel = Selector::parse("script[src*='chapter.js']").unwrap();
//         let js_url = document.select(&js_sel)
//             .next()
//             .and_then(|el| el.value().attr("src"))
//             .ok_or("Could not find chapter.js src")?;
        
//         let raw_js = client.get(js_url).send()?.text()?;
//         // NOTE: You must implement your own SoJsonV4 deobfuscator logic here
//         let deobfuscated_js = so_json_v4_deobfuscator(&raw_js);

//         // 3. Extract Hex Keys (mimicking findHexEncodedVariable)
//         let key_hex = extract_hex_var(&deobfuscated_js, "key")?;
//         let iv_hex = extract_hex_var(&deobfuscated_js, "iv")?;
//         let key = hex::decode(key_hex)?;
//         let iv = hex::decode(iv_hex)?;

//         // 4. AES Decryption (AES/CBC/ZEROBYTEPADDING)
//         let cipher = Aes128Cbc::new_from_slices(&key, &iv)?;
//         let decrypted_data = cipher.decrypt_vec(&encrypted_images)?;
//         let image_list_raw = String::from_utf8(decrypted_data)?;

//         // 5. Unscramble List (Another manual port of your logic)
//         let image_list = unscramble_image_list(&image_list_raw, &deobfuscated_js);

//         // 6. JavaScript Execution (QuickJS)
//         let cols = RE_COLS.captures(&deobfuscated_js)
//             .map(|cap| cap[1].to_string())
//             .unwrap_or_default();

//         // Extract the image key logic (JS string manipulation)
//         let img_keys_js = extract_img_keys_logic(&deobfuscated_js);

//         let rt = Runtime::new()?;
//         let ctx = Context::full(&rt)?;
        
//         let resolved_urls = ctx.with(|ctx| {
//             // Mocking the bytecode and function setup
//             let js_code = format!(
//                 "function getDescramblingKey(url) {{ {}; return key; }};",
//                 img_keys_js.replace("img.src", "url")
//             );
//             ctx.eval::<(), _>(js_code).unwrap();
//             let get_key: rquickjs::Function = ctx.globals().get("getDescramblingKey").unwrap();

//             image_list.split(',').map(|url| {
//                 if url.contains("cspiclink") {
//                     let desc_key: String = get_key.call((url,)).unwrap_or_else(|_| "".to_string());
//                     format!("{}#desckey={}&cols={}", url, desc_key, cols)
//                 } else {
//                     url.to_string()
//                 }
//             }).collect::<Vec<String>>()
//         });

//         Ok(resolved_urls)
//     }
// }

//? Rust port of `getChapterImageUrls` and its dependencies from the Mangago Kotlin extension.
//?
//? # Required dependencies (Cargo.toml)
//? ```toml
//? [dependencies]
//? anyhow   = "1"
//? base64   = "0.22"
//? aes      = "0.8"
//? cbc      = { version = "0.1", features = ["alloc"] }
//? cipher   = { version = "0.4", features = ["block-padding"] }
//? hex      = "0.4"
//? image    = "0.25"
//? regex    = "1"
//? once_cell = "1"
//? boa_engine = "0.19"   # JS runtime replacing QuickJS / qjs.evaluate
//? reqwest  = { version = "0.12", features = ["blocking"] }
//? ```
pub mod old_claude_refactor_mangago_tachiyomi_extension {
    use std::collections::HashSet;
    use std::io::Cursor;

    use aes::Aes128;
    use anyhow::{anyhow, bail, Context, Result};
    use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
    use cbc::cipher::{block_padding::ZeroPadding, BlockDecryptMut, KeyIvInit};
    use boa_engine::{Context as JsContext, Source};
    use image::{DynamicImage, GenericImage, GenericImageView, ImageFormat};
    use once_cell::sync::Lazy;
    use regex::Regex;
    use reqwest::Client;

    // ── Third-party type provided by the caller ──────────────────────────────────

    /// Minimal stand-in for a parsed HTML document.
    /// In a real integration swap this for `scraper::Html` or whatever DOM type
    /// your HTTP layer already produces.
    pub struct Document {
        pub html: String,
        pub location: String,
    }

    // ─── Regex constants (compiled once) ─────────────────────────────────────────

    static IMG_SRCS_RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r#"var imgsrcs\s*=\s*['"]([a-zA-Z0-9+=/]+)['"]"#).unwrap());

    static COLS_RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"var\s*widthnum\s*=\s*heightnum\s*=\s*(\d+);").unwrap());

    static KEY_LOCATION_RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"str\.charAt\(\s*(\d+)\s*\)").unwrap());

    static HEX_VAR_RE_KEY: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r#"var key\s*=\s*CryptoJS\.enc\.Hex\.parse\("([0-9a-fA-F]+)"\)"#).unwrap()
    });

    static HEX_VAR_RE_IV: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r#"var iv\s*=\s*CryptoJS\.enc\.Hex\.parse\("([0-9a-fA-F]+)"\)"#).unwrap()
    });

    // Substrings whose presence disqualifies a JS line from the key-extraction snippet.
    const JS_FILTERS: &[&str] = &[
        "jQuery",
        "document",
        "getContext",
        "toDataURL",
        "getImageData",
        "width",
        "height",
    ];

    // ─── SoJsonV4Deobfuscator (provided) ─────────────────────────────────────────

    struct SoJsonV4Deobfuscator;

    impl SoJsonV4Deobfuscator {
        fn decode(jsf: &str) -> Result<String> {
            if !jsf.starts_with("['sojson.v4']") {
                bail!("Obfuscated code is not sojson.v4");
            }
            let slice = &jsf[240..(jsf.len() - 59)];
            let decoded: String = slice
                .split(|c: char| c.is_alphabetic())
                .filter_map(|s| s.parse::<u32>().ok())
                .filter_map(char::from_u32)
                .collect();
            Ok(decoded)
        }
    }

    // ─── Helper: decode a hex string to bytes ────────────────────────────────────

    fn decode_hex(s: &str) -> Result<Vec<u8>> {
        if s.len() % 2 != 0 {
            bail!("Hex string has odd length");
        }
        hex::decode(s).context("Invalid hex string")
    }

    // ─── Helper: find a hex-encoded CryptoJS variable ────────────────────────────

    /// Extracts the hex payload from lines like:
    ///   `var key = CryptoJS.enc.Hex.parse("deadbeef...")`
    fn find_hex_encoded_variable(js: &str, variable: &str) -> Result<Vec<u8>> {
        // Build the regex on the fly so we can parameterise `variable`.
        // In hot paths consider caching these with a HashMap<&str, Regex>.
        let pattern = format!(
            r#"var {variable}\s*=\s*CryptoJS\.enc\.Hex\.parse\("([0-9a-fA-F]+)"\)"#
        );
        let re = Regex::new(&pattern).unwrap();
        let hex = re
            .captures(js)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str())
            .ok_or_else(|| anyhow!("Could not find hex variable `{variable}` in JS"))?;
        decode_hex(hex)
    }

    // ─── AES-128-CBC decrypt with zero-byte padding ───────────────────────────────

    /// Mirrors `Cipher.getInstance("AES/CBC/ZEROBYTEPADDING")`.
    fn aes_cbc_decrypt_zero_pad(ciphertext: &[u8], key: &[u8], iv: &[u8]) -> Result<Vec<u8>> {
        // cbc + ZeroPadding operates in-place on a Vec whose length is a block multiple.
        let mut buf = ciphertext.to_vec();

        type Aes128CbcDec = cbc::Decryptor<Aes128>;

        let decryptor = Aes128CbcDec::new_from_slices(key, iv)
            .map_err(|e| anyhow!("AES init failed: {e}"))?;

        let plaintext = decryptor
            .decrypt_padded_mut::<ZeroPadding>(&mut buf)
            .map_err(|e| anyhow!("AES decrypt failed: {e}"))?;

        Ok(plaintext.to_vec())
    }

    // ─── Image-list unscrambling ──────────────────────────────────────────────────

    /// Mirrors the Kotlin `String.unscramble(keys)` extension.
    fn unscramble_string(s: &str, keys: &[usize]) -> String {
        let mut chars: Vec<char> = s.chars().collect();
        for &key in keys.iter().rev() {
            let len = chars.len();
            let mut i = len.saturating_sub(1);
            while i >= key {
                if i % 2 != 0 {
                    chars.swap(i - key, i);
                }
                if i == 0 {
                    break;
                }
                i -= 1;
            }
        }
        chars.into_iter().collect()
    }

    /// Mirrors `unscrambleImageList`.
    fn unscramble_image_list(image_list: &str, js: &str) -> String {
        // Collect distinct char positions referenced by `str.charAt(N)` calls.
        let key_locations: Vec<usize> = {
            let mut seen = HashSet::new();
            KEY_LOCATION_RE
                .captures_iter(js)
                .filter_map(|c| c.get(1)?.as_str().parse::<usize>().ok())
                .filter(|&loc| seen.insert(loc)) // distinct, insertion-order preserved
                .collect()
        };

        // Try to parse a digit at each key location; bail out on failure (already unscrambled).
        let chars: Vec<char> = image_list.chars().collect();

        let unscramble_key: Vec<usize> = match key_locations
            .iter()
            .map(|&loc| {
                chars
                    .get(loc)
                    .and_then(|c| c.to_digit(10))
                    .map(|d| d as usize)
                    .ok_or(())
            })
            .collect::<std::result::Result<Vec<_>, _>>()
        {
            Ok(k) => k,
            Err(_) => return image_list.to_owned(), // already unscrambled
        };

        // Remove the key-location characters (accounting for shrinking indices).
        let mut img_chars: Vec<char> = chars;
        for (offset, &loc) in key_locations.iter().enumerate() {
            let adjusted = loc - offset;
            if adjusted < img_chars.len() {
                img_chars.remove(adjusted);
            }
        }

        let intermediate: String = img_chars.into_iter().collect();
        unscramble_string(&intermediate, &unscramble_key)
    }

    // ─── Image tile unscrambling ──────────────────────────────────────────────────

    /// Mirrors `unscrambleImage`.
    /// `key` is the descrambling key string (e.g. `"3a1a0a2a..."`).
    /// `cols` is the grid dimension (cols × cols tiles).
    pub fn unscramble_image(image_bytes: &[u8], key: &str, cols: usize) -> Result<Vec<u8>> {
        let src: DynamicImage =
            image::load_from_memory(image_bytes).context("Failed to decode source image")?;

        let w = src.width() as usize;
        let h = src.height() as usize;
        let unit_w = w / cols;
        let unit_h = h / cols;

        let mut result = DynamicImage::new_rgba8(w as u32, h as u32);

        let key_parts: Vec<usize> = key
            .split('a')
            .map(|s| if s.is_empty() { 0 } else { s.parse().unwrap_or(0) })
            .collect();

        for idx in 0..(cols * cols) {
            let key_val = key_parts.get(idx).copied().unwrap_or(0);

            let dst_row = key_val / cols;
            let dst_col = key_val % cols;
            let dx = (dst_col * unit_w) as u32;
            let dy = (dst_row * unit_h) as u32;

            let src_row = idx / cols;
            let src_col = idx % cols;
            let sx = (src_col * unit_w) as u32;
            let sy = (src_row * unit_h) as u32;

            let tile = src.crop_imm(sx, sy, unit_w as u32, unit_h as u32);
            image::imageops::overlay(&mut result, &tile, dx as i64, dy as i64);
        }

        let mut buf = Cursor::new(Vec::new());
        result
            .write_to(&mut buf, ImageFormat::Jpeg)
            .context("Failed to encode result image")?;
        Ok(buf.into_inner())
    }

    // ─── JS evaluation (replacePos + descrambling key extraction) ────────────────

    /// Evaluates the per-image JS snippet with Boa to recover the descrambling key.
    /// Mirrors the QuickJs block in `getChapterImageUrls`.
    fn get_descrambling_key(url: &str, img_js_snippet: &str) -> Result<String> {
        let replace_pos_js = r#"
            function replacePos(strObj, pos, replacetext) {
                return strObj.substr(0, pos) + replacetext + strObj.substring(pos + 1, strObj.length);
            }
        "#;

        let wrapper = format!(
            r#"
            {replace_pos_js}
            function getDescramblingKey(url) {{
                {img_js_snippet}
                return key;
            }}
            getDescramblingKey("{url}");
            "#
        );

        let mut js_ctx = JsContext::default();
        let result = js_ctx
            .eval(Source::from_bytes(wrapper.as_bytes()))
            .map_err(|e| anyhow!("JS eval error: {e}"))?;

        result
            .as_string()
            .map(|s| s.to_std_string_escaped())
            .ok_or_else(|| anyhow!("JS did not return a string for url `{url}`"))
    }

    // ─── DOM helpers (thin wrappers; adapt to your actual HTML parser) ────────────

    /// Finds the first `<script>` whose text content contains `needle`.
    fn select_script_data<'a>(html: &'a scraper::Html, needle: &str) -> Option<&'a str> {
        // scraper does not support `:containsData`; we iterate manually.
        use scraper::Selector;
        let sel = Selector::parse("script").unwrap();
        html.select(&sel)
            .find(|el| el.text().any(|t| t.contains(needle)))
            .map(|el| el.text().collect::<Vec<_>>().join(""))
            // SAFETY: we return a slice of a local join; callers must own the string.
            // In a real codebase return `String` instead.
            .map(|_| "") // placeholder – see note below
    }

    // NOTE: Because `scraper::ElementRef::text()` yields `&str` tied to the document
    // lifetime it is easier to return `String` from these helpers. The functions below
    // do exactly that.

    fn script_data_containing(html: &scraper::Html, needle: &str) -> Option<String> {
        use scraper::Selector;
        let sel = Selector::parse("script").unwrap();
        html.select(&sel).find_map(|el| {
            let text: String = el.text().collect();
            if text.contains(needle) { Some(text) } else { None }
        })
    }

    fn script_src_containing(html: &scraper::Html, needle: &str) -> Option<String> {
        use scraper::Selector;
        let sel = Selector::parse("script").unwrap();
        html.select(&sel).find_map(|el| {
            let src = el.value().attr("src")?;
            if src.contains(needle) { Some(src.to_owned()) } else { None }
        })
    }

    pub async fn http_get(client: &Client, url: &str) -> Result<String> {
        client
            .get(url)
            .send()
            .await
            .map_err(|e| anyhow!("GET request failed for {url}: {e}"))?
            .error_for_status()
            .map_err(|e| anyhow!("Non-2xx response for {url}: {e}"))?
            .text()
            .await
            .map_err(|e| anyhow!("Failed to decode response body as UTF-8: {e}"))
    }
    // ─── Main function ────────────────────────────────────────────────────────────

    /// Rust port of `getChapterImageUrls(document: Document)`.
    ///
    /// `page_url` is the URL of the chapter page (used to resolve relative script URLs).
    /// `http_get` is an injected closure for HTTP fetching, keeping this function pure
    /// and easily testable.
    pub async fn get_chapter_image_urls(
        html_str: &str,
        page_url: &str,
        client: &Client,
    ) -> Result<Vec<String>> {
        let (
            imgsrcs_script,
            b64_payload_inside,
            ciphertext,
            chapter_js_rel,
            chapter_js_url
        ) = {
            let document = scraper::Html::parse_document(html_str);
            let imgsrcs_script_inside = script_data_containing(&document, "imgsrcs")
            .ok_or_else(|| anyhow!("could not find 'imgsrcs' script"))?;
            // ── 1. Extract and base64-decode the encrypted image list ─────────────────
            let capforb64 = imgsrcs_script_inside.clone();
    
            let b64_payload_inside = IMG_SRCS_RE
                .captures(&capforb64)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().to_owned())
                .ok_or_else(|| anyhow!("imgsrcs regex did not match"))?;
            let ciphertext_inside = B64.decode(&b64_payload_inside).context("base64 decode of imgsrcs failed")?;
            
            // ── 2. Fetch and deobfuscate chapter.js ───────────────────────────────────
            let chapter_js_rel_inside = script_src_containing(&document, "chapter.js")
                .ok_or_else(|| anyhow!("chapter.js <script> not found"))?;
            // Resolve relative URL against the page origin.
            let base = url::Url::parse(page_url).context("invalid page_url")?;
            let chapter_js_url_inside = base
                .join(&chapter_js_rel_inside)
                .context("could not resolve chapter.js URL")?
                .to_string();
            (imgsrcs_script_inside, b64_payload_inside,ciphertext_inside, chapter_js_rel_inside,chapter_js_url_inside )
        };





        

        let raw_js = http_get(client,&chapter_js_url).await?;
        let chapter_js = SoJsonV4Deobfuscator::decode(&raw_js)?;

        // ── 3. Extract AES key + IV and decrypt ───────────────────────────────────
        let key = find_hex_encoded_variable(&chapter_js, "key")?;
        let iv  = find_hex_encoded_variable(&chapter_js, "iv")?;

        let plaintext_bytes = aes_cbc_decrypt_zero_pad(&ciphertext, &key, &iv)?;
        let mut image_list =
            String::from_utf8(plaintext_bytes).context("decrypted bytes are not valid UTF-8")?;

        // ── 4. Unscramble the image URL list ──────────────────────────────────────
        image_list = unscramble_image_list(&image_list, &chapter_js);

        // ── 5. Extract the tile-grid column count ─────────────────────────────────
        let cols = COLS_RE
            .captures(&chapter_js)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str())
            .unwrap_or("")
            .to_owned();

        // ── 6. Build the per-image JS key-extraction snippet ──────────────────────
        // Mirrors the Kotlin code that slices the `renImg` function body.
        let img_js_snippet: String = chapter_js
            .split_once("var renImg = function(img,width,height,id){")
            .map(|(_, after)| after)
            .unwrap_or("")
            .split_once("key = key.split(")
            .map(|(before, _)| before)
            .unwrap_or("")
            .lines()
            .filter(|line| JS_FILTERS.iter().all(|f| !line.contains(f)))
            .collect::<Vec<_>>()
            .join("\n")
            .replace("img.src", "url");

        // ── 7. Resolve each URL (evaluate JS for scrambled images) ────────────────
        let mut js_ctx = JsContext::default();

        // Compile `replacePos` once and reuse across all images.
        let replace_pos_src = r#"
            function replacePos(strObj, pos, replacetext) {
                return strObj.substr(0, pos) + replacetext + strObj.substring(pos + 1, strObj.length);
            }
        "#;
        js_ctx
            .eval(Source::from_bytes(replace_pos_src.as_bytes()))
            .map_err(|e| anyhow!("Failed to compile replacePos: {e}"))?;

        let get_key_fn = format!(
            "function getDescramblingKey(url) {{ {img_js_snippet}; return key; }}"
        );
        js_ctx
            .eval(Source::from_bytes(get_key_fn.as_bytes()))
            .map_err(|e| anyhow!("Failed to compile getDescramblingKey: {e}"))?;

        let resolved: Vec<String> = image_list
            .split(',')
            .map(|raw_url| -> Result<String> {
                let raw_url = raw_url.trim();
                if raw_url.contains("cspiclink") {
                    let call = format!(r#"getDescramblingKey("{raw_url}");"#);
                    let result = js_ctx
                        .eval(Source::from_bytes(call.as_bytes()))
                        .map_err(|e| anyhow!("JS key eval failed: {e}"))?;
                    let desc_key = result
                        .as_string()
                        .map(|s| s.to_std_string_escaped())
                        .ok_or_else(|| anyhow!("JS returned non-string key"))?;
                    Ok(format!("{raw_url}#desckey={desc_key}&cols={cols}"))
                } else {
                    Ok(raw_url.to_owned())
                }
            })
            .collect::<Result<_>>()?;

        Ok(resolved)
    }
}



//? Rust port of `getChapterImageUrls` and its dependencies from the Mangago Kotlin extension.
//?
//? # Required dependencies (Cargo.toml)
//? ```toml
//? [dependencies]
//? anyhow   = "1"
//? base64   = "0.22"
//? aes      = "0.8"
//? cbc      = { version = "0.1", features = ["alloc"] }
//? cipher   = { version = "0.4", features = ["block-padding"] }
//? hex      = "0.4"
//? image    = "0.25"
//? regex    = "1"
//? once_cell = "1"
//? boa_engine = "0.19"   # JS runtime replacing QuickJS / qjs.evaluate
//? reqwest  = { version = "0.12", features = ["blocking"] }
//? ```
pub mod claude_refactor_mangago_tachiyomi_extension {
    use std::collections::HashSet;
    use std::io::Cursor;

    use aes::Aes128;
    use anyhow::{anyhow, bail, Context, Result};
    use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
    use cbc::cipher::{block_padding::ZeroPadding, BlockDecryptMut, KeyIvInit};
    use boa_engine::{Context as JsContext, Source};
    use image::{DynamicImage, GenericImage, GenericImageView, ImageFormat};
    use once_cell::sync::Lazy;
    use regex::Regex;
    use reqwest::Client;

use crate::log;

    // ── Third-party type provided by the caller ──────────────────────────────────

    /// Minimal stand-in for a parsed HTML document.
    /// In a real integration swap this for `scraper::Html` or whatever DOM type
    /// your HTTP layer already produces.
    pub struct Document {
        pub html: String,
        pub location: String,
    }

    // ─── Regex constants (compiled once) ─────────────────────────────────────────

    static IMG_SRCS_RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r#"var imgsrcs\s*=\s*['"]([a-zA-Z0-9+=/]+)['"]"#).unwrap());

    static COLS_RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"var\s*widthnum\s*=\s*heightnum\s*=\s*(\d+);").unwrap());

    static KEY_LOCATION_RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"str\.charAt\(\s*(\d+)\s*\)").unwrap());

    static HEX_VAR_RE_KEY: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r#"var key\s*=\s*CryptoJS\.enc\.Hex\.parse\("([0-9a-fA-F]+)"\)"#).unwrap()
    });

    static HEX_VAR_RE_IV: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r#"var iv\s*=\s*CryptoJS\.enc\.Hex\.parse\("([0-9a-fA-F]+)"\)"#).unwrap()
    });

    // Substrings whose presence disqualifies a JS line from the key-extraction snippet.
    const JS_FILTERS: &[&str] = &[
        "jQuery",
        "document",
        "getContext",
        "toDataURL",
        "getImageData",
        "width",
        "height",
    ];

    // ─── SoJsonV4Deobfuscator (provided) ─────────────────────────────────────────

    struct SoJsonV4Deobfuscator;

    impl SoJsonV4Deobfuscator {
        fn decode(jsf: &str) -> Result<String> {
            if !jsf.starts_with("['sojson.v4']") {
                bail!("Obfuscated code is not sojson.v4");
            }
            let slice = &jsf[240..(jsf.len() - 59)];
            let decoded: String = slice
                .split(|c: char| c.is_alphabetic())
                .filter_map(|s| s.parse::<u32>().ok())
                .filter_map(char::from_u32)
                .collect();
            Ok(decoded)
        }
    }

    // ─── Helper: decode a hex string to bytes ────────────────────────────────────

    fn decode_hex(s: &str) -> Result<Vec<u8>> {
        if s.len() % 2 != 0 {
            bail!("Hex string has odd length");
        }
        hex::decode(s).context("Invalid hex string")
    }

    // ─── Helper: find a hex-encoded CryptoJS variable ────────────────────────────

    /// Extracts the hex payload from lines like:
    ///   `var key = CryptoJS.enc.Hex.parse("deadbeef...")`
    fn find_hex_encoded_variable(js: &str, variable: &str) -> Result<Vec<u8>> {
        // Build the regex on the fly so we can parameterise `variable`.
        // In hot paths consider caching these with a HashMap<&str, Regex>.
        let pattern = format!(
            r#"var {variable}\s*=\s*CryptoJS\.enc\.Hex\.parse\("([0-9a-fA-F]+)"\)"#
        );
        let re = Regex::new(&pattern).unwrap();
        let hex = re
            .captures(js)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str())
            .ok_or_else(|| anyhow!("Could not find hex variable `{variable}` in JS"))?;
        decode_hex(hex)
    }

    // ─── AES-128-CBC decrypt with zero-byte padding ───────────────────────────────

    /// Mirrors `Cipher.getInstance("AES/CBC/ZEROBYTEPADDING")`.
    fn aes_cbc_decrypt_zero_pad(ciphertext: &[u8], key: &[u8], iv: &[u8]) -> Result<Vec<u8>> {
        // cbc + ZeroPadding operates in-place on a Vec whose length is a block multiple.
        let mut buf = ciphertext.to_vec();

        type Aes128CbcDec = cbc::Decryptor<Aes128>;

        let decryptor = Aes128CbcDec::new_from_slices(key, iv)
            .map_err(|e| anyhow!("AES init failed: {e}"))?;

        let plaintext = decryptor
            .decrypt_padded_mut::<ZeroPadding>(&mut buf)
            .map_err(|e| anyhow!("AES decrypt failed: {e}"))?;

        Ok(plaintext.to_vec())
    }

    // ─── Image-list unscrambling ──────────────────────────────────────────────────

    /// Mirrors the Kotlin `String.unscramble(keys)` extension.
    fn unscramble_string(s: &str, keys: &[usize]) -> String {
        let mut chars: Vec<char> = s.chars().collect();
        for &key in keys.iter().rev() {
            let len = chars.len();
            let mut i = len.saturating_sub(1);
            while i >= key {
                if i % 2 != 0 {
                    chars.swap(i - key, i);
                }
                if i == 0 {
                    break;
                }
                i -= 1;
            }
        }
        chars.into_iter().collect()
    }

    /// Mirrors `unscrambleImageList`.
    fn unscramble_image_list(image_list: &str, js: &str) -> String {
        // Collect distinct char positions referenced by `str.charAt(N)` calls.
        let key_locations: Vec<usize> = {
            let mut seen = HashSet::new();
            KEY_LOCATION_RE
                .captures_iter(js)
                .filter_map(|c| c.get(1)?.as_str().parse::<usize>().ok())
                .filter(|&loc| seen.insert(loc)) // distinct, insertion-order preserved
                .collect()
        };

        // Try to parse a digit at each key location; bail out on failure (already unscrambled).
        let chars: Vec<char> = image_list.chars().collect();

        let unscramble_key: Vec<usize> = match key_locations
            .iter()
            .map(|&loc| {
                chars
                    .get(loc)
                    .and_then(|c| c.to_digit(10))
                    .map(|d| d as usize)
                    .ok_or(())
            })
            .collect::<std::result::Result<Vec<_>, _>>()
        {
            Ok(k) => k,
            Err(_) => return image_list.to_owned(), // already unscrambled
        };

        // Remove the key-location characters (accounting for shrinking indices).
        let mut img_chars: Vec<char> = chars;
        for (offset, &loc) in key_locations.iter().enumerate() {
            let adjusted = loc - offset;
            if adjusted < img_chars.len() {
                img_chars.remove(adjusted);
            }
        }

        let intermediate: String = img_chars.into_iter().collect();
        unscramble_string(&intermediate, &unscramble_key)
    }

    // ─── Image tile unscrambling ──────────────────────────────────────────────────

    /// Mirrors `unscrambleImage`.
    /// `key` is the descrambling key string (e.g. `"3a1a0a2a..."`).
    /// `cols` is the grid dimension (cols × cols tiles).
    pub fn unscramble_image(image_bytes: &[u8], key: &str, cols: usize) -> Result<Vec<u8>> {
        let src: DynamicImage =
            image::load_from_memory(image_bytes).context("Failed to decode source image")?;

        let w = src.width() as usize;
        let h = src.height() as usize;
        let unit_w = w / cols;
        let unit_h = h / cols;

        let mut result = DynamicImage::new_rgba8(w as u32, h as u32);

        let key_parts: Vec<usize> = key
            .split('a')
            .map(|s| if s.is_empty() { 0 } else { s.parse().unwrap_or(0) })
            .collect();

        for idx in 0..(cols * cols) {
            let key_val = key_parts.get(idx).copied().unwrap_or(0);

            let dst_row = key_val / cols;
            let dst_col = key_val % cols;
            let dx = (dst_col * unit_w) as u32;
            let dy = (dst_row * unit_h) as u32;

            let src_row = idx / cols;
            let src_col = idx % cols;
            let sx = (src_col * unit_w) as u32;
            let sy = (src_row * unit_h) as u32;

            let tile = src.crop_imm(sx, sy, unit_w as u32, unit_h as u32);
            image::imageops::overlay(&mut result, &tile, dx as i64, dy as i64);
        }

        let mut buf = Cursor::new(Vec::new());
        result
            .write_to(&mut buf, ImageFormat::Jpeg)
            .context("Failed to encode result image")?;
        Ok(buf.into_inner())
    }

    // ─── JS evaluation (replacePos + descrambling key extraction) ────────────────

    /// Evaluates the per-image JS snippet with Boa to recover the descrambling key.
    /// Mirrors the QuickJs block in `getChapterImageUrls`.
    fn get_descrambling_key(url: &str, img_js_snippet: &str) -> Result<String> {
        let replace_pos_js = r#"
            function replacePos(strObj, pos, replacetext) {
                return strObj.substr(0, pos) + replacetext + strObj.substring(pos + 1, strObj.length);
            }
        "#;

        let wrapper = format!(
            r#"
            {replace_pos_js}
            function getDescramblingKey(url) {{
                {img_js_snippet}
                return key;
            }}
            getDescramblingKey("{url}");
            "#
        );

        let mut js_ctx = JsContext::default();
        let result = js_ctx
            .eval(Source::from_bytes(wrapper.as_bytes()))
            .map_err(|e| anyhow!("JS eval error: {e}"))?;

        result
            .as_string()
            .map(|s| s.to_std_string_escaped())
            .ok_or_else(|| anyhow!("JS did not return a string for url `{url}`"))
    }

    // ─── DOM helpers (thin wrappers; adapt to your actual HTML parser) ────────────

    /// Finds the first `<script>` whose text content contains `needle`.
    fn select_script_data<'a>(html: &'a scraper::Html, needle: &str) -> Option<&'a str> {
        // scraper does not support `:containsData`; we iterate manually.
        use scraper::Selector;
        let sel = Selector::parse("script").unwrap();
        html.select(&sel)
            .find(|el| el.text().any(|t| t.contains(needle)))
            .map(|el| el.text().collect::<Vec<_>>().join(""))
            // SAFETY: we return a slice of a local join; callers must own the string.
            // In a real codebase return `String` instead.
            .map(|_| "") // placeholder – see note below
    }

    // NOTE: Because `scraper::ElementRef::text()` yields `&str` tied to the document
    // lifetime it is easier to return `String` from these helpers. The functions below
    // do exactly that.

    fn script_data_containing(html: &scraper::Html, needle: &str) -> Option<String> {
        use scraper::Selector;
        let sel = Selector::parse("script").unwrap();
        html.select(&sel).find_map(|el| {
            let text: String = el.text().collect();
            if text.contains(needle) { Some(text) } else { None }
        })
    }

    fn script_src_containing(html: &scraper::Html, needle: &str) -> Option<String> {
        use scraper::Selector;
        let sel = Selector::parse("script").unwrap();
        html.select(&sel).find_map(|el| {
            let src = el.value().attr("src")?;
            if src.contains(needle) { Some(src.to_owned()) } else { None }
        })
    }

    pub async fn http_get(client: &Client, url: &str) -> Result<String> {
        client
            .get(url)
            .send()
            .await
            .map_err(|e| anyhow!("GET request failed for {url}: {e}"))?
            .error_for_status()
            .map_err(|e| anyhow!("Non-2xx response for {url}: {e}"))?
            .text()
            .await
            .map_err(|e| anyhow!("Failed to decode response body as UTF-8: {e}"))
    }
    // ─── Main function ────────────────────────────────────────────────────────────

    /// Rust port of `getChapterImageUrls(document: Document)`.
    ///
    /// `page_url` is the URL of the chapter page (used to resolve relative script URLs).
    /// `http_get` is an injected closure for HTTP fetching, keeping this function pure
    /// and easily testable.
    pub async fn get_chapter_image_urls(
        html_str: &str,
        page_url: &str,
        client: &Client,
    ) -> Result<Vec<String>> {
        let (
            imgsrcs_script,
            b64_payload_inside,
            ciphertext,
            chapter_js_rel,
            chapter_js_url
        ) = {
            let document = scraper::Html::parse_document(html_str);
            let imgsrcs_script_inside = script_data_containing(&document, "imgsrcs")
            .ok_or_else(|| {log!("could not find 'imgsrcs' script");anyhow!("could not find 'imgsrcs' script")})?;
            
            // ── 1. Extract and base64-decode the encrypted image list ─────────────────
            let capforb64 = imgsrcs_script_inside.clone();
    
            let b64_payload_inside = IMG_SRCS_RE
                .captures(&capforb64)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().to_owned())
                .ok_or_else(|| {log!("imgrcs regex did not match");anyhow!("imgsrcs regex did not match")})?;
            let ciphertext_inside = B64.decode(&b64_payload_inside).context("base64 decode of imgsrcs failed")?;
            
            // ── 2. Fetch and deobfuscate chapter.js ───────────────────────────────────
            let chapter_js_rel_inside = script_src_containing(&document, "chapter.js")
                .ok_or_else(|| {log!("chapter.js <script> not found");anyhow!("chapter.js <script> not found")})?;
            // Resolve relative URL against the page origin.
            let base = url::Url::parse(page_url).context("invalid page_url")?;
            let chapter_js_url_inside = base
                .join(&chapter_js_rel_inside)
                .context("could not resolve chapter.js URL")?
                .to_string();
            (imgsrcs_script_inside, b64_payload_inside,ciphertext_inside, chapter_js_rel_inside,chapter_js_url_inside )
        };





        

        let raw_js = http_get(client,&chapter_js_url).await?;
        let chapter_js = SoJsonV4Deobfuscator::decode(&raw_js)?;

        // ── 3. Extract AES key + IV and decrypt ───────────────────────────────────
        let key = find_hex_encoded_variable(&chapter_js, "key")?;
        let iv  = find_hex_encoded_variable(&chapter_js, "iv")?;

        let plaintext_bytes = aes_cbc_decrypt_zero_pad(&ciphertext, &key, &iv)?;
        let mut image_list =
            String::from_utf8(plaintext_bytes).context("decrypted bytes are not valid UTF-8")?;

        // ── 4. Unscramble the image URL list ──────────────────────────────────────
        image_list = unscramble_image_list(&image_list, &chapter_js);

        // ── 5. Extract the tile-grid column count ─────────────────────────────────
        let cols = COLS_RE
            .captures(&chapter_js)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str())
            .unwrap_or("")
            .to_owned();

        // ── 6. Build the per-image JS key-extraction snippet ──────────────────────
        // Mirrors the Kotlin code that slices the `renImg` function body.
        let img_js_snippet: String = chapter_js
            .split_once("var renImg = function(img,width,height,id){")
            .map(|(_, after)| after)
            .unwrap_or("")
            .split_once("key = key.split(")
            .map(|(before, _)| before)
            .unwrap_or("")
            .lines()
            .filter(|line| JS_FILTERS.iter().all(|f| !line.contains(f)))
            .collect::<Vec<_>>()
            .join("\n")
            .replace("img.src", "url");

        // ── 7. Resolve each URL (evaluate JS for scrambled images) ────────────────
        let mut js_ctx = JsContext::default();

        // Compile `replacePos` once and reuse across all images.
        let replace_pos_src = r#"
            function replacePos(strObj, pos, replacetext) {
                return strObj.substr(0, pos) + replacetext + strObj.substring(pos + 1, strObj.length);
            }
        "#;
        js_ctx
            .eval(Source::from_bytes(replace_pos_src.as_bytes()))
            .map_err(|e| anyhow!("Failed to compile replacePos: {e}"))?;

        let get_key_fn = format!(
            "function getDescramblingKey(url) {{ {img_js_snippet}; return key; }}"
        );
        js_ctx
            .eval(Source::from_bytes(get_key_fn.as_bytes()))
            .map_err(|e| anyhow!("Failed to compile getDescramblingKey: {e}"))?;

        let resolved: Vec<String> = image_list
            .split(',')
            .map(|raw_url| -> Result<String> {
                let raw_url = raw_url.trim();
                if raw_url.contains("cspiclink") {
                    let call = format!(r#"getDescramblingKey("{raw_url}");"#);
                    let result = js_ctx
                        .eval(Source::from_bytes(call.as_bytes()))
                        .map_err(|e| anyhow!("JS key eval failed: {e}"))?;
                    let desc_key = result
                        .as_string()
                        .map(|s| s.to_std_string_escaped())
                        .ok_or_else(|| anyhow!("JS returned non-string key"))?;
                    Ok(format!("{raw_url}#desckey={desc_key}&cols={cols}"))
                } else {
                    Ok(raw_url.to_owned())
                }
            })
            .collect::<Result<_>>()?;

        Ok(resolved)
    }
}