//! Tool definition for Surat Pernyataan Belum Memiliki Rumah (KPR).

use serde_json::{Value, json};

use super::registry::ToolDescriptor;

pub const TOOL_NAME: &str = "generate_surat_kpr_belum_punya_rumah";

/// Get the tool descriptor for MCP tools/list.
pub fn descriptor() -> ToolDescriptor {
    ToolDescriptor {
        name: TOOL_NAME.to_string(),
        description: concat!(
            "Membuat Surat Pernyataan Belum Memiliki Rumah dalam format PDF. ",
            "Surat ini digunakan untuk keperluan pengajuan KPR (Kredit Pemilikan Rumah) di bank. ",
            "[PENTING] INSTRUKSI PENGGUNAAN: ",
            "(1) WAJIB tanyakan semua data kepada warga SEBELUM memanggil tool ini. ",
            "(2) Data yang harus dikumpulkan: nama lengkap, NIK (16 digit), ",
            "tempat/tanggal lahir, jenis kelamin, agama, pekerjaan, alamat lengkap, nomor telepon. ",
            "(3) Tanyakan juga nama bank tujuan KPR (contoh: BTN, BRI, Mandiri). ",
            "(4) DILARANG menggunakan data contoh/dummy seperti 'John Doe' atau NIK palsu. ",
            "(5) Jika data belum lengkap, minta warga melengkapinya terlebih dahulu."
        ).to_string(),
        input_schema: input_schema(),
    }
}

fn input_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "data": {
                "type": "object",
                "description": "Data pemohon KPR",
                "properties": {
                    "nama": { "type": "string", "description": "Nama lengkap pemohon" },
                    "nik": { "type": "string", "description": "NIK (16 digit)" },
                    "ttl": { "type": "string", "description": "Tempat, Tanggal Lahir" },
                    "jk": { "type": "string", "description": "Jenis Kelamin (Laki-laki/Perempuan)" },
                    "agama": { "type": "string", "description": "Agama" },
                    "pekerjaan": { "type": "string", "description": "Pekerjaan" },
                    "alamat": { "type": "string", "description": "Alamat lengkap" },
                    "telp": { "type": "string", "description": "Nomor telepon/HP" }
                },
                "required": ["nama", "nik", "ttl", "jk", "agama", "pekerjaan", "alamat", "telp"]
            },
            "meta": {
                "type": "object",
                "description": "Metadata surat",
                "properties": {
                    "kelurahan": { "type": "string", "description": "Nama kelurahan" },
                    "bank_tujuan": { "type": "string", "description": "Nama bank tujuan KPR" },
                    "tanggal": { "type": "string", "description": "Tanggal surat (opsional, default: hari ini)" }
                },
                "required": ["kelurahan", "bank_tujuan"]
            }
        },
        "required": ["data", "meta"]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_descriptor() {
        let desc = descriptor();
        assert_eq!(desc.name, TOOL_NAME);
        assert!(desc.description.contains("KPR"));
        assert!(desc.input_schema.get("properties").is_some());
    }
}
