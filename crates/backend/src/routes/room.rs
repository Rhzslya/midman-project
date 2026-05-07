use axum::{routing::post, Router};

use crate::{handlers::room::create_room, AppState};

pub fn room_routes() -> Router<AppState> {
    Router::new().route("/create", post(create_room))
}
