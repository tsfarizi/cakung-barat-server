//! Typst rendering engine.
//!
//! Handles the low-level details of writing Typst source to temporary files,
//! invoking the compiler, and managing the output PDF.

use std::fs;
use std::process::Command;
use tempfile::tempdir;
use tempfile::TempDir;

use super::common::{format_indonesian_date, sanitize_filename};
use super::{GeneratedDocument, GeneratorError};

/// Stateless engine for rendering Typst templates to PDF.
pub struct TypstRenderEngine;

impl TypstRenderEngine {
    /// Render a Typst string to a PDF document.
    ///
    /// # Arguments
    /// * `template_filename` - The name of the template file (e.g., "surat.typ") used for reference/logging.
    /// * `typst_source` - The complete, rendered Typst source code string.
    /// * `output_name_base` - The base name for the output file (e.g., citizen's name).
    /// * `date_override` - Optional date to use; defaults to today's Indonesian date.
    pub fn render(
        template_filename: &str,
        typst_source: &str,
        output_name_base: &str,
        date_override: Option<String>,
    ) -> Result<GeneratedDocument, GeneratorError> {
        let tanggal = date_override.unwrap_or_else(format_indonesian_date);

        // Create temp directory for compilation context
        let temp_dir = tempdir().map_err(GeneratorError::TempDir)?;
        let typ_path = temp_dir.path().join(template_filename);
        
        // Write the source to the temp file
        fs::write(&typ_path, typst_source).map_err(GeneratorError::WriteTypst)?;

        // Define output filename
        let safe_name = sanitize_filename(output_name_base, "document");
        let output_filename = format!("output-{}.pdf", safe_name);
        
        // Compile
        let pdf = compile_typst_to_pdf(&temp_dir, template_filename, &output_filename)?;

        // Construct final filename
        // We use the base name to create a nice filename for the user
        let final_filename = format!(
            "{}-{}.pdf", 
            sanitize_filename(template_filename.trim_end_matches(".typ"), "surat"),
            safe_name
        );

        Ok(GeneratedDocument {
            filename: final_filename,
            pdf,
            tanggal,
        })
    }
}

/// Compile a Typst source file to PDF.
fn compile_typst_to_pdf(
    temp_dir: &TempDir,
    typ_filename: &str,
    output_filename: &str,
) -> Result<Vec<u8>, GeneratorError> {
    let typ_path = temp_dir.path().join(typ_filename);
    let output_path = temp_dir.path().join(output_filename);

    let status = Command::new("typst")
        .arg("compile")
        .arg(&typ_path)
        .arg(&output_path)
        .current_dir(temp_dir.path())
        .status()
        .map_err(GeneratorError::TypstIo)?;

    if !status.success() {
        let code = status.code().unwrap_or(-1);
        return Err(GeneratorError::TypstExit(code));
    }

    fs::read(&output_path).map_err(GeneratorError::ReadPdf)
}