use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse, Response},
};
use serde::Serialize;
use std::sync::Arc;

use crate::AppState;

/// The challenge set every new map starts with — the grid's horizontal
/// axis, in order, with each column's value kind. Adding a challenge to
/// this list only affects maps created afterwards.
const DEFAULT_CHALLENGES: &[(&str, &str)] = &[
    ("t", "time"),
    ("dc", "int"),
    ("g", "int"),
    ("j", "int"),
    ("f", "int"),
    ("fr", "int"),
    ("dd", "int"),
    ("to", "int"),
    ("b", "int"),
    ("u", "int"),
    ("hs", "int"),
    ("ds", "int"),
    ("og", "int"),
    ("hg", "int"),
    ("dg", "int"),
    ("s", "int"),
    ("is", "int"),
    ("fz", "int"),
    ("if", "int"),
    ("di", "int"),
    ("h", "int"),
    ("1x", "bool"),
    ("2x", "bool"),
    ("3x", "bool"),
    ("4x", "bool"),
];

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

/// One challenge column header. What the grid template loops over for the
/// horizontal axis. `kind` says what the column's cells hold: "time"
/// (milliseconds), "int" (a count), or "bool" (0/1, shown as a checkbox).
#[derive(Serialize)]
pub struct GridChallenge {
    pub id: i32,
    pub name: String,
    pub kind: String,
}

/// One cell of a room's row, value already formatted — Tera has no
/// business knowing about milliseconds. None = empty cell. `kind` is
/// copied from the column so the template can pick the right widget.
#[derive(Serialize)]
pub struct GridCell {
    pub challenge_id: i32,
    pub kind: String,
    pub value: Option<String>,
}

/// One row of the grid: a room and its cell for every challenge column.
#[derive(Serialize)]
pub struct GridRow {
    pub room_id: i32,
    pub room_name: String,
    pub cells: Vec<GridCell>,
}

/// Milliseconds → "s.mmm" under a minute, "m:ss.mmm" beyond, "h:mm:ss.mmm"
/// past the hour. Cells are mostly short segment times, so the compact form
/// keeps the grid narrow.
fn format_time(ms: i64) -> String {
    let total_secs = ms / 1000;
    let millis = ms % 1000;
    let secs = total_secs % 60;
    let mins = (total_secs / 60) % 60;
    let hours = total_secs / 3600;
    if hours > 0 {
        format!("{hours}:{mins:02}:{secs:02}.{millis:03}")
    } else if mins > 0 {
        format!("{mins}:{secs:02}.{millis:03}")
    } else {
        format!("{secs}.{millis:03}")
    }
}

/// Renders one raw cell/total value according to its column's kind.
fn format_value(kind: &str, v: i64) -> String {
    match kind {
        "time" => format_time(v),
        _ => v.to_string(),
    }
}

/// "45.230" / "1:02,410" / "1:02:03.5" → milliseconds. Accepts a comma as
/// the decimal separator. None = not a time (caller keeps the old value).
fn parse_time(s: &str) -> Option<i64> {
    let s = s.trim().replace(',', ".");
    let mut parts = s.split(':').rev();
    let secs: f64 = parts.next()?.parse().ok()?;
    if !(0.0..60.0).contains(&secs) {
        return None;
    }
    let mut total = (secs * 1000.0).round() as i64;
    let mut unit_ms = 60_000i64;
    for part in parts {
        let v: i64 = part.parse().ok()?;
        if v < 0 {
            return None;
        }
        total += v * unit_ms;
        unit_ms *= 60;
    }
    Some(total)
}

