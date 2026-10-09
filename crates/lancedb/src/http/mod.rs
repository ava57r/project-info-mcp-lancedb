//! HTTP layer: Qdrant-style REST API + embedded dashboard + snapshots.

pub mod dashboard;
pub mod routes;
pub mod snapshots;

use kameo::actor::ActorRef;

use crate::actors::project_info::ProjectInfoActor;

/// Shared state for HTTP handlers.
#[derive(Clone)]
pub struct AppState {
    /// Project info actor for all DB operations.
    pub actor: ActorRef<ProjectInfoActor>,
    /// LanceDB directory (snapshot source / restore target).
    pub db_dir: String,
    /// Snapshot storage directory.
    pub snapshot_dir: String,
    /// Default project name.
    pub default_project: String,
    /// Embedding model name (shown on dashboard).
    pub model: String,
    /// Vector dimension (shown on dashboard).
    pub vector_dimension: usize,
    /// Server start time (unix seconds) for uptime display.
    pub started_unix: i64,
}

/// Builds the axum router with all REST routes + dashboard.
pub fn router(state: AppState) -> axum::Router {
    use axum::routing::{delete, get, post};
    use tower_http::cors::CorsLayer;

    axum::Router::new()
        .route("/", get(routes::dashboard))
        .route("/dashboard", get(routes::dashboard))
        .route("/static/style.css", get(routes::style_css))
        .route("/static/app.js", get(routes::app_js))
        .route("/healthz", get(routes::healthz))
        .route("/readyz", get(routes::readyz))
        .route("/api/version", get(routes::version))
        .route("/api/collections", get(routes::collections))
        .route("/api/collections/stats", get(routes::collection_stats))
        .route("/api/points", get(routes::list_points))
        .route("/api/points/upsert", post(routes::upsert_point))
        .route("/api/points/search", post(routes::search_points))
        .route("/api/points/{id}", delete(routes::delete_point))
        .route("/api/snapshots", get(routes::list_snapshots))
        .route("/api/snapshots", post(routes::create_snapshot))
        .route(
            "/api/snapshots/{name}/restore",
            post(routes::restore_snapshot),
        )
        .route("/api/snapshots/{name}", delete(routes::delete_snapshot))
        .route(
            "/api/snapshots/{name}/download",
            get(routes::download_snapshot),
        )
        .route("/api/optimize", post(routes::optimize))
        .layer(CorsLayer::permissive())
        .with_state(state)
}
