//! Orthogonal edge routing.
//!
//! Computes L-shaped (orthogonal) SVG paths between nodes,
//! avoiding node overlap where possible.

use std::collections::HashMap;

use super::types::{LayoutEdge, LayoutNode};
use crate::state::GraphEdge;

/// Computes edge routes for the given edges using positioned nodes.
///
/// Returns `LayoutEdge` values with SVG path data for orthogonal routing.
#[must_use]
pub fn route_edges(edges: &[GraphEdge], nodes: &[LayoutNode]) -> Vec<LayoutEdge> {
    let node_map: HashMap<&str, &LayoutNode> = nodes.iter().map(|n| (n.id.as_str(), n)).collect();

    edges
        .iter()
        .filter_map(|edge| {
            let src = node_map.get(edge.source.as_str())?;
            let tgt = node_map.get(edge.target.as_str())?;

            let (path_data, label_x, label_y) = orthogonal_path(src, tgt);

            Some(LayoutEdge {
                id: edge.id.clone(),
                source: edge.source.clone(),
                target: edge.target.clone(),
                label: edge.label.clone(),
                edge_type: edge.edge_type.clone(),
                path_data,
                label_x,
                label_y,
            })
        })
        .collect()
}

/// Computes an orthogonal (L-shaped) SVG path between two nodes.
///
/// Returns (path_data, label_x, label_y).
fn orthogonal_path(src: &LayoutNode, tgt: &LayoutNode) -> (String, f64, f64) {
    let src_bottom = src.y + src.height / 2.0;
    let tgt_top = tgt.y - tgt.height / 2.0;

    // If target is below source: go down from source bottom, then horizontal, then down to target
    if tgt.y > src.y {
        let mid_y = (src_bottom + tgt_top) / 2.0;
        let path = format!(
            "M {sx} {sy} L {sx} {my} L {tx} {my} L {tx} {ty}",
            sx = src.x,
            sy = src_bottom,
            my = mid_y,
            tx = tgt.x,
            ty = tgt_top,
        );
        let label_x = (src.x + tgt.x) / 2.0;
        let label_y = mid_y - 8.0;
        (path, label_x, label_y)
    } else {
        // Target above or same level: simple straight line
        let path = format!(
            "M {sx} {sy} L {tx} {ty}",
            sx = src.x,
            sy = src_bottom,
            tx = tgt.x,
            ty = tgt_top,
        );
        let label_x = (src.x + tgt.x) / 2.0;
        let label_y = (src_bottom + tgt_top) / 2.0 - 8.0;
        (path, label_x, label_y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_layout_node(id: &str, x: f64, y: f64) -> LayoutNode {
        LayoutNode {
            id: id.to_string(),
            label: id.to_string(),
            node_type: "test".to_string(),
            badge: None,
            x,
            y,
            width: 100.0,
            height: 40.0,
            layer: 0,
            order: 0,
        }
    }

    fn make_edge(source: &str, target: &str) -> GraphEdge {
        GraphEdge {
            id: format!("{source}->{target}"),
            source: source.to_string(),
            target: target.to_string(),
            label: String::new(),
            edge_type: "test".to_string(),
        }
    }

    #[test]
    fn test_route_simple_edge() {
        let nodes = vec![
            make_layout_node("A", 100.0, 50.0),
            make_layout_node("B", 100.0, 200.0),
        ];
        let edges = vec![make_edge("A", "B")];

        let routed = route_edges(&edges, &nodes);
        assert_eq!(routed.len(), 1);
        assert!(routed[0].path_data.starts_with("M "));
        assert!(routed[0].path_data.contains("L "));
    }

    #[test]
    fn test_route_missing_node_skipped() {
        let nodes = vec![make_layout_node("A", 100.0, 50.0)];
        let edges = vec![make_edge("A", "missing")];

        let routed = route_edges(&edges, &nodes);
        assert!(
            routed.is_empty(),
            "edge with missing target should be skipped"
        );
    }
}
