use std::time::{SystemTime, UNIX_EPOCH};

use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::{extract::State, http::StatusCode, Json};
use bcrypt::{hash, verify, DEFAULT_COST};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde_json::json;
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
    let user_id = user.user_id;

    (StatusCode::OK, Json(json!({ "id": user_id })))
}

pub async fn logout_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    user: AuthUser,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, String)> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .filter(|h| h.starts_with("Bearer "))
        .map(|h| h[7..].to_string());

    let token = match auth_header {
        Some(t) => t,
        None => {
            return Err((
                StatusCode::BAD_REQUEST,
                "Access Denied : token is missing".to_string(),
            ));
        }
    };

    let insert_result = sqlx::query("INSERT INTO token_blacklist (token) VALUES ($1)")
        .bind(token)
        .execute(&state.pool)
        .await;

    match insert_result {
        Ok(_) => Ok((
            StatusCode::OK,
            Json(json!({
                "message":format!("User {} Success logout user",user.user_id)
            })),
        )),
        Err(e) => {
            eprintln!("DB Error (Logout): {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to logout".to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use axum::{
        extract::State,
        http::StatusCode,
        routing::{get, post},
        Json, Router,
    };

    use tower::ServiceExt;

    use crate::{
        handlers::user::{get_my_profile, login_user, logout_user, register_user},
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

    #[sqlx::test]
    async fn test_get_my_profile(pool: sqlx::PgPool) {
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

        let response = login_user(State(state.clone()), Json(login_payload))
            .await
            .unwrap();
        let (_, Json(body)) = response;
        let token = body.token;

        use axum::{routing::get, Router};
        use tower::ServiceExt;

        let app = Router::new()
            .route("/api/user/me", get(get_my_profile))
            .with_state(state);

        let request = axum::http::Request::builder()
            .uri("/api/user/me")
            .header("Authorization", format!("Bearer {}", token))
            .body(axum::body::Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_text = String::from_utf8(body_bytes.to_vec()).unwrap();

        assert!(body_text.contains("\"id\":"));
    }

    #[sqlx::test]
    async fn test_get_my_profile_failure(pool: sqlx::PgPool) {
        let state = AppState {
            pool,
            jwt_secret: "SECRET_KEY".to_string(),
        };

        use axum::{routing::get, Router};
        use tower::ServiceExt;

        let app = Router::new()
            .route("/api/user/me", get(get_my_profile))
            .with_state(state);

        let request = axum::http::Request::builder()
            .uri("/api/user/me")
            .body(axum::body::Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_text = String::from_utf8(body_bytes.to_vec()).unwrap();
        assert_eq!(body_text, "Access Denied : token is missing")
    }

    #[sqlx::test]
    async fn test_logout_user_and_blacklist(pool: sqlx::PgPool) {
        let state = AppState {
            pool,
            jwt_secret: "SECRET_KEY".to_string(),
        };

        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let password = "SuperSecretPassword123!".to_string();
        let email = format!("logouttest{}@test.com", time);

        let reg_payload = RegisterRequest {
            username: format!("out{}", time),
            email: email.clone(),
            password: password.clone(),
            full_name: "Logout Tester".to_string(),
        };
        let _ = register_user(State(state.clone()), Json(reg_payload))
            .await
            .unwrap();

        let login_payload = LoginRequest {
            identifier: email,
            password,
        };
        let (_, Json(body)) = login_user(State(state.clone()), Json(login_payload))
            .await
            .unwrap();
        let token = body.token;

        let app: Router = Router::new()
            .route("/api/user/me", get(get_my_profile))
            .route("/api/user/logout", post(logout_user))
            .with_state(state);

        let req_profile_before = axum::http::Request::builder()
            .uri("/api/user/me")
            .header("Authorization", format!("Bearer {}", token))
            .body(axum::body::Body::empty())
            .unwrap();

        let res_profile_before = app.clone().oneshot(req_profile_before).await.unwrap();
        assert_eq!(res_profile_before.status(), StatusCode::OK);

        let req_logout = axum::http::Request::builder()
            .method("POST")
            .uri("/api/user/logout")
            .header("Authorization", format!("Bearer {}", token))
            .body(axum::body::Body::empty())
            .unwrap();

        let res_logout = app.clone().oneshot(req_logout).await.unwrap();
        assert_eq!(res_logout.status(), StatusCode::OK);

        let req_profile_after = axum::http::Request::builder()
            .uri("/api/user/me")
            .header("Authorization", format!("Bearer {}", token))
            .body(axum::body::Body::empty())
            .unwrap();

        let res_profile_after = app.oneshot(req_profile_after).await.unwrap();

        assert_eq!(res_profile_after.status(), StatusCode::UNAUTHORIZED);

        let body_bytes = axum::body::to_bytes(res_profile_after.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_text = String::from_utf8(body_bytes.to_vec()).unwrap();
        assert_eq!(
            body_text,
            "Access Denied : token has been revoked (logged out)"
        );
    }
}
