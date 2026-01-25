//! MCP Tool for organization structure.
//!
//! Provides access to the organization structure of Kelurahan Cakung Barat.

use serde_json::json;
use super::registry::ToolDescriptor;

pub const GET_ORGANIZATION_STRUCTURE_TOOL: &str = "get_organization_structure";

pub fn get_organization_structure_descriptor() -> ToolDescriptor {
    ToolDescriptor {
        name: GET_ORGANIZATION_STRUCTURE_TOOL.to_string(),
        description: concat!(
            "Melihat struktur organisasi Kelurahan Cakung Barat. ",
            "Tool ini mengembalikan daftar anggota organisasi beserta jabatan, peran, dan hirarkinya. ",
            "Gunakan tool ini untuk mengetahui siapa yang menjabat posisi tertentu."
        )
        .to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {}
        }),
    }
}
