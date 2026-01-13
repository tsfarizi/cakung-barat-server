//! Generator for Surat Pernyataan Tidak Mampu (SKTM).
//!
//! This generator creates a statement letter for citizens who need to prove
//! they are from a low-income family for social assistance purposes.

use serde::Deserialize;
use std::fs;
use tempfile::tempdir;

use super::common::{
    compile_typst_to_pdf, escape_typst_string, format_indonesian_date, get_static_dir,
    sanitize_filename,
};
use super::{GeneratedDocument, GeneratorError};

const TEMPLATE_FILE: &str = "keterangan_tidak_mampu.typ";

/// Data pengisi (orang yang mengisi formulir).
#[derive(Debug, Deserialize, Default)]
pub struct PengisiData {
    pub nama: String,
    pub nik: String,
    /// Tempat dan tanggal lahir
    pub ttl: String,
    /// Jenis kelamin
    pub jk: String,
    pub agama: String,
    pub pekerjaan: String,
    pub alamat: String,
    pub telp: String,
}

/// Data subjek (orang yang dibuatkan surat).
#[derive(Debug, Deserialize, Default)]
pub struct SubjekData {
    pub nama: String,
    pub nik: String,
    pub ttl: String,
    pub jk: String,
    pub agama: String,
    pub pekerjaan: String,
    pub alamat: String,
    /// Hubungan keluarga dengan pengisi
    pub hubungan: String,
}

/// Metadata surat.
#[derive(Debug, Deserialize)]
pub struct SuratTidakMampuMeta {
    /// True jika untuk diri sendiri, false jika untuk orang lain
    #[serde(default = "default_true")]
    pub opsi_sendiri: bool,
    pub kelurahan: String,
    #[serde(default)]
    pub tanggal: Option<String>,
}

fn default_true() -> bool {
    true
}

impl Default for SuratTidakMampuMeta {
    fn default() -> Self {
        Self {
            opsi_sendiri: true,
            kelurahan: String::new(),
            tanggal: None,
        }
    }
}

/// Request untuk membuat Surat Pernyataan Tidak Mampu.
#[derive(Debug, Deserialize, Default)]
pub struct SuratTidakMampuRequest {
    pub pengisi: PengisiData,
    #[serde(default)]
    pub subjek: SubjekData,
    pub meta: SuratTidakMampuMeta,
}

impl SuratTidakMampuRequest {
    /// Validate all input data and return descriptive errors if invalid.
    pub fn validate(&self) -> Result<(), String> {
        use super::validation::*;

        let mut errors = ValidationErrors::new();

        // Validate pengisi data
        validate_required(
            &self.pengisi.nama,
            "pengisi.nama",
            "Nama Pengisi",
            &mut errors,
        );
        validate_nik(&self.pengisi.nik, "pengisi.nik", &mut errors);
        validate_ttl(&self.pengisi.ttl, "pengisi.ttl", &mut errors);
        validate_gender(&self.pengisi.jk, "pengisi.jk", &mut errors);
        validate_required(
            &self.pengisi.agama,
            "pengisi.agama",
            "Agama Pengisi",
            &mut errors,
        );
        validate_required(
            &self.pengisi.pekerjaan,
            "pengisi.pekerjaan",
            "Pekerjaan Pengisi",
            &mut errors,
        );
        validate_required(
            &self.pengisi.alamat,
            "pengisi.alamat",
            "Alamat Pengisi",
            &mut errors,
        );
        validate_phone(&self.pengisi.telp, "pengisi.telp", &mut errors);

        // If not for self, validate subjek data
        if !self.meta.opsi_sendiri {
            validate_required(&self.subjek.nama, "subjek.nama", "Nama Subjek", &mut errors);
            validate_nik_optional(&self.subjek.nik, "subjek.nik", &mut errors);
            validate_ttl(&self.subjek.ttl, "subjek.ttl", &mut errors);
            validate_gender(&self.subjek.jk, "subjek.jk", &mut errors);
            validate_required(
                &self.subjek.agama,
                "subjek.agama",
                "Agama Subjek",
                &mut errors,
            );
            validate_required(
                &self.subjek.pekerjaan,
                "subjek.pekerjaan",
                "Pekerjaan Subjek",
                &mut errors,
            );
            validate_required(
                &self.subjek.alamat,
                "subjek.alamat",
                "Alamat Subjek",
                &mut errors,
            );
            validate_required(
                &self.subjek.hubungan,
                "subjek.hubungan",
                "Hubungan Keluarga",
                &mut errors,
            );
        }

        // Validate meta
        validate_required(
            &self.meta.kelurahan,
            "meta.kelurahan",
            "Nama Kelurahan",
            &mut errors,
        );

        errors.into_result()
    }
}

