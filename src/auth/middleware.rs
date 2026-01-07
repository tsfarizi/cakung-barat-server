use actix_web::error::ErrorUnauthorized;
use actix_web::{Error, HttpMessage, HttpRequest};

use super::jwt::validate_token;
use super::model::Claims;

/// Extract token from Authorization header
fn extract_token(req: &HttpRequest) -> Option<String> {
    req.headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|auth| {
            if auth.starts_with("Bearer ") {
                Some(auth[7..].to_string())
            } else {
                None
            }
        })
}

/// Validate token from HttpRequest and return claims
pub fn validate_request_token(req: &HttpRequest) -> Result<Claims, Error> {
    let token =
        extract_token(req).ok_or_else(|| ErrorUnauthorized("Missing authorization token"))?;

    let claims = validate_token(&token).map_err(|e| {
        log::warn!("Token validation failed: {:?}", e);
        ErrorUnauthorized("Invalid or expired token")
    })?;

    if claims.token_type != "access" {
        return Err(ErrorUnauthorized("Invalid token type"));
    }

    Ok(claims)
}

/// Extension trait for requests to get admin claims
pub trait AdminClaimsExt {
    fn get_admin_claims(&self) -> Option<Claims>;
}

impl<T: HttpMessage> AdminClaimsExt for T {
    fn get_admin_claims(&self) -> Option<Claims> {
        self.extensions().get::<Claims>().cloned()
    }
}
