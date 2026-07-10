//! Route handlers, split by the thing they manage:
//! - `sidebar` — the explorer shell and the tree everything hangs off
//! - `campaigns` / `maps` — CRUD for the two kinds of tree entries
//! - `grid` — a map's room × challenge table and its cell values

pub mod auth;
pub mod campaigns;
pub mod grid;
pub mod maps;
pub mod sidebar;

use axum::response::Html;

use crate::AppState;

/// Render one template with the given context. Every handler ends this
/// way, so the boilerplate lives here once.
pub(crate) fn render(state: &AppState, template: &str, ctx: &tera::Context) -> Html<String> {
    Html(state.tera.render(template, ctx).unwrap())
}