/// Generator untuk Surat Pernyataan Tidak Mampu.
pub struct SuratTidakMampuGenerator {
    template: String,
}

impl SuratTidakMampuGenerator {
    /// Create a new generator instance.
    pub fn new() -> Result<Self, GeneratorError> {
        let template_path = get_static_dir().join(TEMPLATE_FILE);
        let template = fs::read_to_string(&template_path).map_err(GeneratorError::TemplateIo)?;
        Ok(Self { template })
    }

    /// Generate the document from the request data.
    pub fn generate(
        &self,
        request: SuratTidakMampuRequest,
    ) -> Result<GeneratedDocument, GeneratorError> {
        let tanggal = request
            .meta
            .tanggal
            .clone()
            .unwrap_or_else(format_indonesian_date);

        let typst_source = self.render_template(&request, &tanggal);

        let temp_dir = tempdir().map_err(GeneratorError::TempDir)?;
        let typ_path = temp_dir.path().join(TEMPLATE_FILE);
        fs::write(&typ_path, &typst_source).map_err(GeneratorError::WriteTypst)?;

        let output_filename = "surat-pernyataan-tidak-mampu.pdf";
        let pdf = compile_typst_to_pdf(&temp_dir, TEMPLATE_FILE, output_filename)?;

        let filename = format!(
            "sktm-{}.pdf",
            sanitize_filename(&request.pengisi.nama, "document")
        );

        Ok(GeneratedDocument {
            filename,
            pdf,
            tanggal,
        })
    }

    fn render_template(&self, request: &SuratTidakMampuRequest, tanggal: &str) -> String {
        // Generate the function call with all parameters
        let pengisi = &request.pengisi;
        let subjek = &request.subjek;
        let meta = &request.meta;

        format!(
            r#"#let surat_pernyataan(
  pengisi: (
    nama: "{}",
    nik: "{}",
    ttl: "{}",
    jk: "{}",
    agama: "{}",
    pekerjaan: "{}",
    alamat: "{}",
    telp: "{}",
  ),
  subjek: (
    nama: "{}",
    nik: "{}",
    ttl: "{}",
    jk: "{}",
    agama: "{}",
    pekerjaan: "{}",
    alamat: "{}",
    hubungan: "{}",
  ),
  meta: (
    opsi_sendiri: {},
    kelurahan: "{}",
    tanggal: "{}",
  ),
) = {{
{}

#surat_pernyataan()
"#,
            escape_typst_string(&pengisi.nama),
            escape_typst_string(&pengisi.nik),
            escape_typst_string(&pengisi.ttl),
            escape_typst_string(&pengisi.jk),
            escape_typst_string(&pengisi.agama),
            escape_typst_string(&pengisi.pekerjaan),
            escape_typst_string(&pengisi.alamat),
            escape_typst_string(&pengisi.telp),
            escape_typst_string(&subjek.nama),
            escape_typst_string(&subjek.nik),
            escape_typst_string(&subjek.ttl),
            escape_typst_string(&subjek.jk),
            escape_typst_string(&subjek.agama),
            escape_typst_string(&subjek.pekerjaan),
            escape_typst_string(&subjek.alamat),
            escape_typst_string(&subjek.hubungan),
            if meta.opsi_sendiri { "true" } else { "false" },
            escape_typst_string(&meta.kelurahan),
            escape_typst_string(tanggal),
            self.extract_function_body(),
        )
    }

    /// Extract the function body from the template (everything between { and the final }).
    fn extract_function_body(&self) -> String {
        // Find the opening brace after the function signature
        if let Some(start) = self.template.find(") = {") {
            let body_start = start + 5; // Skip ") = {"
            // Find the last occurrence of the function call
            if let Some(end) = self.template.rfind("#surat_pernyataan()") {
                return self.template[body_start..end].to_string();
            }
        }
        // Fallback: return template body without the function definition header
        self.template.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_generator() {
        // This test requires the template file to exist
        let result = SuratTidakMampuGenerator::new();
        assert!(result.is_ok());
    }

    #[test]
    fn test_request_deserialization() {
        let json = r#"{
            "pengisi": {
                "nama": "John Doe",
                "nik": "1234567890123456",
                "ttl": "Jakarta, 1 Januari 1990",
                "jk": "Laki-laki",
                "agama": "Islam",
                "pekerjaan": "Karyawan Swasta",
                "alamat": "Jl. Test No. 1",
                "telp": "08123456789"
            },
            "meta": {
                "opsi_sendiri": true,
                "kelurahan": "Cakung Barat"
            }
        }"#;

        let request: SuratTidakMampuRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.pengisi.nama, "John Doe");
        assert!(request.meta.opsi_sendiri);
    }
}
