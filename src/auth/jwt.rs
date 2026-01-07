use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use std::env;

use super::model::Claims;

const DEFAULT_JWT_SECRET: &str = "cakung-barat-jwt-secret-change-in-production";
const ACCESS_TOKEN_EXPIRY_SECONDS: i64 = 15 * 60; // 15 minutes
const REFRESH_TOKEN_EXPIRY_SECONDS: i64 = 7 * 24 * 60 * 60; // 7 days

fn get_jwt_secret() -> String {
    env::var("JWT_SECRET").unwrap_or_else(|_| {
        log::warn!("JWT_SECRET not set, using default secret. SET THIS IN PRODUCTION!");
        DEFAULT_JWT_SECRET.to_string()
    })
}

/// Generate access token (short-lived)
pub fn generate_access_token(
    admin_id: &str,
    username: &str,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = chrono::Utc::now().timestamp() as usize;
    let claims = Claims {
        sub: admin_id.to_string(),
        username: username.to_string(),
        exp: now + ACCESS_TOKEN_EXPIRY_SECONDS as usize,
        iat: now,
        token_type: "access".to_string(),
    };

    let secret = get_jwt_secret();
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

/// Generate refresh token (long-lived)
pub fn generate_refresh_token(
    admin_id: &str,
    username: &str,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = chrono::Utc::now().timestamp() as usize;
    let claims = Claims {
        sub: admin_id.to_string(),
        username: username.to_string(),
        exp: now + REFRESH_TOKEN_EXPIRY_SECONDS as usize,
        iat: now,
        token_type: "refresh".to_string(),
    };

    let secret = get_jwt_secret();
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

/// Validate and decode a token
pub fn validate_token(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let secret = get_jwt_secret();
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(token_data.claims)
}

/// Get access token expiry in seconds
pub fn get_access_token_expiry() -> i64 {
    ACCESS_TOKEN_EXPIRY_SECONDS
}
