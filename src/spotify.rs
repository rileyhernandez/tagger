use anyhow::{Result, anyhow};
use futures_util::StreamExt;
use rspotify::{AuthCodeSpotify, Credentials, OAuth, prelude::*};
use std::collections::HashSet;

pub async fn create_client_unauthenticated() -> Result<AuthCodeSpotify> {
    let creds = Credentials::from_env().ok_or(anyhow!("No Spotify Credentials"))?;
    let oauth = OAuth::from_env(rspotify::scopes!(
        "user-library-read",
        "user-read-playback-state",
        "user-modify-playback-state"
    ))
    .ok_or(anyhow!("No Redirect URI"))?;

    let spotify = AuthCodeSpotify::new(creds, oauth);
    let _ = spotify.read_token_cache(true).await;

    Ok(spotify)
}

pub fn get_auth_url(spotify: &AuthCodeSpotify) -> String {
    spotify.get_authorize_url(false).unwrap_or_default()
}

pub async fn authenticate(spotify: &AuthCodeSpotify, code: &str) -> Result<()> {
    spotify.request_token(code).await?;
    Ok(())
}

pub async fn fetch_all_liked_tracks(
    spotify: &AuthCodeSpotify,
    existing_ids: &HashSet<String>,
) -> Result<Vec<(String, String, String, String)>> {
    let mut all_tracks = Vec::new();
    println!("Fetching tracks from Spotify...");

    let stream = spotify.current_user_saved_tracks(None);
    let mut items = Box::pin(stream);

    while let Some(item) = items.next().await {
        match item {
            Ok(t) => {
                let track = t.track;
                let id = track.id.as_ref().map(|id| id.to_string()).unwrap_or_default();

                if !id.is_empty() && existing_ids.contains(&id) {
                    println!("Found already synced track: {}. Stopping fetch.", track.name);
                    break;
                }

                let liked_at = t.added_at.to_rfc3339();
                let artist = track
                    .artists
                    .first()
                    .map(|a| a.name.clone())
                    .unwrap_or_default();

                all_tracks.push((id, track.name, artist, liked_at));
            }
            Err(e) => {
                println!("Error fetching track: {:?}", e);
                break;
            }
        }
    }

    println!("Total new tracks found: {}", all_tracks.len());
    Ok(all_tracks)
}
