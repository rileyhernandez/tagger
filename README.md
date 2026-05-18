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
│   └── index.html      # Main UI template
└── GEMINI.md           # Project-specific development instructions
```

## Features & Progress

- [x] **Spotify Sync**: Fetch all liked tracks from Spotify and store them locally.
- [x] **Track Listing**: Paged view of all tracks with their associated tags.
- [x] **Custom Tagging**: Create global tags and link them to individual tracks.
- [x] **Search & Filter**: Search tracks by title or filter by specific tags.
- [ ] **HTMX Refinement**: Replace full-page redirects with partial DOM updates for better UX.
- [ ] **Batch Actions**: Select multiple tracks to add to queue or create a new Spotify playlist.
- [ ] **Advanced Filtering**: Combine multiple tags and search queries.

## Development Insights & Future Improvements

### Technical Debt & Idiomaticity
1. **HTMX Usage**: Currently, most handlers use `Redirect::to("/")`. To truly leverage HTMX, these should return partial HTML fragments (e.g., a single track row or an updated tag list) to avoid full page reloads.
2. **SQL Efficiency**: Tag filtering currently uses `HAVING tag_list LIKE ?`. While functional, this is brittle and inefficient. Moving to a proper `JOIN` with a `WHERE` clause on `tag_id` is recommended.
3. **Error Handling**: Several `.unwrap()` calls and ignored results (`let _ = ...`) exist in `main.rs` and `db.rs`. These should be replaced with proper error propagation and user-facing error messages.
4. **Spotify Sync**: The current sync fetches the entire liked tracks list. Implementing incremental sync (fetching only tracks added since `last_sync`) would significantly improve performance for large libraries.
5. **Data Modeling**: `TrackDisplay` stores tags as a comma-separated string. Converting this to a `Vec<String>` would allow for better manipulation and cleaner template logic.

### Roadmap Insights
- **Filtering**: The long-term goal of checkbox-based selection and batching will require a more robust state management on the frontend, likely using HTMX's `hx-vals` or a small amount of Alpine.js/Vanilla JS to track selections across pages.
- **Playlist/Queue Integration**: Adding songs to a Spotify playlist or queue will require additional scopes (`playlist-modify-public`, `user-modify-playback-state`) and new endpoints in `spotify.rs`.

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
