use crate::models::TrackDisplay;
use anyhow::Result;
use sqlx::{Row, SqlitePool, sqlite::SqlitePoolOptions};
use std::collections::HashSet;

pub async fn get_all_track_ids(pool: &SqlitePool) -> Result<HashSet<String>> {
    let ids: Vec<String> = sqlx::query_scalar("SELECT spotify_id FROM tracks")
        .fetch_all(pool)
        .await?;
    Ok(ids.into_iter().collect())
}

pub async fn setup_db() -> Result<SqlitePool> {
    let db_path = dotenvy::var("DATABASE_PATH")?;
    let database_url = format!("sqlite:{}?mode=rwc", db_path);
    let pool = SqlitePoolOptions::new()
        .connect(&database_url)
        .await?;

    sqlx::query("PRAGMA foreign_keys = ON;")
        .execute(&pool)
        .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS tracks (
            spotify_id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            artist_name TEXT NOT NULL,
            liked_at DATETIME
        );",
    )
    .execute(&pool)
    .await?;
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS tags (
            id INTEGER PRIMARY KEY AUTOINCREMENT, 
            name TEXT NOT NULL UNIQUE
        );",
    )
    .execute(&pool)
    .await?;
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS track_tags (
            spotify_id TEXT NOT NULL, 
            tag_id INTEGER NOT NULL, 
            PRIMARY KEY (spotify_id, tag_id), 
            FOREIGN KEY (spotify_id) REFERENCES tracks (spotify_id) ON DELETE CASCADE, 
            FOREIGN KEY (tag_id) REFERENCES tags (id) ON DELETE CASCADE
            );",
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS kv_store (
            key TEXT PRIMARY KEY,
            value TEXT,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        ) WITHOUT ROWID;",
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}

pub async fn get_kv_store(pool: &SqlitePool, key: &str) -> Result<Option<String>> {
    let res = sqlx::query_scalar::<_, String>("SELECT value FROM kv_store WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await?;
    Ok(res)
}

// // TODO: will add this later on when i implement it
// pub async fn set_kv_store(pool: &SqlitePool, key: &str, value: &str) -> Result<()> {
//     sqlx::query(
//         r#"
//         INSERT INTO kv_store (key, value)
//         VALUES (?, ?)
//         ON CONFLICT(key) DO UPDATE SET
//             value = excluded.value,
//             updated_at = CURRENT_TIMESTAMP
//         "#,
//     )
//     .bind(key)
//     .bind(value)
//     .execute(pool)
//     .await?;
//     Ok(())
// }

pub async fn fetch_by_song_title(
    pool: &SqlitePool,
    song_title: String,
) -> Result<Vec<TrackDisplay>> {
    let rows = sqlx::query(
        r#"
        SELECT t.spotify_id, t.title, t.artist_name, GROUP_CONCAT(tg.name, ',') as tag_list
        FROM tracks t
        LEFT JOIN track_tags tt ON t.spotify_id = tt.spotify_id
        LEFT JOIN tags tg ON tt.tag_id = tg.id
        WHERE t.title LIKE '%' || ? || '%' COLLATE NOCASE
        GROUP BY t.spotify_id
        ORDER BY t.liked_at DESC
        "#,
    )
    .bind(song_title)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let tag_list: Option<String> = row.try_get("tag_list").ok();
            let tags = tag_list
                .map(|s| {
                    s.split(',')
                        .filter(|t| !t.is_empty())
                        .map(|t| t.to_string())
                        .collect()
                })
                .unwrap_or_default();

            TrackDisplay {
                id: row.get("spotify_id"),
                title: row.get("title"),
                artist: row.get("artist_name"),
                tags,
            }
        })
        .collect())
}

pub async fn fetch_unique_tags(pool: &SqlitePool) -> Result<Vec<String>> {
    let tags = sqlx::query_scalar("SELECT name FROM tags ORDER BY name ASC")
        .fetch_all(pool)
        .await?;
    Ok(tags)
}

pub async fn create_global_tag(pool: &SqlitePool, name: &str) -> Result<()> {
    let normalized = name.trim().to_lowercase();
    if !normalized.is_empty() {
        sqlx::query("INSERT OR IGNORE INTO tags (name) VALUES (?)")
            .bind(normalized)
            .execute(pool)
            .await?;
    }
    Ok(())
}

pub async fn add_tag_to_track(pool: &SqlitePool, track_id: &str, tag_name: &str) -> Result<()> {
    let tag_id: i64 = sqlx::query_scalar("SELECT id FROM tags WHERE name = ?")
        .bind(tag_name)
        .fetch_one(pool)
        .await?;

    sqlx::query("INSERT OR IGNORE INTO track_tags (spotify_id, tag_id) VALUES (?, ?)")
        .bind(track_id)
        .bind(tag_id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn remove_tag_from_track(
    pool: &SqlitePool,
    track_id: &str,
    tag_name: &str,
) -> Result<()> {
    let tag_id: i64 = sqlx::query_scalar("SELECT id FROM tags WHERE name = ?")
        .bind(tag_name)
        .fetch_one(pool)
        .await?;

    sqlx::query("DELETE FROM track_tags WHERE spotify_id = ? AND tag_id = ?")
        .bind(track_id)
        .bind(tag_id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn sync_tracks(
    pool: &sqlx::SqlitePool,
    tracks: Vec<(String, String, String, String)>,
) -> Result<()> {
    let mut tx = pool.begin().await?;

    for (spotify_id, title, artist_name, liked_at) in tracks {
        sqlx::query(
            "INSERT INTO tracks (spotify_id, title, artist_name, liked_at) 
             VALUES (?, ?, ?, ?) 
             ON CONFLICT(spotify_id) DO UPDATE SET liked_at = excluded.liked_at",
        )
        .bind(spotify_id)
        .bind(title)
        .bind(artist_name)
        .bind(liked_at)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

pub async fn fetch_tracks_paged(
    pool: &SqlitePool,
    filter_tag: Option<String>,
    page: i64,
    page_size: i64,
) -> Result<Vec<TrackDisplay>> {
    let offset = page * page_size;

    let mut query_str = String::from(
        r#"
        SELECT t.spotify_id, t.title, t.artist_name, GROUP_CONCAT(tg.name, ',') as tag_list
        FROM tracks t
        LEFT JOIN track_tags tt ON t.spotify_id = tt.spotify_id
        LEFT JOIN tags tg ON tt.tag_id = tg.id
        "#,
    );

    if filter_tag.is_some() {
        query_str.push_str(
            r#"
            WHERE EXISTS (
                SELECT 1 FROM track_tags tt2 
                JOIN tags tg2 ON tt2.tag_id = tg2.id 
                WHERE tt2.spotify_id = t.spotify_id AND tg2.name = ?
            )
            "#,
        );
    }

    query_str.push_str(" GROUP BY t.spotify_id ORDER BY t.liked_at DESC LIMIT ? OFFSET ?");

    let mut query = sqlx::query(&query_str);

    if let Some(tag) = filter_tag {
        query = query.bind(tag);
    }

    let rows = query.bind(page_size).bind(offset).fetch_all(pool).await?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let tag_list: Option<String> = row.try_get("tag_list").ok();
            let tags = tag_list
                .map(|s| {
                    s.split(',')
                        .filter(|t| !t.is_empty())
                        .map(|t| t.to_string())
                        .collect()
                })
                .unwrap_or_default();

            TrackDisplay {
                id: row.get("spotify_id"),
                title: row.get("title"),
                artist: row.get("artist_name"),
                tags,
            }
        })
        .collect())
}
