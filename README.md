# Spotify Track Tagger

A lightweight Rust-based web application for managing and tagging Spotify tracks. This tool allows you to organize your music library with custom tags, filter by tags, and maintain a local metadata database.

## System Design

The application follows a classic Server-Side Rendering (SSR) architecture:

- **Web Framework**: [Axum](https://github.com/tokio-rs/axum) handles the HTTP routing and request processing.
- **Templating**: [Askama](https://github.com/djc/askama) provides type-safe HTML templates.
- **Frontend Enhancements**: [HTMX](https://htmx.org/) is used for seamless, partial page updates (in progress), and [Tailwind CSS](https://tailwindcss.com/) for modern, responsive styling.
- **Database**: [SQLite](https://www.sqlite.org/) is used for persistent storage, managed via [SQLx](https://github.com/launchbadge/sqlx) for asynchronous, type-checked queries.
- **Runtime**: Powered by [Tokio](https://tokio.rs/), a multi-threaded async runtime.

### Data Model

The system uses a relational schema to manage tracks and their associations:

- **Tracks**: Stores basic metadata (`spotify_id`, `title`, `artist_name`, `liked_at`).
- **Tags**: A unique collection of user-defined labels.
- **TrackTags**: A junction table enabling a many-to-many relationship between tracks and tags.
- **KV Store**: A simple key-value table for persistent settings like `last_sync` time.

## Project Structure

```text
.
├── Cargo.toml          # Rust dependencies and configuration
├── justfile            # Task runner for common commands
├── music.db            # SQLite database file
├── src/
│   ├── main.rs         # Application entry point and router setup
│   ├── db.rs           # Database initialization and CRUD operations
│   ├── models.rs       # Data structures and Askama templates
│   └── spotify.rs      # Spotify API integration
├── templates/
│   ├── index.html      # Main UI template
│   └── playback_fragment.html # HTMX fragment for playback controls
└── GEMINI.md           # Project-specific development instructions
```

## Features & Progress

- [x] **Spotify Sync**: Incremental sync fetches only new liked tracks from Spotify.
- [x] **Track Listing**: Paged view (50 tracks/page) of all tracks with their associated tags.
- [x] **Custom Tagging**: Create global tags and link/unlink them to individual tracks.
- [x] **Search & Filter**: Search tracks by title or filter by specific tags using efficient SQL queries.
- [x] **Playback Controls**: Real-time "Now Playing" view with Play/Pause, Next, and Previous controls (HTMX powered).
- [x] **Batch Actions**: Select multiple tracks to add them to the Spotify playback queue.
- [ ] **Advanced Filtering**: Combine multiple tags and search queries.

## Development Insights & Future Improvements

### Technical Debt & Idiomaticity
1. **HTMX Usage**: Playback and track selection now leverage HTMX for partial DOM updates. However, tag management still uses full-page redirects and could be further refined.
2. **SQL Efficiency**: Tag filtering has been optimized using `WHERE EXISTS` instead of `LIKE` patterns.
3. **Error Handling**: Logging has been improved, but several `.unwrap()` calls still exist in `main.rs` that should be replaced with proper error propagation.
4. **Data Modeling**: `TrackDisplay` now uses `Vec<String>` for tags, improving template logic and data manipulation.

### Roadmap Insights
- **Filtering**: The long-term goal of combined tag filtering (e.g., songs with both "Rock" and "Lofi") will require a more robust query builder and UI refinements.
- **Playlist Integration**: Beyond just adding to the playback queue, future updates will support creating and updating Spotify playlists directly from selected tracks.

## Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (2024 edition)
- [Just](https://github.com/casey/just) task runner
- [Distrobox](https://distrobox.it/) (optional, used in the default development environment)

### Development Workflow

The `justfile` provides wrappers for common tasks:

- **Build**: `just build`
- **Run**: `just run` (starts server at `http://localhost:8080`)
- **Test**: `just test`
- **Lint**: `just clippy`
- **Format**: `just fmt`
# tagger
