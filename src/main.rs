#![allow(non_snake_case)]
// mod config;
// mod mangatab;
mod test;
mod extension;
mod player;
mod settings;
mod library;
use boa_engine::{builtins::boolean, object::Ref};
use config::{getConfig, load_config};
use toml::value::Time;
mod config;
mod components;
mod source;
mod utils;
mod folders;
mod appmessages;
use std::{cell::RefCell, collections::HashMap, format, process::{Command, Stdio}, rc::Rc, sync::Arc, time::Duration, vec};
use color_eyre::{Result};
use ratatui::{
    DefaultTerminal, Frame, crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers}, layout::{Constraint, Direction, Layout, Rect}, style::{Style, palette::tailwind}, text::{Line, Span, Text}, widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, StatefulWidget}
};
use crate::{appmessages::MessageType, components::{AskToQuit, AskToQuitState, AsyncUpdateAnimeList::{self, AsyncAnimeState, AsyncAnimeStateStatus}, AsyncUpdateMangaList::{self, AsyncMangaState, AsyncMangaStateStatus}, Library::{self, LibraryState, LibraryStateStatus}, ListFuncOption, Search::{self, SearchState, SearchStateStatus}, list::{ActionType, CurrentWidget, ListDiveChild, SelectableList}}, config::getDevice, extension::Extension, library::addToFavourite, settings::{ApplySettingsChange, Settings, SettingsState}, source::{AnimeSource, EpisodeListItem, MangaMetadata, MangaSource, SourceType, animeworld, mangaworld, openManga}};
//use crate::{mangatab::MangaTab, config::{getConfig, load_config, tailwindPaletteFromString}};

#[tokio::main]
async fn main() -> Result<()> {
    load_config();
    utils::utils::setup_panic_hook();
    folders::init_folders();
    color_eyre::install()?;
    let terminal = ratatui::init();
    let app_result = App::new().run(terminal).await;
    ratatui::restore();
    app_result
}

//#[derive(Default)]
struct App {
    state: AppState,
    navigation_menu: SelectableList,
    tx: tokio::sync::mpsc::UnboundedSender<MessageType>,
    rx: tokio::sync::mpsc::UnboundedReceiver<MessageType>,
    // pub anime_list: components::asyncListWidget::AsyncListWidget,
    anime: AnimeApp,
    manga: MangaApp,
    settings: SettingApp,
    askToQuit: AskToQuitApp,
}


struct AnimeApp {
    anime_source: Arc<Box<dyn AnimeSource + Send + Sync + 'static>>,
    tx: tokio::sync::mpsc::UnboundedSender<MessageType>,
    //rx: tokio::sync::mpsc::UnboundedReceiver<MessageType>,
    anime_list_state: Rc<RefCell<AsyncAnimeState>>,
    library: LibraryApp,
    search: SearchApp
}
struct MangaApp {
    manga_source: Arc<Box<dyn MangaSource + Send + Sync + 'static>>,
    tx: tokio::sync::mpsc::UnboundedSender<MessageType>,
    //rx: tokio::sync::mpsc::UnboundedReceiver<MessageType>,
    manga_list_state: Rc<RefCell<AsyncMangaState>>,
    library: LibraryApp,
    search: SearchApp
}
struct SettingApp {
    // tx: tokio::sync::mpsc::UnboundedSender<MessageType>,
    //rx: tokio::sync::mpsc::UnboundedReceiver<MessageType>,
    state: Rc<RefCell<SettingsState>>
}


struct AskToQuitApp {
    state: Rc<RefCell<AskToQuitState>>,
}

struct LibraryApp {
    tx: tokio::sync::mpsc::UnboundedSender<MessageType>,
    //rx: tokio::sync::mpsc::UnboundedReceiver<MessageType>,
    library_list_state: Rc<RefCell<LibraryState>>
}
struct SearchApp {
    tx: tokio::sync::mpsc::UnboundedSender<MessageType>,
    //rx: tokio::sync::mpsc::UnboundedReceiver<MessageType>,
    search_list_state: Rc<RefCell<SearchState>>
}


#[derive(Clone, Copy, PartialEq, Eq)]
enum AppState {
    Running,
    Settings,
    AskToQuit,
    Quitting
}
impl App {
    fn new()->App {
        let (tx,   rx)          = tokio::sync::mpsc::unbounded_channel::<appmessages::MessageType>();
        // let (tx_m,   rx_m)          = tokio::sync::mpsc::unbounded_channel::<AsyncMangaStateStatus>();
        // let (tx_s,     rx_s)        = tokio::sync::mpsc::unbounded_channel::<ApplySettingsChange>();
        // let (tx_la,     rx_la)      = tokio::sync::mpsc::unbounded_channel::<HashMap<String, EpisodeListItem>>();
        // let (tx_las,    rx_las)     = tokio::sync::mpsc::unbounded_channel::<SearchStateStatus>();
        // let (tx_lm,     rx_lm)      = tokio::sync::mpsc::unbounded_channel::<HashMap<String, EpisodeListItem>>();
        // let (tx_lms,    rx_lms)     = tokio::sync::mpsc::unbounded_channel::<SearchStateStatus>();
        let anime_source:Arc<Box<dyn AnimeSource + Send + Sync + 'static>>      = Arc::new(Extension::defaultAnimeSource().unwrap_or(Box::new(animeworld::Animeworld::new("animeworld.ac".to_owned(), "https://animeworld.ac".to_owned())))); // TODO Make anime source selectable
        let manga_source:Arc<Box<dyn MangaSource + Send + Sync + 'static>>      = Arc::new(Extension::defaultMangaSource().unwrap_or(Box::new(mangaworld::Mangaworld::new("mangaworld.mx".to_owned(), "https://mangaworld.mx".to_owned())))); // TODO Make anime source selectable
        let a_s = anime_source.clone();
        let m_s = manga_source.clone();
        // Asktoquit
        let quit_tx_s = tx.clone();
        let stay_tx_s = tx.clone();
        Self {
            state: AppState::Running,
            navigation_menu: SelectableList::default(),
            tx: tx.clone(),
            rx: rx,
            anime: AnimeApp {
                anime_source,
                anime_list_state: Rc::new(RefCell::new(AsyncAnimeState::new(a_s))),
                tx: tx.clone(),
                //rx: rx,
                library: LibraryApp {
                    tx: tx.clone(),
                    //rx: rx,
                    library_list_state: Rc::new(RefCell::new(LibraryState::new()))
                },
                search: SearchApp { tx: tx.clone(),
                    //rx:rx,
                    search_list_state: Rc::new(RefCell::new(SearchState::new(getConfig().Anime.source.clone(), Extension::lsSource(SourceType::Anime).unwrap_or_default()))) }
            },
            manga: MangaApp {
                manga_source,
                tx: tx.clone(),
                //rx: rx,
                manga_list_state: Rc::new(RefCell::new(AsyncMangaState::new(m_s))),
                library: LibraryApp {
                    tx: tx.clone(),
                    //rx: rx,
                    library_list_state: Rc::new(RefCell::new(LibraryState::new()))
                },
                search: SearchApp { tx: tx.clone(), //rx:rx,
                    search_list_state: Rc::new(RefCell::new(SearchState::new(getConfig().Manga.source.clone(), Extension::lsSource(SourceType::Manga).unwrap_or_default()))) }
            },
            settings: SettingApp {
                state: Rc::new(RefCell::new(SettingsState::new(tx))),
                //rx: rx
            },
            askToQuit: AskToQuitApp { state:  Rc::new(RefCell::new(AskToQuitState {
                sel_index: 0,
                items: vec![
                    ListFuncOption {
                        name: "Quit".to_owned(),
                        style: Some(ratatui::style::Style::default().bg(tailwind::RED.c300).white()), // White with c700
                        func: Box::new(move || {
                            quit_tx_s.send(appmessages::MessageType::SettingsChange(ApplySettingsChange::setAppStatus(AppState::Quitting)));
                        })
                    },
                    ListFuncOption {
                        name: "Stay".to_owned(),
                        style: Some(ratatui::style::Style::default().bg(tailwind::GREEN.c300).white()), // White with c700
                        func: Box::new(move || {
                            stay_tx_s.send(MessageType::SettingsChange(ApplySettingsChange::setAppStatus(AppState::Running)));
                        })
                    }
                ]
            }))}
            // library: LibraryApp {
            //     tx: tx_l,
            //     rx: rx_l,
            //     library_list_state: Rc::new(RefCell::new(LibraryState::new()))
            // }
            // anime_list: AsyncListWidget {
            //     title: "Animeworld".into(),
            //     state: AsyncState::Empty,
            //     scroll_state: ListState::default(),
            //     animeurlhash: HashMap::new()
            // },
        }
    }
    fn with_anime_source(&mut self, a: &str) {
        if let Ok(nas) = extension::Extension::buildfromName(source::SourceType::Anime, a) {
            let anime_s = Arc::new(nas.toAnimeSource());
            self.anime.anime_source=anime_s.clone();
            self.anime.anime_list_state.borrow_mut().resetFrom(AsyncAnimeState::new(anime_s));
        }
    }
    fn with_manga_source(&mut self, m: &str) {
        if let Ok(nas) = extension::Extension::buildfromName(source::SourceType::Manga, m) {
            let manga_s = Arc::new(nas.toMangaSource());
            self.manga.manga_source=manga_s.clone();
            self.manga.manga_list_state.borrow_mut().resetFrom(AsyncMangaState::new(manga_s));
        }
    }
    fn init(&mut self) {
        // let tx_clone = self.tx.clone();
        self.navigation_menu = SelectableList::new(HashMap::from([
            ("Anime".to_owned(), ListDiveChild::new(
                "Anime".to_owned(),
                0,
                ActionType::SubMenu,
                Some(HashMap::from([
                    ("Nuovi".to_owned(), ListDiveChild::new(
                        "Nuovi".to_owned(),
                        0,
                        ActionType::Render,
                        None,
                        Some(self.block_anime_updated()),
                    )),
                    ("Libreria".to_owned(), ListDiveChild::new(
                        "Libreria".to_owned(),
                        1,
                        ActionType::Render,
                        None,
                        Some(self.block_library_anime()),//Some(self.new_anime_list()),
                    )),
                    ("Search".to_owned(), ListDiveChild::new(
                        "Search".to_owned(),
                        2,
                        ActionType::Render,
                        None,
                        Some(self.searchAnime())
                    ))
                ])),
                None
            )),
            ("Manga".to_owned(), ListDiveChild::new(
                "Manga".to_owned(),
                1,
                ActionType::SubMenu,
                Some(HashMap::from([
                    ("Nuovi".to_owned(),
                        ListDiveChild::new("Nuovi".to_owned(),
                        0,
                        ActionType::Render,
                        None,
                        Some(self.block_manga_updated())
                    )),
                    ("Library".to_owned(), ListDiveChild::new(
                        "Library".to_owned(),
                        1,
                        ActionType::Render,
                        None,
                        Some(self.block_library_manga())
                    )),
                    ("Search".to_owned(), ListDiveChild::new(
                        "Search".to_owned(),
                        2,
                        ActionType::Render,
                        None,
                        Some(self.searchManga())
                    ))
                ])),
                None
            )),
            // ("Settings".to_owned(), ListDiveChild::new(
            //     "Settings".to_owned(), 2, ActionType::SubMenu, Some(HashMap::from([
            //         ("Source".to_owned(), ListDiveChild::new(
            //             "Source".to_owned(),
            //             0,
            //             ActionType::Render,
            //             None,
            //             Some(self.settings_selectSource())
            //         ))
            //     ])), None))
        ]),Some(Self::generate_block()));
    }

