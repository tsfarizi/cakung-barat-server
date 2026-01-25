use crate::organization::model::{CreateMemberRequest, OrganizationMember, UpdateMemberRequest};
use crate::organization::persistence::ORGANIZATION_CACHE_KEY;
use crate::AppState;
use actix_web::{web, HttpResponse, Responder};
use log;

async fn write_organization_data(
    state: &web::Data<AppState>,
    members: &Vec<OrganizationMember>,
) -> Result<(), String> {
    // Write-through: Update cache immediately for fast reads
    state
        .organization_cache
        .insert(ORGANIZATION_CACHE_KEY.to_string(), members.clone())
        .await;
    log::info!("Organization cache updated with {} members", members.len());

    // Send to background worker for async persistence to storage
    // This makes the response fast while ensuring eventual consistency
    if let Err(e) = state
        .organization_persist_sender
        .send(members.clone())
        .await
    {
        log::error!("Failed to queue organization data for persistence: {}", e);
        // Note: We still return Ok since cache is up-to-date
        // Data will be available from cache until next restart
    } else {
        log::debug!("Organization data queued for background persistence");
    }

    Ok(())
}

#[utoipa::path(
    get,
    path = "/api/organization",
    tag = "Organization",
    responses(
        (status = 200, description = "List all organization members", body = Vec<OrganizationMember>)
    )
)]
pub async fn get_all_members(state: web::Data<AppState>) -> impl Responder {
    match state.get_organization_structure().await {
        Ok(members) => HttpResponse::Ok().json(members),
        Err(e) => HttpResponse::InternalServerError().body(e),
    }
}

#[utoipa::path(
    post,
    path = "/api/organization",
    tag = "Organization",
    request_body = CreateMemberRequest,
    responses(
        (status = 200, description = "Member created successfully", body = OrganizationMember)
    )
)]
pub async fn create_member(
    state: web::Data<AppState>,
    item: web::Json<CreateMemberRequest>,
) -> impl Responder {
    let mut members = match state.get_organization_structure().await {
        Ok(m) => m,
        Err(e) => return HttpResponse::InternalServerError().body(e),
    };

    let new_id = members.iter().map(|m| m.id).max().unwrap_or(0) + 1;
    let new_member = OrganizationMember {
        id: new_id,
        name: Some(item.name.clone()),
        position: item.position.clone(),
        photo: Some(item.photo.clone()),
        parent_id: item.parent_id,
        level: item.level,
        role: item.role.clone(),
    };

    members.push(new_member.clone());

    match write_organization_data(&state, &members).await {
        Ok(_) => HttpResponse::Ok().json(new_member),
        Err(e) => HttpResponse::InternalServerError().body(e),
    }
}

#[utoipa::path(
    put,
    path = "/api/organization/{id}",
    tag = "Organization",
    params(
        ("id" = i32, Path, description = "Member ID")
    ),
    request_body = UpdateMemberRequest,
    responses(
        (status = 200, description = "Member updated successfully", body = OrganizationMember),
        (status = 404, description = "Member not found")
    )
)]
pub async fn update_member(
    state: web::Data<AppState>,
    path: web::Path<i32>,
    item: web::Json<UpdateMemberRequest>,
) -> impl Responder {
    let id = path.into_inner();
    let mut members = match state.get_organization_structure().await {
        Ok(m) => m,
        Err(e) => return HttpResponse::InternalServerError().body(e),
    };

    if let Some(member) = members.iter_mut().find(|m| m.id == id) {
        if let Some(name) = &item.name {
            member.name = Some(name.clone());
        }
        if let Some(position) = &item.position {
            member.position = position.clone();
        }
        if let Some(photo) = &item.photo {
            member.photo = Some(photo.clone());
        }
        if let Some(parent_id) = item.parent_id {
            member.parent_id = Some(parent_id);
        }
        if let Some(level) = item.level {
            member.level = level;
        }
        if let Some(role) = &item.role {
            member.role = role.clone();
        }
    } else {
        return HttpResponse::NotFound().body("Member not found");
    }

    match write_organization_data(&state, &members).await {
        Ok(_) => {
            // Retrieve updated member to return
            let updated = members.iter().find(|m| m.id == id).unwrap();
            HttpResponse::Ok().json(updated)
        }
        Err(e) => HttpResponse::InternalServerError().body(e),
    }
}

#[utoipa::path(
    delete,
    path = "/api/organization/{id}",
    tag = "Organization",
    params(
        ("id" = i32, Path, description = "Member ID")
    ),
    responses(
        (status = 200, description = "Member deleted successfully"),
        (status = 404, description = "Member not found")
    )
)]
pub async fn delete_member(state: web::Data<AppState>, path: web::Path<i32>) -> impl Responder {
    let id = path.into_inner();
    let mut members = match state.get_organization_structure().await {
        Ok(m) => m,
        Err(e) => return HttpResponse::InternalServerError().body(e),
    };

    let initial_len = members.len();
    members.retain(|m| m.id != id);

    if members.len() == initial_len {
        return HttpResponse::NotFound().body("Member not found");
    }

    match write_organization_data(&state, &members).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(e) => HttpResponse::InternalServerError().body(e),
    }
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/organization")
            .route(web::get().to(get_all_members))
            .route(web::post().to(create_member)),
    )
    .service(
        web::resource("/organization/{id}")
            .route(web::put().to(update_member))
            .route(web::delete().to(delete_member)),
    );
}