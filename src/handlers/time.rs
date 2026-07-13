//! The time dashboard: a per-map view dedicated to one time challenge,
//! reached by clicking its column header in the grid. Shows every room's
//! best segment next to the best complete run, the SoB and run totals,
//! checkpoint summary, and — admin only — the paste-import for the
//! speedrun mod's export format:
//!
//!   Room Number,Split,Segment,Best Split,Best Segment
//!   1,1.666,1.666,1.666,1.666
//!   2,5.780,4.114,5.780,4.114
//!
//! Rows map to the site's rooms by order (Nth line = Nth room by
//! position); "Best Segment" improves per-room bests individually, and
//! "Best Split" replaces the stored run as a whole if its final time is
//! better. The scaffolding shared with the other dashboards (rooms,
//! values, checkpoint grouping, response shell, checkpoint mutations)
//! lives in `super::dashboard`; only the run column and the import are
//! specific to this type.

use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse, Response},
};
use axum_extra::extract::cookie::CookieJar;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;

use super::dashboard;
use super::grid::{format_time, parse_time};
use super::render;
use crate::AppState;

const TEMPLATE: &str = "partials/time_dashboard.html";

#[derive(Serialize)]
struct TimeRow {
    room_id: i32,
    room_name: String,
    checkpoint_end: bool,
    checkpoint_real: bool,
    /// Best segment: the room retried in a loop (the grid's value).
    best: Option<String>,
    /// The best complete run's cumulative time at this room.
    run: Option<String>,
}

#[derive(Serialize)]
struct CheckpointRow {
    name: String,
    rooms: String,
    sob: Option<String>,
    run: Option<String>,
}

/// Builds the dashboard context: per-room rows, totals, and the
/// per-checkpoint summary (present once a checkpoint is flagged).
async fn dashboard_context(
    state: &AppState,
    challenge_id: i32,
    map_id: i32,
    challenge_name: &str,
) -> tera::Context {
    let rooms = dashboard::rooms(state, map_id).await;
    let bests = dashboard::values(state, challenge_id).await;

    let splits: HashMap<i32, i64> =
        sqlx::query_as("SELECT room_id, split_ms FROM run_splits WHERE challenge_id = $1")
            .bind(challenge_id)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default()
            .into_iter()
            .collect();

    let mut rows = Vec::with_capacity(rooms.len());
    let mut sob_total: i64 = 0;
    let mut sob_any = false;
    let mut run_total: Option<i64> = None;
    for (room_id, room_name, checkpoint_end, checkpoint_real) in &rooms {
        let best = bests.get(room_id).copied();
        if let Some(b) = best {
            sob_any = true;
            sob_total += b;
        }
        let split = splits.get(room_id).copied();
        if let Some(s) = split {
            run_total = Some(s);
        }
        rows.push(TimeRow {
            room_id: *room_id,
            room_name: room_name.clone(),
            checkpoint_end: *checkpoint_end,
            checkpoint_real: *checkpoint_real,
            best: best.map(format_time),
            run: split.map(format_time),
        });
    }

    let mut prev: i64 = 0;
    let checkpoints: Vec<CheckpointRow> = dashboard::checkpoint_groups(&rooms)
        .iter()
        .enumerate()
        .map(|(i, g)| {
            let mut sob = 0i64;
            let mut sob_all = true;
            for (room_id, ..) in g {
                match bests.get(room_id) {
                    Some(b) => sob += b,
                    None => sob_all = false,
                }
            }
            // Run time for the group = last split minus the split before
            // the group; only meaningful when the run covers it.
            let last_split = g.last().and_then(|(room_id, ..)| splits.get(room_id).copied());
            let run = last_split.map(|s| s - prev);
            if let Some(s) = last_split {
                prev = s;
            }
            CheckpointRow {
                name: format!("cp {}", i + 1),
                rooms: dashboard::group_label(g),
                sob: (sob_all && !g.is_empty()).then(|| format_time(sob)),
                run: run.map(format_time),
            }
        })
        .collect();

    let mut ctx = tera::Context::new();
    ctx.insert("challenge_id", &challenge_id);
    ctx.insert("challenge_name", challenge_name);
    ctx.insert("map_id", &map_id);
    ctx.insert("map_name", &dashboard::map_name(state, map_id).await);
    ctx.insert("rows", &rows);
    ctx.insert("sob_total", &sob_any.then(|| format_time(sob_total)));
    ctx.insert("run_total", &run_total.map(format_time));
    ctx.insert("checkpoints", &checkpoints);
    ctx
}

