//! The bool dashboard: a per-map view dedicated to one bool challenge,
//! reached by clicking its column header in the grid. Mirrors the time
//! dashboard (same checkpoint system), but each row pairs the challenge's
//! boolean with a hand-entered "diff" integer. The map's difficulty is
//! the sum of diffs over the room count; each checkpoint has its own the
//! same way. Everything is entered by hand — no import.

use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse, Response},
};
use axum_extra::extract::cookie::CookieJar;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;

use super::render;
use crate::AppState;

#[derive(Serialize)]
struct BoolRow {
    room_id: i32,
    room_name: String,
    checkpoint_end: bool,
    checkpoint_real: bool,
    /// The challenge's boolean for this room (the grid's checkbox).
    passed: bool,
    /// The hand-entered difficulty contribution — empty string when
    /// unset (Tera has no null test, and "0" must stay distinct from
    /// "unset", so this is rendered here rather than in the template).
    diff: String,
}

#[derive(Serialize)]
struct CheckpointRow {
    name: String,
    rooms: String,
    passed: String,
    difficulty: String,
}

/// The challenge a dashboard belongs to, or None if the id isn't a bool
/// challenge — the difficulty view only makes sense for pass/fail columns.
async fn bool_challenge(state: &AppState, challenge_id: i32) -> Option<(i32, String)> {
    let (map_id, name, kind): (i32, String, String) =
        sqlx::query_as("SELECT map_id, name, kind FROM challenges WHERE id = $1")
            .bind(challenge_id)
            .fetch_optional(&state.db)
            .await
            .ok()
            .flatten()?;
    (kind == "bool").then_some((map_id, name))
}

/// sum / count as a two-decimal string; None when there are no rooms.
fn difficulty(sum: i64, rooms: usize) -> Option<String> {
    (rooms > 0).then(|| format!("{:.2}", sum as f64 / rooms as f64))
}

/// Builds the dashboard context: per-room rows, the map totals, and the
/// per-checkpoint summary (present once a checkpoint is flagged).
async fn dashboard_context(
    state: &AppState,
    challenge_id: i32,
    map_id: i32,
    challenge_name: &str,
) -> tera::Context {
    let (map_name,): (String,) = sqlx::query_as("SELECT name FROM maps WHERE id = $1")
        .bind(map_id)
        .fetch_one(&state.db)
        .await
        .unwrap();

    let rooms: Vec<(i32, String, bool, bool)> = sqlx::query_as(
        "SELECT id, name, checkpoint_end, checkpoint_real
         FROM rooms WHERE map_id = $1 ORDER BY position, id",
    )
    .bind(map_id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let values: HashMap<i32, i64> =
        sqlx::query_as("SELECT room_id, value FROM challenge_values WHERE challenge_id = $1")
            .bind(challenge_id)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default()
            .into_iter()
            .collect();

    // diff is INTEGER (i32); reading it as i64 would silently fail to
    // decode and vanish into unwrap_or_default.
    let diffs: HashMap<i32, i64> =
        sqlx::query_as::<_, (i32, i32)>("SELECT room_id, diff FROM challenge_diffs WHERE challenge_id = $1")
            .bind(challenge_id)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|(room, diff)| (room, diff as i64))
            .collect();

    let mut rows = Vec::with_capacity(rooms.len());
    let mut passed_count = 0;
    let mut diff_total: i64 = 0;
    for (room_id, room_name, checkpoint_end, checkpoint_real) in &rooms {
        let passed = values.get(room_id).is_some_and(|&v| v != 0);
        if passed {
            passed_count += 1;
        }
        let diff = diffs.get(room_id).copied();
        diff_total += diff.unwrap_or(0);
        rows.push(BoolRow {
            room_id: *room_id,
            room_name: room_name.clone(),
            checkpoint_end: *checkpoint_end,
            checkpoint_real: *checkpoint_real,
            passed,
            diff: diff.map(|v| v.to_string()).unwrap_or_default(),
        });
    }

    // Per-checkpoint summary. A flagged room ENDS its segment (checking a
    // room includes it); whatever follows the last flag is the final
    // group. The table shows up once a flag is set.
    let mut checkpoints: Vec<CheckpointRow> = Vec::new();
    if rooms.iter().any(|(_, _, cp, _)| *cp) {
        type Room = (i32, String, bool, bool);
        let mut group: Vec<&Room> = Vec::new();
        let mut groups: Vec<Vec<&Room>> = Vec::new();
        for room in &rooms {
            group.push(room);
            if room.2 {
                groups.push(std::mem::take(&mut group));
            }
        }
        if !group.is_empty() {
            groups.push(group);
        }

        for (i, g) in groups.iter().enumerate() {
            let passed = g
                .iter()
                .filter(|(id, ..)| values.get(id).is_some_and(|&v| v != 0))
                .count();
            let sum: i64 = g.iter().map(|(id, ..)| diffs.get(id).copied().unwrap_or(0)).sum();
            checkpoints.push(CheckpointRow {
                name: format!("cp {}", i + 1),
                rooms: format!("{} \u{2192} {}", g.first().unwrap().1, g.last().unwrap().1),
                passed: format!("{}/{}", passed, g.len()),
                difficulty: difficulty(sum, g.len()).unwrap_or_else(|| "\u{2014}".to_string()),
            });
        }
    }

    let mut ctx = tera::Context::new();
    ctx.insert("challenge_id", &challenge_id);
    ctx.insert("challenge_name", challenge_name);
    ctx.insert("map_id", &map_id);
    ctx.insert("map_name", &map_name);
    ctx.insert("rows", &rows);
    ctx.insert("passed_total", &format!("{}/{}", passed_count, rooms.len()));
    ctx.insert("diff_total", &diff_total);
    ctx.insert("map_difficulty", &difficulty(diff_total, rooms.len()));
    ctx.insert("checkpoints", &checkpoints);
    ctx
}

