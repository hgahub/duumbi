//! Graph layout algorithms.
//!
//! Implements Sugiyama-style layered layout for DAGs and orthogonal
//! edge routing. Used by all C4 levels to position nodes and edges.

pub mod edge_routing;
pub mod sugiyama;
pub mod types;

pub use sugiyama::{compute_layout, compute_layout_horizontal, compute_layout_radial};
pub use types::{LayoutEdge, LayoutNode};
