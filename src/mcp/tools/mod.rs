//! MCP Tools module - defines tools exposed via JSON-RPC.
//!
//! Each tool wraps a document generator and provides:
//! - Tool descriptor (name, description, input schema)
//! - Argument parsing and validation
//! - Execution and result formatting

pub mod browse_posts;
pub mod registry;
mod surat_kpr;
mod surat_nib_npwp;
mod surat_tidak_mampu;

pub use registry::ToolRegistry;
