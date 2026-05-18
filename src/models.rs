use askama::Template;
use rspotify::{AuthCodeSpotify, model::CurrentPlaybackContext, model::PlayableItem};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::collections::HashSet;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::SqlitePool,
    pub spotify: Arc<AuthCodeSpotify>,
    pub selected_tracks: Arc<RwLock<HashSet<String>>>,
    pub authed: Arc<RwLock<bool>>,
}

#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    pub tracks: Vec<TrackDisplay>,
    pub all_tags: Vec<String>,
    pub current_filter: Option<String>,
    pub current_page: i64,
    pub has_next: bool,
    pub last_sync: String,
    pub selected_ids: HashSet<String>,
}

#[derive(Template)]
#[template(path = "playback_fragment.html")]
pub struct PlaybackTemplate {
    pub playback: Option<PlaybackView>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TrackDisplay {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub tags: Vec<String>,
}

#[derive(Deserialize)]
pub struct NewTagRequest {
    pub tag_name: String,
}

#[derive(Deserialize)]
pub struct FilterQuery {
    pub tag: Option<String>,
    pub q: Option<String>,
    pub page: Option<i64>,
}

pub struct PlaybackView {
    pub name: String,
    pub artist: String,
    pub image_url: String,
    pub is_playing: bool,
    pub device_name: String,
}

impl PlaybackView {
    pub fn from_context(ctx: &CurrentPlaybackContext) -> Option<Self> {
        let item = ctx.item.as_ref()?;

        let (name, artist, image_url) = match item {
            PlayableItem::Track(t) => (
                t.name.clone(),
                t.artists
                    .first()
                    .map(|a| a.name.clone())
                    .unwrap_or_default(),
                t.album
                    .images
                    .first()
                    .map(|i| i.url.clone())
                    .unwrap_or_default(),
            ),
            PlayableItem::Episode(e) => (
                e.name.clone(),
                "Podcast".to_string(),
                e.images.first().map(|i| i.url.clone()).unwrap_or_default(),
            ),
            PlayableItem::Unknown(v) => (
                v["name"].as_str().unwrap_or("Unknown").to_string(),
                v["artists"][0]["name"]
                    .as_str()
                    .unwrap_or("Various")
                    .to_string(),
                v["album"]["images"][0]["url"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            ),
        };

        Some(Self {
            name,
            artist,
            image_url,
            is_playing: ctx.is_playing,
            device_name: ctx.device.name.clone(),
        })
    }
}
