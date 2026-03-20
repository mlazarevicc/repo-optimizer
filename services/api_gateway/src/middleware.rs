use axum::{
    extract::Request,
    http::{StatusCode, header},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String, // user_id
    pub email: String,
    pub exp: usize,
}

pub async fn auth_middleware(mut req: Request, next: Next) -> Result<Response, StatusCode> {
    // 1. Izvuci Authorization header
    let auth_header = req.headers().get(header::AUTHORIZATION);

    let auth_header = match auth_header {
        Some(header) => header.to_str().map_err(|_| StatusCode::UNAUTHORIZED)?,
        None => return Err(StatusCode::UNAUTHORIZED),
    };

    // 2. Proveri da li počinje sa "Bearer "
    if !auth_header.starts_with("Bearer ") {
        return Err(StatusCode::UNAUTHORIZED);
    }

    // 3. Iseci "Bearer " prefiks da dobiješ sam token
    let token = &auth_header[7..];
    
    // 4. Dekodiraj i verifikuj token
    let secret = env::var("JWT_SECRET").unwrap_or_else(|_| "dev_secret".to_string());

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )
    .map_err(|e| {
        tracing::error!("JWT verifikacija propala: {}", e);
        StatusCode::UNAUTHORIZED
    })?;

    // 5. Ubaci podatke o korisniku u request ekstenzije kako bi rute mogle da ih čitaju
    req.extensions_mut().insert(token_data.claims);

    // Pusti zahtev dalje ka kontroleru
    Ok(next.run(req).await)
}