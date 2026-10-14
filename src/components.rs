use std::clone;

use boa_engine::builtins::boolean;
use ratatui::{crossterm::event::KeyCode, layout::{Constraint, Layout}, text::Text, widgets::{Block, Borders, List, ListItem, ListState, StatefulWidget, Widget}};

pub mod list {
    use std::{collections::HashMap};
    use ratatui::{Frame, crossterm::event::KeyCode, layout::Rect, style::{Style, Stylize}, widgets::{Block, BorderType, Borders, List, ListState}};

use crate::config::getConfig;
    
    pub struct CurrentWidget {
        pub render: Box<dyn Fn(&mut Frame, Rect,Rect)>,
        pub handle_input: Box<dyn Fn(KeyCode) -> bool>
    }
    type SLWidgetNotOption = CurrentWidget;
    type SLWidget = Option<SLWidgetNotOption>;
    type Callback = fn();


    #[derive(Default)]
    pub struct SelectableList {
        children_list: Vec<String>,
        children_hash: HashMap<String, ListDiveChild>,
        path: Vec<String>,
        pub lastpath:String,
        pub state: ListState,
        pub widget: SLWidget
    }
    pub struct ListDiveChild {
        pub title: String,
        pub index: usize,
        pub action_type: ActionType,
        pub children_list: Vec<String>,
        pub children_hash: Option<HashMap<String, ListDiveChild>>,
        pub widget: SLWidget
    }
    impl ListDiveChild {
        pub fn new(title: String, index:usize, action_type:ActionType, children: Option<HashMap<String, ListDiveChild>>, widget:SLWidget) -> Self {
            let childrenlist: Vec<String> = match children.as_ref() {
                Some(chs) => {
                    let mut x:Vec<&ListDiveChild> = chs.values().collect();
                    x.sort_by_key(|v| v.index);
                    x.into_iter().map(|v| v.title.to_owned()).collect()
                },
                _ => vec![]
            };
            Self { 
                title,
                index, 
                action_type, 
                children_list: childrenlist, 
                children_hash: children, 
                widget
            }
        }
    }

    #[derive(Clone)]
    pub enum ActionType {
        None,
        SubMenu,
        Render,
    }

