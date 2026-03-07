//! Layout type definitions.
//!
//! Data structures used by the layout algorithm and consumed by
//! SVG rendering components.

use serde::{Deserialize, Serialize};

/// A node with computed position and dimensions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutNode {
    /// Node identifier.
    pub id: String,
    /// Display label.
    pub label: String,
    /// Node type for styling.
    pub node_type: String,
    /// Optional badge text.
    pub badge: Option<String>,
    /// X coordinate (center).
    pub x: f64,
    /// Y coordinate (center).
    pub y: f64,
    /// Width.
    pub width: f64,
    /// Height.
    pub height: f64,
    /// Layer assigned by Sugiyama (0 = top).
    pub layer: usize,
    /// Position within the layer.
    pub order: usize,
}

/// An edge with computed routing points.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutEdge {
    /// Edge identifier.
    pub id: String,
    /// Source node id.
    pub source: String,
    /// Target node id.
    pub target: String,
    /// Edge label.
    pub label: String,
    /// Edge type for styling.
    pub edge_type: String,
    /// SVG path data string (e.g., "M 10 20 L 30 40").
    pub path_data: String,
    /// Label position (midpoint of edge).
    pub label_x: f64,
    /// Label position (midpoint of edge).
    pub label_y: f64,
}

/// Bounding box of the entire layout.
#[derive(Debug, Clone, Default)]
pub struct BBox {
    /// Minimum X.
    pub min_x: f64,
    /// Minimum Y.
    pub min_y: f64,
    /// Maximum X.
    pub max_x: f64,
    /// Maximum Y.
    pub max_y: f64,
}

impl BBox {
    /// Width of the bounding box.
    #[must_use]
    pub fn width(&self) -> f64 {
        self.max_x - self.min_x
    }

    /// Height of the bounding box.
    #[must_use]
    pub fn height(&self) -> f64 {
        self.max_y - self.min_y
    }
}
