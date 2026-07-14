//! Standalone map-difficulty calculator, reached from the sidebar.
//!
//! Each room is graded on an open-ended tier scale (Beginner …
//! Intermediate … Expert … GM, GM+1, GM+2, …), each tier split into three
//! shades (Green < Yellow < Red). A grade encodes to an integer
//! `base * 3 + shade`. The map's difficulty is a rank-weighted average of
//! its rooms sorted hardest-first, the weight decaying geometrically by
//! rank (`r`, default 0.7): easy rooms fall to the tail with weight ~0
//! (no dilution), a lone hard room is pulled down by softer neighbours,
//! and a cluster of hard rooms sustains a high result.
//!
//! Pure computation, no persistence — the whole state lives in the query
//! string, so every control is a GET (public, unguarded by the write
//! middleware) that recomputes and re-renders the pane.

use axum::{
    extract::{Query, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use axum_extra::extract::cookie::CookieJar;
use serde::Serialize;
use std::sync::Arc;

use super::render;
use crate::AppState;

const TEMPLATE: &str = "partials/grade.html";
const DEFAULT_R: f64 = 0.7;
const DEFAULT_CAP: f64 = 2.0;

/// The calculator's own version (major.minor), independent of the app
/// version. Bump it whenever the grading or aggregation behavior
/// changes, and record what changed in `static/docs/diffs.md` under that
/// version. Shown on the calculator, linked to that doc.
const CALC_VERSION: &str = "2.0";

#[derive(Clone, Copy, Debug)]
pub enum Shade {
    Green,
    Yellow,
    Red,
}

#[derive(Clone, Copy, Debug)]
pub enum Tier {
    Beginner,
    Intermediate,
    Advanced,
    Expert,
    Gm(u32), // Gm(0) = GM, Gm(1) = GM+1, … (unbounded)
}

/// Encodes a room grade to its numeric value.
pub fn difficulty_value(tier: Tier, shade: Shade) -> f64 {
    let base = match tier {
        Tier::Beginner => 0,
        Tier::Intermediate => 1,
        Tier::Advanced => 2,
        Tier::Expert => 3,
        Tier::Gm(n) => 4 + n,
    };
    let shade = match shade {
        Shade::Green => 0,
        Shade::Yellow => 1,
        Shade::Red => 2,
    };
    (base * 3 + shade) as f64
}

/// Decodes a numeric value to a readable label (rounded to the nearest step).
pub fn value_to_label(v: f64) -> String {
    let idx = v.round().max(0.0) as u32;
    let base = idx / 3;
    let shade = match idx % 3 {
        0 => "Green",
        1 => "Yellow",
        _ => "Red",
    };
    let tier = match base {
        0 => "Beginner".to_string(),
        1 => "Intermediate".to_string(),
        2 => "Advanced".to_string(),
        3 => "Expert".to_string(),
        4 => "GM".to_string(),
        n => format!("GM+{}", n - 4),
    };
    format!("{tier} {shade}")
}

/// v1 — rank-weighted geometric average of the sorted rooms. Kept
/// alongside v2 for the on-page comparison. Its flaw: because it divides
/// by the (growing) sum of weights, adding rooms can *lower* D (dilution),
/// and a lone hard room is pulled below the peak by softer neighbours.
///
/// `r` is the per-rank weight decay (0.7 by default).
pub fn difficulty_v1(room_values: &[f64], r: f64) -> f64 {
    if room_values.is_empty() {
        return 0.0;
    }
    let mut sorted: Vec<f64> = room_values.to_vec();
    sorted.sort_by(|a, b| b.partial_cmp(a).unwrap()); // descending
    let mut num = 0.0;
    let mut den = 0.0;
    let mut w = 1.0;
    for d in sorted {
        num += w * d;
        den += w;
        w *= r;
    }
    num / den
}

/// v2 — peak-anchored with a bounded sustained-difficulty bonus. The map
/// is at least as hard as its hardest room (the peak); each additional
/// near-peak room, sorted hardest-first, adds a diminishing rank-weighted
/// amount to an "effective count" E, and the bonus saturates toward `cap`
/// as E grows: `D = peak + cap·(1 − rᴱ)`.
///
/// So adding a room never lowers the result (monotone), the result never
/// exceeds `peak + cap`, and a run of near-peak rooms pushes D up toward
/// that ceiling. Easy rooms fall to the tail with weight ~0.
///
/// `r` is the per-rank weight decay (0.7 by default); `cap` is the most
/// the sustained bonus can add above the peak (2.0 by default).
pub fn difficulty_v2(room_values: &[f64], r: f64, cap: f64) -> f64 {
    let peak = room_values.iter().cloned().fold(f64::MIN, f64::max);
    if room_values.len() <= 1 || peak <= 0.0 {
        return peak.max(0.0);
    }
    let mut sorted: Vec<f64> = room_values.to_vec();
    sorted.sort_by(|a, b| b.partial_cmp(a).unwrap()); // descending
    // Effective count of near-peak rooms beyond the hardest, each
    // rank-weighted (the first extra room counts fully) and scaled by how
    // close it is to the peak.
    let mut e = 0.0;
    let mut w = 1.0;
    for &d in &sorted[1..] {
        e += w * (d / peak);
        w *= r;
    }
    peak + cap * (1.0 - r.powf(e))
}

fn tier_from_base(base: u32) -> Tier {
    match base {
        0 => Tier::Beginner,
        1 => Tier::Intermediate,
        2 => Tier::Advanced,
        3 => Tier::Expert,
        n => Tier::Gm(n - 4),
    }
}

fn shade_from_int(s: u32) -> Shade {
    match s {
        0 => Shade::Green,
        1 => Shade::Yellow,
        _ => Shade::Red,
    }
}

#[derive(serde::Deserialize)]
pub struct CalcQuery {
    /// Current rooms as a comma-separated list of encoded values.
    rooms: Option<String>,
    r: Option<f64>,
    cap: Option<f64>,
    action: Option<String>,
    base: Option<u32>,
    shade: Option<u32>,
    remove: Option<usize>,
}

#[derive(Serialize)]
struct RoomView {
    label: String,
}

/// GET /difficulty — the calculator pane. Applies the requested edit
/// (add / remove / r change) carried in the query, recomputes, and
/// re-renders. A bare load (sidebar click, refresh) starts empty.
pub async fn calculator(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    headers: HeaderMap,
    Query(q): Query<CalcQuery>,
) -> Response {
    let mut rooms: Vec<i64> = q
        .rooms
        .unwrap_or_default()
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();

    if let Some(i) = q.remove {
        if i < rooms.len() {
            rooms.remove(i);
        }
    } else if q.action.as_deref() == Some("add") {
        if let (Some(base), Some(shade)) = (q.base, q.shade) {
            rooms.push(difficulty_value(tier_from_base(base), shade_from_int(shade)) as i64);
        }
    }

    let r = q.r.unwrap_or(DEFAULT_R).clamp(0.01, 0.99);
    let cap = q.cap.unwrap_or(DEFAULT_CAP).clamp(0.0, 6.0);
    let values: Vec<f64> = rooms.iter().map(|&v| v as f64).collect();
    let d1 = difficulty_v1(&values, r);
    let d2 = difficulty_v2(&values, r, cap);
    let peak = values.iter().cloned().fold(f64::MIN, f64::max);

    let room_views: Vec<RoomView> = rooms
        .iter()
        .map(|&v| RoomView {
            label: value_to_label(v as f64),
        })
        .collect();
    let rooms_csv = rooms
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(",");

    let mut ctx = tera::Context::new();
    ctx.insert("calc_version", CALC_VERSION);
    ctx.insert("rooms", &room_views);
    ctx.insert("rooms_csv", &rooms_csv);
    ctx.insert("has_rooms", &!rooms.is_empty());
    ctx.insert("r", &format!("{r:.2}"));
    ctx.insert("cap", &format!("{cap:.1}"));
    ctx.insert("d1_value", &format!("{d1:.2}"));
    ctx.insert("d1_label", &value_to_label(d1));
    ctx.insert("d2_value", &format!("{d2:.2}"));
    ctx.insert("d2_label", &value_to_label(d2));
    ctx.insert(
        "peak_label",
        &(if rooms.is_empty() {
            "\u{2014}".to_string()
        } else {
            value_to_label(peak)
        }),
    );

    let view = render(&state, TEMPLATE, &ctx);
    if super::is_htmx(&headers) {
        view.into_response()
    } else {
        let is_admin = super::auth::is_admin(&state, &jar).await;
        super::full_page(&state, is_admin, &view.0)
            .await
            .into_response()
    }
}
