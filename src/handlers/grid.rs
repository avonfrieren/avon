//! A map's challenge grid: rooms down the side, challenges across the
//! top, one value per cell, and the SoB (Sum of Best) footer. Everything
//! that reads or edits the grid lives here.

use axum::{
    extract::{Path, State},
    response::Html,
};
use serde::Serialize;
use std::str::FromStr;
use std::sync::Arc;

use super::render;
use crate::AppState;

/// What a challenge column's cells hold. Stored as lowercase text in the
/// database (CHECK-constrained); parsed once at the edges so the rest of
/// the code matches on the enum instead of comparing strings — a typo'd
/// kind can't silently fall through anymore.
#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// Milliseconds, formatted/parsed as m:ss.mmm.
    Time,
    /// A plain count (deaths, dashes, ...).
    Int,
    /// 0/1 pass/fail, rendered as a checkbox.
    Bool,
}

impl FromStr for Kind {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "time" => Ok(Kind::Time),
            "int" => Ok(Kind::Int),
            "bool" => Ok(Kind::Bool),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Kind::Time => "time",
            Kind::Int => "int",
            Kind::Bool => "bool",
        })
    }
}

/// Celeste advances its timer by exactly 17 ms per frame, so every
/// legitimate in-game time is a multiple of this. A stored time that
/// isn't one is almost certainly a typo — flagged in the UI, not
/// rejected, so it can be saved now and fixed later.
const FRAME_MS: i64 = 17;

/// One challenge column header. What the grid template loops over for
/// the horizontal axis.
#[derive(Serialize)]
pub struct GridChallenge {
    pub id: i32,
    pub name: String,
    pub kind: Kind,
}

