//! Cranelift compiler module.
//!
//! Lowers the validated semantic graph to Cranelift IR and emits
//! native object files. Also provides linker invocation to produce
//! the final native binary.

pub mod linker;
pub mod lowering;

use thiserror::Error;

use crate::errors::codes;

/// Errors that can occur during compilation and linking.
#[derive(Debug, Error)]
pub enum CompileError {
    /// A Cranelift codegen error.
    #[error("[COMPILE] Cranelift error: {message}")]
    Cranelift {
        /// Description of the Cranelift error.
        message: String,
    },

    /// An error during object file emission.
    #[error("[COMPILE] Object emission error: {message}")]
    ObjectEmission {
        /// Description of the emission error.
        message: String,
    },

    /// Linker invocation failed.
    #[error("[{code}] Link failed: {message}")]
    LinkFailed {
        /// Error code for diagnostics.
        code: &'static str,
        /// Description of the link failure.
        message: String,
    },

    /// The C compiler / linker could not be found.
    #[error("[{code}] C compiler not found: {message}")]
    CompilerNotFound {
        /// Error code for diagnostics.
        code: &'static str,
        /// Description.
        message: String,
    },
}

impl CompileError {
    /// Creates a link failure error.
    #[must_use]
    pub fn link_failed(message: impl Into<String>) -> Self {
        CompileError::LinkFailed {
            code: codes::E008_LINK_FAILED,
            message: message.into(),
        }
    }
}
