use std::time::{SystemTime, UNIX_EPOCH};

use axum::response::IntoResponse;
use axum::{extract::State, http::StatusCode, Json};
use bcrypt::{hash, verify, DEFAULT_COST};
use jsonwebtoken::{encode, EncodingKey, Header};
use validator::Validate;

use crate::middleware::AuthUser;
use crate::model::user::{
    Claims, LoginRequest, LoginResponse, RegisterRequest, RegisterResponse, User, UserRow,
};

use crate::AppState;

pub async fn register_user(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<RegisterResponse>), (StatusCode, String)> {
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
            let response = RegisterResponse {
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

pub async fn login_user(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<(StatusCode, Json<LoginResponse>), (StatusCode, String)> {
    let db_user: Option<UserRow> = sqlx::query_as(
        "SELECT id, full_name, username, email, created_at, updated_at, password FROM users WHERE email = $1 OR username = $1",
    ).bind(&payload.identifier)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| {
        eprintln!("DB Error : {:?}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "Database Error".to_string())
    })?;

    let user = match db_user {
        Some(u) => u,
        None => {
            return Err((
                StatusCode::UNAUTHORIZED,
                "Email/Username or Password is Invalid".to_string(),
            ))
        }
    };

    let is_password_valid = verify(payload.password, &user.password).unwrap_or(false);

    if !is_password_valid {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Email/Username or Password is Invalid".to_string(),
        ));
    }

    let expiration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize
        + (60 * 60 * 24);

    let claims = Claims {
        sub: user.id,
        exp: expiration,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    )
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed Create Token".to_string(),
        )
    })?;

    let response = LoginResponse {
        token,
        user: User {
            id: user.id,
            full_name: user.full_name,
            username: user.username,
            email: user.email,
            created_at: user.created_at,
            updated_at: user.updated_at,
        },
    };

    Ok((StatusCode::OK, Json(response)))
}

pub async fn get_my_profile(user: AuthUser) -> impl IntoResponse {
    let pesan = format!(
        "Selamat datang di area VIP! ID kamu adalah: {}",
        user.user_id
    );

    (StatusCode::OK, pesan)
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use axum::{extract::State, http::StatusCode, Json};
    use sqlx::{pool, postgres::PgPoolOptions, Error, Pool, Postgres};

    use crate::{
        handlers::user::{login_user, register_user},
        model::user::{LoginRequest, RegisterRequest},
        AppState,
    };

    #[test]
    fn test() {
        println!("Hello World")
    }

    #[sqlx::test]
    async fn test_register_user_success(pool: sqlx::PgPool) {
        let state = AppState {
            pool,
            jwt_secret: "SECRET_KEY".to_string(),
        };

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

    #[sqlx::test]
    async fn test_register_user_failure(pool: sqlx::PgPool) {
        let state = AppState {
            pool,
            jwt_secret: "SECRET_KEY".to_string(),
        };

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
            "Response must be Err, because password is weak",
        );

        let (status, error_message) = response.unwrap_err();

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(error_message.contains("Password must contain"));
    }

    #[sqlx::test]
    async fn test_login_user_success(pool: sqlx::PgPool) {
        let state = AppState {
            pool,
            jwt_secret: "SECRET_KEY".to_string(),
        };

        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();

        let password = "SuperSecretPassword123!".to_string();
        let email = format!("logintest{}@test.com", time);
        let username = format!("logusr{}", time);

        let reg_payload = RegisterRequest {
            username: username.clone(),
            email: email.clone(),
            password: password.clone(),
            full_name: "Login Tester".to_string(),
        };
        let _ = register_user(State(state.clone()), Json(reg_payload))
            .await
            .unwrap();

        let login_payload = LoginRequest {
            identifier: email,
            password: password.clone(),
        };

        let response = login_user(State(state), Json(login_payload)).await;

        assert!(
            response.is_ok(),
            "Login Must be Successful, but got {:?}",
            response.err()
        );

        let (status, Json(body)) = response.unwrap();

        assert_eq!(status, StatusCode::OK);
        assert!(!body.token.is_empty(), "Token must not be empty");
        assert_eq!(body.user.username, username, "Username must be same");
        assert_eq!(body.user.full_name, "Login Tester");
    }
}
