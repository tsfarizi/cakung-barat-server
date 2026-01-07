use actix_web::{web, HttpRequest, HttpResponse, Responder};
use bcrypt::{hash, verify, DEFAULT_COST};

use super::jwt::{
    generate_access_token, generate_refresh_token, get_access_token_expiry, validate_token,
};
use super::middleware::validate_request_token;
use super::model::{
    AdminInfo, AuthStatusResponse, CreateAdminRequest, LoginRequest, RefreshRequest, TokenResponse,
};
use crate::AppState;

const DEFAULT_ADMIN_USERNAME: &str = "admin";
const DEFAULT_ADMIN_PASSWORD: &str = "admin123";

/// Check if setup is required (no admins exist)
#[utoipa::path(
    get,
    path = "/api/auth/status",
    tag = "Authentication",
    responses(
        (status = 200, description = "Auth status", body = AuthStatusResponse)
    )
)]
pub async fn get_auth_status(state: web::Data<AppState>) -> impl Responder {
    let count = state.get_admin_count().await.unwrap_or(0);
    HttpResponse::Ok().json(AuthStatusResponse {
        has_admins: count > 0,
        setup_required: count == 0,
    })
}

/// Login endpoint
#[utoipa::path(
    post,
    path = "/api/auth/login",
    tag = "Authentication",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = TokenResponse),
        (status = 401, description = "Invalid credentials")
    )
)]
pub async fn login(state: web::Data<AppState>, body: web::Json<LoginRequest>) -> impl Responder {
    let admin_count = state.get_admin_count().await.unwrap_or(0);

    // First-time setup mode: allow login with default credentials
    if admin_count == 0 {
        if body.username == DEFAULT_ADMIN_USERNAME && body.password == DEFAULT_ADMIN_PASSWORD {
            // Generate temporary tokens for setup mode
            let temp_id = "setup-mode";
            let access_token = match generate_access_token(temp_id, &body.username) {
                Ok(t) => t,
                Err(e) => {
                    log::error!("Failed to generate access token: {:?}", e);
                    return HttpResponse::InternalServerError().json(
                        crate::ErrorResponse::internal_error("Failed to generate token"),
                    );
                }
            };

            let refresh_token = match generate_refresh_token(temp_id, &body.username) {
                Ok(t) => t,
                Err(e) => {
                    log::error!("Failed to generate refresh token: {:?}", e);
                    return HttpResponse::InternalServerError().json(
                        crate::ErrorResponse::internal_error("Failed to generate token"),
                    );
                }
            };

            return HttpResponse::Ok().json(TokenResponse {
                access_token,
                refresh_token,
                token_type: "Bearer".to_string(),
                expires_in: get_access_token_expiry(),
                setup_mode: true,
            });
        } else {
            return HttpResponse::Unauthorized().json(crate::ErrorResponse::new(
                "Unauthorized",
                "Invalid credentials. Use admin/admin123 for first-time setup.",
            ));
        }
    }

    // Normal login flow
    let admin = match state.get_admin_by_username(&body.username).await {
        Ok(Some(admin)) => admin,
        Ok(None) => {
            return HttpResponse::Unauthorized().json(crate::ErrorResponse::new(
                "Unauthorized",
                "Invalid username or password",
            ));
        }
        Err(e) => {
            log::error!("Database error during login: {:?}", e);
            return HttpResponse::InternalServerError()
                .json(crate::ErrorResponse::internal_error("Login failed"));
        }
    };

    // Verify password
    let password_valid = verify(&body.password, &admin.password_hash).unwrap_or(false);
    if !password_valid {
        return HttpResponse::Unauthorized().json(crate::ErrorResponse::new(
            "Unauthorized",
            "Invalid username or password",
        ));
    }

    // Generate tokens
    let admin_id = admin.id.to_string();
    let access_token = match generate_access_token(&admin_id, &admin.username) {
        Ok(t) => t,
        Err(e) => {
            log::error!("Failed to generate access token: {:?}", e);
            return HttpResponse::InternalServerError().json(crate::ErrorResponse::internal_error(
                "Failed to generate token",
            ));
        }
    };

    let refresh_token = match generate_refresh_token(&admin_id, &admin.username) {
        Ok(t) => t,
        Err(e) => {
            log::error!("Failed to generate refresh token: {:?}", e);
            return HttpResponse::InternalServerError().json(crate::ErrorResponse::internal_error(
                "Failed to generate token",
            ));
        }
    };

    // Store refresh token in database (invalidates any previous session)
    if let Err(e) = state
        .update_admin_refresh_token(&admin.id, &refresh_token)
        .await
    {
        log::error!("Failed to store refresh token: {:?}", e);
        // Continue anyway, token is still valid
    }

    HttpResponse::Ok().json(TokenResponse {
        access_token,
        refresh_token,
        token_type: "Bearer".to_string(),
        expires_in: get_access_token_expiry(),
        setup_mode: false,
    })
}

