//! Tool definition for Surat Pernyataan Tidak Mampu (SKTM).

use serde_json::{Value, json};

use super::registry::ToolDescriptor;

pub const TOOL_NAME: &str = "generate_surat_tidak_mampu";

/// Get the tool descriptor for MCP tools/list.
pub fn descriptor() -> ToolDescriptor {
    ToolDescriptor {
        name: TOOL_NAME.to_string(),
        description: concat!(
            "Membuat Surat Pernyataan Tidak Mampu (SKTM) dalam format PDF. ",
            "Surat ini digunakan untuk keperluan bantuan sosial, keringanan biaya pendidikan, ",
            "atau layanan kesehatan bagi warga yang berasal dari keluarga tidak mampu. ",
            "[PENTING] INSTRUKSI PENGGUNAAN: ",
            "(1) WAJIB tanyakan semua data kepada warga SEBELUM memanggil tool ini. ",
            "(2) Data pengisi yang harus dikumpulkan: nama lengkap, NIK (16 digit), ",
            "tempat/tanggal lahir, jenis kelamin, agama, pekerjaan, alamat lengkap, nomor telepon. ",
            "(3) Tanyakan apakah SKTM untuk diri sendiri atau untuk orang lain (anak/keluarga). ",
            "(4) Jika untuk orang lain, kumpulkan juga data subjek dan hubungan keluarga. ",
            "(5) DILARANG menggunakan data contoh/dummy seperti 'John Doe' atau NIK palsu. ",
            "(6) Jika data belum lengkap, minta warga melengkapinya terlebih dahulu."
        ).to_string(),
        input_schema: input_schema(),
    }
}

fn input_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "pengisi": {
                "type": "object",
                "description": "Data orang yang mengisi/menandatangani surat",
                "properties": {
                    "nama": { "type": "string", "description": "Nama lengkap pengisi" },
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
            "subjek": {
                "type": "object",
                "description": "Data orang yang dibuatkan SKTM (jika berbeda dengan pengisi)",
                "properties": {
                    "nama": { "type": "string", "description": "Nama lengkap subjek" },
                    "nik": { "type": "string", "description": "NIK (bila ada)" },
                    "ttl": { "type": "string", "description": "Tempat, Tanggal Lahir" },
                    "jk": { "type": "string", "description": "Jenis Kelamin" },
                    "agama": { "type": "string", "description": "Agama" },
                    "pekerjaan": { "type": "string", "description": "Pekerjaan" },
                    "alamat": { "type": "string", "description": "Alamat" },
                    "hubungan": { "type": "string", "description": "Hubungan keluarga dengan pengisi" }
                }
            },
            "meta": {
                "type": "object",
                "description": "Metadata surat",
                "properties": {
                    "opsi_sendiri": {
                        "type": "boolean",
                        "description": "True jika SKTM untuk diri sendiri, false jika untuk orang lain",
                        "default": true
                    },
                    "kelurahan": { "type": "string", "description": "Nama kelurahan" },
                    "tanggal": { "type": "string", "description": "Tanggal surat (opsional, default: hari ini)" }
                },
                "required": ["kelurahan"]
            }
        },
        "required": ["pengisi", "meta"]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_descriptor() {
        let desc = descriptor();
        assert_eq!(desc.name, TOOL_NAME);
        assert!(!desc.description.is_empty());
        assert!(desc.input_schema.get("properties").is_some());
    }
}
