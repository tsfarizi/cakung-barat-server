//! Generator for Surat Pernyataan Belum Memiliki Rumah (KPR).
//!
//! This generator creates a statement letter for citizens who need to prove
//! they don't own a house yet, typically for KPR (mortgage) applications.

use serde::Deserialize;
use std::fs;

use super::common::{escape_typst_string, format_indonesian_date, get_static_dir};
use super::engine::TypstRenderEngine;
use super::traits::{Generator, Validator};
use super::{GeneratedDocument, GeneratorError};

const TEMPLATE_FILE: &str = "kpr_belum_memiliki_rumah.typ";

/// Data pemohon KPR.
#[derive(Debug, Deserialize, Default)]
pub struct KprData {
    pub nama: String,
    pub nik: String,
    /// Tempat dan tanggal lahir
    pub ttl: String,
    /// Jenis kelamin (true: Laki-laki, false: Perempuan)
    pub jk: bool,
    pub agama: String,
    pub pekerjaan: String,
    pub alamat: String,
    pub telp: String,
}

/// Metadata surat KPR.
#[derive(Debug, Deserialize, Default)]
pub struct SuratKprMeta {
    pub kelurahan: String,
    pub bank_tujuan: String,
    #[serde(default)]
    pub tanggal: Option<String>,
}

/// Request untuk membuat Surat Pernyataan Belum Memiliki Rumah.
#[derive(Debug, Deserialize, Default)]
pub struct SuratKprRequest {
    pub data: KprData,
    pub meta: SuratKprMeta,
}

impl Validator for SuratKprRequest {
    /// Validate all input data and return descriptive errors if invalid.
    fn validate(&self) -> Result<(), String> {
        use super::validation::*;

        let mut errors = ValidationErrors::new();

        // Validate data
        validate_required(&self.data.nama, "data.nama", "Nama Pemohon", &mut errors);
        validate_nik(&self.data.nik, "data.nik", &mut errors);
        validate_ttl(&self.data.ttl, "data.ttl", &mut errors);
        // validate_gender(&self.data.jk, "data.jk", &mut errors);
        validate_required(&self.data.agama, "data.agama", "Agama", &mut errors);
        validate_required(
            &self.data.pekerjaan,
            "data.pekerjaan",
            "Pekerjaan",
            &mut errors,
        );
        validate_required(&self.data.alamat, "data.alamat", "Alamat", &mut errors);
        validate_phone(&self.data.telp, "data.telp", &mut errors);

        // Validate meta
        validate_required(
            &self.meta.kelurahan,
            "meta.kelurahan",
            "Nama Kelurahan",
            &mut errors,
        );
        validate_required(
            &self.meta.bank_tujuan,
            "meta.bank_tujuan",
            "Bank Tujuan KPR",
            &mut errors,
        );

        errors.into_result()
    }
}

// Keep the inherent validate method for backward compatibility if needed, 
// or just redirect it to the trait implementation.
impl SuratKprRequest {
    pub fn validate(&self) -> Result<(), String> {
        Validator::validate(self)
    }
}

/// Generator untuk Surat Pernyataan Belum Memiliki Rumah.
pub struct SuratKprGenerator {
    template: String,
}

impl SuratKprGenerator {
    /// Create a new generator instance.
    pub fn new() -> Result<Self, GeneratorError> {
        let template_path = get_static_dir().join(TEMPLATE_FILE);
        let template = fs::read_to_string(&template_path).map_err(GeneratorError::TemplateIo)?;
        Ok(Self { template })
    }

    fn render_template(&self, request: &SuratKprRequest, tanggal: &str) -> String {
        let data = &request.data;
        let meta = &request.meta;
        let jk_str = if data.jk { "Laki-laki" } else { "Perempuan" };

        format!(
            r#"#let surat_pernyataan_kpr(
  data: (
    nama: "{}",
    nik: "{}",
    ttl: "{}",
    jk: "{}",
    agama: "{}",
    pekerjaan: "{}",
    alamat: "{}",
    telp: "{}",
  ),
  meta: (
    kelurahan: "{}",
    bank_tujuan: "{}",
    tanggal: "{}",
  ),
) = {{
{}

#surat_pernyataan_kpr()
"#,
            escape_typst_string(&data.nama),
            escape_typst_string(&data.nik),
            escape_typst_string(&data.ttl),
            escape_typst_string(jk_str),
            escape_typst_string(&data.agama),
            escape_typst_string(&data.pekerjaan),
            escape_typst_string(&data.alamat),
            escape_typst_string(&data.telp),
            escape_typst_string(&meta.kelurahan),
            escape_typst_string(&meta.bank_tujuan),
            escape_typst_string(tanggal),
            self.extract_function_body(),
        )
    }

    fn extract_function_body(&self) -> String {
        if let Some(start) = self.template.find(") = {") {
            let body_start = start + 5;
            if let Some(end) = self.template.rfind("#surat_pernyataan_kpr()") {
                return self.template[body_start..end].to_string();
            }
        }
        self.template.clone()
    }
}

impl Generator<SuratKprRequest> for SuratKprGenerator {
    /// Generate the document from the request data.
    fn generate(&self, request: SuratKprRequest) -> Result<GeneratedDocument, GeneratorError> {
        let tanggal = request
            .meta
            .tanggal
            .clone()
            .unwrap_or_else(format_indonesian_date);

        let typst_source = self.render_template(&request, &tanggal);

        TypstRenderEngine::render(
            TEMPLATE_FILE,
            &typst_source,
            &request.data.nama,
            Some(tanggal),
        )
    }
}

// Inherent impl for backward compatibility / ease of use
impl SuratKprGenerator {
    pub fn generate(&self, request: SuratKprRequest) -> Result<GeneratedDocument, GeneratorError> {
        Generator::generate(self, request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_generator() {
        let result = SuratKprGenerator::new();
        assert!(result.is_ok());
    }

    #[test]
    fn test_request_deserialization() {
        let json = r#"{
            "data": {
                "nama": "Jane Doe",
                "nik": "1234567890123456",
                "ttl": "Jakarta, 15 Maret 1985",
                "jk": false,
                "agama": "Kristen",
                "pekerjaan": "PNS",
                "alamat": "Jl. Melati No. 5",
                "telp": "08198765432"
            },
            "meta": {
                "kelurahan": "Cakung Barat",
                "bank_tujuan": "Bank BTN"
            }
        }"#;

        let request: SuratKprRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.data.nama, "Jane Doe");
        assert_eq!(request.meta.bank_tujuan, "Bank BTN");
    }
}