/// Refresh access token
#[utoipa::path(
    post,
    path = "/api/auth/refresh",
    tag = "Authentication",
    request_body = RefreshRequest,
    responses(
        (status = 200, description = "Token refreshed", body = TokenResponse),
        (status = 401, description = "Invalid refresh token")
    )
)]
pub async fn refresh_token(
    state: web::Data<AppState>,
    body: web::Json<RefreshRequest>,
) -> impl Responder {
    // Validate refresh token
    let claims = match validate_token(&body.refresh_token) {
        Ok(c) => c,
        Err(e) => {
            log::warn!("Invalid refresh token: {:?}", e);
            return HttpResponse::Unauthorized().json(crate::ErrorResponse::new(
                "Unauthorized",
                "Invalid or expired refresh token",
            ));
        }
    };

    if claims.token_type != "refresh" {
        return HttpResponse::Unauthorized().json(crate::ErrorResponse::new(
            "Unauthorized",
            "Invalid token type",
        ));
    }

    // Check if this refresh token matches what's in database (single device session)
    let admin = match state.get_admin_by_refresh_token(&body.refresh_token).await {
        Ok(Some(admin)) => admin,
        Ok(None) => {
            return HttpResponse::Unauthorized().json(crate::ErrorResponse::new(
                "Unauthorized",
                "Session expired. Please login again.",
            ));
        }
        Err(e) => {
            log::error!("Database error during refresh: {:?}", e);
            return HttpResponse::InternalServerError()
                .json(crate::ErrorResponse::internal_error("Refresh failed"));
        }
    };

    // Generate new access token only (keep same refresh token)
    let admin_id = admin.id.to_string();
    let access_token = match generate_access_token(&admin_id, &admin.username) {
        Ok(t) => t,
        Err(e) => {
            log::error!("Failed to generate access token: {:?}", e);
            return HttpResponse::InternalServerError().json(crate::ErrorResponse::internal_error(
                "Failed to generate token",
            ));
        }
    };

    HttpResponse::Ok().json(TokenResponse {
        access_token,
        refresh_token: body.refresh_token.clone(),
        token_type: "Bearer".to_string(),
        expires_in: get_access_token_expiry(),
        setup_mode: false,
    })
}

