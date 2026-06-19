use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, StatusCode},
};
use jsonwebtoken::{decode, DecodingKey, Validation};

use crate::{model::user::Claims, AppState};

pub struct AuthUser {
    pub user_id: i32,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = (StatusCode, String);

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // 1. Ambil Header
        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok());

        let auth_header = match auth_header {
            Some(header) => header,
            None => {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    "Access Denied : token is missing".to_string(),
                ));
            }
        };

        if !auth_header.starts_with("Bearer ") {
            return Err((
                StatusCode::UNAUTHORIZED,
                "Access Denied : token is invalid".to_string(),
            ));
        }

        let token = &auth_header[7..];

        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|_| {
            (
                StatusCode::UNAUTHORIZED,
                "Access Denied : token is invalid or expired".to_string(),
            )
        })?;

        let is_blacklisted: Option<(String,)> =
            sqlx::query_as("SELECT token FROM token_blacklist WHERE token = $1")
                .bind(token)
                .fetch_optional(&state.pool)
                .await
                .map_err(|e| {
                    eprintln!("DB Error (Middleware): {:?}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Database error while checking token".to_string(),
                    )
                })?;

        if is_blacklisted.is_some() {
            return Err((
                StatusCode::UNAUTHORIZED,
                "Access Denied : token has been revoked (logged out)".to_string(),
            ));
        }

        Ok(AuthUser {
            user_id: token_data.claims.sub,
        })
    }
}
