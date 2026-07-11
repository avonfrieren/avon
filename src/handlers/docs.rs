//! Markdown documents. Like imgs, the static/docs/ folder is the source
//! of truth: drop a .md in there (scp/git — files are authored locally,
//! never edited on the site) and it shows up in the sidebar; the site
//! only renders and deletes. Rendering uses pulldown-cmark with the
//! usual extensions (tables, strikethrough, footnotes, task lists).

use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse, Response},
};
use axum_extra::extract::cookie::CookieJar;
use pulldown_cmark::{html, Options, Parser};
use std::sync::Arc;

use super::render;
use crate::AppState;

const DOCS_DIR: &str = "static/docs";

/// Only accept a bare "<name>.md" filename — no separators, no dot-dot —
/// so a crafted URL can't reach outside static/docs/.
fn safe_doc_path(filename: &str) -> Option<std::path::PathBuf> {
    if !filename.ends_with(".md")
        || filename.contains('/')
        || filename.contains('\\')
        || filename.contains("..")
    {
        return None;
    }
    Some(std::path::Path::new(DOCS_DIR).join(filename))
}

/// Sorted .md filenames from static/docs/, for the sidebar tree.
pub(crate) async fn list_docs() -> Vec<String> {
    let mut docs = Vec::new();
    if let Ok(mut entries) = tokio::fs::read_dir(DOCS_DIR).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            if let Some(name) = entry.file_name().to_str() {
                if name.ends_with(".md") {
                    docs.push(name.to_string());
                }
            }
        }
    }
    docs.sort();
    docs
}

/// GET /docs/:filename — render one markdown file into the content pane.
/// The delete cross only shows for a logged-in session (the DELETE route
/// is guarded by the write middleware regardless).
pub async fn doc_view(
    State(state): State<Arc<AppState>>,
    Path(filename): Path<String>,
    jar: CookieJar,
) -> Response {
    let Some(path) = safe_doc_path(&filename) else {
        return Html("not found".to_string()).into_response();
    };
    let Ok(source) = tokio::fs::read_to_string(&path).await else {
        return Html("not found".to_string()).into_response();
    };

    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_TASKLISTS);
    let parser = Parser::new_ext(&source, options);
    let mut content = String::new();
    html::push_html(&mut content, parser);

    let mut ctx = tera::Context::new();
    ctx.insert("filename", &filename);
    ctx.insert("content", &content);
    ctx.insert("is_admin", &super::auth::is_admin(&state, &jar).await);
    render(&state, "partials/doc_view.html", &ctx).into_response()
}

/// DELETE /docs/:filename — remove the file from disk (write-guarded by
/// the auth middleware), then HX-Redirect home to rebuild the sidebar.
pub async fn delete_doc(Path(filename): Path<String>) -> Response {
    if let Some(path) = safe_doc_path(&filename) {
        tokio::fs::remove_file(path).await.ok();
    }
    ([("HX-Redirect", "/")], "").into_response()
}
