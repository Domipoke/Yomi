use ratatui::{crossterm::event::KeyEvent, style::Style, widgets::{List, StatefulWidget, Widget}};

use crate::{App, AppState, appmessages::MessageType, components::ListOption::{Item, ListOption, ListOptionState, Section}, config::{getConfig, getDevice, load_config, setAnimeSource, setDevice, setMangaSource}, source::getSources, utils::utils::ColorPickerOption};

// pub static testSection: crate::components::ListOption::Section = ;

#[derive(Clone)]
pub struct  SettingsState {
    pub listoption_sections: Vec<Section>,
    listoptionstate: ListOptionState
    // tx: tokio::sync::mpsc::UnboundedSender<ApplySettingsChange>
}

pub struct Settings {
}
impl Settings {
    pub fn new() -> Settings {
        Settings {
            
        }
    }
}
impl SettingsState {
    pub fn new(tx: tokio::sync::mpsc::UnboundedSender<MessageType>) -> SettingsState {
        let mut v: Vec<Section> = vec![];
        v.push(Self::buildSettings(tx.clone()));
        v.push(Self::buildColorCustomization(tx.clone()));
        SettingsState {
            listoptionstate: ListOptionState::default(),
            listoption_sections: v
        }
    }

    pub fn buildSettings(tx: tokio::sync::mpsc::UnboundedSender<MessageType>) -> Section {
        let tx_a = tx.clone();
        let tx_m = tx.clone();
        let mut s = Section::new("Generali".to_owned(), Style::default())
        .add_item(Item {
            label: "Sorgente Anime predefinita".to_owned(),
            option: crate::components::ListOption::ItemOption::new_select_option(Box::new(|| {
                getConfig().Anime.source
            }),Box::new(  || {
                getSources("anime")
            }),Box::new(move |v| {
                setAnimeSource(v.clone());
                tx_a.send(MessageType::SettingsChange(ApplySettingsChange::SetAnime(v)));
                true
            })),
        })
        .add_item(Item {
            label: "Sorgente Manga predefinita".to_owned(),
            option: crate::components::ListOption::ItemOption::new_select_option(Box::new(|| {
                getConfig().Manga.source
            }),Box::new(  || {
                getSources("manga")
            }),Box::new(move |v| {
                setMangaSource(v.clone());
                tx_m.send(MessageType::SettingsChange(ApplySettingsChange::SetManga(v)));
                true
            })),
        });
        if (cfg!(feature = "remote")) {
            s=s.add_item(Item { label: "Device".to_owned(), option: crate::components::ListOption::ItemOption::new_select_option(Box::new(||{
                if let Some(dev) = getDevice() {
                    return dev.name.to_owned()
                }
                "None".to_owned()
            }), Box::new(  || {
                let dev = getConfig().Device;
                std::iter::once("None".to_string())
                    .chain(dev.iter().map(|x| x.name.clone()))
                    .collect()
            }), Box::new(move |v| {
                if (v.to_lowercase() == "none") {
                    setDevice(None);
                    return true;
                } else if let Some(dev) = getConfig().Device.iter().find(|x| x.name==v) {
                    setDevice(Some(dev.clone()));
                    return true;
                }
                return false;
            })) });
        }
        s
    }
    
    pub fn buildColorCustomization(tx: tokio::sync::mpsc::UnboundedSender<MessageType>) -> Section {
        let tx_a = tx.clone();
        let tx_m = tx.clone();
        let s = Section::new("Theme".to_owned(), Style::default())
        .add_item(Item {
            label: "".to_owned(),
            option: crate::components::ListOption::ItemOption::new_color_picker(Box::new(|| {
                return ColorPickerOption::default(); // TODO
            }), Box::new(move |c| {
                tx_m.send(MessageType::SettingsChange(ApplySettingsChange::SetColorTheme("example".to_owned(), c)));
                true
            })),
        });
        s
    }
    pub fn selectnext(&mut self) {
        self.listoptionstate.selectnext();
    }
    pub fn selectprev(&mut self) {
        self.listoptionstate.selectprev();
    }
    pub fn select_sec_next(&mut self) {
        self.listoptionstate.select_sec_next();
    }
    pub fn select_sec_prev(&mut self) {
        self.listoptionstate.select_sec_prev();
    }
    pub fn is_editing(&self) -> bool {
        self.listoptionstate.edit_option
    }
    pub fn handle_inputs(&mut self, k: ratatui::crossterm::event::KeyCode) -> bool {
        self.listoptionstate.handle_input(k)
    }
    pub fn turn_editing_mode(&mut self, b:bool) {
        self.listoptionstate.edit_option = b;
    }
    pub fn handle_control_inputs(&mut self, k:ratatui::crossterm::event::KeyCode) {
        self.listoptionstate.handle_control_inputs(k);
    }
}
impl StatefulWidget for Settings {
    type State = SettingsState;

    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer, state: &mut Self::State) {

        // Mutable but one creation only,  
        ListOption::new().add_sections(state.listoption_sections.clone())
        .render(area, buf, &mut state.listoptionstate);
    }
}


pub enum ApplySettingsChange {
    SetManga(String),
    SetAnime(String),
    SetAppStatus(AppState),
    SetColorTheme(String, crate::utils::utils::ColorPickerOption)
}