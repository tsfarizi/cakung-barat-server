//! Tool definition for Surat Pernyataan Akan Mengurus NIB & NPWP.

use serde_json::{Value, json};

use super::registry::ToolDescriptor;

pub const TOOL_NAME: &str = "generate_surat_nib_npwp";

/// Get the tool descriptor for MCP tools/list.
pub fn descriptor() -> ToolDescriptor {
    ToolDescriptor {
        name: TOOL_NAME.to_string(),
        description: concat!(
            "Membuat Surat Pernyataan Akan Mengurus NIB (Nomor Induk Berusaha) ",
            "dan NPWP (Nomor Pokok Wajib Pajak) dalam format PDF. Surat ini digunakan oleh ",
            "pelaku usaha yang belum memiliki NIB dan NPWP serta berkomitmen untuk mengurusnya ",
            "dalam waktu maksimal 3 bulan. ",
            "[PENTING] INSTRUKSI PENGGUNAAN: ",
            "(1) WAJIB tanyakan semua data kepada warga SEBELUM memanggil tool ini. ",
            "(2) Data yang harus dikumpulkan: nama lengkap, NIK (16 digit), jabatan dalam usaha. ",
            "(3) Data usaha yang diperlukan: bidang usaha, kegiatan usaha, jenis usaha ",
            "(Mikro/Kecil/Menengah), dan alamat lengkap lokasi usaha. ",
            "(4) DILARANG menggunakan data contoh/dummy seperti 'John Doe' atau NIK palsu. ",
            "(5) Jika data belum lengkap, minta warga melengkapinya terlebih dahulu."
        )
        .to_string(),
        input_schema: input_schema(),
    }
}

fn input_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "data": {
                "type": "object",
                "description": "Data pelaku usaha",
                "properties": {
                    "nama": { "type": "string", "description": "Nama lengkap pelaku usaha" },
                    "nik": { "type": "string", "description": "NIK (16 digit)" },
                    "jabatan": { "type": "string", "description": "Jabatan dalam usaha (mis: Pemilik, Direktur)" },
                    "bidang_usaha": { "type": "string", "description": "Bidang usaha (mis: Perdagangan, Jasa)" },
                    "kegiatan_usaha": { "type": "string", "description": "Deskripsi kegiatan usaha" },
                    "jenis_usaha": { "type": "string", "description": "Jenis usaha (Usaha Mikro/Kecil/Menengah)" },
                    "alamat_usaha": { "type": "string", "description": "Alamat lengkap lokasi usaha" }
                },
                "required": ["nama", "nik", "jabatan", "bidang_usaha", "kegiatan_usaha", "jenis_usaha", "alamat_usaha"]
            },
            "meta": {
                "type": "object",
                "description": "Metadata surat",
                "properties": {
                    "tanggal": { "type": "string", "description": "Tanggal surat (opsional, default: hari ini)" }
                }
            }
        },
        "required": ["data"]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_descriptor() {
        let desc = descriptor();
        assert_eq!(desc.name, TOOL_NAME);
        assert!(desc.description.contains("NIB"));
        assert!(desc.description.contains("NPWP"));
        assert!(desc.input_schema.get("properties").is_some());
    }
}