    impl SelectableList {
        pub fn new(
            children: HashMap<String, ListDiveChild>,widget:SLWidget
        ) -> Self {
            let mut childrenlist = vec!["".to_string();children.keys().len()];
            for (_,v) in &children {
                childrenlist[v.index] = v.title.to_owned();
            }
            Self {
                children_hash: children,
                children_list: childrenlist,
                path: vec![],
                lastpath: "".to_owned(), 
                state: ListState::default(),
                widget,
            }
        }
        
        
        fn get_current_item_list(&self) -> &Vec<String> {
            if self.path.len() > 0 {
                if let Some(first_path) = self.path.first() {
                    let mut ocurr = self.children_hash.get(first_path);
                    for i in 1..self.path.len() {
                        if let Some(curr) = ocurr {
                            if let Some(h) = &curr.children_hash {
                                match self.path.get(i) {
                                    Some(p) => ocurr = h.get(p),
                                    _ => break
                                }
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                    if let Some(curr) = ocurr {
                        return &curr.children_list;
                    }
                }
            }
            return &self.children_list
        }
        fn get_current_widget(&self) -> Option<&SLWidgetNotOption> {
            let mut w = self.widget.as_ref();
            if self.path.len() > 0 {
                if let Some(first_path) = self.path.first() {
                    let mut ocurr = self.children_hash.get(first_path);
                    for i in 1..self.path.len() {
                        ocurr = ocurr.and_then(|curr| curr.children_hash.as_ref())
                        .and_then(|h| self.path.get(i).and_then(|p| h.get(p)));
                        if let Some(cw) = ocurr.and_then(|c| c.widget.as_ref()) {
                            w = Some(cw)
                        }
                    }
                    if !self.lastpath.is_empty() {
                        ocurr = ocurr.and_then(|curr| curr.children_hash.as_ref())
                        .and_then(|h| h.get(&self.lastpath));
                        if let Some(cw) = ocurr.and_then(|c| c.widget.as_ref()) {
                            w = Some(cw)
                        }
                    }
                }
            }
            return w;
        }
        fn get_current_children_hash(&self) -> Option<&HashMap<String, ListDiveChild>>{
            if self.path.len() > 0 {
                if let Some(first_path) = self.path.first() {
                    let mut ocurr = self.children_hash.get(first_path);
                    for i in 1..self.path.len() {
                        if let Some(curr) = ocurr {
                            if let Some(h) = &curr.children_hash {
                                match self.path.get(i) {
                                    Some(p) => ocurr = h.get(p),
                                    _ => break
                                }
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                    if let Some(curr) = ocurr {
                        return curr.children_hash.as_ref();
                    }
                }
            }
            return Some(&self.children_hash);
        }
        
            
        pub fn render_list(&mut self, frame: &mut Frame, area: Rect) {
            let l = List::new(self.get_current_item_list().clone())
            .highlight_style(Style::new()
                .fg(getConfig().Theme.menu_selection.to_ratatui()) // colore del testo del menu alla selezione 
            )
            .fg(getConfig().Theme.menu_text.to_ratatui()) //colore del testo del menu non selezionato
            .highlight_symbol("> ")
            .highlight_spacing(ratatui::widgets::HighlightSpacing::Always)
            .block(
                Block::new()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::new().fg(getConfig().Theme.base_border.to_ratatui()))
            );

            frame.render_stateful_widget(l,area, &mut self.state);
        }
        pub fn render_menu(&mut self, frame: &mut Frame, area: Rect, cmdArea: Rect) {
            if let Some(f) = self.get_current_widget() {
                (f.render)(frame, area, cmdArea)
            }
        }
        pub fn enter(&mut self) {
            if let Some(s) = self.state.selected()
                .and_then(|i| self.get_current_item_list().get(i)) {
                if let Some(ldc) = self.get_current_children_hash().and_then(|h| h.get(s)) {
                    match ldc.action_type {
                        ActionType::SubMenu => {
                            self.path.push(s.to_owned());
                            self.lastpath = "".to_owned();
                        },
                        ActionType::Render => {
                            self.lastpath = s.to_owned();
                        },
                        ActionType::None => {}
                    }
                }
            }
        }
        pub fn goback(&mut self)->bool {
            if self.path.len()>0 {
                if !self.lastpath.is_empty() {
                    self.lastpath="".to_owned();
                } else {
                    self.path.pop();
                }
                return true;
            }
            return false;
        }
        pub fn handle_input(&mut self, kc: ratatui::crossterm::event::KeyCode) -> bool {
            if let Some(gcw) = self.get_current_widget() {
                let f = (gcw.handle_input)(kc);
                return f;
            }
            false
        }
    }
    

}


pub mod AsyncUpdateAnimeList {
    use std::{collections::HashMap, format, sync::Arc};

    use ratatui::{buffer::Buffer, layout::Rect, style::{Style}, widgets::{Block, Borders, List, ListItem, ListState, Paragraph, StatefulWidget, Widget}};

    use crate::{appmessages::MessageType, config::getConfig, source::AnimeSource, utils::utils::{first_capital, hash_sort}};

    pub enum AsyncAnimeStateStatus {
        Empty,
        Loading,
        Success(HashMap<String,String>),
        Error(String)
    }
    pub struct AsyncAnimeState {
        pub list_state: ListState, 
        pub status: AsyncAnimeStateStatus,
        pub page: usize,
        pub res: Option<HashMap<String, String>>,
        pub animesource: Arc<Box<dyn AnimeSource + Send + Sync>>
    }

    impl AsyncAnimeState {
        pub fn new(animesource: Arc<Box<dyn AnimeSource + Send + Sync>>) -> Self {
            let mut list_state = ListState::default();
            list_state.select_first();

            Self {
                list_state,
                status: AsyncAnimeStateStatus::Empty,
                page: 1,
                res: None,
                animesource
            }
        }
        pub fn resetFrom(&mut self, newanimesource: AsyncAnimeState) {
            self.animesource = newanimesource.animesource;
            self.list_state.select_first();
            self.page = 1;
            self.res = None;
            self.status = AsyncAnimeStateStatus::Empty;
        }
    }
    pub struct AsyncUpdateAnimeList {
        trasmitter_to_app: tokio::sync::mpsc::UnboundedSender<MessageType>,
    }
    impl AsyncUpdateAnimeList {
        pub fn new(trasmitter_to_app: tokio::sync::mpsc::UnboundedSender<MessageType>) -> Self {
            Self {
                trasmitter_to_app,
            }
        } 
        pub fn formatEpisodeIndexTitle(index: usize, title:&str) -> String {
            return format!("Ep. {:>4} di {}",index,title);
        }
    }
    impl StatefulWidget for AsyncUpdateAnimeList {
        type State = AsyncAnimeState;

        fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
            let block = Block::default().title(format!(" {} - Page {} ", first_capital(&(state.animesource.get_hostname())), state.page)).borders(Borders::ALL);
            match &mut state.status {
                AsyncAnimeStateStatus::Empty => {
                    state.status = AsyncAnimeStateStatus::Loading;
                    let tx = self.trasmitter_to_app.clone();
                    let source = state.animesource.clone();
                    let h = source.get_hostname();
                    let p = state.page.clone();
                    tokio::spawn(async move {
                        let res = source.listupdated(p).await;
                        match res {
                            Ok(o) => {
                                let mut h = HashMap::new();
                                for a in o {
                                    let s = Self::formatEpisodeIndexTitle(a.index,&a.title);
                                    h.insert(s.clone(), a.url.clone());
                                }
                                tx.send(MessageType::AnimeState(AsyncAnimeStateStatus::Success(h)))
                            },
                            Err(_e) => tx.send(MessageType::AnimeState(AsyncAnimeStateStatus::Error("Failed to load".to_owned())))
                        }
                    });
                    Paragraph::new("Starting the request").block(block).render(area, buf);
                }
                AsyncAnimeStateStatus::Loading => {
                    Paragraph::new("Loading").block(block.border_style(Style::new().fg(getConfig().Theme.status_loading.to_ratatui()))).render(area, buf);
                
                }
                AsyncAnimeStateStatus::Error(err) => {
                    Paragraph::new(format!("Error: {}", err)).block(block).render(area, buf);
                }
                AsyncAnimeStateStatus::Success(res) => {
                    state.res = Some(res.clone());
                    let items = hash_sort(&res);
                    let list_items: Vec<ListItem> = items.iter()
                    .map(|a| ListItem::new(format!("{}", a)))
                    .collect();
                    StatefulWidget::render(
                        List::new(list_items).block(block.border_style(Style::new().fg(ratatui::style::Color::Green))).highlight_symbol(">> "),
                        area,
                        buf,
                        &mut state.list_state
                    );
                }
            }
        }
    }
}

pub mod Library {
    use std::{cmp::Ordering, collections::HashMap, hash::Hash, sync::Arc};

    use ratatui::{buffer::Buffer, layout::{Alignment, Constraint, Layout, Rect}, style::Style, widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, StatefulWidget, Widget}};

    use crate::{appmessages::MessageType, library::{self, LibraryItem}, source::{EpisodeListItem, MangaSource}, utils::utils::hash_sort};
    pub struct Library {
        pub trasmitter_to_app:tokio::sync::mpsc::UnboundedSender<MessageType>,
        pub library_type: LibraryType
    }
    impl Library {
        pub fn new(library_type: LibraryType,trasmitter_to_app: tokio::sync::mpsc::UnboundedSender<MessageType>) -> Self {
            Self {
                library_type,
                trasmitter_to_app,
            }
        } 
        pub fn sorted_keys_from_hash(hs: &HashMap<String, EpisodeListItem>) -> Vec<&String> {
            let mut items: Vec<&String> = hs.keys().collect::<Vec<&String>>();
            items.sort_by(|ad: &&String,bd: &&String| {
                let mut a_s = ad.split(".");
                let a = a_s.nth(0).unwrap_or(ad);
                let zero_string = "0".to_string();
                let adec = a_s.nth(1).unwrap_or(&zero_string);
                let mut b_s = bd.split(".");
                let b = b_s.nth(0).unwrap_or(bd);
                let bdec = a_s.nth(1).unwrap_or(&zero_string);

                match (a.parse::<usize>(), b.parse::<usize>()) {
                    (Ok(uai), Ok(ubi)) => match ubi.cmp(&uai) {
                        Ordering::Equal => match (adec.parse::<usize>(), bdec.parse::<usize>()) {
                            (Ok(adecp), Ok(bdecp)) => adecp.cmp(&bdecp),
                            _ => Ordering::Less
                        },
                        a => a
                    },
                    (Ok(uai), Err(_))=>Ordering::Greater,
                    (Err(_), Ok(ubi))=>Ordering::Less,
                    (Err(_), Err(_))=>b.to_lowercase().cmp(&a.to_lowercase()),
                }
            }); // ? REVERSE SORT - MAX TO MIN -> NEWEST FIRST
            items
        }
    }
    pub enum LibraryStateStatus {
        Empty,
        Lib(usize),
    }

    #[derive(Default, Clone)]
    pub struct LibraryHashMap(HashMap<String, Vec<LibraryItem>>);
    impl LibraryHashMap  {
        pub fn new(a: HashMap<String, Vec<LibraryItem>>) -> Self {
            LibraryHashMap(a)
        }
        pub fn list_labels(&self) -> Vec<String> {
            let mut a:Vec<String> = self.0.keys().cloned().collect();
            a.sort();
            a        
        }
        pub fn list_values(&self, a: Option<&String>) -> Vec<LibraryItem> {
            if let Some(b) = a {
                if let Some(res) = self.0.get(b) {
                    return res.clone();
                }
            }
            self.0.values().flatten().cloned().collect()
        }
        pub fn inner(self) -> HashMap<String, Vec<LibraryItem>> {
            self.0
        }
        pub fn keys(&self) -> Vec<&String> {
            let mut a:Vec<&String> = self.0.keys().collect();
            a.sort();
            a
        }
        pub fn keys_len(&self) -> usize {
            self.0.keys().len()
        }
        pub fn get(&self, sel:usize, filter: Option<&String>) -> Option<LibraryItem> {
            match self.list_values(filter).get(sel) {
                Some(a) => Some(a.clone()),
                _ => None
            }
        }
    }

    pub struct LibraryLabel {
        labels: Vec<String>,
        curr: Option<usize>
    }
    impl LibraryLabel {
        pub fn next(&mut self) -> bool {
            if self.labels.is_empty() {
                self.curr = None;
                return false;
            }

            let len = self.labels.len();
            self.curr = Some(match self.curr {
                Some(c) => (c + 1) % len,
                None => 0,
            });
            true
        }

        pub fn prev(&mut self) -> bool {
            if self.labels.is_empty() {
                self.curr = None;
                return false;
            }

            let len = self.labels.len();
            self.curr = Some(match self.curr {
                Some(0) | None => len - 1,
                Some(c) => c - 1,
            });
            true
        }
        pub fn get_current(&self) -> Option<&String> {
            if self.curr.is_none() {
                return None
            } else {
                return self.labels.get(self.curr?)
            }
        }
        pub fn clear(&mut self) -> bool {
            self.curr = None;
            true
        }
    }

    pub struct LibraryState {
        pub label: LibraryLabel,
        pub list_state_source: ListState, 
        pub list_state_eps: ListState, 
        pub lib: LibraryHashMap,
        pub status: LibraryStateStatus,
        pub entry_selected: bool,
        pub flag_reverse: bool,
        pub episodes_loaded: Option<HashMap<String, EpisodeListItem>>,
        pub current_source: Option<String>
    }
    impl LibraryState {
        pub fn new() -> Self {
            let mut list_state_source = ListState::default();
            let mut list_state_eps = ListState::default();
            list_state_source.select_first();
            list_state_eps.select_first();

            Self {
                label: LibraryLabel { labels: vec![], curr: None },
                lib: LibraryHashMap(HashMap::new()),
                list_state_source,
                list_state_eps,
                entry_selected: false,
                status: LibraryStateStatus::Empty,
                flag_reverse: false,
                episodes_loaded: None,
                current_source: None
            }
        }
    }
    #[derive(Clone,Copy)]
    pub enum LibraryType {
        Anime, Manga
    }
    // TODO
    impl StatefulWidget for Library {
        type State = LibraryState;

        fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
            let block = Block::default().title("Library").borders(Borders::ALL);
            let hor = Layout::default().direction(ratatui::layout::Direction::Horizontal).constraints([
                Constraint::Percentage(100),
                Constraint::Min(30)
            ]).split(area);
            let lay = Layout::default().direction(ratatui::layout::Direction::Vertical).constraints(([
                Constraint::Percentage(100),
                Constraint::Length(2)
            ])).margin(0).spacing(0).split(hor[0]);

            match &mut state.status {
                LibraryStateStatus::Empty => {
                    let f = match self.library_type {
                        LibraryType::Anime => "animelibrary.toml",
                        LibraryType::Manga => "mangalibrary.toml" 
                    };
                    if let Ok(lib) = library::LibraryItem::buildfromName(f) {
                        state.lib = LibraryHashMap(lib);
                        state.label.labels = state.lib.list_labels();
                        state.status = LibraryStateStatus::Lib(0);
                        state.entry_selected = false;
                        return Paragraph::new("Lib loading").block(block).render(area, buf);
                    } else {
                        return Paragraph::new("No lib found. Edit ./library/animelibrary.toml or ./library/mangalibrary.toml. Format:\n[[Animeworld]]\nname=\"Frieren\"\nepisodes=\"/path/to/a_generic_episode\"").block(block).render(area, buf);
                    }
                }
                LibraryStateStatus::Lib(index) => {
                    let mut keys: Vec<&String> = state.lib.keys();
                    let i = index.clone();
                    let l = keys.len();
                    let block_sc = Block::new()
                    .borders(Borders::all().difference(Borders::BOTTOM))
                    .border_type(BorderType::Rounded)
                    .border_style(Style::new().fg(ratatui::style::Color::Cyan));
                    let block_eps = Block::new()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::new().fg(ratatui::style::Color::Cyan));
                    if let Some(k) = keys.get(i % l) {
                        let key = *k;
                            state.current_source = Some(key.clone());
                            let list = List::new(state.lib.list_values(state.label.get_current()).iter().map(|li| ListItem::new(li.name.clone())).collect::<Vec<ListItem>>()).block(block_sc).highlight_symbol(">> ");
                            StatefulWidget::render(list, lay[0], buf, &mut state.list_state_source);

                    } else {
                        Widget::render(block_sc, lay[0], buf);
                    }
                    let labelTitle = match state.label.get_current() {
                        Some(x) => x.clone(),
                        None => "All".to_owned()
                    };
                    let label = Paragraph::new(labelTitle).alignment(Alignment::Center).block(
                        Block::new()
                        .borders(Borders::all().difference(Borders::TOP))
                        .border_type(BorderType::Rounded)
                        .border_style(Style::new().fg(ratatui::style::Color::Cyan))
                    );
                    
                    
                    Widget::render(label, lay[1], buf);
                    
                    if state.entry_selected && let Some(eps_hash) = &state.episodes_loaded {
                        let mut items = Self::sorted_keys_from_hash(eps_hash);
                        if (state.flag_reverse) {
                            items.reverse();
                        }
                        let mut eps = Vec::with_capacity(items.len());
                        for it in items {
                            let s = eps_hash.get(it).and_then(|f| Some(f.title.clone())).unwrap_or(it.clone());
                            eps.push(ListItem::new(s));
                        }
                        let list_ep = List::new(eps).block(block_eps).highlight_symbol(">> ");;
                        StatefulWidget::render(list_ep, hor[1], buf, &mut state.list_state_eps);
                    } else {
                        Widget::render(block_eps, hor[1], buf);
                    }
                }
            }
        }
    }
}

pub mod AsyncUpdateMangaList {
    use std::{collections::HashMap, sync::Arc};

