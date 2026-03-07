//! Graph layout algorithms.
//!
//! Implements Sugiyama-style layered layout for DAGs and orthogonal
//! edge routing. Used by all C4 levels to position nodes and edges.

pub mod edge_routing;
pub mod sugiyama;
pub mod types;

pub use sugiyama::compute_layout;
pub use types::{LayoutEdge, LayoutNode};