    fn update(&mut self) {
        match self.rx.try_recv() {
            Ok(MessageType::AnimeState(a))=> {
                self.anime.anime_list_state.borrow_mut().status=a;
            },
            Ok(MessageType::LoadEpisodes(h)) => {
                self.anime.library.library_list_state.borrow_mut().episodes_loaded = Some(h);
                self.anime.library.library_list_state.borrow_mut().entry_selected = true;
            },
            Ok(MessageType::MangaState(a)) => {
                self.manga.manga_list_state.borrow_mut().status=a;
            },
            Ok(MessageType::LoadChapters(h)) =>{
                self.manga.library.library_list_state.borrow_mut().episodes_loaded = Some(h);
                self.manga.library.library_list_state.borrow_mut().entry_selected = true;
            },
            Ok(MessageType::SettingsChange(a)) => {
                match a {
                    ApplySettingsChange::setAnime(a) => {
                        self.with_anime_source(&a);
                    },
                    ApplySettingsChange::setManga(m) => {
                        self.with_manga_source(&m);
                    }
                    ApplySettingsChange::setAppStatus(app_state) => {
                        self.state = app_state
                    },

                    _=> {}
                }
            },
            Ok(MessageType::AnimeSearch(a)) => {
                match a {
                    SearchStateStatus::Empty => {},
                    _ => {}
                }
            },
            Ok(MessageType::FoundMangaFromQuery(m)) => {
                self.manga.search.search_list_state.borrow_mut().lib = m.into_iter().map(|e| (e.title.clone(), e)).collect();
            },
            Ok(MessageType::ListChaptersFromQuery(m))=>{
                self.manga.search.search_list_state.borrow_mut().episodes_loaded = m;
            },
            Ok(MessageType::FoundAnimeFromQuery(m)) => {
                self.anime.search.search_list_state.borrow_mut().lib = m.into_iter().map(|e| (e.title.clone(), e)).collect();
            },
            Ok(MessageType::ListEpisodesFromQuery(m))=>{
                self.anime.search.search_list_state.borrow_mut().episodes_loaded = m;
            }
            Ok(_) => {},
            Err(_) => {}
        }
    }
    async fn run(&mut self, mut terminal: DefaultTerminal) -> Result<()> {
        self.init();
        while self.state != AppState::Quitting {
            self.update();
            // log!("Draw");
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn handle_events(&mut self) -> Result<()> {
        if event::poll(Duration::from_millis(500))? {
            match &self.state {
                AppState::Running => {
                    if let Event::Key(key) = event::read()? {
                        if key.kind == KeyEventKind::Press {
                            if key.modifiers.contains(KeyModifiers::CONTROL) {
                                match key.code {
                                    KeyCode::Char('c') => {self.state = AppState::Quitting;return Ok(())},
                                    KeyCode::Char('s') => {self.state = AppState::Settings; return Ok(())},
                                    _=>{}
                                }
                            }
                            match key.code {
                                k @ (KeyCode::Esc | KeyCode::Backspace) => {
                                    if !self.navigation_menu.handle_input(k) {
                                        if !self.navigation_menu.goback() {
                                            self.quit()
                                        } else {
                                            self.anime.anime_list_state.borrow_mut().status = AsyncAnimeStateStatus::Empty;
                                            self.manga.manga_list_state.borrow_mut().status = AsyncMangaStateStatus::Empty;
                                        }
                                    }
                                },
                                // KeyCode::Backspace => {self.navigation_menu.goback();},
                                k @ (KeyCode::Up | KeyCode::Char('w'))=> {
                                    if self.navigation_menu.lastpath.is_empty() {
                                        self.navigation_menu.state.select_previous();
                                    } else {
                                        self.navigation_menu.handle_input(k);
                                    }
                                },
                                k @ (KeyCode::Down | KeyCode::Char('s'))=> {
                                    if self.navigation_menu.lastpath.is_empty() {
                                        self.navigation_menu.state.select_next();
                                    } else {
                                        self.navigation_menu.handle_input(k);
                                    }
                                },
                                KeyCode::Enter => {
                                    if self.navigation_menu.lastpath.is_empty() {
                                        self.navigation_menu.enter();
                                    } else {
                                        self.navigation_menu.handle_input(KeyCode::Enter);
                                    }
                                },
                                a => {
                                    if !self.navigation_menu.lastpath.is_empty() {
                                        self.navigation_menu.handle_input(a);
                                    }
                                }
                            }
                        }
                    }
                },
                AppState::Settings => {
                    if let Event::Key(key) = event::read()? {
                        if key.kind == KeyEventKind::Press {
                            if key.modifiers.contains(KeyModifiers::CONTROL) {
                                match key.code {
                                    KeyCode::Char('c') => {self.quit();return Ok(())},
                                    _=>{}
                                }
                            }
                            let mut stg_state = self.settings.state.borrow_mut();


                            if stg_state.is_editing() {
                                if key.modifiers.contains(KeyModifiers::CONTROL) {
                                    match key.code {

                                        k @ (KeyCode::Backspace | KeyCode::Char(_) | KeyCode::Right | KeyCode::Left | KeyCode::Up | KeyCode::Down) => {stg_state.handle_control_inputs(k);return Ok(())},
                                        _=>{}
                                    }
                                }
                                match key.code {
                                    k @ (KeyCode::Esc ) => {
                                        self.state = AppState::Settings;
                                        // ? DO NOT SAVE
                                        stg_state.turn_editing_mode(false);
                                    },
                                    k @ (KeyCode::Backspace | KeyCode::Char(_) | KeyCode::Enter | KeyCode::Right | KeyCode::Left | KeyCode::Up | KeyCode::Down | KeyCode::Insert | KeyCode::Delete) => {
                                        stg_state.handle_inputs(k);
                                    }
                                    _ => {}
                                }
                            } else {
                                if key.modifiers.contains(KeyModifiers::SHIFT) {
                                    match key.code {
                                        k @ (KeyCode::Up | KeyCode::Char('w')) => {
                                            stg_state.select_sec_prev();
                                            return Ok(());
                                        },
                                        k @ (KeyCode::Down | KeyCode::Char('s')) => {
                                            stg_state.select_sec_next();
                                            return Ok(());
                                        }
                                        _=>{}
                                    }
                                }
                                match key.code {
                                    k @ (KeyCode::Esc | KeyCode::Backspace) => {
                                        self.state = AppState::Running
                                    },
                                    KeyCode::Enter => {
                                        stg_state.turn_editing_mode(true)
                                    }
                                    k @ (KeyCode::Up | KeyCode::Char('w')) => {
                                        stg_state.selectprev();
                                    },
                                    k @ (KeyCode::Down | KeyCode::Char('s')) => {
                                        stg_state.selectnext();
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                },
                AppState::Quitting => {}
                AppState::AskToQuit => {
                    if let Event::Key(key) = event::read()? {
                        if key.kind == KeyEventKind::Press {
                            let mut ask_to_quit_state = self.askToQuit.state.borrow_mut();
                            match key.code {
                                k @ (KeyCode::Esc | KeyCode::Backspace) => {
                                    self.state = AppState::Running
                                },
                                KeyCode::Enter => {
                                    if let Some(s) = ask_to_quit_state.get_selected_item() {
                                        (s.func)();
                                    }
                                }
                                k @ (KeyCode::Left | KeyCode::Char('a')) => {
                                    ask_to_quit_state.select_previous();
                                },
                                k @ (KeyCode::Right | KeyCode::Char('d')) => {
                                    ask_to_quit_state.select_next();
                                }
                                _ => {}
                            }
                        }
                    }
                },
            }
        }
        Ok(())
    }
    pub fn quit(&mut self) {
        if (getConfig().General.askToQuit) {
            self.state = AppState::AskToQuit;
        } else {
            self.state = AppState::Quitting;
        }
    }

    fn draw(&mut self, frame: &mut Frame) {
        let lay = Layout::default()
        .direction(Direction::Horizontal)
        .margin(1)
        .constraints(vec![
            Constraint::Percentage(15),
            Constraint::Percentage(85),
        ])
        .split(frame.area());
        let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![
            Constraint::Percentage(85),
            Constraint::Percentage(15),
        ])
        .split(lay[0]);
        let cmdAndView = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![
            Constraint::Fill(1),
            Constraint::Length(3)
        ]);
        self.navigation_menu.render_list(frame, vert[0]);
        // self.navigation_menu(frame, lay[1]);
        self.navigation_menu.render_menu(frame, lay[1], vert[1]);

        if self.state == AppState::Settings {
            let popup_block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(tailwind::INDIGO.c200))
            .title("Settings");
            let centered_area = lay[1].centered(Constraint::Percentage(60), Constraint::Percentage(20));
            let innarea = popup_block.inner(centered_area);
            // clears out any background in the area before rendering the popup
            frame.render_widget(Clear, centered_area);

            let mut stg = Settings::new();
            let mut state = self.settings.state.borrow_mut();

            frame.render_widget(popup_block, centered_area);

            frame.render_stateful_widget(stg, innarea, &mut *state);
        } else if self.state == AppState::AskToQuit {
            let popup_block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(tailwind::RED.c800))
            .title("Quit?");
            let centered_area = lay[1].centered(Constraint::Percentage(60), Constraint::Percentage(10));
            let innarea = popup_block.inner(centered_area);
            // clears out any background in the area before rendering the popup
            frame.render_widget(Clear, centered_area);

            let mut atq = AskToQuit {};
            let mut state = self.askToQuit.state.borrow_mut();

            frame.render_widget(popup_block, centered_area);

            frame.render_stateful_widget(atq, innarea, &mut *state);
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

/**
 * Widget inside app
 */
pub enum CmdListType {
    Single,Multi
}
impl App {
    pub fn generate_cmd_list(list: Vec<(CmdListType,&str, &str)>) -> Paragraph<'static> {
        let b = Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(ratatui::style::Color::LightCyan));
        let mut string_v:Vec<Span<'_>> = Vec::with_capacity(list.len()*5);
        let cmd_style = Style::new().fg(ratatui::style::Color::LightCyan);
        let text_style = Style::new().fg(ratatui::style::Color::White);
        for l in list {
            match l.0 {
                CmdListType::Multi => {
                    for key in l.1.split(" or ") {
                        let mut sp= key.split(",");
                        if let Some(first) = sp.next() {
                            string_v.push(Span::styled("[",text_style));
                            string_v.push(Span::styled(first.to_owned(),cmd_style));
                            while let Some(k) = sp.next() {
                                string_v.push(Span::styled(",",text_style));
                                string_v.push(Span::styled(k.to_string(),cmd_style));
                            }
                            string_v.push(Span::styled("]",text_style));
                            string_v.push(Span::styled(" or ",text_style));
                        }
                    }
                    string_v.pop();
                    string_v.push(Span::styled(" ",text_style));
                    string_v.push(Span::styled(l.2.to_owned(),text_style));
                },
                CmdListType::Single => {
                    string_v.push(Span::styled("[",text_style));
                    string_v.push(Span::styled(l.1.to_owned(),cmd_style));
                    string_v.push(Span::styled("] ",text_style));
                    string_v.push(Span::styled(l.2.to_owned(),text_style));
                }
            }
            string_v.push(Span::styled(";",text_style));
            string_v.push(Span::styled(" ",text_style));
        }
        string_v.pop();
        let line: Line<'_> = Line::from(string_v);
        Paragraph::new(line).block(b).centered()
        // Paragraph::new(String::from("[W,A,S,D] or [Arrows] to move;\t[Enter] select;\t[Esc] Go Back")).block(b).centered()
    }
    pub fn generate_block() -> CurrentWidget {
        CurrentWidget {
            render: Box::new(|frame: &mut Frame, area: Rect, cmdarea: Rect| {
                let b = Block::new()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::new().fg(ratatui::style::Color::Cyan));
                // let lay = Layout::default()
                // .direction(Direction::Vertical)
                // .constraints(vec![
                //     Constraint::Percentage(100),
                //     Constraint::Min(3)
                // ]).split(area);
                let cmd_list = Self::generate_cmd_list(vec![
                    (CmdListType::Multi, "W,A,S,D or Arrows","to move"),
                    (CmdListType::Single, "Enter","select"),
                    (CmdListType::Single, "Esc","Go Back"),
                ]);
                frame.render_widget(b, area);
                frame.render_widget(cmd_list, cmdarea);
            }),
            handle_input: Box::new(|_| {false})
        }
    }
    pub fn block_anime_updated(&mut self) -> CurrentWidget {
        // let ls =.borrow_mut(); //
        let render_ls = Rc::clone(&self.anime.anime_list_state);
        let input_ls = Rc::clone(&self.anime.anime_list_state);
        let tx = self.anime.tx.clone();
        // let sc = self.anime_source.clone();
        CurrentWidget {
            render: Box::new(move |frame: &mut Frame, area: Rect, cmdarea| {
                let b = Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::new().fg(ratatui::style::Color::LightCyan));
                // let lay = Layout::default()
                // .direction(Direction::Vertical)
                // .constraints(vec![
                //     Constraint::Percentage(100),
                //     Constraint::Min(3)
                // ]).split(area);
                // let fld = match read_anime_folder() {
                //     Ok(a) => a,
                //     _ => vec::Vec::new()
                // };
                // let items = fld.iter().enumerate().map(|(i, folder)| {
                //     ListItem::new(folder.to_string_lossy().into_owned())
                // });
                let mut state = render_ls.borrow_mut();
                let tx_clone = tx.clone();

                let l = AsyncUpdateAnimeList::AsyncUpdateAnimeList::new(tx_clone);//AsyncListWidget::new("Animeworld".to_owned(), self.animeSource);

                frame.render_stateful_widget( l, area, &mut  *state);
                let cmd_list = Self::generate_cmd_list(vec![
                    (CmdListType::Multi, "W,A,S,D or Arrows","to move"),
                    (CmdListType::Single, "Enter","select"),
                    (CmdListType::Single, "Esc","Go Back"),
                ]);  //// NOT WORK with match: Keybinds configuration from file;
                frame.render_widget( cmd_list, cmdarea);

            }),
            handle_input: Box::new(move |k: KeyCode| {
                let mut state = input_ls.borrow_mut();
                match state.status {
                    AsyncAnimeStateStatus::Success(_) => match k {
                        KeyCode::Up | KeyCode::Char('w') => {
                            state.list_state.select_previous();
                            true
                        },
                        KeyCode::Down | KeyCode::Char('s')=> {
                            state.list_state.select_next();
                            true
                        },
                        KeyCode::Left | KeyCode::Char('a') => {
                            if state.animesource.has_update_page() {
                                state.list_state.select(None);
                                state.page = std::cmp::max(state.page - 1, 1);
                                state.status = AsyncAnimeStateStatus::Empty;
                                return true;
                            }
                            false
                        },
                        KeyCode::Right | KeyCode::Char('d') => {
                            if state.animesource.has_update_page() {
                                state.list_state.select(None);
                                state.page = state.page + 1;
                                state.status = AsyncAnimeStateStatus::Empty;
                                return true;
                            }
                            false
                        },
                        KeyCode::Enter => {
                            if let Some(u) = state.list_state.selected() && let Some(res) = state.res.clone() {
                                let items = utils::utils::hash_sort(&res);
                                if let Some(item) = items.get(u) {
                                    if let Some(url) = res.get(*item) {
                                        //source.episode_play_url(url);
                                        let so = state.animesource.clone();
                                        let u = url.clone();
                                        tokio::spawn(async move {
                                            let mut player = getConfig().Anime.player.buildPlayer(getDevice());
                                            if let Ok(vlc_url) = so.episode_play_url(&u).await {
                                                if !vlc_url.is_empty() {
                                                    let _ = player.play(&vlc_url);
                                                }
                                            }
                                        });
                                    }
                                }
                            }
                            true
                        }
                        _ => {false}
                    },
                    _=>{false}
                }
                // match k {
                //     KeyCode::Up => {
                //         state.list_state.select_previous();
                //     },
                //     KeyCode::Down => {
                //         state.list_state.select_next();
                //     }
                //     _ => {}
                // }
            })
        }
    }
    pub fn block_manga_updated(&mut self) -> CurrentWidget {
        // let ls =.borrow_mut(); //
        let render_ls = Rc::clone(&self.manga.manga_list_state);
        let input_ls = Rc::clone(&self.manga.manga_list_state);
        let tx = self.manga.tx.clone();
        // let sc = self.manga_source.clone();
        CurrentWidget {
            render: Box::new(move |frame: &mut Frame, area: Rect, carea: Rect| {
                let b = Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::new().fg(ratatui::style::Color::LightCyan));
                // let lay = Layout::default()
                // .direction(Direction::Vertical)
                // .constraints(vec![
                //     Constraint::Percentage(100),
                //     Constraint::Min(3)
                // ]).split(area);
                let mut state = render_ls.borrow_mut();
                let tx_clone = tx.clone();

                let l = AsyncUpdateMangaList::AsyncUpdateMangaList::new(tx_clone);

                frame.render_stateful_widget( l, area, &mut  *state);
                let cmd_list = Self::generate_cmd_list(vec![
                    (CmdListType::Multi, "W,A,S,D or Arrows","to move"),
                    (CmdListType::Single, "Enter","select"),
                    (CmdListType::Single, "Esc","Go Back"),
                ]);
                frame.render_widget( cmd_list, carea);

            }),
            handle_input: Box::new(move |k: KeyCode| {
                let mut state = input_ls.borrow_mut();
                match state.status {
                    AsyncMangaStateStatus::Success(_) => match k {
                        KeyCode::Up | KeyCode::Char('w') => {
                            state.list_state.select_previous();
                            true
                        },
                        KeyCode::Down | KeyCode::Char('s')=> {
                            state.list_state.select_next();
                            true
                        },
                        KeyCode::Left | KeyCode::Char('a') => {
                            if state.mangasource.has_update_page() {
                                state.list_state.select(None);
                                state.page = std::cmp::max(state.page - 1, 1);
                                state.status = AsyncMangaStateStatus::Empty;
                                return true;
                            }
                            false
                        },
                        KeyCode::Right | KeyCode::Char('d') => {
                            if state.mangasource.has_update_page() {
                                state.list_state.select(None);
                                state.page = state.page + 1;
                                state.status = AsyncMangaStateStatus::Empty;
                                return true;
                            }
                            false
                        },
                        KeyCode::Enter => {
                            if let Some(u) = state.list_state.selected() && let Some(res) = state.res.clone() {
                                let items = utils::utils::hash_sort(&res);
                                if let Some(item) = items.get(u) {
                                    if let Some(url) = res.get(*item) {
                                        //source.episode_play_url(url);
                                        let so: Arc<Box<dyn MangaSource + Send + Sync>> = state.mangasource.clone();
                                        let u = url.clone();
                                        let title = item.split_once("di");
                                        source::openManga(so, MangaMetadata {
                                            title: title.map(|x| x.1.trim().to_owned()), url: url.clone(), chapter: title.map(|x| x.0.trim().trim_start_matches("Capitolo").trim().to_owned()), volume: None
                                        });
                                    }
                                }
                            }
                            true
                        }
                        _ => {false}
                    },
                    _=>{false}
                }
                // match k {
                //     KeyCode::Up => {
                //         state.list_state.select_previous();
                //     },
                //     KeyCode::Down => {
                //         state.list_state.select_next();
                //     }
                //     _ => {}
                // }
            })
        }
    }
    pub fn block_library_anime(&mut self) -> CurrentWidget {
        // let render_ls = Rc::clone(list_state);
        // let input_ls = Rc::clone(list_state);
        let lib_type = Library::LibraryType::Anime;
        let render_ls = Rc::clone(&self.anime.library.library_list_state);
        let input_ls = Rc::clone(&self.anime.library.library_list_state);
        let tx_clone_render = self.anime.library.tx.clone();
        let tx_clone_input = self.anime.library.tx.clone();
        CurrentWidget {
            render: Box::new(move |frame: &mut Frame, area: Rect, carea: Rect| {
                let b = Block::new()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::new().fg(ratatui::style::Color::Cyan));
                let mut state = render_ls.borrow_mut();
                let l = Library::Library::new(lib_type,tx_clone_render.clone()); //TODO
                let cmd_list = Self::generate_cmd_list(vec![
                    (CmdListType::Multi, "W,A,S,D or Arrows","to move"),
                    (CmdListType::Single, "Enter","select"),
                    (CmdListType::Single, "Esc","Go Back"),
                    (CmdListType::Single, "R","Reverse list of episodes")
                ]);
                frame.render_widget( cmd_list, carea);
                //frame.render_widget(b, area);
                frame.render_stateful_widget(l, area, &mut *state);
            }), handle_input:  Box::new(move |k: KeyCode| {
                let mut state = input_ls.borrow_mut();
                let tx_clone_input_clone = tx_clone_input.clone();
                match  state.status {
                    LibraryStateStatus::Lib(p) => {
                        match k {
                            KeyCode::Char(c) => {
                                match (c) {
                                    'r'=>{
                                        if state.entry_selected {
                                            state.flag_reverse = !state.flag_reverse;
                                        }else {
                                            state.status=LibraryStateStatus::Empty;
                                        }
                                        true
                                    }
                                    _ =>{false}
                                }
                            }
                            KeyCode::Tab =>{
                                if !state.entry_selected {
                                    return state.label.next();
                                }
                                false
                            }
                            KeyCode::BackTab  => {
                                if !state.entry_selected {
                                    return state.label.prev();
                                }
                                false
                            },
                            KeyCode::Tab if k.is_modifier(event::ModifierKeyCode::LeftShift) => {
                                if !state.entry_selected {
                                    return state.label.prev();
                                }
                                false
                            },
                            KeyCode::Left | KeyCode::Char('a') => {
                                if state.entry_selected {

                                } else {
                                    state.list_state_source.select(None);
                                    if p>0 {
                                        state.status = LibraryStateStatus::Lib(p-1);
                                    } else {
                                        state.status = LibraryStateStatus::Lib(state.lib.keys_len()-1);
                                    }
                                }
                                true
                            },
                            KeyCode::Right | KeyCode::Char('d') => {
                                if state.entry_selected {

                                } else {
                                    state.list_state_source.select(None);
                                    state.status = LibraryStateStatus::Lib(p+1);
                                }
                                true
                            },
                            KeyCode::Up | KeyCode::Char('w') => {
                                if state.entry_selected {
                                    if state.list_state_eps.selected().is_none() {
                                        state.list_state_eps.select_last();
                                    }else {
                                        state.list_state_eps.select_previous();
                                    }
                                } else {
                                    if state.list_state_source.selected().is_none() {
                                        state.list_state_source.select_last();
                                    }else {
                                        state.list_state_source.select_previous();
                                    }
                                }
                                true
                            },
                            KeyCode::Down | KeyCode::Char('s')=> {
                                if state.entry_selected {
                                    if state.list_state_eps.selected().is_none() {
                                        state.list_state_eps.select_first();
                                    }else {
                                        state.list_state_eps.select_next();
                                    }
                                } else {
                                    if state.list_state_source.selected().is_none() {
                                        state.list_state_source.select_first();
                                    }else {
                                        state.list_state_source.select_next();
                                    }
                                }
                                true
                            },
                            KeyCode::Enter => {
                                let state_bo = &state;
                                if let Some(sel) = state_bo.list_state_source.selected() {
                                    if let Some(u) = state_bo.lib.get(sel, state_bo.label.get_current()) {
                                        let u_clone = u.clone();
                                        if state_bo.entry_selected {
                                            if let Some(uu) = state_bo.list_state_eps.selected() && let Some(episode_loaded) = &state_bo.episodes_loaded {
                                                let mut items = Library::Library::sorted_keys_from_hash(&episode_loaded);
                                                if (state.flag_reverse) {
                                                    items.reverse();
                                                }
                                                if let Some(item) = items.get(uu) {
                                                    if let Some(eli) = episode_loaded.get(*item).clone() {
                                                        let eli_c = (*eli).clone();
                                                        tokio::spawn(async move {
                                                            if let Ok(ex) = Extension::buildfromName(SourceType::Anime, &u_clone.ext){
                                                                let s_a = ex.toAnimeSource();
                                                                let _ = s_a.lib_ep_select(eli_c).await;
                                                            }
                                                        });
                                                    }
                                                }
                                            }
                                        } else {
                                            tokio::spawn(async move {
                                                if let Ok(ex) = Extension::buildfromName(SourceType::Anime, &u_clone.ext) {
                                                    let a: Box<dyn AnimeSource + Send + Sync> = ex.toAnimeSource();
                                                    let hs = a.lib_url_parse(u_clone.source).await;
                                                    if let Ok(o) = hs {
                                                        let _ = tx_clone_input_clone.send(MessageType::LoadEpisodes(o));
                                                    }
                                                }
                                            });
                                        }
                                    }
                                }
                                true
                            },
                            KeyCode::Esc | KeyCode::Backspace => {
                                if state.entry_selected {
                                    state.entry_selected = false;
                                    return true;
                                } else if state.label.get_current().is_some() {
                                    return state.label.clear();
                                }
                                false
                            }
                            _ => {false}
                        }
                    },
                    _ => {false}
                }
            })
        }
    }
    pub fn block_library_manga(&mut self) -> CurrentWidget {
        // let lib_type = Library::LibraryType::Manga;
        // let render_ls = Rc::clone(list_state);
        // let input_ls = Rc::clone(list_state);
        let render_ls = Rc::clone(&self.manga.library.library_list_state);
        let input_ls = Rc::clone(&self.manga.library.library_list_state);
        let tx_clone_render = self.manga.library.tx.clone();
        let tx_clone_input = self.manga.library.tx.clone();
        CurrentWidget {
            render: Box::new(move |frame: &mut Frame, area: Rect, carea: Rect| {
                let b = Block::new()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::new().fg(ratatui::style::Color::Cyan));
                let mut state = render_ls.borrow_mut();
                let l = Library::Library::new(Library::LibraryType::Manga,tx_clone_render.clone()); //TODO
                let cmd_list = Self::generate_cmd_list(vec![
                    (CmdListType::Multi, "W,A,S,D or Arrows","to move"),
                    (CmdListType::Single, "Enter","select"),
                    (CmdListType::Single, "Esc","Go Back"),
                    (CmdListType::Single, "R","Reverse list of chpaters")
                ]);
                frame.render_widget( cmd_list, carea);
                //frame.render_widget(b, area);
                frame.render_stateful_widget(l, area, &mut *state);
            }), handle_input:  Box::new(move |k: KeyCode| {
                let mut state = input_ls.borrow_mut();
                let tx_clone_input_clone = tx_clone_input.clone();
                match  state.status {
                    LibraryStateStatus::Lib(p) => {
                        match k {
                            KeyCode::Char(c) => {
                                match (c) {
                                    'r'=>{
                                        if state.entry_selected {
                                            state.flag_reverse = !state.flag_reverse;
                                        }else {
                                            state.status=LibraryStateStatus::Empty;
                                        }
                                        true
                                    }
                                    _ =>{false}
                                }
                            },
                            KeyCode::Left | KeyCode::Char('a') => {
                                if state.entry_selected {

                                } else {
                                    state.list_state_source.select(None);
                                    if p>0 {
                                        state.status = LibraryStateStatus::Lib(p-1);
                                    } else {
                                        state.status = LibraryStateStatus::Lib(state.lib.keys_len()-1);
                                    }
                                }
                                true
                            },
                            KeyCode::Right | KeyCode::Char('d') => {
                                if state.entry_selected {

                                } else {
                                    state.list_state_source.select(None);
                                    state.status = LibraryStateStatus::Lib(p+1);
                                }
                                true
                            },
                            KeyCode::Up | KeyCode::Char('w') => {
                                if state.entry_selected {
                                    if state.list_state_eps.selected().is_none() {
                                        state.list_state_eps.select_last();
                                    }else {
                                        state.list_state_eps.select_previous();
                                    }
                                } else {
                                    if state.list_state_source.selected().is_none() {
                                        state.list_state_source.select_last();
                                    }else {
                                        state.list_state_source.select_previous();
                                    }
                                }
                                true
                            },
                            KeyCode::Down | KeyCode::Char('s')=> {
                                if state.entry_selected {
                                    if state.list_state_eps.selected().is_none() {
                                        state.list_state_eps.select_first();
                                    }else {
                                        state.list_state_eps.select_next();
                                    }
                                } else {
                                    if state.list_state_source.selected().is_none() {
                                        state.list_state_source.select_first();
                                    }else {
                                        state.list_state_source.select_next();
                                    }
                                }
                                true
                            },
                            KeyCode::Tab =>{
                                if !state.entry_selected {
                                    return state.label.next();
                                }
                                false
                            },
                            KeyCode::BackTab => {
                                if !state.entry_selected {
                                    return state.label.prev();
                                }
                                false
                            },
                            KeyCode::Tab if k.is_modifier(event::ModifierKeyCode::LeftShift) => {
                                if !state.entry_selected {
                                    return state.label.prev();
                                }
                                false
                            },
                            KeyCode::Enter => {
                                let state_bo = &state;
                                if let Some(sel) = state_bo.list_state_source.selected() {
                                    if let Some(u) = state_bo.lib.get(sel, state_bo.label.get_current()) {
                                        let u_clone = u.clone();
                                        if state_bo.entry_selected {
                                            if let Some(uu) = state_bo.list_state_eps.selected() && let Some(episode_loaded) = &state_bo.episodes_loaded {
                                                let mut items = Library::Library::sorted_keys_from_hash(&episode_loaded);
                                                if (state.flag_reverse) {
                                                    items.reverse();
                                                }
                                                if let Some(item) = items.get(uu) {
                                                    if let Some(eli) = episode_loaded.get(*item).clone() {
                                                        let eli_c = (*eli).clone();
                                                        let title = state_bo.list_state_source.selected().and_then(|sel| state_bo.lib.get(sel,state_bo.label.get_current()).and_then(|f| Some(f.name)));
                                                        if let Ok(ex) = Extension::buildfromName(SourceType::Manga, &u_clone.ext){
                                                            let s_m: Box<dyn MangaSource + Send + Sync> = ex.toMangaSource();
                                                            let as_m = Arc::new(s_m);
                                                            let chp =  eli_c.title.split_whitespace().last().map(|x| x.to_owned());
                                                            openManga(as_m, MangaMetadata { title: title, url: eli_c.url, chapter: chp, volume: None});
                                                        }
                                                        // });
                                                    }
                                                }
                                            }
                                        } else {

                                            let u_clone = u.clone();
                                            // let lib_type_clone = lib_type.clone();
                                            tokio::spawn(async move {
                                                if let Ok(ex) = Extension::buildfromName(SourceType::Manga, &u_clone.ext) {
                                                    let m: Box<dyn MangaSource + Send + Sync> = ex.toMangaSource();
                                                    let hs = m.lib_url_parse(u_clone.source).await;
                                                    if let Ok(o) = hs {
                                                        let _ = tx_clone_input_clone.send(appmessages::MessageType::LoadChapters(o));
                                                    }
                                                }
                                            });
                                        }
                                    }
                                }
                                true
                            },
                            KeyCode::Esc | KeyCode::Backspace => {
                                if state.entry_selected {
                                    state.entry_selected = false;
                                    return true;
                                } else if state.label.get_current().is_some() {
                                    return state.label.clear();
                                }
                                false
                            }
                            _ => {false}
                        }
                    },
                    _ => {false}
                }
            })
        }
    }
    /***
     *  Input   |
     *  List    | ListEpisodes
     * -----------------------
     * Keybinds Each Char(' ') | Backspace | Delete | Ins -> Input
     * Enter if select -> List else to input to search
     * ---------------------------------------
     * On write on keyboard for input clear select.
     */
    pub fn searchManga(&mut self) -> CurrentWidget {
        let render_ls = Rc::clone(&self.manga.search.search_list_state);
        let input_ls = Rc::clone(&self.manga.search.search_list_state);
        let tx_clone_render = self.manga.search.tx.clone();
        let tx_clone_input = self.manga.search.tx.clone();
        CurrentWidget {
            render: Box::new(move |frame: &mut Frame, area: Rect, carea: Rect| {
                let b = Block::new()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::new().fg(ratatui::style::Color::Cyan));
                let mut state = render_ls.borrow_mut();
                let l = Search::Search::new(Search::SearchType::Manga,tx_clone_render.clone()); //TODO
                let cmd_list = Self::generate_cmd_list(vec![
                    (CmdListType::Multi, "Arrows","to move"),
                    (CmdListType::Single, "Enter","select"),
                    (CmdListType::Single, "Esc","Go Back"),
                    (CmdListType::Single, "P","Add to Favourite"),
                    (CmdListType::Single, "R","Reverse list of chpaters")
                ]);
                frame.render_widget( cmd_list, carea);
                frame.render_stateful_widget(l, area, &mut *state);
            }), handle_input:  Box::new(move |k: KeyCode| {
                let mut state = input_ls.borrow_mut();
                let tx_clone_input_clone = tx_clone_input.clone();
                match  state.status {
                    SearchStateStatus::Empty => {
                        match k {
                            ke @ (KeyCode::Char(_) | KeyCode::Left | KeyCode::Right | KeyCode::Backspace | KeyCode::Delete)=> {
                                state.text_input_state.dispatchEvent(ke)
                            },
                            KeyCode::Enter => {
                                state.text_input_state.editingmode = false;
                                let search_query = state.text_input_state.value.clone();
                                if let Some(task) = state.current_search.take() {
                                    task.abort();
                                }
                                if let Some(current) = state.current_source.clone() {
                                    let handle = tokio::spawn(async move {
                                        if let Ok(ex) = Extension::buildfromName(SourceType::Manga, &current) {
                                            let m: Box<dyn MangaSource + Send + Sync> = ex.toMangaSource();
                                            let hs = m.search(search_query, 1).await;
                                            if let Ok(o) = hs {
                                                // let titles: Vec<_> = o.iter().map(|item| &item.title).collect();
                                                // log!("searchManga found titles: {:?}", titles);
                                                let _ = tx_clone_input_clone.send(appmessages::MessageType::FoundMangaFromQuery(o));
                                            }
                                        }
                                    });
                                    state.current_search = Some(handle);
                                }
                                state.status = Search::SearchStateStatus::EntryList;
                                true
                            },
                            KeyCode::Tab => {
                                if let Some(c) = state.current_source.clone() {
                                    if let Some(index) = state.all_source.iter().position(|s| s == &c) {
                                        let next_index = (index + 1) % state.all_source.len();
                                        if let Some(next_source) = state.all_source.get(next_index) {
                                            state.current_source = Some(next_source.clone());
                                        }
                                    }
                                } else {
                                    if state.all_source.len() == 0 {
                                        state.current_source = None;
                                    } else {
                                        state.current_source = state.all_source.first().cloned();
                                    }
                                }
                                state.status = Search::SearchStateStatus::Empty;
                                true
                            }
                            _ => {false}
                        }
                    },
                    Search::SearchStateStatus::EntryList => {
                        match k {
                            KeyCode::Char(c) => {
                                match (c) {
                                    'p'=> {
                                        let mut keys: Vec<String> = state.lib.keys().cloned().collect();
                                        keys.sort();
                                        if let Some(u) = state.list_state_entries.selected() && let Some(current) = &state.current_source {
                                            if let Some(sel) = keys.get(u) {
                                                if let Some(entry) = state.lib.get(sel) {
                                                    addToFavourite(SourceType::Manga, entry, current);
                                                    return true;
                                                }
                                            }
                                        }
                                    },
                                    _ => {}
                                }
                                let u = state.text_input_state.dispatchEvent(KeyCode::Char(c));
                                if u {state.status = Search::SearchStateStatus::Empty;}
                                u
                            },
                            ke @ (KeyCode::Left | KeyCode::Right | KeyCode::Backspace | KeyCode::Delete)=> {
                                // if ()
                                let u = state.text_input_state.dispatchEvent(ke);
                                if u {state.status = Search::SearchStateStatus::Empty;}
                                u
                            },
                            KeyCode::Enter => {
                                if state.text_input_state.editingmode {
                                    state.text_input_state.editingmode = false;
                                    let search_query = state.text_input_state.value.clone();
                                    if let Some(task) = state.current_search.take() {
                                        task.abort();
                                    }
                                    if let Some(current) = state.current_source.clone() {
                                        let handle = tokio::spawn(async move {
                                            if let Ok(ex) = Extension::buildfromName(SourceType::Manga, &current) {
                                                let m: Box<dyn MangaSource + Send + Sync> = ex.toMangaSource();
                                                let hs = m.search(search_query, 1).await;
                                                if let Ok(o) = hs {
                                                    let _ = tx_clone_input_clone.send(appmessages::MessageType::FoundMangaFromQuery(o));
                                                }
                                            }
                                        });
                                        state.current_search = Some(handle);
                                    }
                                } else {
                                    if let Some(current) = state.current_source.clone() {
                                        let mut keys: Vec<String> = state.lib.keys().cloned().collect();
                                        keys.sort();
                                        if let Some(u) = state.list_state_entries.selected() {
                                            if let Some(sel) = keys.get(u) {
                                                if let Some(entry) = state.lib.get(sel) {
                                                    let url = entry.url.clone();
                                                    let handle = tokio::spawn(async move {
                                                        if let Ok(ex) = Extension::buildfromName(SourceType::Manga, &current) {
                                                            let m: Box<dyn MangaSource + Send + Sync> = ex.toMangaSource();
                                                            let hs = m.lib_url_parse(url).await;
                                                            if let Ok(o) = hs {
                                                                let _ = tx_clone_input_clone.send(appmessages::MessageType::ListChaptersFromQuery(o));
                                                            }
                                                        }
                                                    });
                                                }
                                            }
                                        }
                                    }
                                    state.status = Search::SearchStateStatus::EpisodeList;
                                }
                                true
                            },
                            KeyCode::Tab => {
                                if let Some(c) = state.current_source.clone() {
                                    if let Some(index) = state.all_source.iter().position(|s| s == &c) {
                                        let next_index = (index + 1) % state.all_source.len();
                                        if let Some(next_source) = state.all_source.get(next_index) {
                                            state.current_source = Some(next_source.clone());
                                        }
                                    }
                                } else {
                                    if state.all_source.len() == 0 {
                                        state.current_source = None;
                                    } else {
                                        state.current_source = state.all_source.first().cloned();
                                    }
                                }
                                state.status = Search::SearchStateStatus::Empty;
                                true
                            },
                            KeyCode::Up  => {
                                if state.list_state_entries.selected().is_none() {
                                    state.list_state_entries.select_last();
                                } else {
                                    state.list_state_entries.select_previous();
                                }
                                true
                            },
                            KeyCode::Down => {
                                if state.list_state_entries.selected().is_none() {
                                    state.list_state_entries.select_first();
                                } else {
                                    state.list_state_entries.select_next();
                                }
                                true
                            },
                            KeyCode::Backspace => {
                                state.status = Search::SearchStateStatus::Empty;
                                true
                            },
                            _ => {false}
                        }
                    },
                    Search::SearchStateStatus::EpisodeList => {
                        match k {
                            KeyCode::Char(c) => {
                                match (c) {
                                    'r'=> {
                                        state.flag_reverse = !state.flag_reverse;
                                        return true;
                                    },
                                    _ => {}
                                }
                                let u = state.text_input_state.dispatchEvent(KeyCode::Char(c));
                                if u {state.status = Search::SearchStateStatus::Empty;}
                                u
                            },
                            ke @ (KeyCode::Left | KeyCode::Right | KeyCode::Backspace | KeyCode::Delete)=> {
                                let u = state.text_input_state.dispatchEvent(ke);
                                if u {state.status = Search::SearchStateStatus::Empty;}
                                u
                            },
                            KeyCode::Up  => {
                                if state.list_state_eps.selected().is_none() {
                                    state.list_state_eps.select_last();
                                }else {
                                    state.list_state_eps.select_previous();
                                }
                                true
                            },
                            KeyCode::Down => {
                                if state.list_state_eps.selected().is_none() {
                                    state.list_state_eps.select_first();
                                }else {
                                    state.list_state_eps.select_next();
                                }
                                true
                            },
                            KeyCode::Enter => {
                                if let Some(current) = state.current_source.clone() {
                                    let mut keys: Vec<String> = state.lib.keys().cloned().collect();
                                    let mut ekeys: Vec<&String> = Library::Library::sorted_keys_from_hash(&state.episodes_loaded);//state.episodes_loaded.keys().cloned().collect();
                                    keys.sort();
                                    // ekeys.sort();
                                    if (state.flag_reverse) {
                                        ekeys.reverse();
                                    }
                                    if let Some(u) = state.list_state_eps.selected() && let Some(v) = state.list_state_entries.selected() {
                                        if let Some(sel) = ekeys.get(u) && let Some(lsel) = keys.get(v) {
                                            if let Some(entry) = state.episodes_loaded.get(*sel) && let Some(manga) = state.lib.get(lsel) {
                                                let url = entry.url.clone();
                                                let eli_c = (*entry).clone();
                                                let mangatitle = manga.title.clone();
                                                let handle = tokio::spawn(async move {
                                                    if let Ok(ex) = Extension::buildfromName(SourceType::Manga, &current) {
                                                        let m: Box<dyn MangaSource + Send + Sync> = ex.toMangaSource();
                                                        let chp =  eli_c.title.split_whitespace().last().map(|x| x.to_owned());
                                                        openManga(Arc::new(m), MangaMetadata { title: Some(mangatitle), url: eli_c.url, chapter: chp, volume: None});
                                                    }
                                                });
                                            }
                                        }
                                    }
                                }
                                false
                            },
                            KeyCode::Esc | KeyCode::Backspace => {
                                if state.entry_selected {
                                    state.entry_selected = false;
                                    return true;
                                }
                                false
                            },
                            KeyCode::Tab => {
                                if let Some(c) = state.current_source.clone() {
                                    if let Some(index) = state.all_source.iter().position(|s| s == &c) {
                                        let next_index = (index + 1) % state.all_source.len();
                                        if let Some(next_source) = state.all_source.get(next_index) {
                                            state.current_source = Some(next_source.clone());
                                        }
                                    }
                                } else {
                                    if state.all_source.len() == 0 {
                                        state.current_source = None;
                                    } else {
                                        state.current_source = state.all_source.first().cloned();
                                    }
                                }
                                state.status = Search::SearchStateStatus::Empty;
                                true
                            },
                            KeyCode::Backspace => {
                                state.status = Search::SearchStateStatus::EntryList;
                                true
                            }
                            _ => {false}
                        }
                    },
                    _ => {false}
                }
            })
        }
    }

    pub fn searchAnime(&mut self) -> CurrentWidget {
        let render_ls = Rc::clone(&self.anime.search.search_list_state);
        let input_ls = Rc::clone(&self.anime.search.search_list_state);
        let tx_clone_render = self.anime.search.tx.clone();
        let tx_clone_input = self.anime.search.tx.clone();
        CurrentWidget {
            render: Box::new(move |frame: &mut Frame, area: Rect, carea: Rect| {
                let b = Block::new()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::new().fg(ratatui::style::Color::Cyan));
                // let lay = Layout::default()
                // .direction(Direction::Vertical)
                // .constraints(vec![
                //     Constraint::Percentage(100),
                //     Constraint::Min(3)
                // ]).split(area);
                let mut state = render_ls.borrow_mut();
                let l = Search::Search::new(Search::SearchType::Anime,tx_clone_render.clone()); //TODO
                let cmd_list = Self::generate_cmd_list(vec![
                    (CmdListType::Multi, "Arrows","to move"),
                    (CmdListType::Single, "Enter","select"),
                    (CmdListType::Single, "Esc","Go Back"),
                    (CmdListType::Single, "P","Add to Favourite"),
                    (CmdListType::Single, "R","Reverse list of episodes"),
                ]);
                frame.render_widget( cmd_list, carea);
                frame.render_stateful_widget(l, area, &mut *state);
            }), handle_input:  Box::new(move |k: KeyCode| {
                let mut state = input_ls.borrow_mut();
                let tx_clone_input_clone = tx_clone_input.clone();
                match  state.status {
                    SearchStateStatus::Empty => {
                        match k {
                            ke @ (KeyCode::Char(_) | KeyCode::Left | KeyCode::Right | KeyCode::Backspace | KeyCode::Delete)=> {
                                state.text_input_state.dispatchEvent(ke)
                            },
                            KeyCode::Enter => {
                                state.text_input_state.editingmode = false;
                                let search_query = state.text_input_state.value.clone();
                                if let Some(task) = state.current_search.take() {
                                    task.abort();
                                }
                                if let Some(current) = state.current_source.clone() {
                                    let handle = tokio::spawn(async move {
                                        if let Ok(ex) = Extension::buildfromName(SourceType::Anime, &current) {
                                            let m: Box<dyn AnimeSource + Send + Sync> = ex.toAnimeSource();
                                            let hs = m.search(search_query, 1).await;
                                            if let Ok(o) = hs {
                                                let _ = tx_clone_input_clone.send(appmessages::MessageType::FoundAnimeFromQuery(o));
                                            }
                                        }
                                    });
                                    state.current_search = Some(handle);
                                }
                                state.status = Search::SearchStateStatus::EntryList;
                                true
                            },
                            KeyCode::Tab => {
                                if let Some(c) = state.current_source.clone() {
                                    if let Some(index) = state.all_source.iter().position(|s| s == &c) {
                                        let next_index = (index + 1) % state.all_source.len();
                                        if let Some(next_source) = state.all_source.get(next_index) {
                                            state.current_source = Some(next_source.clone());
                                        }
                                    }
                                } else {
                                    if state.all_source.len() == 0 {
                                        state.current_source = None;
                                    } else {
                                        state.current_source = state.all_source.first().cloned();
                                    }
                                }
                                state.status = Search::SearchStateStatus::Empty;
                                true
                            }
                            _ => {false}
                        }
                    },
                    Search::SearchStateStatus::EntryList => {
                        match k {
                            KeyCode::Char(c) => {
                                match (c) {
                                    'p'=> {
                                        let mut keys: Vec<String> = state.lib.keys().cloned().collect();
                                        keys.sort();
                                        if let Some(u) = state.list_state_entries.selected() && let Some(current) = &state.current_source {
                                            if let Some(sel) = keys.get(u) {
                                                if let Some(entry) = state.lib.get(sel) {
                                                    addToFavourite(SourceType::Anime, entry, current);
                                                    return true;
                                                }
                                            }
                                        }
                                    },
                                    _ => {}
                                }
                                let u = state.text_input_state.dispatchEvent(KeyCode::Char(c));
                                if u {state.status = Search::SearchStateStatus::Empty;}
                                u
                            },
                            ke @ (KeyCode::Left | KeyCode::Right | KeyCode::Backspace | KeyCode::Delete)=> {
                                // if ()
                                let u = state.text_input_state.dispatchEvent(ke);
                                if u {state.status = Search::SearchStateStatus::Empty;}
                                u
                            },
                            KeyCode::Enter => {
                                if state.text_input_state.editingmode {
                                    state.text_input_state.editingmode = false;
                                    let search_query = state.text_input_state.value.clone();
                                    if let Some(task) = state.current_search.take() {
                                        task.abort();
                                    }
                                    if let Some(current) = state.current_source.clone() {
                                        let handle = tokio::spawn(async move {
                                            if let Ok(ex) = Extension::buildfromName(SourceType::Anime, &current) {
                                                let m: Box<dyn AnimeSource + Send + Sync> = ex.toAnimeSource();
                                                let hs = m.search(search_query, 1).await;
                                                if let Ok(o) = hs {
                                                    let _ = tx_clone_input_clone.send(appmessages::MessageType::FoundAnimeFromQuery(o));
                                                }
                                            }
                                        });
                                        state.current_search = Some(handle);
                                    }
                                } else {
                                    if let Some(current) = state.current_source.clone() {
                                        let mut keys: Vec<String> = state.lib.keys().cloned().collect();
                                        keys.sort();
                                        if let Some(u) = state.list_state_entries.selected() {
                                            if let Some(sel) = keys.get(u) {
                                                if let Some(entry) = state.lib.get(sel) {
                                                    let url = entry.url.clone();
                                                    let handle = tokio::spawn(async move {
                                                        if let Ok(ex) = Extension::buildfromName(SourceType::Anime, &current) {
                                                            let m: Box<dyn AnimeSource + Send + Sync> = ex.toAnimeSource();
                                                            let hs = m.lib_url_parse(url).await;
                                                            if let Ok(o) = hs {
                                                                let _ = tx_clone_input_clone.send(appmessages::MessageType::ListEpisodesFromQuery(o));
                                                            }
                                                        }
                                                    });
                                                }
                                            }
                                        }
                                    }
                                    state.status = Search::SearchStateStatus::EpisodeList;
                                }
                                true
                            },
                            KeyCode::Tab => {
                                if let Some(c) = state.current_source.clone() {
                                    if let Some(index) = state.all_source.iter().position(|s| s == &c) {
                                        let next_index = (index + 1) % state.all_source.len();
                                        if let Some(next_source) = state.all_source.get(next_index) {
                                            state.current_source = Some(next_source.clone());
                                        }
                                    }
                                } else {
                                    if state.all_source.len() == 0 {
                                        state.current_source = None;
                                    } else {
                                        state.current_source = state.all_source.first().cloned();
                                    }
                                }
                                state.status = Search::SearchStateStatus::Empty;
                                true
                            },
                            KeyCode::Up  => {
                                if state.list_state_entries.selected().is_none() {
                                    state.list_state_entries.select_last();
                                } else {
                                    state.list_state_entries.select_previous();
                                }
                                true
                            },
                            KeyCode::Down => {
                                if state.list_state_entries.selected().is_none() {
                                    state.list_state_entries.select_first();
                                } else {
                                    state.list_state_entries.select_next();
                                }
                                true
                            },
                            KeyCode::Backspace => {
                                state.status = Search::SearchStateStatus::Empty;
                                true
                            },
                            _ => {false}
                        }
                    },
                    Search::SearchStateStatus::EpisodeList => {
                        match k {
                            KeyCode::Char(c) => {
                                match (c) {
                                    'r'=> {
                                        state.flag_reverse = !state.flag_reverse;
                                        return true;
                                    },
                                    _ => {}
                                }
                                let u = state.text_input_state.dispatchEvent(KeyCode::Char(c));
                                if u {state.status = Search::SearchStateStatus::Empty;}
                                u
                            },
                            ke @ (KeyCode::Left | KeyCode::Right | KeyCode::Backspace | KeyCode::Delete)=> {
                                let u = state.text_input_state.dispatchEvent(ke);
                                if u {state.status = Search::SearchStateStatus::Empty;}
                                u
                            },
                            KeyCode::Up  => {
                                if state.list_state_eps.selected().is_none() {
                                    state.list_state_eps.select_last();
                                }else {
                                    state.list_state_eps.select_previous();
                                }
                                true
                            },
                            KeyCode::Down => {
                                if state.list_state_eps.selected().is_none() {
                                    state.list_state_eps.select_first();
                                }else {
                                    state.list_state_eps.select_next();
                                }
                                true
                            },
                            KeyCode::Enter => {
                                if let Some(current) = state.current_source.clone() {
                                    let mut keys: Vec<String> = state.lib.keys().cloned().collect();
                                    let mut ekeys: Vec<&String> = Library::Library::sorted_keys_from_hash(&state.episodes_loaded);//state.episodes_loaded.keys().cloned().collect();
                                    // let mut ekeys: Vec<String> = state.episodes_loaded.keys().cloned().collect();
                                    // ekeys.sort();
                                    keys.sort();
                                    if (state.flag_reverse) {
                                        ekeys.reverse();
                                    }
                                    if let Some(u) = state.list_state_eps.selected() && let Some(v) = state.list_state_entries.selected() {
                                        if let Some(sel) = ekeys.get(u) && let Some(lsel) = keys.get(v) {
                                            if let Some(entry) = state.episodes_loaded.get(*sel) && let Some(anime) = state.lib.get(lsel) {
                                                let url = entry.url.clone();
                                                let eli_c = (*entry).clone();
                                                let animetitle = anime.title.clone();
                                                let handle = tokio::spawn(async move {
                                                    if let Ok(ex) = Extension::buildfromName(SourceType::Anime, &current) {
                                                        let a: Box<dyn AnimeSource + Send + Sync> = ex.toAnimeSource();
                                                        //let chp =  eli_c.title.split_whitespace().last().map(|x| x.to_owned());
                                                        // openManga(Arc::new(a), MangaMetadata { title: Some(Animetitle), url: eli_c.url, chapter: chp, volume: None});
                                                        a.lib_ep_select(eli_c).await;
                                                    }
                                                });
                                            }
                                        }
                                    }
                                }
                                false
                            },
                            KeyCode::Esc | KeyCode::Backspace => {
                                if state.entry_selected {
                                    state.entry_selected = false;
                                    return true;
                                }
                                false
                            },
                            KeyCode::Tab => {
                                if let Some(c) = state.current_source.clone() {
                                    if let Some(index) = state.all_source.iter().position(|s| s == &c) {
                                        let next_index = (index + 1) % state.all_source.len();
                                        if let Some(next_source) = state.all_source.get(next_index) {
                                            state.current_source = Some(next_source.clone());
                                        }
                                    }
                                } else {
                                    if state.all_source.len() == 0 {
                                        state.current_source = None;
                                    } else {
                                        state.current_source = state.all_source.first().cloned();
                                    }
                                }
                                state.status = Search::SearchStateStatus::Empty;
                                true
                            },
                            KeyCode::Backspace => {
                                state.status = Search::SearchStateStatus::EntryList;
                                true
                            }
                            _ => {false}
                        }
                    },
                    _ => {false}
                }
            })
        }
    }

}

