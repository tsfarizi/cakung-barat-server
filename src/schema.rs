use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, ToSchema, Eq, Hash, PartialEq)]
#[schema(value_type = String, format = "uuid")]
pub struct Uuid(pub uuid::Uuid);

#[derive(Debug, Serialize, Deserialize, Clone, Copy, ToSchema, PartialEq)]
#[schema(value_type = String, format = "date")]
pub struct NaiveDate(pub chrono::NaiveDate);
