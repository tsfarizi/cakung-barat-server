//! Generator for Surat Pernyataan Akan Mengurus NIB & NPWP.
//!
//! This generator creates a statement letter for business owners who commit
//! to registering for NIB (Nomor Induk Berusaha) and NPWP (tax ID).

use serde::Deserialize;
use std::fs;
use tempfile::tempdir;

use super::common::{
    compile_typst_to_pdf, escape_typst_string, format_indonesian_date, get_static_dir,
    sanitize_filename,
};
use super::{GeneratedDocument, GeneratorError};

const TEMPLATE_FILE: &str = "surat_pernyataan_akan_mengurus_nib_npwp.typ";

/// Data pelaku usaha.
#[derive(Debug, Deserialize, Default)]
pub struct NibNpwpData {
    pub nama: String,
    pub nik: String,
    pub jabatan: String,
    pub bidang_usaha: String,
    pub kegiatan_usaha: String,
    pub jenis_usaha: String,
    pub alamat_usaha: String,
}

/// Metadata surat NIB/NPWP.
#[derive(Debug, Deserialize, Default)]
pub struct SuratNibNpwpMeta {
    #[serde(default)]
    pub tanggal: Option<String>,
}

/// Request untuk membuat Surat Pernyataan Akan Mengurus NIB & NPWP.
#[derive(Debug, Deserialize, Default)]
pub struct SuratNibNpwpRequest {
    pub data: NibNpwpData,
    #[serde(default)]
    pub meta: SuratNibNpwpMeta,
}

impl SuratNibNpwpRequest {
    /// Validate all input data and return descriptive errors if invalid.
    pub fn validate(&self) -> Result<(), String> {
        use super::validation::*;

        let mut errors = ValidationErrors::new();

        // Validate data pelaku usaha
        validate_required(
            &self.data.nama,
            "data.nama",
            "Nama Pelaku Usaha",
            &mut errors,
        );
        validate_nik(&self.data.nik, "data.nik", &mut errors);
        validate_required(&self.data.jabatan, "data.jabatan", "Jabatan", &mut errors);
        validate_required(
            &self.data.bidang_usaha,
            "data.bidang_usaha",
            "Bidang Usaha",
            &mut errors,
        );
        validate_required(
            &self.data.kegiatan_usaha,
            "data.kegiatan_usaha",
            "Kegiatan Usaha",
            &mut errors,
        );
        validate_required(
            &self.data.jenis_usaha,
            "data.jenis_usaha",
            "Jenis Usaha",
            &mut errors,
        );
        validate_required(
            &self.data.alamat_usaha,
            "data.alamat_usaha",
            "Alamat Usaha",
            &mut errors,
        );

        errors.into_result()
    }
}

/// Generator untuk Surat Pernyataan Akan Mengurus NIB & NPWP.
pub struct SuratNibNpwpGenerator {
    template: String,
}

impl SuratNibNpwpGenerator {
    /// Create a new generator instance.
    pub fn new() -> Result<Self, GeneratorError> {
        let template_path = get_static_dir().join(TEMPLATE_FILE);
        let template = fs::read_to_string(&template_path).map_err(GeneratorError::TemplateIo)?;
        Ok(Self { template })
    }

    /// Generate the document from the request data.
    pub fn generate(
        &self,
        request: SuratNibNpwpRequest,
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

        let output_filename = "surat-pernyataan-nib-npwp.pdf";
        let pdf = compile_typst_to_pdf(&temp_dir, TEMPLATE_FILE, output_filename)?;

        let filename = format!(
            "surat-nib-npwp-{}.pdf",
            sanitize_filename(&request.data.nama, "document")
        );

        Ok(GeneratedDocument {
            filename,
            pdf,
            tanggal,
        })
    }

    fn render_template(&self, request: &SuratNibNpwpRequest, tanggal: &str) -> String {
        let data = &request.data;

        format!(
            r#"#let surat_pernyataan_nib_npwp(
  data: (
    nama: "{}",
    nik: "{}",
    jabatan: "{}",
    bidang_usaha: "{}",
    kegiatan_usaha: "{}",
    jenis_usaha: "{}",
    alamat_usaha: "{}",
  ),
  meta: (
    tanggal: "{}",
  ),
) = {{
{}

#surat_pernyataan_nib_npwp()
"#,
            escape_typst_string(&data.nama),
            escape_typst_string(&data.nik),
            escape_typst_string(&data.jabatan),
            escape_typst_string(&data.bidang_usaha),
            escape_typst_string(&data.kegiatan_usaha),
            escape_typst_string(&data.jenis_usaha),
            escape_typst_string(&data.alamat_usaha),
            escape_typst_string(tanggal),
            self.extract_function_body(),
        )
    }

    fn extract_function_body(&self) -> String {
        if let Some(start) = self.template.find(") = {") {
            let body_start = start + 5;
            if let Some(end) = self.template.rfind("#surat_pernyataan_nib_npwp()") {
                return self.template[body_start..end].to_string();
            }
        }
        self.template.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_generator() {
        let result = SuratNibNpwpGenerator::new();
        assert!(result.is_ok());
    }

    #[test]
    fn test_request_deserialization() {
        let json = r#"{
            "data": {
                "nama": "Ahmad Wirawan",
                "nik": "3171234567890123",
                "jabatan": "Pemilik",
                "bidang_usaha": "Perdagangan",
                "kegiatan_usaha": "Toko Kelontong",
                "jenis_usaha": "Usaha Mikro",
                "alamat_usaha": "Jl. Pasar No. 10"
            }
        }"#;

        let request: SuratNibNpwpRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.data.nama, "Ahmad Wirawan");
        assert_eq!(request.data.jenis_usaha, "Usaha Mikro");
    }
}
