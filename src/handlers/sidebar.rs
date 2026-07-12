//! The explorer shell: the `/` page and the sidebar tree it is built
//! around, plus the disk-scanned imgs folder. The tree-building context
//! is shared with the handlers that re-render the tree after a change.

use axum::{
    extract::{Path, State},
    response::Html,
};
use axum_extra::extract::cookie::CookieJar;
use serde::Serialize;
use std::sync::Arc;

use super::render;
use crate::AppState;

#[derive(sqlx::FromRow, Serialize)]
pub struct Campaign {
    pub id: i32,
    pub name: String,
    pub link: Option<String>,
    pub is_collab: bool,
}

#[derive(sqlx::FromRow, Serialize)]
pub struct MapRow {
    pub id: i32,
    pub name: String,
    // NULL = standalone map. No separate is_standalone flag — this column
    // alone is the source of truth, so there's nothing to keep in sync.
    pub campaign_id: Option<i32>,
    pub nb_rooms: Option<i32>,
    pub cleared: bool,
    pub clearing_date: Option<chrono::DateTime<chrono::Utc>>,
}

/// A campaign bundled with the maps that belong to it — the shape the
/// sidebar template actually wants (a campaign "folder" containing map
/// "files").
#[derive(Serialize)]
pub struct CampaignWithMaps {
    pub id: i32,
    pub name: String,
    pub maps: Vec<MapRow>,
}

pub(crate) const MAP_COLUMNS: &str = "id, name, campaign_id, nb_rooms, cleared, clearing_date";

/// Fetches every campaign, ordered by name.
pub(crate) async fn all_campaigns(state: &AppState) -> Vec<Campaign> {
    sqlx::query_as("SELECT id, name, link, is_collab FROM campaigns ORDER BY name")
        .fetch_all(&state.db)
        .await
        .unwrap_or_default()
}

/// Everything the sidebar tree needs, as a Tera context. Shared by the
/// full-page index and the handlers that re-render just the tree.
/// `is_admin` decides whether the tree renders its edit controls.
pub(crate) async fn sidebar_context(state: &AppState, is_admin: bool) -> tera::Context {
    let campaigns = all_campaigns(state).await;

    // One query per campaign to fetch its maps. Fine at hobby-project scale
    // (a handful of campaigns); if this ever grows large, switch to one
    // query for all maps and group them in Rust instead.
    let mut campaigns_with_maps = Vec::with_capacity(campaigns.len());
    for c in campaigns {
        let query = format!("SELECT {MAP_COLUMNS} FROM maps WHERE campaign_id = $1 ORDER BY name");
        let maps: Vec<MapRow> = sqlx::query_as(&query)
            .bind(c.id)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();
        campaigns_with_maps.push(CampaignWithMaps {
            id: c.id,
            name: c.name,
            maps,
        });
    }

    let standalone_query =
        format!("SELECT {MAP_COLUMNS} FROM maps WHERE campaign_id IS NULL ORDER BY name");
    let standalone_maps: Vec<MapRow> = sqlx::query_as(&standalone_query)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

    let images = list_images().await;
    let docs = super::docs::list_docs().await;

    let mut ctx = tera::Context::new();
    ctx.insert("campaigns", &campaigns_with_maps);
    ctx.insert("standalone_maps", &standalone_maps);
    ctx.insert("images", &images);
    ctx.insert("docs", &docs);
    ctx.insert("is_admin", &is_admin);
    ctx
}

/// GET / — explorer shell. Builds the sidebar tree from the DB:
/// each campaign becomes a folder containing its maps, plus a flat
/// "maps" folder for standalone ones.
pub async fn index(State(state): State<Arc<AppState>>, jar: CookieJar) -> Html<String> {
    let is_admin = super::auth::is_admin(&state, &jar).await;
    let mut ctx = sidebar_context(&state, is_admin).await;
    // No pane selected — the template falls back to its hint.
    ctx.insert("content", "");
    render(&state, "explorer.html", &ctx)
}

/// Reads static/imgs/ and returns sorted filenames. Not DB-driven — this
/// folder is just scanned directly, so dropping a file in there is enough
/// to make it show up in the sidebar.
async fn list_images() -> Vec<String> {
    let mut images = Vec::new();
    if let Ok(mut entries) = tokio::fs::read_dir("static/imgs").await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            if let Some(name) = entry.file_name().to_str() {
                images.push(name.to_string());
            }
        }
    }
    images.sort();
    images
}

/// GET /imgs/:filename — still just points at a static file (not
/// DB-driven yet). htmx gets the bare pane; a direct load gets the
/// whole explorer page.
pub async fn image_view(
    State(state): State<Arc<AppState>>,
    Path(filename): Path<String>,
    jar: CookieJar,
    headers: axum::http::HeaderMap,
) -> Html<String> {
    let mut ctx = tera::Context::new();
    ctx.insert("filename", &filename);
    let view = render(&state, "partials/image_view.html", &ctx);

    if super::is_htmx(&headers) {
        view
    } else {
        let is_admin = super::auth::is_admin(&state, &jar).await;
        super::full_page(&state, is_admin, &view.0).await
    }
}