    use ratatui::{buffer::Buffer, layout::Rect, style::{Style}, widgets::{Block, Borders, List, ListItem, ListState, Paragraph, StatefulWidget, Widget}};

    use crate::{appmessages::MessageType, config::getConfig, source::MangaSource, utils::utils::{first_capital, hash_sort}};

    pub enum AsyncMangaStateStatus {
        Empty,
        Loading,
        Success(HashMap<String,String>),
        Error(String)
    }
    pub struct AsyncMangaState {
        pub list_state: ListState, 
        pub status: AsyncMangaStateStatus,
        pub page: usize,
        pub res: Option<HashMap<String, String>>,
        pub mangasource: Arc<Box<dyn MangaSource + Send + Sync>>
    }

    impl AsyncMangaState {
        pub fn new(mangasource: Arc<Box<dyn MangaSource + Send + Sync>>) -> Self {
            let mut list_state = ListState::default();
            list_state.select_first();

            Self {
                list_state,
                status: AsyncMangaStateStatus::Empty,
                page: 1,
                res: None,
                mangasource
            }
        }
        pub fn resetFrom(&mut self, newmangasource: AsyncMangaState) {
            self.mangasource = newmangasource.mangasource;
            self.list_state.select_first();
            self.page = 1;
            self.res = None;
            self.status = AsyncMangaStateStatus::Empty;
        }
    }
    pub struct AsyncUpdateMangaList {
        trasmitter_to_app: tokio::sync::mpsc::UnboundedSender<MessageType>,
    }
    impl AsyncUpdateMangaList {
        pub fn new(trasmitter_to_app: tokio::sync::mpsc::UnboundedSender<MessageType>) -> Self {
            Self {
                trasmitter_to_app,
            }
        } 
        pub fn formatEpisodeIndexTitle(index: usize, title:&str) -> String {
            return format!("Capitolo {:>4} di {}",index,title);
        }
    }
    impl StatefulWidget for AsyncUpdateMangaList {
        type State = AsyncMangaState;

        fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
            let block = Block::default().title(format!(" {} - Page {} ", first_capital(&(state.mangasource.get_hostname())), state.page)).borders(Borders::ALL);
            match &mut state.status {
                AsyncMangaStateStatus::Empty => {
                    state.status = AsyncMangaStateStatus::Loading;
                    let tx = self.trasmitter_to_app.clone();
                    let source = state.mangasource.clone();
                    let h = source.get_hostname();
                    let p = state.page.clone();
                    tokio::spawn(async move {
                        let res = source.listupdated(p).await;
                        match res {
                            Ok(o) => {
                                let mut h = HashMap::new();
                                for a in o {
                                    let s = Self::formatEpisodeIndexTitle(a.index,&a.title);
                                    h.insert(s.clone(), a.url.clone());
                                }
                                tx.send(MessageType::MangaState(AsyncMangaStateStatus::Success(h)))
                            },
                            Err(_e) => tx.send(MessageType::MangaState(AsyncMangaStateStatus::Error("Failed to load".to_owned())))
                        }
                    });
                    Paragraph::new("Starting the request").block(block).render(area, buf);
                }
                AsyncMangaStateStatus::Loading => {
                    Paragraph::new("Loading").block(block.border_style(Style::new().fg(getConfig().Theme.status_loading.to_ratatui()))).render(area, buf);
                
                }
                AsyncMangaStateStatus::Error(err) => {
                    Paragraph::new(format!("Error: {}", err)).block(block).render(area, buf);
                }
                AsyncMangaStateStatus::Success(res) => {
                    state.res = Some(res.clone());
                    let items = hash_sort(&res);
                    let list_items: Vec<ListItem> = items.iter()
                    .map(|a| ListItem::new(format!("{}", a)))
                    .collect();
                    StatefulWidget::render(
                        List::new(list_items).block(block.border_style(Style::new().fg(ratatui::style::Color::Green))).highlight_symbol(">> "),
                        area,
                        buf,
                        &mut state.list_state
                    );
                }
            }
        }
    }
}

pub mod ListOption {
    use std::{cell::RefCell, collections::HashMap, ops::Index, rc::Rc};

    use boa_engine::ast::expression::Optional;
use ratatui::{buffer::Buffer, crossterm::event::KeyCode, layout::{Constraint, Layout, Rect, Rows}, style::{Style}, symbols::line, text::{Line, Span, Text}, widgets::{Block, BorderType, List, StatefulWidget, Table, Widget}};

    use crate::{components::TextInputState, config::getConfig, utils::utils::ColorPickerOption};
    pub struct ListOption {
        pub sections: Vec<Section>    
    }
    #[derive(Clone)]
    pub struct Section {
        pub label: String,
        pub style: Style,
        pub items: Vec<Rc<RefCell<Item>>>
    }
    impl Section {
        pub fn new(label: String, style: Style) -> Self {
            Section { label, style, items: vec![] }
        }
        pub fn add_item(mut self, item: Item) -> Self {
            self.items.push(Rc::new(RefCell::new(item)));
            self
        }
        pub fn add_items(self, items: Vec<Item>) -> Self {
            items.into_iter().fold(self, |acc, x| acc.add_item(x))
        }
    }
    pub struct  Item {
        pub label: String,
        pub option: ItemOption
    }
    impl Item {
        pub fn build(&self, prefix: &str, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer, editingmode: bool) {
            self.option.build(prefix, &self.label, editingmode, area, buf)
        }
    }
  
    #[derive(Clone)]
    pub struct ListOptionState {
        pub selected_section: usize,
        pub selected_item: usize,
        pub itm_len: usize,
        pub sec_len:usize,
        pub edit_option: bool,
        pub selected_item_ref: Option<Rc<RefCell<Item>>>
    }
    impl ListOptionState {
        pub fn default() -> Self {
            ListOptionState { selected_section: 0, selected_item: 0, itm_len: 0, sec_len: 0, selected_item_ref: None, edit_option: false }
        }
        
