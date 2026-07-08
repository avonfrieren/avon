use axum::{
    extract::{Path, State},
    response::Html,
};
use serde::Serialize;
use std::sync::Arc;

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

const MAP_COLUMNS: &str = "id, name, campaign_id, nb_rooms, cleared, clearing_date";

/// GET / — explorer shell. Builds the sidebar tree from the DB:
/// each campaign becomes a folder containing its maps, plus a flat
/// "maps" folder for standalone ones.
pub async fn index(State(state): State<Arc<AppState>>) -> Html<String> {
    let campaigns: Vec<Campaign> =
        sqlx::query_as("SELECT id, name, link, is_collab FROM campaigns ORDER BY name")
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();

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

    let mut ctx = tera::Context::new();
    ctx.insert("campaigns", &campaigns_with_maps);
    ctx.insert("standalone_maps", &standalone_maps);
    ctx.insert("images", &images);
    let rendered = state.tera.render("explorer.html", &ctx).unwrap();
    Html(rendered)
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

/// GET /map/:id — content pane for a single map, whether it's standalone
/// or belongs to a campaign.
pub async fn map_detail(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> Html<String> {
    let query = format!("SELECT {MAP_COLUMNS} FROM maps WHERE id = $1");
    let map: MapRow = sqlx::query_as(&query)
        .bind(id)
        .fetch_one(&state.db)
        .await
        .unwrap();

    let mut ctx = tera::Context::new();
    ctx.insert("map", &map);
    let rendered = state.tera.render("partials/map_detail.html", &ctx).unwrap();
    Html(rendered)
}

/// GET /imgs/:filename — unchanged, still just points at a static file.
/// Not DB-driven yet.
pub async fn image_view(
    State(state): State<Arc<AppState>>,
    Path(filename): Path<String>,
) -> Html<String> {
    let mut ctx = tera::Context::new();
    ctx.insert("filename", &filename);
    let rendered = state.tera.render("partials/image_view.html", &ctx).unwrap();
    Html(rendered)
}
