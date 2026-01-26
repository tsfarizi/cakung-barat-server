//! Generators module - business logic for creating PDF documents from Typst templates.
//!
//! This module contains specialized generators for each document type:
//! - `SuratTidakMampu` - SKTM (Surat Keterangan Tidak Mampu)
//! - `SuratKpr` - Surat Pernyataan Belum Memiliki Rumah
//! - `SuratNibNpwp` - Surat Pernyataan Akan Mengurus NIB & NPWP

pub mod common;
pub mod engine;
pub mod surat_kpr;
pub mod surat_nib_npwp;
pub mod surat_tidak_mampu;
pub mod traits;
pub mod validation;

pub use engine::TypstRenderEngine;
pub use surat_kpr::{SuratKprGenerator, SuratKprRequest};
pub use surat_nib_npwp::{SuratNibNpwpGenerator, SuratNibNpwpRequest};
pub use surat_tidak_mampu::{SuratTidakMampuGenerator, SuratTidakMampuRequest};
pub use traits::{Generator, Validator};

use thiserror::Error;

/// Errors that can occur during document generation.
#[derive(Debug, Error)]
pub enum GeneratorError {
    #[error("failed to load Typst template: {0}")]
    TemplateIo(#[source] std::io::Error),
    #[error("failed to create temporary directory: {0}")]
    TempDir(#[source] std::io::Error),
    #[error("failed to write Typst source: {0}")]
    WriteTypst(#[source] std::io::Error),
    #[error("Typst CLI execution failed: {0}")]
    TypstIo(#[source] std::io::Error),
    #[error("Typst CLI exited with status {0}")]
    TypstExit(i32),
    #[error("failed to read generated PDF: {0}")]
    ReadPdf(#[source] std::io::Error),
}

/// Result of a successful document generation.
#[derive(Debug)]
pub struct GeneratedDocument {
    pub filename: String,
    pub pdf: Vec<u8>,
    pub tanggal: String,
}
