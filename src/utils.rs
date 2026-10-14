pub mod utils {
    use block_padding::ZeroPadding;
    use ratatui::style::palette::tailwind;
use regex::Regex;
    pub type AsyncError = Box<dyn std::error::Error + Send + Sync>;
    use aes::{Aes128, cipher::BlockDecryptMut}; // Use Aes256 if your key is 32 bytes
    use cbc::{Decryptor, cipher::KeyIvInit};
    use std::collections::HashMap;
    use std::panic;
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::backtrace::Backtrace;
    use std::process::{Command, Stdio};

    #[cfg(feature = "debug")]
    #[macro_export]
    macro_rules! log {
        ($($arg:tt)*) => {{
            let file_result = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("./debug.log");

            if let Ok(mut file) = file_result {
                use std::io::Write;
                let _ = writeln!(file, $($arg)*);
            }
        }};
    }


    #[cfg(not(feature = "debug"))]
    #[macro_export]
    macro_rules! log {
        ($($arg:tt)*) => {{
            
        }};
    }
    /// Gemini checkpoint:
    /// define macro checkpoint in rust avaible only with cfg feature debug 
    /// that has a hashmap initialized only if cfg feature debug is active which map string, int. 
    /// if checkpoint has 1 attribute string passed it set hashmap<string>++ and set current checkpoint to string. 
    /// if nothing passed it use as valued current checkpoint and  hashmap<currentcheckpoint>++;
    /// add possibility to leave checkpoint in code without error if feature is not there
    // ==========================================
    // When `feature = "debug"` is ENABLED
    // ==========================================
    #[cfg(feature = "debug")]
    pub mod debug_checkpoint {
        use std::collections::HashMap;
        use std::sync::{LazyLock, Mutex};

        pub struct CheckpointRegistry {
            pub counts: HashMap<String, usize>,
            pub current: Option<String>,
        }

        pub static REGISTRY: LazyLock<Mutex<CheckpointRegistry>> = LazyLock::new(|| {
            Mutex::new(CheckpointRegistry {
                counts: HashMap::new(),
                current: None,
            })
        });

        pub fn record(name: Option<&str>) {
            let mut reg = REGISTRY.lock().unwrap();

            let target_name = match name {
                Some(n) => {
                    let s = n.to_string();
                    reg.current = Some(s.clone());
                    s
                }
                None => reg
                    .current
                    .clone()
                    .expect("checkpoint!() called without argument before setting an initial name"),
            };

            // Increment counter
            let counter = reg.counts.entry(target_name.clone()).or_insert(0);
            *counter += 1;
            let new_value = *counter;

            // Emit log with the new value (e.g. log::info! or println!)
            log!("{}: {}", target_name, new_value);
        }

        pub fn clear(name: Option<&str>) {
            let mut reg = REGISTRY.lock().unwrap();

            let target_name = match name {
                Some(n) => {
                    let s = n.to_string();
                    reg.current = Some(s.clone());
                    s
                }
                None => reg
                    .current
                    .clone()
                    .expect("checkpoint!() called without argument before setting an initial name"),
            };

            // Increment counter
            *reg.counts.entry(target_name.clone()).or_insert(0) = 0;
        }
        /// Helper to read or inspect checkpoints in debug mode
        pub fn print_summary() {
            let reg = REGISTRY.lock().unwrap();
            println!("Checkpoints: {:#?}", reg.counts);
        }
    }

    // ==========================================
    // When `feature = "debug"` is DISABLED
    // ==========================================
    #[cfg(not(feature = "debug"))]
    pub mod debug_checkpoint {
        use std::collections::HashMap;

        /// No-op registry stub so calls like `debug_checkpoint::print_summary()`
        /// or reading counts still compile without error.
        pub fn print_summary() {}

        pub fn get_counts() -> HashMap<String, usize> {
            HashMap::new()
        }
    }

    // ==========================================
    // Macro Definitions
    // ==========================================
    #[cfg(feature = "debug")]
    #[macro_export]
    macro_rules! checkpoint {
        ($name:expr) => {
            $crate::utils::utils::debug_checkpoint::record(Some($name));
        };
        () => {
            $crate::utils::utils::debug_checkpoint::record(None);
        };
    }

    #[cfg(not(feature = "debug"))]
    #[macro_export]
    macro_rules! checkpoint {
        // Consumes expressions without evaluating them or triggering unused-variable warnings
        ($name:expr) => {
            let _ = &$name;
        };
        () => {};
    }
    #[cfg(feature = "debug")]
    #[macro_export]
    macro_rules! resetcheckpoint {
        ($name:expr) => {
            $crate::utils::utils::debug_checkpoint::clear(Some($name));
            $crate::utils::utils::debug_checkpoint::record(Some($name));
        };
        () => {
            $crate::utils::utils::debug_checkpoint::record(None);
        };
    }

    #[cfg(not(feature = "debug"))]
    #[macro_export]
    macro_rules! resetcheckpoint {
        // Consumes expressions without evaluating them or triggering unused-variable warnings
        ($name:expr) => {
            let _ = &$name;
        };
        () => {};
    }
    ///
    pub static JS_FILTERS: &[&str] = &[
        "jQuery",
        "document",
        "getContext",
        "toDataURL",
        "getImageData",
        "width",
        "height",
    ];

    pub fn first_capital(s: &str) -> String {
        let mut chars = s.chars();
        match chars.next() {
            None => String::new(),
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        }
    }

    pub fn hash_sort(res: &HashMap<String, String>) -> Vec<&String> {
        let mut items = res.keys().collect::<Vec<&String>>();
        items.sort_by(|a,b|{
            let aname = a.splitn(2, "di ").nth(1).unwrap_or("");
            let bname = b.splitn(2, "di ").nth(1).unwrap_or("");
            match aname.cmp(bname) {
                std::cmp::Ordering::Equal => a.cmp(b),
                o => o
            }
        });
        items
    }
    
    // pub fn image_regex() -> Regex {
    //     return Regex::new(r#"var imgsrcs\s*=\s*['"]([a-zA-Z0-9+=/]+)['"]"#).unwrap();
    // }
    pub struct MangaCbz {
        pub path: String,
        pub pages: HashMap<usize, u32>
    }
    impl MangaCbz {
        pub fn new(mut path: String, truncate: bool) -> Result<Self, AsyncError>{
            if !path.ends_with(".cbz") {
                path = path+".cbz";
            }
            let mut pages = HashMap::new();
            let ex = std::fs::exists(&path).unwrap_or(true);
            if truncate || !ex {
                let p = std::path::Path::new(&path).parent().unwrap();
                std::fs::create_dir_all(&p)?;
                let file = std::fs::File::create(&path)?;
                let zip = zip::ZipWriter::new(file);
                zip.finish()?;
            } else {
                let file = std::fs::File::open(&path)?;
                let mut zip = zip::ZipArchive::new(file)?;
                for i in 0..zip.len() {
                    if let Ok(img) = zip.by_index(i) {
                        if let Some(s) = img.name().rsplitn(2,".").nth(0) {
                            if let Ok(u) = s.parse::<usize>() {
                                pages.insert(u, img.crc32());
                            }
                        }
                    }
                };
            }
            return Ok(MangaCbz {
                path,
                pages
            });
        }
        pub fn add_page_path(&self, image_path: String) -> Result<(),AsyncError>{     
            let sta = Command::new("7z")
            .arg("a")
            .arg(&self.path)
            .arg(image_path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
            log!("{}",sta);
            Ok(())
        }   
        pub fn has_page(&self, i: usize, hs: u32) -> bool {
            match self.pages.get(&i) {
                Some(u) => u == &hs,
                _ => false
            }
        }
    }
    
    // Found on stackoverflow
    #[cfg(feature = "debug")]
    pub fn setup_panic_hook() {
        panic::set_hook(Box::new(|info| {
            let backtrace = Backtrace::capture();

            let msg = format!(
                "=== PANIC ===\nMessage:  {}\nLocation: {}\n\nBacktrace:\n{}\n",
                info.payload()
                    .downcast_ref::<&str>()
                    .copied()
                    .or_else(|| info.payload().downcast_ref::<String>().map(|s| s.as_str()))
                    .unwrap_or("<no message>"),
                info.location()
                    .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
                    .unwrap_or_else(|| "<unknown location>".to_owned()),
                backtrace,
            );

            // also print to stderr so you see it in the terminal
            eprintln!("{msg}");

            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open("./panic.log")
            {
                let _ = writeln!(file, "{msg}");
            }
        }));
    }
    #[cfg(not(feature = "debug"))]
    pub fn setup_panic_hook() {}


    pub struct ColorPickerOption {
        pub _type:ColorPickerOptionType,
        red: u8,
        green: u8,
        blue: u8
    }  
    pub enum ColorPickerOptionType {
        RGB, HEX
    }
    impl ColorPickerOption {
        pub fn default() -> ColorPickerOption {
            ColorPickerOption { _type: ColorPickerOptionType::HEX, red: 0, green: 0, blue: 0 }
        }
        pub fn redString(&self) -> String {
            match self._type {
                ColorPickerOptionType::RGB => format!("{:0<3}", self.red.to_string()),
                ColorPickerOptionType::HEX => format!("{:0x?}", self.red).get(2..).unwrap_or("00").to_owned(),
            }
        }
        pub fn greenString(&self) -> String {
            match self._type {
                ColorPickerOptionType::RGB => format!("{:0<3}", self.green.to_string()),
                ColorPickerOptionType::HEX => format!("{:0x?}", self.green).get(2..).unwrap_or("00").to_owned(),
            }
        }
        pub fn blueString(&self) -> String {
            match self._type {
                ColorPickerOptionType::RGB => format!("{:0<3}", self.blue.to_string()),
                ColorPickerOptionType::HEX => format!("{:0x?}", self.blue).get(2..).unwrap_or("00").to_owned(),
            }
        }
    }
}
