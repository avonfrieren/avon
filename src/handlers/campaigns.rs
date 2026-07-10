//! Campaign CRUD: creation form, inline rename from the sidebar, and
//! deletion. A campaign's maps are never deleted with it — the FK sets
//! their campaign_id to NULL, so they become standalone.

use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse, Response},
};
use std::sync::Arc;

use super::{render, sidebar::sidebar_context};
use crate::AppState;

/// GET /campaigns/new — the campaign creation form, loaded into #content.
pub async fn campaign_form(State(state): State<Arc<AppState>>) -> Html<String> {
    render(&state, "partials/campaign_form.html", &tera::Context::new())
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
        return render(&state, "partials/campaign_form.html", &ctx).into_response();
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
    render(&state, "partials/campaign_rename.html", &ctx)
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
    render(&state, "partials/sidebar.html", &ctx)
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
