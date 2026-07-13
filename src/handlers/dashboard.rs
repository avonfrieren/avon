//! Shared scaffolding for the per-type dashboards (time, difficulty, and
//! any future one). Each type keeps its own context builder and columns;
//! everything that is identical regardless of type lives here: the map,
//! room and challenge-value fetches, the checkpoint grouping, the GET
//! response shell (htmx fragment vs full page), and the checkpoint
//! mutations — which only touch `rooms`, so they are type-blind.

use axum::{
    http::HeaderMap,
    response::{Html, IntoResponse, Response},
};
use std::collections::HashMap;

use crate::AppState;

/// A room as every dashboard needs it: id, name, and the two checkpoint
/// flags (working state + real reference).
pub type Room = (i32, String, bool, bool);

/// Confirms the challenge exists and is of the expected kind, returning
/// its `(map_id, name)`. None means wrong kind or missing — the caller
/// answers with a small error.
pub async fn guard(state: &AppState, challenge_id: i32, kind: &str) -> Option<(i32, String)> {
    let (map_id, name, k): (i32, String, String) =
        sqlx::query_as("SELECT map_id, name, kind FROM challenges WHERE id = $1")
            .bind(challenge_id)
            .fetch_optional(&state.db)
            .await
            .ok()
            .flatten()?;
    (k == kind).then_some((map_id, name))
}

/// A map's display name.
pub async fn map_name(state: &AppState, map_id: i32) -> String {
    sqlx::query_as::<_, (String,)>("SELECT name FROM maps WHERE id = $1")
        .bind(map_id)
        .fetch_one(&state.db)
        .await
        .map(|(n,)| n)
        .unwrap_or_default()
}

/// A map's rooms in play order, with their checkpoint flags.
pub async fn rooms(state: &AppState, map_id: i32) -> Vec<Room> {
    sqlx::query_as(
        "SELECT id, name, checkpoint_end, checkpoint_real
         FROM rooms WHERE map_id = $1 ORDER BY position, id",
    )
    .bind(map_id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default()
}

/// A challenge's stored per-room values (challenge_values), as room → value.
pub async fn values(state: &AppState, challenge_id: i32) -> HashMap<i32, i64> {
    sqlx::query_as("SELECT room_id, value FROM challenge_values WHERE challenge_id = $1")
        .bind(challenge_id)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default()
        .into_iter()
        .collect()
}

/// Splits rooms into checkpoint groups: a flagged room ENDS its group
/// (it is included), and whatever trails the last flag forms the final
/// group. Empty when no room is flagged.
pub fn checkpoint_groups(rooms: &[Room]) -> Vec<Vec<&Room>> {
    if !rooms.iter().any(|(_, _, cp, _)| *cp) {
        return Vec::new();
    }
    let mut group: Vec<&Room> = Vec::new();
    let mut groups: Vec<Vec<&Room>> = Vec::new();
    for room in rooms {
        group.push(room);
        if room.2 {
            groups.push(std::mem::take(&mut group));
        }
    }
    if !group.is_empty() {
        groups.push(group);
    }
    groups
}

/// A group's room-range label, e.g. "r01 → r04".
pub fn group_label(group: &[&Room]) -> String {
    format!(
        "{} \u{2192} {}",
        group.first().unwrap().1,
        group.last().unwrap().1
    )
}

/// GET-handler shell: htmx callers get the bare fragment, a direct load
/// (refresh, shared link) gets it wrapped in the whole explorer page.
pub async fn respond(
    state: &AppState,
    is_admin: bool,
    headers: &HeaderMap,
    view: Html<String>,
) -> Response {
    if super::is_htmx(headers) {
        view.into_response()
    } else {
        super::full_page(state, is_admin, &view.0)
            .await
            .into_response()
    }
}

/// Flip one room's working checkpoint flag.
pub async fn toggle_checkpoint(state: &AppState, room_id: i32) {
    sqlx::query("UPDATE rooms SET checkpoint_end = NOT checkpoint_end WHERE id = $1")
        .bind(room_id)
        .execute(&state.db)
        .await
        .ok();
}

/// Freeze a map's working checkpoints as its real reference.
pub async fn save_checkpoints(state: &AppState, map_id: i32) {
    sqlx::query("UPDATE rooms SET checkpoint_real = checkpoint_end WHERE map_id = $1")
        .bind(map_id)
        .execute(&state.db)
        .await
        .ok();
}

/// Restore a map's working checkpoints from its real reference.
pub async fn reset_checkpoints(state: &AppState, map_id: i32) {
    sqlx::query("UPDATE rooms SET checkpoint_end = checkpoint_real WHERE map_id = $1")
        .bind(map_id)
        .execute(&state.db)
        .await
        .ok();
}
