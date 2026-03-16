//! Cranelift compiler module.
//!
//! Lowers the validated semantic graph to Cranelift IR and emits
//! native object files. Also provides linker invocation to produce
//! the final native binary.
//!
//! **Cranelift Dependency Policy:** `cranelift-*` crates are used ONLY
//! within `src/compiler/`. The [`CodegenBackend`] trait is the boundary
//! between the graph layer and the compiler — graph/parser modules
//! never import Cranelift types.

pub mod linker;
pub mod lowering;

use std::collections::HashMap;

use thiserror::Error;

use crate::errors::codes;
use crate::graph::SemanticGraph;
use crate::graph::program::Program;

/// Abstraction over code generation backends.
///
/// Currently only [`CraneliftBackend`] is implemented, but this trait
/// establishes the boundary between the graph layer and the compiler.
/// Graph and parser modules interact with compilation exclusively
/// through this trait — they never import Cranelift types directly.
#[allow(dead_code)] // Trait used starting from Phase 9a-1 codegen refactors
pub trait CodegenBackend {
    /// Compiles a single semantic graph (module) to object file bytes.
    fn compile_graph(&mut self, graph: &SemanticGraph) -> Result<Vec<u8>, CompileError>;

    /// Compiles a multi-module program to per-module object file bytes.
    fn compile_program(
        &mut self,
        program: &Program,
    ) -> Result<HashMap<String, Vec<u8>>, CompileError>;
}

/// Cranelift-based code generation backend.
///
/// Wraps the existing Cranelift lowering implementation behind the
/// [`CodegenBackend`] trait interface.
#[allow(dead_code)] // Used starting from Phase 9a-1 codegen refactors
pub struct CraneliftBackend;

#[allow(dead_code)] // Used starting from Phase 9a-1 codegen refactors
impl CraneliftBackend {
    /// Creates a new Cranelift backend.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for CraneliftBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl CodegenBackend for CraneliftBackend {
    fn compile_graph(&mut self, graph: &SemanticGraph) -> Result<Vec<u8>, CompileError> {
        lowering::compile_to_object(graph)
    }

    fn compile_program(
        &mut self,
        program: &Program,
    ) -> Result<HashMap<String, Vec<u8>>, CompileError> {
        lowering::compile_program(program)
    }
}

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
