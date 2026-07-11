mod db;
mod handlers;

use axum::{
    routing::{get, post},
    Router,
};
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

    use handlers::{auth, campaigns, docs, grid, maps, sidebar, time};
    let app = Router::new()
        .route("/", get(sidebar::index))
        .route("/login", get(auth::login))
        .route("/logout", post(auth::logout))
        .route("/imgs/:filename", get(sidebar::image_view))
        .route(
            "/docs/:filename",
            get(docs::doc_view).delete(docs::delete_doc),
        )
        .route("/map/:id", get(maps::map_detail).delete(maps::delete_map))
        .route("/map/:id/rename", post(maps::rename_map))
        .route("/maps/new", get(maps::map_form))
        .route("/maps", post(maps::create_map))
        .route(
            "/campaign/:id",
            axum::routing::delete(campaigns::delete_campaign),
        )
        .route(
            "/campaign/:id/rename",
            get(campaigns::campaign_rename_form).post(campaigns::rename_campaign),
        )
        .route("/campaigns/new", get(campaigns::campaign_form))
        .route("/campaigns", post(campaigns::create_campaign))
        .route("/time/:challenge_id", get(time::dashboard))
        .route("/time/:challenge_id/import", post(time::import))
        .route("/time/:challenge_id/cps/save", post(time::save_checkpoints))
        .route("/time/:challenge_id/cps/reset", post(time::reset_checkpoints))
        .route(
            "/time/:challenge_id/checkpoint/:room_id",
            post(time::toggle_checkpoint),
        )
        .route("/cell/:room_id/:challenge_id", post(grid::update_cell))
        .route("/room/:id", post(grid::rename_room))
        .route("/map/:id/challenges", post(grid::add_challenge))
        .route(
            "/challenge/:id",
            post(grid::rename_challenge).delete(grid::delete_challenge),
        )
        .nest_service("/static", ServeDir::new("static"))
        // The write guard: GETs stay public, everything else needs a
        // session. Sits outside the routes so no handler can forget it.
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth::require_admin_for_writes,
        ))
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
