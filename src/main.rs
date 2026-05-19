mod db;
mod models;
mod spotify;

use crate::models::{
    AppState, FilterQuery, IndexTemplate, NewTagRequest, PlaybackTemplate, PlaybackView,
};
use axum::extract::Query;
use axum::{
    Router,
    extract::{Form, Path, State},
    response::{IntoResponse, Redirect},
    routing::{delete, get, post},
    http::HeaderMap,
};
use rspotify::model::{PlayableId, TrackId};
use rspotify::prelude::OAuthClient;
use tokio::sync::RwLock;
use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Arc;

#[derive(serde::Deserialize)]
pub struct CallbackQuery {
    pub code: String,
}

async fn auth_handler(State(state): State<AppState>) -> impl IntoResponse {
    let url = spotify::get_auth_url(&state.spotify);
    // TODO: debug line
    println!("--- Generated Spotify Auth Link: {}", url);
    Redirect::to(&url)
}

// async fn callback_handler(
//     State(state): State<AppState>,
//     Query(query): Query<CallbackQuery>,
// ) -> impl IntoResponse {
//     if let Err(e) = spotify::authenticate(&state.spotify, &query.code).await {
//         eprintln!("Auth error: {:?}", e);
//     }
//     *state.authed.write().await = true;
//     Redirect::to("/")
// }

// TODO: for debugging
async fn callback_handler(
    State(state): State<AppState>,
    req: axum::http::Request<axum::body::Body>, // Captures the full raw request
) -> impl IntoResponse {
    println!("--- Raw Incoming URI: {:?}", req.uri());
    println!("--- Raw Incoming Headers: {:?}", req.headers());
    let query_str = req.uri().query().unwrap_or("");
    let parsed: Result<CallbackQuery, _> = serde_urlencoded::from_str(query_str);
    match parsed {
        Ok(query) => {
            if let Err(e) = spotify::authenticate(&state.spotify, &query.code).await {
                eprintln!("Auth error: {:?}", e);
            }
            *state.authed.write().await = true;
            Redirect::to("/").into_response()
        }
        Err(e) => {
            eprintln!("--- Serde failed to parse query parameters: {:?}", e);
            format!(
                "Error: Query string did not contain a readable 'code' field.\nRaw query seen by server: '{}'", 
                query_str
            ).into_response()
        }
    }
}

async fn require_auth(state: &AppState) -> Option<Redirect> {
    if !*state.authed.read().await {
        Some(Redirect::to("/auth"))
    } else {
        None
    }
}

async fn add_tag_handler(
    State(state): State<AppState>,
    Path(track_id): Path<String>,
    headers: HeaderMap,
    Form(payload): Form<NewTagRequest>,
) -> impl IntoResponse {
    let _ = db::add_tag_to_track(&state.pool, &track_id, &payload.tag_name).await;
    let redirect_to = headers
        .get("referer")
        .and_then(|v| v.to_str().ok())
        .and_then(|url| url.split_once('?').map(|(_, qs)| format!("/?{}", qs)))
        .unwrap_or_else(|| "/".to_string());

    Redirect::to(&redirect_to)
}

async fn list_tracks_handler(
    State(state): State<AppState>,
    Query(filter): Query<FilterQuery>,
) -> impl IntoResponse {
    if let Some(r) = require_auth(&state).await { return r.into_response(); }
    
    let page = filter.page.unwrap_or(0);
    let page_size = 50;

    let tracks = if let Some(title_query) = filter.q.clone() {
        db::fetch_by_song_title(&state.pool, title_query)
            .await
            .unwrap()
    } else {
        db::fetch_tracks_paged(&state.pool, filter.tag.clone(), page, page_size + 1)
            .await
            .unwrap()
    };

    let has_next = tracks.len() > page_size as usize && filter.q.is_none();

    let all_tags = db::fetch_unique_tags(&state.pool).await.unwrap();

    let last_sync = db::get_kv_store(&state.pool, "last_sync")
        .await
        .unwrap_or(None)
        .unwrap_or_else(|| "Never".to_string());

    IndexTemplate {
        tracks,
        all_tags,
        current_filter: filter.tag.or(filter.q),
        current_page: page,
        has_next,
        last_sync,
        selected_ids: state.selected_tracks.read().await.clone()
    }.into_response()
}

async fn create_tag_handler(
    State(state): State<AppState>,
    Form(payload): Form<NewTagRequest>,
) -> impl IntoResponse {
    let _ = db::create_global_tag(&state.pool, &payload.tag_name).await;
    Redirect::to("/")
}

async fn remove_tag_handler(
    State(state): State<AppState>,
    Path((track_id, tag_name)): Path<(String, String)>,
) -> impl IntoResponse {
    let _ = db::remove_tag_from_track(&state.pool, &track_id, &tag_name).await;
    Redirect::to("/")
}

