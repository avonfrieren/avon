//! Route handlers, split by the thing they manage:
//! - `sidebar` — the explorer shell and the tree everything hangs off
//! - `campaigns` / `maps` — CRUD for the two kinds of tree entries
//! - `grid` — a map's room × challenge table and its cell values

pub mod auth;
pub mod campaigns;
pub mod docs;
pub mod grid;
pub mod maps;
pub mod sidebar;
pub mod time;

use axum::{http::HeaderMap, response::Html};

use crate::AppState;

/// Render one template with the given context. Every handler ends this
/// way, so the boilerplate lives here once.
pub(crate) fn render(state: &AppState, template: &str, ctx: &tera::Context) -> Html<String> {
    Html(state.tera.render(template, ctx).unwrap())
}

/// Whether the request comes from htmx (which wants a bare fragment)
/// rather than a full browser load (refresh, shared link, bookmark).
pub(crate) fn is_htmx(headers: &HeaderMap) -> bool {
    headers.contains_key("hx-request")
}

/// Wrap a content-pane fragment in the whole explorer shell. Content
/// URLs are pushed into the address bar (hx-push-url), so a refresh hits
/// them directly — this is what turns the fragment back into a page.
pub(crate) async fn full_page(state: &AppState, is_admin: bool, content: &str) -> Html<String> {
    let mut ctx = sidebar::sidebar_context(state, is_admin).await;
    ctx.insert("content", content);
    render(state, "explorer.html", &ctx)
}
