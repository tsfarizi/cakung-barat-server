use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct OrganizationMember {
    pub id: i32,
    pub name: Option<String>,
    pub position: String,
    pub photo: Option<String>,
    pub parent_id: Option<i32>,
    pub level: i32,
    pub role: String,
}

#[derive(Deserialize, Serialize, Debug, ToSchema)]
pub struct CreateMemberRequest {
    pub name: String,
    pub position: String,
    pub photo: String,
    pub parent_id: Option<i32>,
    pub level: i32,
    pub role: String,
}

#[derive(Deserialize, Serialize, Debug, ToSchema)]
pub struct UpdateMemberRequest {
    pub name: Option<String>,
    pub position: Option<String>,
    pub photo: Option<String>,
    pub parent_id: Option<i32>,
    pub level: Option<i32>,
    pub role: Option<String>,
}
