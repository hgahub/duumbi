//! Orthogonal edge routing.
//!
//! Computes L-shaped (orthogonal) SVG paths between nodes,
//! avoiding node overlap where possible.

use std::collections::HashMap;

use super::types::{LayoutEdge, LayoutNode};
use crate::state::GraphEdge;

/// Determines which side of a node faces toward a target point.
fn border_side(node: &LayoutNode, tx: f64, ty: f64) -> Side {
    let dx = tx - node.x;
    let dy = ty - node.y;
    let hw = node.width / 2.0;
    let hh = node.height / 2.0;
    if dx == 0.0 && dy == 0.0 {
        return Side::Bottom;
    }
    if dx.abs() * hh > dy.abs() * hw {
        if dx > 0.0 { Side::Right } else { Side::Left }
    } else if dy > 0.0 {
        Side::Bottom
    } else {
        Side::Top
    }
}

/// Side of a rectangular node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Side {
    Top,
    Bottom,
    Left,
    Right,
}

/// Computes a connection point distributed evenly along a node side.
///
/// `index` is the 0-based slot, `count` is the total edges on that side.
fn distributed_point(node: &LayoutNode, side: Side, index: usize, count: usize) -> (f64, f64) {
    let frac = (index as f64 + 1.0) / (count as f64 + 1.0);
    let hw = node.width / 2.0;
    let hh = node.height / 2.0;
    match side {
        Side::Top => (node.x - hw + node.width * frac, node.y - hh),
        Side::Bottom => (node.x - hw + node.width * frac, node.y + hh),
        Side::Left => (node.x - hw, node.y - hh + node.height * frac),
        Side::Right => (node.x + hw, node.y - hh + node.height * frac),
    }
}

/// Computes edge routes for the given edges using positioned nodes.
///
/// Connection points are evenly distributed along the side of each node.
/// If one edge connects, it attaches at 1/2; two edges at 1/3 and 2/3, etc.
///
/// Returns `LayoutEdge` values with SVG path data for orthogonal routing.
#[must_use]
pub fn route_edges(edges: &[GraphEdge], nodes: &[LayoutNode]) -> Vec<LayoutEdge> {
    let node_map: HashMap<&str, &LayoutNode> = nodes.iter().map(|n| (n.id.as_str(), n)).collect();

    // First pass: determine side + collect port counts per (node, side).
    // Each entry: (src, tgt, src_side, tgt_side) — only for valid edges.
    let mut edge_sides: Vec<Option<(&LayoutNode, &LayoutNode, Side, Side)>> = Vec::new();
    // Key: (node_id, side) → list of edge indices
    let mut port_map: HashMap<(&str, Side), Vec<usize>> = HashMap::new();

    for (i, edge) in edges.iter().enumerate() {
        // Skip self-loop edges (e.g. recursive calls to the same function)
        if edge.source == edge.target {
            edge_sides.push(None);
            continue;
        }

        let pair = (|| {
            let src = node_map.get(edge.source.as_str())?;
            let tgt = node_map.get(edge.target.as_str())?;
            let src_side = border_side(src, tgt.x, tgt.y);
            let tgt_side = border_side(tgt, src.x, src.y);
            Some((*src, *tgt, src_side, tgt_side))
        })();

        if let Some((src, tgt, ss, ts)) = pair {
            port_map
                .entry((edge.source.as_str(), ss))
                .or_default()
                .push(i);
            port_map
                .entry((edge.target.as_str(), ts))
                .or_default()
                .push(i);
            edge_sides.push(Some((src, tgt, ss, ts)));
        } else {
            edge_sides.push(None);
        }
    }

    // Second pass: compute paths using distributed connection points.
    edges
        .iter()
        .enumerate()
        .filter_map(|(i, edge)| {
            let (src, tgt, src_side, tgt_side) = edge_sides[i]?;

            let src_slots = &port_map[&(edge.source.as_str(), src_side)];
            let tgt_slots = &port_map[&(edge.target.as_str(), tgt_side)];
            let src_idx = src_slots.iter().position(|&idx| idx == i).unwrap_or(0);
            let tgt_idx = tgt_slots.iter().position(|&idx| idx == i).unwrap_or(0);

            let (sx, sy) = distributed_point(src, src_side, src_idx, src_slots.len());
            let (tx, ty) = distributed_point(tgt, tgt_side, tgt_idx, tgt_slots.len());

            let (path_data, label_x, label_y) = orthogonal_path_xy(sx, sy, tx, ty);

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

/// Computes an orthogonal (L-shaped) SVG path between two explicit points.
///
/// Returns (path_data, label_x, label_y).
fn orthogonal_path_xy(sx: f64, sy: f64, tx: f64, ty: f64) -> (String, f64, f64) {
    if (ty - sy).abs() > 1.0 {
        let mid_y = (sy + ty) / 2.0;
        let path = format!("M {sx} {sy} L {sx} {mid_y} L {tx} {mid_y} L {tx} {ty}",);
        let label_x = (sx + tx) / 2.0;
        let label_y = mid_y - 8.0;
        (path, label_x, label_y)
    } else {
        // Nearly same Y — horizontal line
        let path = format!("M {sx} {sy} L {tx} {ty}");
        let label_x = (sx + tx) / 2.0;
        let label_y = sy - 8.0;
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

    #[test]
    fn test_distributed_ports_two_edges_from_same_side() {
        // A at top, B and C below — both edges leave A's bottom side
        let nodes = vec![
            make_layout_node("A", 200.0, 50.0),
            make_layout_node("B", 100.0, 200.0),
            make_layout_node("C", 300.0, 200.0),
        ];
        let edges = vec![make_edge("A", "B"), make_edge("A", "C")];

        let routed = route_edges(&edges, &nodes);
        assert_eq!(routed.len(), 2);

        // Extract start X from path_data ("M <x> <y> L ...")
        let sx0: f64 = routed[0]
            .path_data
            .split_whitespace()
            .nth(1)
            .unwrap()
            .parse()
            .unwrap();
        let sx1: f64 = routed[1]
            .path_data
            .split_whitespace()
            .nth(1)
            .unwrap()
            .parse()
            .unwrap();

        // A is centered at x=200, width=100, so left=150, right=250.
        // Two edges on bottom → slots at 1/3 and 2/3 of width.
        // Expected: 150 + 100*(1/3) ≈ 183.3 and 150 + 100*(2/3) ≈ 216.7
        let expected_0 = 150.0 + 100.0 / 3.0;
        let expected_1 = 150.0 + 200.0 / 3.0;
        assert!(
            (sx0 - expected_0).abs() < 1.0,
            "first edge start X: {sx0} != {expected_0}"
        );
        assert!(
            (sx1 - expected_1).abs() < 1.0,
            "second edge start X: {sx1} != {expected_1}"
        );
    }
}
