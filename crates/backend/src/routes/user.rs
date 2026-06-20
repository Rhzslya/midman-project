use axum::{routing::get, Router};

use crate::{handlers::user::get_my_profile, AppState};

pub fn user_routes() -> Router<AppState> {
    Router::new().route("/me", get(get_my_profile))
}