        pub fn selectnext(&mut self) {
            if self.selected_item+1<self.itm_len {
                self.selected_item = self.selected_item+1;
            }
        }
        pub fn selectprev(&mut self) {
            if self.selected_item>=1 {
                self.selected_item = self.selected_item-1;
            }
        }
        pub fn select_sec_next(&mut self) {
            if self.selected_section+1<self.sec_len {
                self.selected_section = self.selected_section+1;
                self.selected_item=0;
            }
        }
        pub fn select_sec_prev(&mut self) {
            if self.selected_section>=1 {
                self.selected_section = self.selected_section-1;
                self.selected_item=0;
            }
        }
        pub fn handle_input(&mut self, kc: KeyCode) -> bool {
            if let Some(sel) = &self.selected_item_ref {
                // match &item.borrow_mut().option {
                //     ItemOption::TextInputOption { get, set, state } => todo!(),
                //     ItemOption::SwitchOption { get, set, state } => todo!(),
                //     ItemOption::SelectOption { get, get_options, set, state } => todo!(),
                //     _ => todo!(),
                // }
                return match kc {
                    k @ (KeyCode::Enter) => {
                        if (self.edit_option){
                            let mut item = sel.borrow_mut();
                            match &mut item.option {
                                ItemOption::TextInputOption { get, set, state } => {
                                    set(state.value.clone());
                                },
                                ItemOption::SwitchOption { get, set, state } => {},
                                ItemOption::SelectOption { get, get_options, set, state } => {
                                    set(state.selected_value.clone());
                                },
                                _ => {},
                            }
                            self.edit_option=false;
                        } else {
                            self.edit_option=true;
                        }

                        true
                    },
                    k @ (KeyCode::Backspace | KeyCode::Char(_) | KeyCode::Delete | KeyCode::Left | KeyCode::Right) => {
                            let mut item = sel.borrow_mut();
                            match &mut item.option {
                                ItemOption::TextInputOption { get, set, state } => {
                                    state.dispatchEvent(k);
                                },
                                ItemOption::SwitchOption { get, set, state } => {},
                                ItemOption::SelectOption { get, get_options, set, state } => {},
                                _ => {},
                            }
                            return true;
                    },
                    
                    
                    KeyCode::Down => {
                        let mut item = sel.borrow_mut();
                        match &mut item.option {
                            ItemOption::TextInputOption { get, set, state } => {},
                            ItemOption::SwitchOption { get, set, state } => {},
                            ItemOption::SelectOption { get, get_options, set, state } => {
                                if state.selected_index < state.values.len() - 1 {
                                    state.selected_index = state.selected_index + 1;
                                    if let Some(x) = state.values.get(state.selected_index) {
                                        state.selected_value = x.to_string();
                                    }
                                }
                            },
                            _ => {},
                        };
                        true
                    },
                    KeyCode::Up => {
                        let mut item = sel.borrow_mut();
                        match &mut item.option {
                            ItemOption::TextInputOption { get, set, state } => {},
                            ItemOption::SwitchOption { get, set, state } => {},
                            ItemOption::SelectOption { get, get_options, set, state } => {
                                if state.selected_index > 0 {
                                    state.selected_index = state.selected_index.saturating_sub(1);
                                    if let Some(x) = state.values.get(state.selected_index) {
                                        state.selected_value = x.to_string();
                                    }
                                }
                            },
                            _ => {},
                        };
                        true
                    },
                    _ => {false}
                }
            }
            false
        }
        pub fn handle_control_inputs(&mut self, kc: KeyCode) -> bool {
            if let Some(item) = &self.selected_item_ref {
                // match &item.borrow_mut().option {
                //     ItemOption::TextInputOption { get, set, state } => todo!(),
                //     ItemOption::SwitchOption { get, set, state } => todo!(),
                //     ItemOption::SelectOption { get, get_options, set, state } => todo!(),
                //     _ => todo!(),
                // }
                return match kc {
                    KeyCode::Backspace => {
                        let mut item = item.borrow_mut();
                        match &mut item.option {
                            ItemOption::TextInputOption { get, set, state } => {
                                // state.value.insert(state.cursorpointer, c);
                                if state.cursorpointer > 0 {
                                    let before = state.value[0..state.cursorpointer].to_owned();
                                    let nbef = if let Some(spl) = before.rsplit_once(" ") {
                                        format!("{} ", spl.0)
                                    } else {
                                        format!("")
                                    };
                                    let after = state.value[state.cursorpointer..].to_owned();
                                    state.value = format!("{}{}", nbef, after);
                                    state.cursorpointer = nbef.len();
                                }
                            },
                            ItemOption::SwitchOption { get, set, state } => {},
                            ItemOption::SelectOption { get, get_options, set, state } => {},
                            _ => {},
                        }
                        return true;
                    }
                    _ => {false}
                }
            }
            false
        }
    }
    impl ListOption {
        pub fn new() -> ListOption {
            ListOption { sections: vec![]}
        }
        pub fn add_section(mut self, section: Section) -> Self {
            self.sections.push(section);
            self
        }
        pub fn add_sections(self, sections: Vec<Section>) -> Self {
            sections.into_iter().fold(self, |acc, x| acc.add_section(x))
        }
    }
    impl StatefulWidget for ListOption {
        type State = ListOptionState;
        
        fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer, state: &mut Self::State) {
            if self.sections.len() <= 0 {return;}
            state.sec_len=self.sections.len();
            if let Some(sec) = match self.sections.get(state.selected_section) {
                Some(s) => Some(s),
                _ => self.sections.first()
            } {
                state.itm_len=sec.items.len();
                let l = Block::bordered()
                    .border_type(BorderType::Rounded)
                    .border_style(Style::new().fg(getConfig().Theme.list_border.to_ratatui()))
                    .title(sec.label.clone());//Line::styled(sec.label.clone(), sec.style);
                let content_area = l.inner(area);
                l.render(area, buf);
                let h = (content_area.height) as usize;
                let (nitems, start_index) = if sec.items.len() > h {
                    let half_h = (h / 2) as usize;
                    let si = state.selected_item.saturating_sub(half_h);
                    
                    (h as usize, si.min(sec.items.len() - h ))
                }else {
                    (sec.items.len(), 0)
                };
                let t = sec.items.iter().enumerate().skip(start_index).take(nitems);
                for ((index,item), row_area) in t.zip(content_area.rows()) {
                    if index == state.selected_item {
                        state.selected_item_ref=Some(Rc::clone(item));
                        item.borrow().build(">> ", row_area, buf,state.edit_option);       
                    } else {
                        item.borrow().build("   ", row_area, buf,false);
                    };
                    
                }
            }
        }
    }


    type GetOption<S> = Box<dyn Fn() -> S>;
    type SetOption<S> = Box<dyn Fn(S) -> bool>;
    
    pub enum ItemOption {
        TextInput {
            state: TextInputState
        },
        TextInputOption { 
            get: GetOption<String>,
            set: SetOption<String>,
            state: TextInputState            
        },
        SwitchOption { 
            get: GetOption<bool>,
            set: SetOption<bool>,
            state: SwitchOptionState
        },
        SelectOption {
            get: GetOption<String>,
            get_options: GetOption<Vec<String>>,
            set: SetOption<String>,
            state: SelectOptionState
        },
        ColorPicker {
            get: GetOption<ColorPickerOption>, // TODO: insere in <> a color stuct and manage the possibility to change only a number at time. The struct of the field is: label type_which_is_a_select r,g,b_or_other_color_opt 
            set: SetOption<ColorPickerOption>,
            state: ColorPickerOptionState
        },
        DebugOption {}
    }
    impl ItemOption {
        pub fn new_text_input(get: GetOption<String>, set: SetOption<String>) -> ItemOption {
            let value = get();
            ItemOption::TextInputOption {
                get, set,
                state: TextInputState::new(value)
            }
        }
        pub fn new_text_input_with_state(state: TextInputState) -> ItemOption {
            ItemOption::TextInput {
                state
            }
        }
        pub fn new_select_option(get: GetOption<String>, get_options: GetOption<Vec<String>>, set: SetOption<String>) -> ItemOption {
            let value = get();
            let values = get_options();
            ItemOption::SelectOption {
                get, set,
                get_options,
                state: SelectOptionState::new(values, value)
            }
        }
        pub fn new_color_picker(get: GetOption<ColorPickerOption>, set: SetOption<ColorPickerOption>) -> ItemOption {
            let value = get();
            ItemOption::ColorPicker { get, set, state: ColorPickerOptionState::new(value) }
        }
        pub fn build(&self, prefix:&str,label:&str, editingmode:bool, area:Rect, buf: &mut Buffer) {
            let lay = Layout::default().constraints(vec![
                Constraint::Percentage(50),
                Constraint::Percentage(50)
            ]).direction(ratatui::layout::Direction::Horizontal).split(area);
            match &self {
                ItemOption::SelectOption { get,get_options, set, state } => {
                    let opt = get();
                    let vals = get_options();
                    Text::raw(format!("{}{}",prefix, label)).render(lay[0], buf);
                    if editingmode {
                        Text::raw(format!("> {} {}/{}", state.selected_value, state.selected_index+1, vals.len())).render(lay[1], buf);
                    } else {
                        Text::raw(format!("> {}", opt)).render(lay[1], buf);
                    }
                }
                ItemOption::TextInputOption { get, set, state } => {
                    let opt = &state.value;
                    Text::raw(format!("{}{}",prefix, label)).render(lay[0], buf);
                    if editingmode {
                        if state.ins {

                        } else {
                            let mut v = opt.clone();
                            v.insert(state.cursorpointer, '|');
                            Text::raw(format!("<{}>", v)).render(lay[1], buf);
                        }
                    } else {
                        Text::raw(format!("<{}>", opt)).render(lay[1], buf);
                    }
                },
                ItemOption::SwitchOption { get, set, state } => {

                },
                _ => {
                    Text::raw(format!("{}{}",prefix, label)).render(lay[0], buf);
                    // let opt = get();
                    if editingmode {
                        
                    } else {
                        Text::raw(format!("[{}]", "Debug")).render(lay[1], buf);
                    }
                },
                ItemOption::ColorPicker { get, set, state } => {
                    let opt = get();
                    if editingmode {
                        let r = format!("{:0<3}", opt.redString());
                        let g = format!("{:0<3}", opt.greenString());
                        let b = format!("{:0<3}", opt.blueString());
                        match opt._type {
                            crate::utils::utils::ColorPickerOptionType::RGB => {
                                Line::from(vec![
                                    Span::styled("rgb", if state.selected_index==0 { Style::default().underlined() } else { Style::default() }),
                                    Span::styled("rgb(", Style::default()),
                                    Span::styled(r.chars().nth(0).unwrap_or_default().to_string(), if state.selected_index==1 { Style::default().underlined() } else { Style::default() }),
                                    Span::styled(r.chars().nth(1).unwrap_or_default().to_string(), if state.selected_index==2 { Style::default().underlined() } else { Style::default() }),
                                    Span::styled(r.chars().nth(2).unwrap_or_default().to_string(), if state.selected_index==3 { Style::default().underlined() } else { Style::default() }),
                                    Span::styled(",", Style::default()),
                                    Span::styled(g.chars().nth(0).unwrap_or_default().to_string(), if state.selected_index==4 { Style::default().underlined() } else { Style::default() }),
                                    Span::styled(g.chars().nth(1).unwrap_or_default().to_string(), if state.selected_index==5 { Style::default().underlined() } else { Style::default() }),
                                    Span::styled(g.chars().nth(2).unwrap_or_default().to_string(), if state.selected_index==6 { Style::default().underlined() } else { Style::default() }),
                                    // Span::styled(, if state.selected_index>3 && state.selected_index<7 { Style::default().underlined() } else { Style::default() }),
                                    Span::styled(",", Style::default()),
                                    Span::styled(b.chars().nth(0).unwrap_or_default().to_string(), if state.selected_index==7 { Style::default().underlined() } else { Style::default() }),
                                    Span::styled(b.chars().nth(1).unwrap_or_default().to_string(), if state.selected_index==8 { Style::default().underlined() } else { Style::default() }),
                                    Span::styled(b.chars().nth(2).unwrap_or_default().to_string(), if state.selected_index==9 { Style::default().underlined() } else { Style::default() }),
                                    Span::styled(")", Style::default()),
                                ]).render(lay[1],buf)
                            },
                            crate::utils::utils::ColorPickerOptionType::HEX => {
                                Line::from(vec![
                                    Span::styled("hex", if state.selected_index==0 { Style::default().underlined() } else { Style::default() }),
                                    Span::styled("#", Style::default()),
                                    Span::styled(r.chars().nth(0).unwrap_or_default().to_string(), if state.selected_index==1 { Style::default().underlined() } else { Style::default() }),
                                    Span::styled(r.chars().nth(1).unwrap_or_default().to_string(), if state.selected_index==2 { Style::default().underlined() } else { Style::default() }),
                                    Span::styled(",", Style::default()),
                                    Span::styled(g.chars().nth(0).unwrap_or_default().to_string(), if state.selected_index==3 { Style::default().underlined() } else { Style::default() }),
                                    Span::styled(g.chars().nth(1).unwrap_or_default().to_string(), if state.selected_index==4 { Style::default().underlined() } else { Style::default() }),
                                    Span::styled(",", Style::default()),
                                    Span::styled(b.chars().nth(0).unwrap_or_default().to_string(), if state.selected_index==5 { Style::default().underlined() } else { Style::default() }),
                                    Span::styled(b.chars().nth(1).unwrap_or_default().to_string(), if state.selected_index==6 { Style::default().underlined() } else { Style::default() }),
                                ]).render(lay[1],buf)
                            },
                        }
                    } else {
                        match opt._type {
                            crate::utils::utils::ColorPickerOptionType::RGB => {
                                Text::raw(format!("rgb({},{},{})", opt.redString(), opt.greenString(), opt.greenString())).render(lay[1], buf);
                            },
                            crate::utils::utils::ColorPickerOptionType::HEX => {
                                Text::raw(format!("#{}{}{}", opt.redString(), opt.greenString(), opt.greenString())).render(lay[1], buf);
                            },
                        }
                    }
                },
            }
        }
        pub fn buildOnlyItem(&self, editingmode:bool, area:Rect, buf: &mut Buffer, prefix:Option<&str>, suffix:Option<&str>) {
            match &self {
                ItemOption::SelectOption { get,get_options, set, state } => {
                    let opt = get();
                    let vals = get_options();
                    if editingmode {
                        Text::raw(format!("> {} {}/{}", state.selected_value, state.selected_index+1, vals.len())).render(area, buf);
                    } else {
                        Text::raw(format!("> {}", opt)).render(area, buf);
                    }
                }
                ItemOption::TextInputOption { get, set, state } => {
                    let opt = &state.value;
                    // Text::raw(format!("{}{}",prefix, label)).render(lay[0], buf); //TODO ??
                    if editingmode {
                        if state.ins {

                        } else {
                            let mut v = opt.clone();
                            v.insert(state.cursorpointer, '|');
                            Text::raw(format!("{}{}{}", prefix.unwrap_or(""), v, suffix.unwrap_or(""))).render(area, buf);
                        }
                    } else {
                        Text::raw(format!("{}{}{}",  prefix.unwrap_or(""), opt, suffix.unwrap_or(""))).render(area, buf);
                    }
                },
                 ItemOption::TextInput { state } => {
                    let opt = &state.value;
                    // Text::raw(format!("{}{}",prefix, label)).render(lay[0], buf); //TODO ??
                    if editingmode {
                        if state.ins {

                        } else {
                            let mut v = opt.clone();
                            v.insert(state.cursorpointer, '|');
                            Text::raw(format!("{}{}{}", prefix.unwrap_or(""), v, suffix.unwrap_or(""))).render(area, buf);
                        }
                    } else {
                        Text::raw(format!("{}{}{}",  prefix.unwrap_or(""), opt, suffix.unwrap_or(""))).render(area, buf);
                    }
                },
                ItemOption::SwitchOption { get, set, state } => {

                },
                _ => {
                    // Text::raw(format!("{}{}",prefix, label)).render(lay[0], buf);
                    // let opt = get();
                    if editingmode {

                    } else {
                        Text::raw(format!("[{}]", "Debug")).render(area, buf);
                    }
                },
                ItemOption::ColorPicker { get, set, state } => {

                },
            }
        }
    }

    pub struct  SwitchOptionState {
        value: bool
    }

    pub struct SelectOptionState {
        values: Vec<String>,
        selected_value: String,
        selected_index: usize
    }
    impl SelectOptionState {
        fn new(values: Vec<String>, value: String) -> SelectOptionState {
            SelectOptionState {
                selected_index: values.iter().position(|x| *x==value).unwrap_or(0),
                selected_value: value,
                values
            }
        }
    }
    pub struct  ColorPickerOptionState {
        value: ColorPickerOption,
        selected_index: u8,
        selected_index_sub: u8
    }
    impl ColorPickerOptionState {
        fn new(value: ColorPickerOption) -> ColorPickerOptionState {
            ColorPickerOptionState { value, selected_index: 0, selected_index_sub: 0 }
        }
    }
}

