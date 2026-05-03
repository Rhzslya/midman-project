use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};

#[derive(Debug, Serialize, Deserialize, Clone, Validate)]
pub struct RegisterRequest {
    #[validate(
        length(
            min = 3,
            max = 20,
            message = "Name must be between 3 and 20 characters"
        ),
        custom(function = "validate_username_chars")
    )]
    pub username: String,
    #[validate(email(message = "Email is invalid"))]
    pub email: String,
    #[validate(
        length(min = 8, message = "Password must be at least 8 characters"),
        custom(function = "validate_strong_password")
    )]
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthResponse {
    pub token: String,
    pub message: String,
}

fn validate_username_chars(username: &str) -> Result<(), ValidationError> {
    if !username.chars().all(|c| c.is_ascii_alphanumeric()) {
        let mut error = ValidationError::new("invalid_username_chars");
        error.message = Some("Username can only contain alphanumeric characters".into());
        return Err(error);
    }
    Ok(())
}

fn validate_strong_password(password: &str) -> Result<(), ValidationError> {
    let has_uppercase = password.chars().any(|c| c.is_ascii_uppercase());
    let has_lowercase = password.chars().any(|c| c.is_ascii_lowercase());
    let has_number = password.chars().any(|c| c.is_ascii_digit());
    let has_symbol = password.chars().any(|c| !c.is_ascii_alphanumeric());

    if !has_uppercase || !has_lowercase || !has_number || !has_symbol {
        let mut error = ValidationError::new("invalid_password");
        error.message = Some(
            "Password must contain at least one uppercase, lowercase, number and symbol".into(),
        );
        return Err(error);
    }

    Ok(())
}
