use axum::{routing::post, Router};

use crate::{
    handlers::user::{login_user, register_user},
    AppState,
};

pub fn auth_routes() -> Router<AppState> {
    Router::new()
        .route("/register", post(register_user))
        .route("/login", post(login_user))
}
