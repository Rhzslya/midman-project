use axum::{
    extract::FromRequestParts,
    http::{
        header::{self, AUTHORIZATION},
        StatusCode,
    },
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

        Ok(AuthUser {
            user_id: token_data.claims.sub,
        })
    }
}
