use std::collections::HashMap;

use crate::{components::{AsyncUpdateAnimeList::AsyncAnimeStateStatus, AsyncUpdateMangaList::AsyncMangaStateStatus, Search::SearchStateStatus}, settings::ApplySettingsChange, source::{Episode, EpisodeListItem}};


pub enum MessageType {
    AnimeState(AsyncAnimeStateStatus),
    MangaState(AsyncMangaStateStatus),
    SettingsChange(ApplySettingsChange),
    LoadEpisodes(HashMap<String, EpisodeListItem>),
    LoadChapters(HashMap<String, EpisodeListItem>),
    AnimeSearch(SearchStateStatus),
    MangaSearch(SearchStateStatus),
    FoundMangaFromQuery(Vec<Episode>),
    FoundAnimeFromQuery(Vec<Episode>),
    ListChaptersFromQuery(HashMap<String, EpisodeListItem>),
    ListEpisodesFromQuery(HashMap<String, EpisodeListItem>)
}