//! Session auth built around one shared secret (no accounts, no
//! passwords). Visiting /login?key=<ADMIN_KEY> exchanges the secret for
//! a random session token: its SHA-256 goes into the sessions table, the
//! raw value into an HttpOnly cookie. GETs stay public (the templates
//! render read-only); a middleware bounces any other method that doesn't
//! carry a valid session.

use axum::{
    extract::{Query, Request, State},
    http::Method,
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use sha2::{Digest, Sha256};
use std::sync::Arc;

use super::render;
use crate::AppState;

const SESSION_COOKIE: &str = "session";

fn sha256_hex(s: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn new_token() -> String {
    let bytes: [u8; 32] = rand::random();
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Whether the request's session cookie matches a stored session.
pub(crate) async fn is_admin(state: &AppState, jar: &CookieJar) -> bool {
    let Some(cookie) = jar.get(SESSION_COOKIE) else {
        return false;
    };
    sqlx::query_as::<_, (i32,)>("SELECT 1 FROM sessions WHERE token_hash = $1")
        .bind(sha256_hex(cookie.value()))
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten()
        .is_some()
}

/// Middleware: every GET/HEAD passes (the site is publicly readable);
/// anything else needs a session. htmx callers get an HX-Redirect to the
/// login page, plain ones a real redirect.
pub async fn require_admin_for_writes(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    req: Request,
    next: Next,
) -> Response {
    let read_only = matches!(*req.method(), Method::GET | Method::HEAD);
    if read_only || is_admin(&state, &jar).await {
        return next.run(req).await;
    }
    if req.headers().contains_key("hx-request") {
        ([("HX-Redirect", "/login")], "").into_response()
    } else {
        Redirect::to("/login").into_response()
    }
}

#[derive(serde::Deserialize)]
pub struct LoginQuery {
    pub key: Option<String>,
}

/// GET /login — with the right ?key=, trades the shared secret for a
/// fresh session and goes home. Without a key, shows the key form; with
/// a wrong one, the same form plus an error.
pub async fn login(
    State(state): State<Arc<AppState>>,
    Query(q): Query<LoginQuery>,
    jar: CookieJar,
) -> Response {
    if is_admin(&state, &jar).await {
        return Redirect::to("/").into_response();
    }

    let admin_key = std::env::var("ADMIN_KEY").unwrap_or_default();
    match q.key {
        Some(key) if !admin_key.is_empty() && key == admin_key => {
            let token = new_token();
            sqlx::query("INSERT INTO sessions (token_hash) VALUES ($1)")
                .bind(sha256_hex(&token))
                .execute(&state.db)
                .await
                .unwrap();

            let cookie = Cookie::build((SESSION_COOKIE, token))
                .path("/")
                .http_only(true)
                .same_site(SameSite::Lax)
                .permanent()
                .build();
            (jar.add(cookie), Redirect::to("/")).into_response()
        }
        Some(_) => {
            let mut ctx = tera::Context::new();
            ctx.insert("error", "wrong key");
            render(&state, "login.html", &ctx).into_response()
        }
        None => render(&state, "login.html", &tera::Context::new()).into_response(),
    }
}

/// POST /logout — forget the session on both sides: the row in the DB
/// and the cookie in the browser.
pub async fn logout(State(state): State<Arc<AppState>>, jar: CookieJar) -> Response {
    if let Some(cookie) = jar.get(SESSION_COOKIE) {
        sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
            .bind(sha256_hex(cookie.value()))
            .execute(&state.db)
            .await
            .ok();
    }
    let removal = Cookie::build((SESSION_COOKIE, "")).path("/").build();
    (jar.remove(removal), [("HX-Redirect", "/")], "").into_response()
}