/// Re-render the dashboard as its editable variant (only the
/// write-guarded POST handlers reach this).
async fn rerender(state: &AppState, challenge_id: i32, map_id: i32, name: &str) -> Response {
    let mut ctx = dashboard_context(state, challenge_id, map_id, name).await;
    ctx.insert("is_admin", &true);
    render(state, TEMPLATE, &ctx).into_response()
}

/// GET /time/:challenge_id — the dashboard pane.
pub async fn dashboard(
    State(state): State<Arc<AppState>>,
    Path(challenge_id): Path<i32>,
    jar: CookieJar,
    headers: axum::http::HeaderMap,
) -> Response {
    let Some((map_id, name)) = dashboard::guard(&state, challenge_id, "time").await else {
        return Html("not a time challenge".to_string()).into_response();
    };
    let is_admin = super::auth::is_admin(&state, &jar).await;

    let mut ctx = dashboard_context(&state, challenge_id, map_id, &name).await;
    ctx.insert("is_admin", &is_admin);
    let view = render(&state, TEMPLATE, &ctx);
    dashboard::respond(&state, is_admin, &headers, view).await
}

/// One parsed line of the mod's export: the run's cumulative best split
/// and the room's individual best segment.
struct ImportRow {
    best_split: i64,
    best_segment: i64,
}

/// Parses the pasted export. Order defines the room mapping, so the row
/// count must match the map exactly; anything unparseable is an error
/// rather than a guess.
fn parse_import(text: &str, nb_rooms: usize) -> Result<Vec<ImportRow>, String> {
    let mut rows = Vec::new();
    for (i, line) in text.lines().map(str::trim).enumerate() {
        if line.is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split(',').map(str::trim).collect();
        // The header line, if pasted along, starts with a non-number.
        if i == 0 && fields[0].parse::<i64>().is_err() {
            continue;
        }
        if fields.len() < 5 {
            return Err(format!("line {}: expected 5 comma-separated columns", i + 1));
        }
        let best_split = parse_time(fields[3])
            .ok_or_else(|| format!("line {}: can't parse best split '{}'", i + 1, fields[3]))?;
        let best_segment = parse_time(fields[4])
            .ok_or_else(|| format!("line {}: can't parse best segment '{}'", i + 1, fields[4]))?;
        rows.push(ImportRow {
            best_split,
            best_segment,
        });
    }
    if rows.is_empty() {
        return Err("no data rows found".to_string());
    }
    if rows.len() != nb_rooms {
        return Err(format!(
            "row count mismatch: {} data rows for {} rooms — rows map to rooms by order, so they must match exactly",
            rows.len(),
            nb_rooms
        ));
    }
    Ok(rows)
}

#[derive(serde::Deserialize)]
pub struct ImportForm {
    pub data: String,
}

