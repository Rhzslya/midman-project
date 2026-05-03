use axum::{extract::State, http::StatusCode, Json};
use bcrypt::{hash, DEFAULT_COST};
use validator::Validate;

use crate::model::user::{AuthResponse, RegisterRequest};

use crate::AppState;

pub async fn register_user(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), (StatusCode, String)> {
    if let Err(errors) = payload.validate() {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("Validation error: {:?}", errors),
        ));
    }

    let existing_user: Option<(String, String)> =
        sqlx::query_as("SELECT username, email FROM users WHERE email = $1 OR username = $2")
            .bind(&payload.email)
            .bind(&payload.username)
            .fetch_optional(&state.pool)
            .await
            .map_err(|e| {
                eprintln!("DB Error : {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "DB Error".to_string())
            })?;

    if let Some((existing_username, existing_email)) = existing_user {
        if existing_email == payload.email {
            return Err((StatusCode::CONFLICT, "Email already exists".to_string()));
        }

        if existing_username == payload.username {
            return Err((StatusCode::CONFLICT, "Username already exists".to_string()));
        }
    }

    let hashed_password = hash(payload.password.as_bytes(), DEFAULT_COST).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Password hashing failed!".to_string(),
        )
    })?;

    let insert_result =
        sqlx::query("INSERT INTO users (username, email, password) VALUES($1, $2, $3)")
            .bind(payload.username)
            .bind(payload.email)
            .bind(hashed_password)
            .execute(&state.pool)
            .await;

    match insert_result {
        Ok(_) => {
            let response = AuthResponse {
                token: "TODO".to_string(),
                message: "User registered successfully!".to_string(),
            };
            Ok((StatusCode::CREATED, Json(response)))
        }
        Err(e) => {
            eprintln!("DB Error : {:?}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "DB Error".to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test() {
        println!("Hello World")
    }
}
