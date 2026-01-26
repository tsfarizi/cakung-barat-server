//! Traits for generator system standardization.

use super::{GeneratedDocument, GeneratorError};

/// Trait for validating request objects.
pub trait Validator {
    /// Validate the state of the object.
    fn validate(&self) -> Result<(), String>;
}

/// Trait for document generators.
pub trait Generator<Req> {
    /// Generate a document from the request.
    fn generate(&self, request: Req) -> Result<GeneratedDocument, GeneratorError>;
}