/// Create new admin (protected - requires admin auth)
#[utoipa::path(
    post,
    path = "/api/auth/admins",
    tag = "Authentication",
    request_body = CreateAdminRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 201, description = "Admin created", body = AdminInfo),
        (status = 401, description = "Unauthorized"),
        (status = 409, description = "Username already exists")
    )
)]
pub async fn create_admin(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<CreateAdminRequest>,
) -> impl Responder {
    // Check authorization
    let claims = match validate_request_token(&req) {
        Ok(c) => c,
        Err(e) => return e.error_response(),
    };

    // Get creator admin id (might be "setup-mode" for first admin)
    let created_by = if claims.sub == "setup-mode" {
        None
    } else {
        uuid::Uuid::parse_str(&claims.sub).ok()
    };

    // Check if username already exists
    if let Ok(Some(_)) = state.get_admin_by_username(&body.username).await {
        return HttpResponse::Conflict().json(crate::ErrorResponse::new(
            "Conflict",
            "Username already exists",
        ));
    }

    // Hash password
    let password_hash = match hash(&body.password, DEFAULT_COST) {
        Ok(h) => h,
        Err(e) => {
            log::error!("Failed to hash password: {:?}", e);
            return HttpResponse::InternalServerError().json(crate::ErrorResponse::internal_error(
                "Failed to create admin",
            ));
        }
    };

    // Create admin
    let admin = match state
        .create_admin(
            &body.username,
            &password_hash,
            body.display_name.as_deref(),
            created_by,
        )
        .await
    {
        Ok(admin) => admin,
        Err(e) => {
            log::error!("Failed to create admin: {:?}", e);
            return HttpResponse::InternalServerError().json(crate::ErrorResponse::internal_error(
                "Failed to create admin",
            ));
        }
    };

    HttpResponse::Created().json(AdminInfo::from(admin))
}

/// List all admins (protected)
#[utoipa::path(
    get,
    path = "/api/auth/admins",
    tag = "Authentication",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Admin list", body = Vec<AdminInfo>),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn list_admins(req: HttpRequest, state: web::Data<AppState>) -> impl Responder {
    // Check authorization
    if let Err(e) = validate_request_token(&req) {
        return e.error_response();
    }

    match state.get_all_admins().await {
        Ok(admins) => {
            let admin_infos: Vec<AdminInfo> = admins.into_iter().map(AdminInfo::from).collect();
            HttpResponse::Ok().json(admin_infos)
        }
        Err(e) => {
            log::error!("Failed to get admins: {:?}", e);
            HttpResponse::InternalServerError()
                .json(crate::ErrorResponse::internal_error("Failed to get admins"))
        }
    }
}

/// Delete admin (protected)
#[utoipa::path(
    delete,
    path = "/api/auth/admins/{id}",
    tag = "Authentication",
    params(("id" = String, Path, description = "Admin ID")),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Admin deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Admin not found")
    )
)]
pub async fn delete_admin(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<uuid::Uuid>,
) -> impl Responder {
    // Check authorization
    let claims = match validate_request_token(&req) {
        Ok(c) => c,
        Err(e) => return e.error_response(),
    };

    let admin_id = path.into_inner();

    // Prevent self-deletion
    if claims.sub == admin_id.to_string() {
        return HttpResponse::BadRequest().json(crate::ErrorResponse::bad_request(
            "Cannot delete your own account",
        ));
    }

    // Ensure at least one admin remains
    let admin_count = state.get_admin_count().await.unwrap_or(0);
    if admin_count <= 1 {
        return HttpResponse::BadRequest().json(crate::ErrorResponse::bad_request(
            "Cannot delete the last admin",
        ));
    }

    match state.delete_admin(&admin_id).await {
        Ok(true) => HttpResponse::Ok().finish(),
        Ok(false) => {
            HttpResponse::NotFound().json(crate::ErrorResponse::not_found("Admin not found"))
        }
        Err(e) => {
            log::error!("Failed to delete admin: {:?}", e);
            HttpResponse::InternalServerError().json(crate::ErrorResponse::internal_error(
                "Failed to delete admin",
            ))
        }
    }
}

/// Configure auth routes
pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/auth")
            .route("/status", web::get().to(get_auth_status))
            .route("/login", web::post().to(login))
            .route("/refresh", web::post().to(refresh_token))
            .route("/admins", web::get().to(list_admins))
            .route("/admins", web::post().to(create_admin))
            .route("/admins/{id}", web::delete().to(delete_admin)),
    );
}