/// POST /time/:challenge_id/import — parse the pasted export, keep
/// whatever is better: per-room best segments individually, and the run
/// as a whole (its final split against the stored one). Re-renders the
/// dashboard with a summary of what changed.
pub async fn import(
    State(state): State<Arc<AppState>>,
    Path(challenge_id): Path<i32>,
    axum::Form(form): axum::Form<ImportForm>,
) -> Response {
    let Some((map_id, name)) = dashboard::guard(&state, challenge_id, "time").await else {
        return Html("not a time challenge".to_string()).into_response();
    };

    let rooms: Vec<(i32,)> =
        sqlx::query_as("SELECT id FROM rooms WHERE map_id = $1 ORDER BY position, id")
            .bind(map_id)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();

    let message = match parse_import(&form.data, rooms.len()) {
        Err(e) => format!("import failed — {e}"),
        Ok(parsed) => {
            // Per-room bests: an import can only improve them.
            let mut improved = 0;
            for ((room_id,), row) in rooms.iter().zip(&parsed) {
                let current: Option<(i64,)> = sqlx::query_as(
                    "SELECT value FROM challenge_values WHERE room_id = $1 AND challenge_id = $2",
                )
                .bind(room_id)
                .bind(challenge_id)
                .fetch_optional(&state.db)
                .await
                .ok()
                .flatten();
                if current.is_none_or(|(v,)| row.best_segment < v) {
                    sqlx::query(
                        "INSERT INTO challenge_values (room_id, challenge_id, value)
                         VALUES ($1, $2, $3)
                         ON CONFLICT (room_id, challenge_id)
                         DO UPDATE SET value = EXCLUDED.value, updated_at = now()",
                    )
                    .bind(room_id)
                    .bind(challenge_id)
                    .bind(row.best_segment)
                    .execute(&state.db)
                    .await
                    .ok();
                    improved += 1;
                }
            }

            // The run: replaced as a whole if its final split is better.
            let new_total = parsed.last().unwrap().best_split;
            let stored_total: Option<(i64,)> =
                sqlx::query_as("SELECT MAX(split_ms) FROM run_splits WHERE challenge_id = $1")
                    .bind(challenge_id)
                    .fetch_optional(&state.db)
                    .await
                    .ok()
                    .flatten()
                    .filter(|(v,)| *v > 0);

            let run_msg = if stored_total.is_none_or(|(t,)| new_total < t) {
                sqlx::query("DELETE FROM run_splits WHERE challenge_id = $1")
                    .bind(challenge_id)
                    .execute(&state.db)
                    .await
                    .ok();
                for ((room_id,), row) in rooms.iter().zip(&parsed) {
                    sqlx::query(
                        "INSERT INTO run_splits (challenge_id, room_id, split_ms) VALUES ($1, $2, $3)",
                    )
                    .bind(challenge_id)
                    .bind(room_id)
                    .bind(row.best_split)
                    .execute(&state.db)
                    .await
                    .ok();
                }
                format!("run imported ({})", format_time(new_total))
            } else {
                format!(
                    "run kept (stored {} beats pasted {})",
                    format_time(stored_total.unwrap().0),
                    format_time(new_total)
                )
            };

            format!("{run_msg} — {improved} room best(s) improved")
        }
    };

    let mut ctx = dashboard_context(&state, challenge_id, map_id, &name).await;
    ctx.insert("is_admin", &true);
    ctx.insert("message", &message);
    render(&state, TEMPLATE, &ctx).into_response()
}

/// POST /time/:challenge_id/cps/save — freeze the working checkpoints.
pub async fn save_checkpoints(
    State(state): State<Arc<AppState>>,
    Path(challenge_id): Path<i32>,
) -> Response {
    let Some((map_id, name)) = dashboard::guard(&state, challenge_id, "time").await else {
        return Html("not a time challenge".to_string()).into_response();
    };
    dashboard::save_checkpoints(&state, map_id).await;
    rerender(&state, challenge_id, map_id, &name).await
}

/// POST /time/:challenge_id/cps/reset — restore the working checkpoints.
pub async fn reset_checkpoints(
    State(state): State<Arc<AppState>>,
    Path(challenge_id): Path<i32>,
) -> Response {
    let Some((map_id, name)) = dashboard::guard(&state, challenge_id, "time").await else {
        return Html("not a time challenge".to_string()).into_response();
    };
    dashboard::reset_checkpoints(&state, map_id).await;
    rerender(&state, challenge_id, map_id, &name).await
}

/// POST /time/:challenge_id/checkpoint/:room_id — flag/unflag a room.
pub async fn toggle_checkpoint(
    State(state): State<Arc<AppState>>,
    Path((challenge_id, room_id)): Path<(i32, i32)>,
) -> Response {
    let Some((map_id, name)) = dashboard::guard(&state, challenge_id, "time").await else {
        return Html("not a time challenge".to_string()).into_response();
    };
    dashboard::toggle_checkpoint(&state, room_id).await;
    rerender(&state, challenge_id, map_id, &name).await
}