pub struct AskToQuit {

}

pub struct ListFuncOption {
    pub name: String,
    pub func: Box<dyn Fn()>,
    pub style: Option<ratatui::style::Style>
}
pub struct AskToQuitState {
    pub sel_index: usize,
    pub items: Vec<ListFuncOption>
}
impl AskToQuitState {
    pub fn get_selected_item(&self) -> Option<&ListFuncOption> {
        return self.items.get(self.sel_index);
    }
    pub fn select_next(&mut self) {
        if self.sel_index < self.items.len()-1  {
            self.sel_index = self.sel_index+1;
        }
    }
    pub fn select_previous(&mut self) {
        if self.sel_index > 0  {
            self.sel_index = self.sel_index.saturating_sub(1);
        }
    }
}

impl StatefulWidget for AskToQuit {
    type State = AskToQuitState;

    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer, state: &mut Self::State) {
        // let block = Block::default().title("Quit?").borders(Borders::ALL);
        let sel = state.get_selected_item();
        let sstyle = state.get_selected_item().and_then(|s| s.style);
        let n = state.items.len();
        let nn = n as u32;
        let carea = area.centered(Constraint::Percentage(100), Constraint::Length(1));
        // let mut li = List::new(list_items).block(block).highlight_style(sstyle.unwrap_or(ratatui::style::Style::new().bg(tailwind::WHITE).fg(tailwind::BLACK)));
        let areas = Layout::default().direction(ratatui::layout::Direction::Horizontal).constraints(vec![Constraint::Ratio(1, nn); n]).split(carea);
        for i in 0..n {
            if let Some(item) = state.items.get(i) {
                let sty = if state.sel_index==i {
                    item.style.unwrap_or(ratatui::style::Style::new().bg(ratatui::style::palette::tailwind::YELLOW.c200).black())
                } else {
                    ratatui::style::Style::default()
                };
                Text::styled(item.name.clone(), sty).centered().render(areas[i], buf);
            }
        }
    }
}


#[derive(Clone)]
pub struct TextInputState {
    pub editingmode: bool,
    pub value: String,
    pub ins: bool,
    pub cursorpointer: usize, // if ins this is put on the letter of index, otherwise it is put before
}
impl TextInputState {
    pub fn new(value: String) -> Self {
        TextInputState {
            editingmode: false,
            value: value.clone(), 
            ins: false,
            cursorpointer: value.len()
        }
    }
    pub fn dispatchEvent(&mut self, k: KeyCode) -> bool {
        match k {
            KeyCode::Char(c) => {
                if (c.is_ascii()) {
                    self.editingmode = true;
                    self.value.insert(self.cursorpointer, c);
                    self.cursorpointer = self.value.len().min(self.cursorpointer+1);
                    true
                } else {
                    false
                }
            },
            KeyCode::Backspace => {
                if self.editingmode {
                    if self.cursorpointer > 0 {
                        self.value.remove(self.cursorpointer-1);
                        self.cursorpointer = self.cursorpointer.saturating_sub(1);
                    }
                    return true;
                } 
                false
            },
            KeyCode::Delete => {
                self.editingmode = true;
                if self.cursorpointer < self.value.len() {
                    self.value.remove(self.cursorpointer);
                }
                true
            }
            KeyCode::Left => {
                self.cursorpointer = self.cursorpointer.saturating_sub(1);
                true
            },
            KeyCode::Right => {
                self.cursorpointer = self.value.len().min(self.cursorpointer+1);
                true
            }
            _ => {false}
        }
    }
}
pub mod Search {
    use std::{cmp::Ordering, collections::HashMap, hash::Hash, sync::Arc};

