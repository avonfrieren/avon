//! Map CRUD: detail pane, creation (with its empty grid generated),
//! rename from the detail-pane title, and deletion (FK cascades take the
//! rooms, challenges and cell values along).

use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse, Response},
};
use axum_extra::extract::cookie::CookieJar;
use std::sync::Arc;

use super::grid::{insert_grid_context, Kind};
use super::sidebar::{all_campaigns, sidebar_context, MapRow, MAP_COLUMNS};
use super::render;
use crate::AppState;

/// The challenge set every new map starts with — the grid's horizontal
/// axis, in order, with each column's value kind. Adding a challenge to
/// this list only affects maps created afterwards.
const DEFAULT_CHALLENGES: &[(&str, Kind)] = &[
    ("t", Kind::Time),
    ("dc", Kind::Int),
    ("g", Kind::Int),
    ("j", Kind::Int),
    ("f", Kind::Int),
    ("fr", Kind::Int),
    ("dd", Kind::Int),
    ("to", Kind::Int),
    ("b", Kind::Int),
    ("u", Kind::Int),
    ("hs", Kind::Int),
    ("ds", Kind::Int),
    ("og", Kind::Int),
    ("hg", Kind::Int),
    ("dg", Kind::Int),
    ("s", Kind::Int),
    ("is", Kind::Int),
    ("fz", Kind::Int),
    ("if", Kind::Int),
    ("di", Kind::Int),
    ("h", Kind::Int),
    ("1x", Kind::Bool),
    ("2x", Kind::Bool),
    ("3x", Kind::Bool),
    ("4x", Kind::Bool),
];

/// Fetches one map by id.
async fn map_by_id(state: &AppState, id: i32) -> MapRow {
    let query = format!("SELECT {MAP_COLUMNS} FROM maps WHERE id = $1");
    sqlx::query_as(&query)
        .bind(id)
        .fetch_one(&state.db)
        .await
        .unwrap()
}

/// GET /map/:id — content pane for a single map, whether it's standalone
/// or belongs to a campaign. Read-only unless the visitor is logged in.
pub async fn map_detail(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
    jar: CookieJar,
) -> Html<String> {
    let map = map_by_id(&state, id).await;

    let mut ctx = tera::Context::new();
    ctx.insert("map", &map);
    ctx.insert("is_admin", &super::auth::is_admin(&state, &jar).await);
    insert_grid_context(&state, id, &mut ctx).await;
    render(&state, "partials/map_detail.html", &ctx)
}

/// GET /maps/new — the map creation form (needs the campaign list for the
/// optional "belongs to" select), loaded into #content.
pub async fn map_form(State(state): State<Arc<AppState>>) -> Html<String> {
    let mut ctx = tera::Context::new();
    ctx.insert("campaigns", &all_campaigns(&state).await);
    render(&state, "partials/map_form.html", &ctx)
}

#[derive(serde::Deserialize)]
pub struct MapForm {
    pub name: String,
    // Kept as text so a bad value re-renders the form instead of a 422.
    pub nb_rooms: String,
    // "" = standalone.
    pub campaign_id: Option<String>,
    pub cleared: Option<String>,
    pub clearing_date: Option<String>,
}

/// POST /maps — create a map plus its empty grid: `nb_rooms` generated
/// rooms and the full DEFAULT_CHALLENGES column set, no values yet. Then
/// HX-Redirect to / so the new map shows up in the sidebar.
pub async fn create_map(
    State(state): State<Arc<AppState>>,
    axum::Form(form): axum::Form<MapForm>,
) -> Response {
    let name = form.name.trim();
    let nb_rooms: Option<i32> = form
        .nb_rooms
        .trim()
        .parse()
        .ok()
        .filter(|n| (1..=999).contains(n));

    let error = if name.is_empty() {
        Some("name is required")
    } else if nb_rooms.is_none() {
        Some("rooms must be a number between 1 and 999")
    } else {
        None
    };
    if let Some(error) = error {
        let mut ctx = tera::Context::new();
        ctx.insert("campaigns", &all_campaigns(&state).await);
        ctx.insert("error", error);
        return render(&state, "partials/map_form.html", &ctx).into_response();
    }
    let nb_rooms = nb_rooms.unwrap();

    let campaign_id: Option<i32> = form
        .campaign_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .and_then(|s| s.parse().ok());

    let clearing_date = form
        .clearing_date
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .map(|d| {
            chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
                d.and_hms_opt(0, 0, 0).unwrap(),
                chrono::Utc,
            )
        });

    let (map_id,): (i32,) = sqlx::query_as(
        "INSERT INTO maps (name, campaign_id, nb_rooms, cleared, clearing_date)
         VALUES ($1, $2, $3, $4, $5) RETURNING id",
    )
    .bind(name)
    .bind(campaign_id)
    .bind(nb_rooms)
    .bind(form.cleared.is_some())
    .bind(clearing_date)
    .fetch_one(&state.db)
    .await
    .unwrap();

    // Room names are just numbered placeholders (r01, r02, ...), padded to
    // the width of the room count so they sort the same everywhere.
    sqlx::query(
        "INSERT INTO rooms (map_id, name, position)
         SELECT $1, 'r' || lpad(i::text, greatest(2, char_length($2::text)), '0'), i
         FROM generate_series(1, $2) AS i",
    )
    .bind(map_id)
    .bind(nb_rooms)
    .execute(&state.db)
    .await
    .unwrap();

    for (position, (name, kind)) in DEFAULT_CHALLENGES.iter().enumerate() {
        sqlx::query(
            "INSERT INTO challenges (map_id, name, position, kind) VALUES ($1, $2, $3, $4)",
        )
        .bind(map_id)
        .bind(name)
        .bind(position as i32 + 1)
        .bind(kind.to_string())
        .execute(&state.db)
        .await
        .unwrap();
    }

    ([("HX-Redirect", "/")], "").into_response()
}

#[derive(serde::Deserialize)]
pub struct MapRenameForm {
    pub name: String,
}

/// POST /map/:id/rename — rename a map from its detail-pane title, then
/// return the refreshed detail pane plus an out-of-band sidebar swap so
/// the tree shows the new name without a reload. Blank names are ignored.
pub async fn rename_map(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
    axum::Form(form): axum::Form<MapRenameForm>,
) -> Html<String> {
    let name = form.name.trim();
    if !name.is_empty() {
        sqlx::query("UPDATE maps SET name = $1 WHERE id = $2")
            .bind(name)
            .bind(id)
            .execute(&state.db)
            .await
            .ok();
    }

    // Only a logged-in session reaches this handler (write middleware).
    let map = map_by_id(&state, id).await;
    let mut ctx = tera::Context::new();
    ctx.insert("map", &map);
    ctx.insert("is_admin", &true);
    insert_grid_context(&state, id, &mut ctx).await;
    let detail = render(&state, "partials/map_detail.html", &ctx);

    let mut side_ctx = sidebar_context(&state, true).await;
    side_ctx.insert("oob", &true);
    let sidebar = render(&state, "partials/sidebar.html", &side_ctx);

    Html(format!("{}\n{}", detail.0, sidebar.0))
}

/// DELETE /map/:id — delete a map and, via FK cascades, its rooms,
/// challenges and cell values. HX-Redirect rebuilds the sidebar.
pub async fn delete_map(State(state): State<Arc<AppState>>, Path(id): Path<i32>) -> Response {
    sqlx::query("DELETE FROM maps WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await
        .ok();
    ([("HX-Redirect", "/")], "").into_response()
}
