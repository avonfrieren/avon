mod db;
mod handlers;

use axum::{routing::get, Router};
use std::sync::Arc;
use tera::Tera;
use tower_http::services::ServeDir;

/// Shared application state, handed to every handler via `State<Arc<AppState>>`.
pub struct AppState {
    pub db: sqlx::PgPool,
    pub tera: Tera,
}

#[tokio::main]
async fn main() {
    // Load variables from .env into the process environment (no-op in prod if you
    // set real env vars instead).
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = db::init_pool(&database_url).await;

    // Loads every .html file under templates/ recursively.
    let tera = Tera::new("templates/**/*.html").expect("Failed to load templates");

    let state = Arc::new(AppState { db: pool, tera });

    let app = Router::new()
        .route("/", get(handlers::explorer::index))
        .route("/map/:id", get(handlers::explorer::map_detail))
        .route(
            "/cell/:room_id/:challenge_id",
            axum::routing::post(handlers::explorer::update_cell),
        )
        .route("/campaigns/new", get(handlers::explorer::campaign_form))
        .route(
            "/campaigns",
            axum::routing::post(handlers::explorer::create_campaign),
        )
        .route("/maps/new", get(handlers::explorer::map_form))
        .route("/maps", axum::routing::post(handlers::explorer::create_map))
        .route("/imgs/:filename", get(handlers::explorer::image_view))
        .nest_service("/static", ServeDir::new("static"))
        .with_state(state);

    // Alwaysdata injects PORT (and IP) into the environment for custom sites.
    // Fall back to 3000/0.0.0.0 for local dev where those aren't set.
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);
    let ip: std::net::IpAddr = std::env::var("IP")
        .ok()
        .and_then(|ip| ip.parse().ok())
        .unwrap_or_else(|| std::net::IpAddr::from([0, 0, 0, 0]));
    let addr = std::net::SocketAddr::new(ip, port);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("Listening on http://{addr}");
    axum::serve(listener, app).await.unwrap();
}
