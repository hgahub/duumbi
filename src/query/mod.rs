//! Read-only conversational Query mode.

pub mod context;
pub mod display;
pub mod engine;
pub mod prompt;
pub mod sources;

pub use display::{DisplayAnswer, split_thinking_blocks};
pub use engine::{QueryEngine, QueryError, QueryRequest};
pub use sources::{AnswerConfidence, ModeHandoff, QueryAnswer, SourceRef};