/// Re-render helper: the dashboard is always the editable variant when
/// reached through the write-guarded POST handlers.
async fn rerender(state: &AppState, challenge_id: i32, map_id: i32, name: &str) -> Response {
    let mut ctx = dashboard_context(state, challenge_id, map_id, name).await;
    ctx.insert("is_admin", &true);
    render(state, "partials/difficulty_dashboard.html", &ctx).into_response()
}

/// GET /bool/:challenge_id — the dashboard pane. htmx gets the bare
/// fragment; a direct load (refresh, shared link) gets the whole page.
pub async fn dashboard(
    State(state): State<Arc<AppState>>,
    Path(challenge_id): Path<i32>,
    jar: CookieJar,
    headers: axum::http::HeaderMap,
) -> Response {
    let Some((map_id, name)) = bool_challenge(&state, challenge_id).await else {
        return Html("not a bool challenge".to_string()).into_response();
    };
    let is_admin = super::auth::is_admin(&state, &jar).await;

    let mut ctx = dashboard_context(&state, challenge_id, map_id, &name).await;
    ctx.insert("is_admin", &is_admin);
    let view = render(&state, "partials/difficulty_dashboard.html", &ctx);

    if super::is_htmx(&headers) {
        view.into_response()
    } else {
        super::full_page(&state, is_admin, &view.0).await.into_response()
    }
}

#[derive(serde::Deserialize)]
pub struct ValueForm {
    // Absent for an unchecked box, mirroring the grid's checkbox.
    pub value: Option<String>,
}

/// POST /bool/:challenge_id/value/:room_id — set the room's boolean, then
/// re-render. Checked stores 1; unchecked clears the row (= not passed).
pub async fn set_value(
    State(state): State<Arc<AppState>>,
    Path((challenge_id, room_id)): Path<(i32, i32)>,
    axum::Form(form): axum::Form<ValueForm>,
) -> Response {
    let Some((map_id, name)) = bool_challenge(&state, challenge_id).await else {
        return Html("not a bool challenge".to_string()).into_response();
    };

    if form.value.is_some() {
        sqlx::query(
            "INSERT INTO challenge_values (room_id, challenge_id, value)
             VALUES ($1, $2, 1)
             ON CONFLICT (room_id, challenge_id)
             DO UPDATE SET value = 1, updated_at = now()",
        )
        .bind(room_id)
        .bind(challenge_id)
        .execute(&state.db)
        .await
        .ok();
    } else {
        sqlx::query("DELETE FROM challenge_values WHERE room_id = $1 AND challenge_id = $2")
            .bind(room_id)
            .bind(challenge_id)
            .execute(&state.db)
            .await
            .ok();
    }

    rerender(&state, challenge_id, map_id, &name).await
}