/// Loads a map's whole grid and drops it into `ctx`: challenge columns,
/// one row per room, and the SoB (Sum of Best) footer — one total per
/// challenge column, summing whatever cells are filled in (a partial sum
/// until the whole column is).
async fn insert_grid_context(state: &AppState, map_id: i32, ctx: &mut tera::Context) {
    let challenges: Vec<(i32, String, String)> = sqlx::query_as(
        "SELECT id, name, kind FROM challenges WHERE map_id = $1 ORDER BY position, id",
    )
    .bind(map_id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let rooms: Vec<(i32, String)> =
        sqlx::query_as("SELECT id, name FROM rooms WHERE map_id = $1 ORDER BY position, id")
            .bind(map_id)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();

    let values: Vec<(i32, i32, i64)> = sqlx::query_as(
        "SELECT cv.room_id, cv.challenge_id, cv.value
         FROM challenge_values cv
         JOIN rooms r ON r.id = cv.room_id
         WHERE r.map_id = $1",
    )
    .bind(map_id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let by_cell: std::collections::HashMap<(i32, i32), i64> = values
        .into_iter()
        .map(|(room_id, challenge_id, v)| ((room_id, challenge_id), v))
        .collect();

    let rows: Vec<GridRow> = rooms
        .iter()
        .map(|(room_id, room_name)| GridRow {
            room_id: *room_id,
            room_name: room_name.clone(),
            cells: challenges
                .iter()
                .map(|(challenge_id, _, kind)| GridCell {
                    challenge_id: *challenge_id,
                    kind: kind.clone(),
                    value: by_cell
                        .get(&(*room_id, *challenge_id))
                        .map(|&v| format_value(kind, v)),
                })
                .collect(),
        })
        .collect();

    // Per-column total. For 't' that's the actual Sum of Best; for counts
    // it's the total across rooms; for pass/fail it's how many rooms pass.
    let sob: Vec<Option<String>> = challenges
        .iter()
        .map(|(challenge_id, _, kind)| {
            let mut any = false;
            let mut total = 0i64;
            for (room_id, _) in &rooms {
                if let Some(v) = by_cell.get(&(*room_id, *challenge_id)) {
                    any = true;
                    total += v;
                }
            }
            any.then(|| format_value(kind, total))
        })
        .collect();

    let challenges: Vec<GridChallenge> = challenges
        .into_iter()
        .map(|(id, name, kind)| GridChallenge { id, name, kind })
        .collect();

    ctx.insert("grid_challenges", &challenges);
    ctx.insert("grid_rows", &rows);
    ctx.insert("grid_sob", &sob);
}

const MAP_COLUMNS: &str = "id, name, campaign_id, nb_rooms, cleared, clearing_date";

/// Everything the sidebar tree needs, as a Tera context. Shared by the
/// full-page index and the rename handlers that re-render just the tree.
async fn sidebar_context(state: &AppState) -> tera::Context {
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
    ctx
}

/// GET / — explorer shell. Builds the sidebar tree from the DB:
/// each campaign becomes a folder containing its maps, plus a flat
/// "maps" folder for standalone ones.
pub async fn index(State(state): State<Arc<AppState>>) -> Html<String> {
    let ctx = sidebar_context(&state).await;
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
pub async fn map_detail(State(state): State<Arc<AppState>>, Path(id): Path<i32>) -> Html<String> {
    let query = format!("SELECT {MAP_COLUMNS} FROM maps WHERE id = $1");
    let map: MapRow = sqlx::query_as(&query)
        .bind(id)
        .fetch_one(&state.db)
        .await
        .unwrap();

    let mut ctx = tera::Context::new();
    ctx.insert("map", &map);
    insert_grid_context(&state, id, &mut ctx).await;
    let rendered = state.tera.render("partials/map_detail.html", &ctx).unwrap();
    Html(rendered)
}

#[derive(serde::Deserialize)]
pub struct CellForm {
    // Absent entirely for an unchecked checkbox, hence the Option.
    pub value: Option<String>,
}

/// What one edit should do to its `challenge_values` row.
enum CellAction {
    Set(i64),
    Clear,
    /// Unparseable input: leave the stored value alone; the re-rendered
    /// grid puts the old value back in the input.
    Keep,
}

/// POST /cell/:room_id/:challenge_id — save one grid cell, then re-render
/// the whole grid partial. Swapping the full grid (not just the cell) is
/// what keeps the SoB footer in sync for free; fine at this scale.
/// The input is interpreted per the column's kind: a time for "time", a
/// non-negative count for "int", checkbox presence for "bool". Emptying a
/// text cell (or unchecking a box) clears it.
pub async fn update_cell(
    State(state): State<Arc<AppState>>,
    Path((room_id, challenge_id)): Path<(i32, i32)>,
    axum::Form(form): axum::Form<CellForm>,
) -> Html<String> {
    let (map_id,): (i32,) = sqlx::query_as("SELECT map_id FROM rooms WHERE id = $1")
        .bind(room_id)
        .fetch_one(&state.db)
        .await
        .unwrap();

    let (kind,): (String,) = sqlx::query_as("SELECT kind FROM challenges WHERE id = $1")
        .bind(challenge_id)
        .fetch_one(&state.db)
        .await
        .unwrap();

    let action = if kind == "bool" {
        match form.value {
            Some(_) => CellAction::Set(1),
            None => CellAction::Clear,
        }
    } else {
        let raw = form.value.unwrap_or_default();
        let raw = raw.trim();
        if raw.is_empty() {
            CellAction::Clear
        } else {
            let parsed = match kind.as_str() {
                "time" => parse_time(raw),
                _ => raw.parse::<i64>().ok().filter(|v| *v >= 0),
            };
            match parsed {
                Some(v) => CellAction::Set(v),
                None => CellAction::Keep,
            }
        }
    };

    match action {
        CellAction::Set(v) => {
            sqlx::query(
                "INSERT INTO challenge_values (room_id, challenge_id, value)
                 VALUES ($1, $2, $3)
                 ON CONFLICT (room_id, challenge_id)
                 DO UPDATE SET value = EXCLUDED.value, updated_at = now()",
            )
            .bind(room_id)
            .bind(challenge_id)
            .bind(v)
            .execute(&state.db)
            .await
            .ok();
        }
        CellAction::Clear => {
            sqlx::query("DELETE FROM challenge_values WHERE room_id = $1 AND challenge_id = $2")
                .bind(room_id)
                .bind(challenge_id)
                .execute(&state.db)
                .await
                .ok();
        }
        CellAction::Keep => {}
    }

    let mut ctx = tera::Context::new();
    insert_grid_context(&state, map_id, &mut ctx).await;
    let rendered = state
        .tera
        .render("partials/challenge_grid.html", &ctx)
        .unwrap();
    Html(rendered)
}

#[derive(serde::Deserialize)]
pub struct RoomRenameForm {
    pub name: String,
}

/// POST /room/:id — rename a room from its grid header cell, then
/// re-render the grid. A blanked-out name is ignored: the old one comes
/// back with the swap.
pub async fn rename_room(
    State(state): State<Arc<AppState>>,
    Path(room_id): Path<i32>,
    axum::Form(form): axum::Form<RoomRenameForm>,
) -> Html<String> {
    let (map_id,): (i32,) = sqlx::query_as("SELECT map_id FROM rooms WHERE id = $1")
        .bind(room_id)
        .fetch_one(&state.db)
        .await
        .unwrap();

    let name = form.name.trim();
    if !name.is_empty() {
        sqlx::query("UPDATE rooms SET name = $1 WHERE id = $2")
            .bind(name)
            .bind(room_id)
            .execute(&state.db)
            .await
            .ok();
    }

    let mut ctx = tera::Context::new();
    insert_grid_context(&state, map_id, &mut ctx).await;
    Html(
        state
            .tera
            .render("partials/challenge_grid.html", &ctx)
            .unwrap(),
    )
}

#[derive(serde::Deserialize)]
pub struct ChallengeAddForm {
    pub name: String,
    pub kind: String,
}

/// POST /map/:id/challenges — append a challenge column to an existing
/// map's grid (at the end of the axis), then re-render the grid.
pub async fn add_challenge(
    State(state): State<Arc<AppState>>,
    Path(map_id): Path<i32>,
    axum::Form(form): axum::Form<ChallengeAddForm>,
) -> Html<String> {
    let name = form.name.trim();
    let kind = match form.kind.as_str() {
        "time" | "bool" => form.kind.as_str(),
        _ => "int",
    };
    if !name.is_empty() {
        sqlx::query(
            "INSERT INTO challenges (map_id, name, position, kind)
             VALUES ($1, $2,
                     (SELECT COALESCE(MAX(position), 0) + 1 FROM challenges WHERE map_id = $1),
                     $3)",
        )
        .bind(map_id)
        .bind(name)
        .bind(kind)
        .execute(&state.db)
        .await
        .ok();
    }

    let mut ctx = tera::Context::new();
    insert_grid_context(&state, map_id, &mut ctx).await;
    Html(
        state
            .tera
            .render("partials/challenge_grid.html", &ctx)
            .unwrap(),
    )
}

#[derive(serde::Deserialize)]
pub struct ChallengeRenameForm {
    pub name: String,
}

/// POST /challenge/:id — rename a challenge from its column header, then
/// re-render the grid. Same contract as room renaming: a blanked-out name
/// is ignored and the old one comes back with the swap.
pub async fn rename_challenge(
    State(state): State<Arc<AppState>>,
    Path(challenge_id): Path<i32>,
    axum::Form(form): axum::Form<ChallengeRenameForm>,
) -> Html<String> {
    let (map_id,): (i32,) = sqlx::query_as("SELECT map_id FROM challenges WHERE id = $1")
        .bind(challenge_id)
        .fetch_one(&state.db)
        .await
        .unwrap();

    let name = form.name.trim();
    if !name.is_empty() {
        sqlx::query("UPDATE challenges SET name = $1 WHERE id = $2")
            .bind(name)
            .bind(challenge_id)
            .execute(&state.db)
            .await
            .ok();
    }

    let mut ctx = tera::Context::new();
    insert_grid_context(&state, map_id, &mut ctx).await;
    Html(
        state
            .tera
            .render("partials/challenge_grid.html", &ctx)
            .unwrap(),
    )
}

/// DELETE /challenge/:id — drop a challenge column; its cell values go
/// with it (FK cascade). Re-renders the grid.
pub async fn delete_challenge(
    State(state): State<Arc<AppState>>,
    Path(challenge_id): Path<i32>,
) -> Html<String> {
    let (map_id,): (i32,) = sqlx::query_as("SELECT map_id FROM challenges WHERE id = $1")
        .bind(challenge_id)
        .fetch_one(&state.db)
        .await
        .unwrap();

    sqlx::query("DELETE FROM challenges WHERE id = $1")
        .bind(challenge_id)
        .execute(&state.db)
        .await
        .ok();

    let mut ctx = tera::Context::new();
    insert_grid_context(&state, map_id, &mut ctx).await;
    Html(
        state
            .tera
            .render("partials/challenge_grid.html", &ctx)
            .unwrap(),
    )
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

    let query = format!("SELECT {MAP_COLUMNS} FROM maps WHERE id = $1");
    let map: MapRow = sqlx::query_as(&query)
        .bind(id)
        .fetch_one(&state.db)
        .await
        .unwrap();
    let mut ctx = tera::Context::new();
    ctx.insert("map", &map);
    insert_grid_context(&state, id, &mut ctx).await;
    let detail = state.tera.render("partials/map_detail.html", &ctx).unwrap();

    let mut side_ctx = sidebar_context(&state).await;
    side_ctx.insert("oob", &true);
    let sidebar = state.tera.render("partials/sidebar.html", &side_ctx).unwrap();

    Html(format!("{detail}\n{sidebar}"))
}

/// GET /campaign/:id/rename — swaps the sidebar's "rename campaign" link
/// for an inline input pre-filled with the current name.
pub async fn campaign_rename_form(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> Html<String> {
    let (name,): (String,) = sqlx::query_as("SELECT name FROM campaigns WHERE id = $1")
        .bind(id)
        .fetch_one(&state.db)
        .await
        .unwrap();

    let mut ctx = tera::Context::new();
    ctx.insert("id", &id);
    ctx.insert("name", &name);
    Html(
        state
            .tera
            .render("partials/campaign_rename.html", &ctx)
            .unwrap(),
    )
}

#[derive(serde::Deserialize)]
pub struct CampaignRenameForm {
    pub name: String,
}

/// POST /campaign/:id/rename — save the new name and re-render the whole
/// sidebar tree (the content pane is left untouched). Blank names are
/// ignored, which also serves as the "cancel" path.
pub async fn rename_campaign(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
    axum::Form(form): axum::Form<CampaignRenameForm>,
) -> Html<String> {
    let name = form.name.trim();
    if !name.is_empty() {
        sqlx::query("UPDATE campaigns SET name = $1 WHERE id = $2")
            .bind(name)
            .bind(id)
            .execute(&state.db)
            .await
            .ok();
    }

    let ctx = sidebar_context(&state).await;
    Html(state.tera.render("partials/sidebar.html", &ctx).unwrap())
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

/// DELETE /campaign/:id — delete a campaign. Its maps are NOT deleted:
/// their campaign_id is SET NULL by the FK, so they become standalone.
pub async fn delete_campaign(State(state): State<Arc<AppState>>, Path(id): Path<i32>) -> Response {
    sqlx::query("DELETE FROM campaigns WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await
        .ok();
    ([("HX-Redirect", "/")], "").into_response()
}

/// GET /campaigns/new — the campaign creation form, loaded into #content.
pub async fn campaign_form(State(state): State<Arc<AppState>>) -> Html<String> {
    let ctx = tera::Context::new();
    Html(
        state
            .tera
            .render("partials/campaign_form.html", &ctx)
            .unwrap(),
    )
}

#[derive(serde::Deserialize)]
pub struct CampaignForm {
    pub name: String,
    pub link: Option<String>,
    // Checkbox: present when checked, absent otherwise.
    pub is_collab: Option<String>,
}

/// POST /campaigns — create a campaign, then HX-Redirect to / so the
/// sidebar tree is rebuilt with the new entry.
pub async fn create_campaign(
    State(state): State<Arc<AppState>>,
    axum::Form(form): axum::Form<CampaignForm>,
) -> Response {
    let name = form.name.trim();
    if name.is_empty() {
        let mut ctx = tera::Context::new();
        ctx.insert("error", "name is required");
        return Html(
            state
                .tera
                .render("partials/campaign_form.html", &ctx)
                .unwrap(),
        )
        .into_response();
    }

    let link = form
        .link
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    sqlx::query("INSERT INTO campaigns (name, link, is_collab) VALUES ($1, $2, $3)")
        .bind(name)
        .bind(link)
        .bind(form.is_collab.is_some())
        .execute(&state.db)
        .await
        .unwrap();

    ([("HX-Redirect", "/")], "").into_response()
}

/// GET /maps/new — the map creation form (needs the campaign list for the
/// optional "belongs to" select), loaded into #content.
pub async fn map_form(State(state): State<Arc<AppState>>) -> Html<String> {
    let campaigns: Vec<Campaign> =
        sqlx::query_as("SELECT id, name, link, is_collab FROM campaigns ORDER BY name")
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();

    let mut ctx = tera::Context::new();
    ctx.insert("campaigns", &campaigns);
    Html(state.tera.render("partials/map_form.html", &ctx).unwrap())
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
        let campaigns: Vec<Campaign> =
            sqlx::query_as("SELECT id, name, link, is_collab FROM campaigns ORDER BY name")
                .fetch_all(&state.db)
                .await
                .unwrap_or_default();
        let mut ctx = tera::Context::new();
        ctx.insert("campaigns", &campaigns);
        ctx.insert("error", error);
        return Html(state.tera.render("partials/map_form.html", &ctx).unwrap()).into_response();
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
        .bind(kind)
        .execute(&state.db)
        .await
        .unwrap();
    }

    ([("HX-Redirect", "/")], "").into_response()
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