    use openssl::derive;
use ratatui::{buffer::Buffer, layout::{Constraint, Layout, Rect}, style::Style, text::Text, widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, StatefulWidget, Widget, canvas::Label}};

    use crate::{appmessages::MessageType, components::{ListOption::Item, TextInputState}, config::getConfig, library::{self, LibraryItem}, settings::ApplySettingsChange, source::{AnimeSource, Episode, EpisodeListItem, MangaSource}, utils::utils::hash_sort};
    pub struct Search {
        pub trasmitter_to_app:tokio::sync::mpsc::UnboundedSender<MessageType>,
        // pub trasmitter_to_app_input:tokio::sync::mpsc::UnboundedSender<String>,
        pub library_type: SearchType
    }
    impl Search {
        pub fn new(library_type: SearchType,trasmitter_to_app: tokio::sync::mpsc::UnboundedSender<MessageType>) -> Self {
            Self {
                library_type,
                trasmitter_to_app
            }
        } 
        pub fn sorted_keys_from_hash(hs: &HashMap<String, EpisodeListItem>) -> Vec<&String> {
            let mut items: Vec<&String> = hs.keys().collect::<Vec<&String>>();
            items.sort_by(|a: &&String,b: &&String| {
                match (a.parse::<isize>(), b.parse::<isize>()) {
                    (Ok(ai), Ok(bi)) => bi.cmp(&ai).then_with(|| b.cmp(a)),
                    (Ok(_), Err(_)) => Ordering::Less,
                    (Err(_), Ok(_)) => Ordering::Greater,
                    (Err(_), Err(_)) => b.to_lowercase()
                        .cmp(&a.to_lowercase())
                        .then_with(|| b.cmp(a)),
                }
            }); // ? REVERSE SORT - MAX TO MIN -> NEWEST FIRST
            items
        }
    }
    #[derive(PartialEq, Eq)]
    pub enum SearchStateStatus {
        Empty,
        EntryList,
        EpisodeList,
    }

    
    pub struct SearchState {
        pub list_state_entries: ListState, 
        pub list_state_eps: ListState, 
        pub lib: HashMap<String, Episode>,
        pub status: SearchStateStatus,
        pub flag_reverse: bool,
        pub entry_selected: bool,
        pub episodes_loaded: HashMap<String, EpisodeListItem>,
        pub current_source: Option<String>,
        pub text_input_state: TextInputState,
        pub current_search: Option<tokio::task::JoinHandle<()>>,
        pub all_source: Vec<String>
    }
    impl SearchState {
        pub fn new(source: String, all_source: Vec<String>) -> Self {
            let mut list_state_entries = ListState::default();
            let mut list_state_eps = ListState::default();
            list_state_entries.select_first();
            list_state_eps.select_first();

            Self {
                lib: HashMap::new(),
                list_state_entries,
                list_state_eps,
                entry_selected: false,
                status: SearchStateStatus::Empty,
                episodes_loaded: HashMap::new(),
                current_source: Some(source),
                text_input_state: TextInputState::new("".to_string()),
                current_search: None,
                all_source: all_source,
                flag_reverse: false
            }
        }
    }
    #[derive(Clone,Copy)]
    pub enum SearchType {
        Anime, Manga
    }
    // TODO
    impl StatefulWidget for Search {
        type State = SearchState;

        fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
            let block = Block::default().title("Search").borders(Borders::ALL);
            let area_v = Layout::default().direction(ratatui::layout::Direction::Vertical).constraints([
                Constraint::Length(3),
                Constraint::Percentage(100),
            ]).split(area);
            let tx = self.trasmitter_to_app.clone();
            let text_input = crate::components::ListOption::ItemOption::new_text_input_with_state(state.text_input_state.clone());

            
            let hor = Layout::default().direction(ratatui::layout::Direction::Horizontal).constraints([
                Constraint::Percentage(100),
                Constraint::Min(30)
            ]).split(area_v[0]);
            text_input.buildOnlyItem(
                state.text_input_state.editingmode, 
                hor[0].centered(Constraint::Percentage(100), Constraint::Ratio(1, 3)), 
                buf,
                Some(">"),
                None
            );
            let block_sc = Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(ratatui::style::Color::Cyan));
            let block_eps = Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(ratatui::style::Color::Cyan));
            let block_select = Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(ratatui::style::Color::Cyan));
            let mut keys: Vec<String> = state.lib.keys().cloned().collect();
            keys.sort();
            let list_res = List::new(keys).block(block_eps.clone()).highlight_symbol(">> ");;
        
            if state.status == SearchStateStatus::EpisodeList {
                let lay = Layout::default().direction(ratatui::layout::Direction::Horizontal).constraints([
                    Constraint::Percentage(100),
                    Constraint::Min(30)
                ]).split(area_v[1]);
                let mut ekeys: Vec<&String> = Self::sorted_keys_from_hash(&state.episodes_loaded);//state.episodes_loaded.keys().cloned().collect();
                if (state.flag_reverse) {
                    ekeys.reverse();
                }
                let mut eps = Vec::with_capacity(ekeys.len());
                for it in ekeys {
                    let s = state.episodes_loaded.get(it).and_then(|f| Some(f.title.clone())).unwrap_or(it.clone());
                    eps.push(ListItem::new(s));
                }
                let list_eps = List::new(eps).block(block_eps).highlight_symbol(">> ");;
                StatefulWidget::render(list_res, lay[0], buf, &mut state.list_state_entries);
                StatefulWidget::render(list_eps, lay[1], buf, &mut state.list_state_eps);
            
            } else {
                StatefulWidget::render(list_res, area_v[1], buf, &mut state.list_state_entries);
            }
            let selecsource = Paragraph::new(state.current_source.clone().unwrap_or("Default".to_owned())).block(block_select);
            
            Widget::render(block_sc, hor[0], buf);
            Widget::render(selecsource, hor[1], buf);
            // match &mut state.status {
            //     SearchStateStatus::Empty => {
                    
            //     }
                // SearchStateStatus::SearchFor(v) =>{
                //     let mut keys: Vec<&String> = state.lib.keys().collect();
                //     keys.sort();
                //     let l = keys.len();
                //     Widget::render(block_sc, hor[0], buf);
                //     Widget::render(selecsource, hor[1], buf);
                //     if state.entry_selected && let Some(eps_hash) = &state.episodes_loaded {
                //         let items = Self::sorted_keys_from_hash(eps_hash);
                //         let mut eps = Vec::with_capacity(items.len());
                //         for it in items {
                //             let s = eps_hash.get(it).and_then(|f| Some(f.title.clone())).unwrap_or(it.clone());
                //             eps.push(ListItem::new(s));
                //         }
                //         let list_ep = List::new(eps).block(block_eps).highlight_symbol(">> ");;
                //         StatefulWidget::render(list_ep, lay[1], buf, &mut state.list_state_eps);
                //     } else {
                //         Widget::render(block_eps, lay[1], buf);
                //     }
                // },
                // SearchStateStatus::SearchResult(index) => {
                //     let mut keys: Vec<&String> = state.lib.keys().collect();
                //     keys.sort();
                //     let l = keys.len();
                //     Widget::render(block_sc, hor[0], buf);
                //     Widget::render(selecsource, hor[1], buf);
                //     if state.entry_selected && let Some(eps_hash) = &state.episodes_loaded {
                //         let items = Self::sorted_keys_from_hash(eps_hash);
                //         let mut eps = Vec::with_capacity(items.len());
                //         for it in items {
                //             let s = eps_hash.get(it).and_then(|f| Some(f.title.clone())).unwrap_or(it.clone());
                //             eps.push(ListItem::new(s));
                //         }
                //         let list_ep = List::new(eps).block(block_eps).highlight_symbol(">> ");;
                //         StatefulWidget::render(list_ep, lay[1], buf, &mut state.list_state_eps);
                //     } else {
                //         Widget::render(block_eps, lay[1], buf);
                //     }
                // }
            }
        }
}