#[derive(serde::Deserialize)]
pub struct DiffForm {
    pub value: String,
}

/// POST /bool/:challenge_id/diff/:room_id — set the room's diff integer,
/// then re-render. Empty clears it; anything unparseable leaves it be.
pub async fn set_diff(
    State(state): State<Arc<AppState>>,
    Path((challenge_id, room_id)): Path<(i32, i32)>,
    axum::Form(form): axum::Form<DiffForm>,
) -> Response {
    let Some((map_id, name)) = bool_challenge(&state, challenge_id).await else {
        return Html("not a bool challenge".to_string()).into_response();
    };

    let raw = form.value.trim();
    if raw.is_empty() {
        sqlx::query("DELETE FROM challenge_diffs WHERE room_id = $1 AND challenge_id = $2")
            .bind(room_id)
            .bind(challenge_id)
            .execute(&state.db)
            .await
            .ok();
    } else if let Ok(v) = raw.parse::<i32>() {
        sqlx::query(
            "INSERT INTO challenge_diffs (challenge_id, room_id, diff)
             VALUES ($1, $2, $3)
             ON CONFLICT (challenge_id, room_id)
             DO UPDATE SET diff = EXCLUDED.diff, updated_at = now()",
        )
        .bind(challenge_id)
        .bind(room_id)
        .bind(v)
        .execute(&state.db)
        .await
        .ok();
    }

    rerender(&state, challenge_id, map_id, &name).await
}

/// POST /bool/:challenge_id/checkpoint/:room_id — flag/unflag a room as a
/// checkpoint end (shared with the time dashboard: checkpoints live on
/// the map, not the challenge), then re-render.
pub async fn toggle_checkpoint(
    State(state): State<Arc<AppState>>,
    Path((challenge_id, room_id)): Path<(i32, i32)>,
) -> Response {
    let Some((map_id, name)) = bool_challenge(&state, challenge_id).await else {
        return Html("not a bool challenge".to_string()).into_response();
    };

    sqlx::query("UPDATE rooms SET checkpoint_end = NOT checkpoint_end WHERE id = $1")
        .bind(room_id)
        .execute(&state.db)
        .await
        .ok();

    rerender(&state, challenge_id, map_id, &name).await
}

/// POST /bool/:challenge_id/cps/save — freeze the working checkpoints as
/// the map's real ones.
pub async fn save_checkpoints(
    State(state): State<Arc<AppState>>,
    Path(challenge_id): Path<i32>,
) -> Response {
    let Some((map_id, name)) = bool_challenge(&state, challenge_id).await else {
        return Html("not a bool challenge".to_string()).into_response();
    };

    sqlx::query("UPDATE rooms SET checkpoint_real = checkpoint_end WHERE map_id = $1")
        .bind(map_id)
        .execute(&state.db)
        .await
        .ok();

    rerender(&state, challenge_id, map_id, &name).await
}

/// POST /bool/:challenge_id/cps/reset — restore the working checkpoints
/// from the map's real ones.
pub async fn reset_checkpoints(
    State(state): State<Arc<AppState>>,
    Path(challenge_id): Path<i32>,
) -> Response {
    let Some((map_id, name)) = bool_challenge(&state, challenge_id).await else {
        return Html("not a bool challenge".to_string()).into_response();
    };

    sqlx::query("UPDATE rooms SET checkpoint_end = checkpoint_real WHERE map_id = $1")
        .bind(map_id)
        .execute(&state.db)
        .await
        .ok();

    rerender(&state, challenge_id, map_id, &name).await
}