/// One cell of a room's row, value already formatted — Tera has no
/// business knowing about milliseconds. None = empty cell. `kind` is
/// copied from the column so the template can pick the right widget.
#[derive(Serialize)]
pub struct GridCell {
    pub challenge_id: i32,
    pub kind: Kind,
    pub value: Option<String>,
    /// Time cells only: the stored value isn't a whole number of frames.
    pub off_frame: bool,
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
fn format_value(kind: Kind, v: i64) -> String {
    match kind {
        Kind::Time => format_time(v),
        Kind::Int | Kind::Bool => v.to_string(),
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
/// one row per room, and the SoB footer — one total per challenge
/// column, summing whatever cells are filled in (a partial sum until the
/// whole column is).
pub(crate) async fn insert_grid_context(state: &AppState, map_id: i32, ctx: &mut tera::Context) {
    let challenges: Vec<(i32, String, String)> = sqlx::query_as(
        "SELECT id, name, kind FROM challenges WHERE map_id = $1 ORDER BY position, id",
    )
    .bind(map_id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();
    let challenges: Vec<(i32, String, Kind)> = challenges
        .into_iter()
        .map(|(id, name, kind)| (id, name, kind.parse().unwrap_or(Kind::Int)))
        .collect();

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
                .map(|&(challenge_id, _, kind)| {
                    let raw = by_cell.get(&(*room_id, challenge_id)).copied();
                    GridCell {
                        challenge_id,
                        kind,
                        value: raw.map(|v| format_value(kind, v)),
                        off_frame: kind == Kind::Time
                            && raw.is_some_and(|v| v % FRAME_MS != 0),
                    }
                })
                .collect(),
        })
        .collect();

    // Per-column total. For a time column that's the actual Sum of Best;
    // for counts it's the total across rooms; for pass/fail it's how many
    // rooms pass.
    let sob: Vec<Option<String>> = challenges
        .iter()
        .map(|&(challenge_id, _, kind)| {
            let mut any = false;
            let mut total = 0i64;
            for (room_id, _) in &rooms {
                if let Some(v) = by_cell.get(&(*room_id, challenge_id)) {
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

/// The re-rendered grid partial — what every grid-editing handler
/// answers with, so the SoB footer always reflects the change.
async fn grid_partial(state: &AppState, map_id: i32) -> Html<String> {
    let mut ctx = tera::Context::new();
    // Only a logged-in session reaches the grid-editing handlers (write
    // middleware), so the re-rendered grid is always the editable one.
    ctx.insert("is_admin", &true);
    insert_grid_context(state, map_id, &mut ctx).await;
    render(state, "partials/challenge_grid.html", &ctx)
}

/// The map a room belongs to.
async fn map_of_room(state: &AppState, room_id: i32) -> i32 {
    let (map_id,): (i32,) = sqlx::query_as("SELECT map_id FROM rooms WHERE id = $1")
        .bind(room_id)
        .fetch_one(&state.db)
        .await
        .unwrap();
    map_id
}

/// The map a challenge belongs to.
async fn map_of_challenge(state: &AppState, challenge_id: i32) -> i32 {
    let (map_id,): (i32,) = sqlx::query_as("SELECT map_id FROM challenges WHERE id = $1")
        .bind(challenge_id)
        .fetch_one(&state.db)
        .await
        .unwrap();
    map_id
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
/// The input is interpreted per the column's kind. Emptying a text cell
/// (or unchecking a box) clears it.
pub async fn update_cell(
    State(state): State<Arc<AppState>>,
    Path((room_id, challenge_id)): Path<(i32, i32)>,
    axum::Form(form): axum::Form<CellForm>,
) -> Html<String> {
    let map_id = map_of_room(&state, room_id).await;

    let (kind,): (String,) = sqlx::query_as("SELECT kind FROM challenges WHERE id = $1")
        .bind(challenge_id)
        .fetch_one(&state.db)
        .await
        .unwrap();
    let kind = kind.parse().unwrap_or(Kind::Int);

    let action = match kind {
        Kind::Bool => match form.value {
            Some(_) => CellAction::Set(1),
            None => CellAction::Clear,
        },
        Kind::Time | Kind::Int => {
            let raw = form.value.unwrap_or_default();
            let raw = raw.trim();
            if raw.is_empty() {
                CellAction::Clear
            } else {
                let parsed = match kind {
                    Kind::Time => parse_time(raw),
                    _ => raw.parse::<i64>().ok().filter(|v| *v >= 0),
                };
                match parsed {
                    Some(v) => CellAction::Set(v),
                    None => CellAction::Keep,
                }
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

    grid_partial(&state, map_id).await
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
    let map_id = map_of_room(&state, room_id).await;

    let name = form.name.trim();
    if !name.is_empty() {
        sqlx::query("UPDATE rooms SET name = $1 WHERE id = $2")
            .bind(name)
            .bind(room_id)
            .execute(&state.db)
            .await
            .ok();
    }

    grid_partial(&state, map_id).await
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
    let kind: Kind = form.kind.parse().unwrap_or(Kind::Int);
    if !name.is_empty() {
        sqlx::query(
            "INSERT INTO challenges (map_id, name, position, kind)
             VALUES ($1, $2,
                     (SELECT COALESCE(MAX(position), 0) + 1 FROM challenges WHERE map_id = $1),
                     $3)",
        )
        .bind(map_id)
        .bind(name)
        .bind(kind.to_string())
        .execute(&state.db)
        .await
        .ok();
    }

    grid_partial(&state, map_id).await
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
    let map_id = map_of_challenge(&state, challenge_id).await;

    let name = form.name.trim();
    if !name.is_empty() {
        sqlx::query("UPDATE challenges SET name = $1 WHERE id = $2")
            .bind(name)
            .bind(challenge_id)
            .execute(&state.db)
            .await
            .ok();
    }

    grid_partial(&state, map_id).await
}

/// DELETE /challenge/:id — drop a challenge column; its cell values go
/// with it (FK cascade). Re-renders the grid.
pub async fn delete_challenge(
    State(state): State<Arc<AppState>>,
    Path(challenge_id): Path<i32>,
) -> Html<String> {
    let map_id = map_of_challenge(&state, challenge_id).await;

    sqlx::query("DELETE FROM challenges WHERE id = $1")
        .bind(challenge_id)
        .execute(&state.db)
        .await
        .ok();

    grid_partial(&state, map_id).await
}
