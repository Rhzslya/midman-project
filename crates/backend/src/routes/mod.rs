use axum::Router;

use crate::AppState;

pub mod auth;
pub mod room;

pub fn create_router() -> Router<AppState> {
    Router::new()
        .nest("/auth", auth::auth_routes())
        .nest("/room", room::room_routes())
}