async fn sync_handler(State(state): State<AppState>) -> impl IntoResponse {
    let existing_ids = db::get_all_track_ids(&state.pool).await.unwrap_or_default();
    match spotify::fetch_all_liked_tracks(&state.spotify, &existing_ids).await {
        Ok(tracks) => {
            let _ = db::sync_tracks(&state.pool, tracks).await;
            sqlx::query(
                "INSERT INTO kv_store (key, value) VALUES ('last_sync', CURRENT_TIMESTAMP) 
                            ON CONFLICT(key) DO UPDATE SET value = CURRENT_TIMESTAMP",
            )
            .execute(&state.pool)
            .await
            .unwrap();
            Redirect::to("/").into_response()
        }
        Err(e) => {
            eprintln!("Sync error: {:?}", e);
            let mut headers = axum::http::HeaderMap::new();
            headers.insert("hx-redirect", "/auth".parse().unwrap());
            (headers, "").into_response()
        }
    }
}

async fn get_playback_handler(State(state): State<AppState>) -> impl IntoResponse {
    let current = state
        .spotify
        .current_playback(None, None::<&[rspotify::model::AdditionalType]>)
        .await
        .ok()
        .flatten();
    let view = current.as_ref().and_then(PlaybackView::from_context);
    PlaybackTemplate { playback: view }
}

async fn toggle_select_handler(
    State(state): State<AppState>,
    Path(track_id): Path<String>,
) -> impl IntoResponse {
    let mut selected = state.selected_tracks.write().await;
    if !selected.remove(&track_id) {
        selected.insert(track_id);
    }
    format!("<span id='selection-count'>{} selected</span>", selected.len())
}

async fn add_selected_to_queue(State(state): State<AppState>) -> impl IntoResponse {
    let mut selected = state.selected_tracks.write().await;
    
    for track_id_str in selected.iter() {
        let clean_id = track_id_str.trim().strip_prefix("spotify:track:").unwrap_or(track_id_str);
        match TrackId::from_id(clean_id) {
            Ok(tid) => {
                let play_id = PlayableId::Track(tid);
                if let Err(e) = state.spotify.add_item_to_queue(play_id, None).await {
                    eprintln!("Spotify API error: {}", e);
                }
            }
            Err(_) => eprintln!("Skipping invalid track ID: {}", track_id_str),
        }
    }
    
    selected.clear();
    Redirect::to("/")
}

async fn clear_selected(State(state): State<AppState>) -> impl IntoResponse {
    state.selected_tracks.write().await.clear();
    Redirect::to("/")
}

async fn pause_track_playback(State(state): State<AppState>) -> impl IntoResponse {
    let _ = state.spotify.pause_playback(None).await;
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    get_playback_handler(State(state)).await
}

async fn resume_track_playback(State(state): State<AppState>) -> impl IntoResponse {
    let _ = state.spotify.resume_playback(None, None).await;
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    get_playback_handler(State(state)).await
}

async fn next_track_playback(State(state): State<AppState>) -> impl IntoResponse {
    let _ = state.spotify.next_track(None).await;
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    get_playback_handler(State(state)).await
}

async fn previous_track_playback(State(state): State<AppState>) -> impl IntoResponse {
    let _ = state.spotify.previous_track(None).await;
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    get_playback_handler(State(state)).await
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let pool = db::setup_db().await?;

    let spotify = spotify::create_client_unauthenticated().await?;

    let state = AppState {
        pool,
        spotify: Arc::new(spotify),
        selected_tracks: Arc::new(RwLock::new(HashSet::new())),
        authed: Arc::new(RwLock::new(false)),
    };

    let app = Router::new()
        .route("/auth", get(auth_handler))
        .route("/callback", get(callback_handler))
        .route("/", get(list_tracks_handler))
        .route("/tags", post(create_tag_handler))
        .route("/tracks/:id/tags", post(add_tag_handler))
        .route("/tracks/:id/tags/:tag_name", delete(remove_tag_handler))
        .route("/tracks/:id/toggle", post(toggle_select_handler))
        .route("/sync", post(sync_handler))
        .route("/playback", get(get_playback_handler))
        .route("/queue", post(add_selected_to_queue))
        .route("/playback/pause", post(pause_track_playback))
        .route("/playback/play", post(resume_track_playback))
        .route("/playback/next", post(next_track_playback))
        .route("/playback/prev", post(previous_track_playback))
        .route("/clear", post(clear_selected))
        .with_state(state);


    // let redirect_uri = std::env::var("RSPOTIFY_REDIRECT_URI")
    //     .expect("RSPOTIFY_REDIRECT_URI must be set in .env");
    let addr = SocketAddr::from(([0, 0, 0, 0], 8888));
    println!("🚀 Server listening locally on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
