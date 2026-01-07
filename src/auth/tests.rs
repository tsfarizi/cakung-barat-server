//! Unit tests for authentication module

#[cfg(test)]
mod tests {
    use crate::auth::jwt::{generate_access_token, generate_refresh_token, validate_token};
    use crate::auth::model::{Admin, AdminInfo, Claims, LoginRequest, TokenResponse};
    use uuid::Uuid;

    #[test]
    fn test_generate_and_validate_access_token() {
        let admin_id = Uuid::new_v4().to_string();
        let username = "testuser";

        let token =
            generate_access_token(&admin_id, username).expect("Failed to generate access token");

        let claims = validate_token(&token).expect("Failed to validate token");

        assert_eq!(claims.sub, admin_id);
        assert_eq!(claims.username, username);
        assert_eq!(claims.token_type, "access");
    }

    #[test]
    fn test_generate_and_validate_refresh_token() {
        let admin_id = Uuid::new_v4().to_string();
        let username = "testuser";

        let token =
            generate_refresh_token(&admin_id, username).expect("Failed to generate refresh token");

        let claims = validate_token(&token).expect("Failed to validate token");

        assert_eq!(claims.sub, admin_id);
        assert_eq!(claims.username, username);
        assert_eq!(claims.token_type, "refresh");
    }

    #[test]
    fn test_token_contains_correct_claims() {
        let admin_id = "test-admin-id";
        let username = "admin";

        let token = generate_access_token(admin_id, username).expect("Failed to generate token");

        let claims = validate_token(&token).expect("Failed to validate token");

        assert!(!claims.sub.is_empty());
        assert!(!claims.username.is_empty());
        assert!(claims.exp > claims.iat);
    }

    #[test]
    fn test_invalid_token_returns_error() {
        let result = validate_token("invalid.token.here");
        assert!(result.is_err());
    }

    #[test]
    fn test_admin_to_admin_info_conversion() {
        let admin = Admin {
            id: Uuid::new_v4(),
            username: "testadmin".to_string(),
            password_hash: "hashedpassword".to_string(),
            display_name: Some("Test Admin".to_string()),
            refresh_token: Some("refresh_token_here".to_string()),
            created_at: Some(chrono::Utc::now()),
            updated_at: Some(chrono::Utc::now()),
            created_by: None,
        };

        let info: AdminInfo = admin.clone().into();

        assert_eq!(info.id, admin.id);
        assert_eq!(info.username, admin.username);
        assert_eq!(info.display_name, admin.display_name);
        // AdminInfo should not contain sensitive fields like password_hash or refresh_token
    }

    #[test]
    fn test_claims_clone() {
        let claims = Claims {
            sub: "test-id".to_string(),
            username: "testuser".to_string(),
            exp: 12345,
            iat: 12340,
            token_type: "access".to_string(),
        };

        let cloned = claims.clone();

        assert_eq!(claims.sub, cloned.sub);
        assert_eq!(claims.username, cloned.username);
        assert_eq!(claims.exp, cloned.exp);
        assert_eq!(claims.iat, cloned.iat);
        assert_eq!(claims.token_type, cloned.token_type);
    }

    #[test]
    fn test_login_request_deserialize() {
        let json = r#"{"username": "admin", "password": "admin123"}"#;
        let request: LoginRequest = serde_json::from_str(json).expect("Failed to deserialize");

        assert_eq!(request.username, "admin");
        assert_eq!(request.password, "admin123");
    }

    #[test]
    fn test_token_response_serialize() {
        let response = TokenResponse {
            access_token: "access".to_string(),
            refresh_token: "refresh".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: 900,
            setup_mode: false,
        };

        let json = serde_json::to_string(&response).expect("Failed to serialize");

        assert!(json.contains("access_token"));
        assert!(json.contains("refresh_token"));
        assert!(json.contains("token_type"));
        assert!(json.contains("expires_in"));
        assert!(json.contains("setup_mode"));
    }

    #[test]
    fn test_access_token_expiry_is_shorter_than_refresh() {
        let admin_id = "test-id";
        let username = "testuser";

        let access_token =
            generate_access_token(admin_id, username).expect("Failed to generate access token");
        let refresh_token =
            generate_refresh_token(admin_id, username).expect("Failed to generate refresh token");

        let access_claims = validate_token(&access_token).expect("Failed to validate access token");
        let refresh_claims =
            validate_token(&refresh_token).expect("Failed to validate refresh token");

        // Refresh token should expire later than access token
        assert!(refresh_claims.exp > access_claims.exp);
    }
}
