use crate::models::Claims;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use std::env;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum JwtError {
    #[allow(dead_code)]
    #[error("Invalid token")]
    InvalidToken,
    #[error("Token creation error")]
    TokenCreation,
}

pub fn create_jwt(user_id: &str, email: &str) -> Result<String, JwtError> {
    let secret = env::var("JWT_SECRET").unwrap_or_else(|_| "dev_secret".to_string());
    let expiration_hours: i64 = env::var("JWT_EXPIRATION_HOURS")
        .ok()
        .and_then(|h| h.parse().ok())
        .unwrap_or(24);

    let exp = (chrono::Utc::now() + chrono::Duration::hours(expiration_hours)).timestamp() as usize;

    let claims = Claims {
        sub: user_id.to_string(),
        email: email.to_string(),
        exp,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|_| JwtError::TokenCreation)
}

#[allow(dead_code)]
pub fn verify_jwt(token: &str) -> Result<Claims, JwtError> {
    let secret = env::var("JWT_SECRET").unwrap_or_else(|_| "dev_secret".to_string());

    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|_| JwtError::InvalidToken)
}
