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

    let insert_result = sqlx::query(
        "INSERT INTO users (username, email, password, full_name) VALUES($1, $2, $3, $4)",
    )
    .bind(payload.username)
    .bind(payload.email)
    .bind(hashed_password)
    .bind(payload.full_name)
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
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use axum::{extract::State, http::StatusCode, Json};
    use sqlx::{postgres::PgPoolOptions, Error, Pool, Postgres};

    use crate::{handlers::user::register_user, model::user::RegisterRequest, AppState};

    #[test]
    fn test() {
        println!("Hello World")
    }

    async fn get_test_pool() -> Result<Pool<Postgres>, Error> {
        let url = "postgres://seira:RootPassword123@localhost:5432/midman-db";
        PgPoolOptions::new()
            .max_connections(10)
            .min_connections(5)
            .acquire_timeout(Duration::from_secs(5))
            .idle_timeout(Duration::from_secs(60))
            .connect(url)
            .await
    }

    #[tokio::test]
    async fn test_register_user_success() {
        let pool = get_test_pool().await.expect("Database connection failed!");
        println!("Database connection success!");
        let state = AppState { pool };

        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();

        let payload = RegisterRequest {
            username: format!("usr{}", time),
            email: format!("testuser{}@test.com", time),
            password: "Password123!".to_string(),
            full_name: "Test User".to_string(),
        };

        let response = register_user(State(state), Json(payload)).await;

        assert!(
            response.is_ok(),
            "Response must be OK, but got {:?}",
            response
        );

        let (status, Json(body)) = response.unwrap();

        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(body.message, "User registered successfully!");
    }

    #[tokio::test]
    async fn test_register_user_failure() {
        let pool = get_test_pool().await.expect("Database connection failed!");
        println!("Database connection success!");
        let state = AppState { pool };

        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();

        let payload = RegisterRequest {
            username: format!("usr{}", time),
            email: format!("testuser{}@test.com", time),
            password: "weak".to_string(),
            full_name: "Test User".to_string(),
        };

        let response = register_user(State(state), Json(payload)).await;

        assert!(
            response.is_err(),
            "Response harusnya error karena password lemah!"
        );

        let (status, error_message) = response.unwrap_err();

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(error_message.contains("Password must contain"));
    }
}
