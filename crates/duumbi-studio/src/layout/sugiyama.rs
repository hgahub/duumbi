//! Sugiyama-style layered layout algorithm.
//!
//! Implements a simplified version of the Sugiyama framework:
//! 1. Layer assignment (longest path from sources)
//! 2. Node ordering within layers (median heuristic)
//! 3. Coordinate assignment (center nodes in layer)

use std::collections::HashMap;

use crate::state::GraphData;

use super::types::{BBox, LayoutNode};

/// Minimum spacing constants for the layout.
/// Actual spacing is derived from node dimensions to keep
/// small nodes (code-level ops) tighter and large nodes (C4) more spacious.
const MIN_LAYER_SPACING: f64 = 100.0;
const MIN_NODE_SPACING: f64 = 40.0;
const MIN_PADDING: f64 = 40.0;

/// Derives layout spacing from the maximum node dimensions in the graph.
fn derive_spacing(data: &GraphData) -> (f64, f64, f64) {
    let max_h = data.nodes.iter().map(|n| n.height).fold(0.0_f64, f64::max);
    let max_w = data.nodes.iter().map(|n| n.width).fold(0.0_f64, f64::max);
    // Layer spacing: node height + gap (at least 60% of height)
    let layer_sp = (max_h * 1.6).max(MIN_LAYER_SPACING);
    // Node spacing: at least 40% of max width
    let node_sp = (max_w * 0.4).max(MIN_NODE_SPACING);
    let padding = (max_w * 0.4).max(MIN_PADDING);
    (layer_sp, node_sp, padding)
}

/// Computes a layered layout for the given graph data.
///
/// Returns positioned nodes and the bounding box of the layout.
#[must_use]
pub fn compute_layout(data: &GraphData) -> (Vec<LayoutNode>, BBox) {
    if data.nodes.is_empty() {
        return (Vec::new(), BBox::default());
    }

    // Build adjacency for layer assignment
    let node_ids: Vec<&str> = data.nodes.iter().map(|n| n.id.as_str()).collect();
    let id_to_idx: HashMap<&str, usize> = node_ids
        .iter()
        .enumerate()
        .map(|(i, id)| (*id, i))
        .collect();

    // Adjacency list: who does each node point to?
    let mut successors: Vec<Vec<usize>> = vec![Vec::new(); data.nodes.len()];
    let mut predecessors: Vec<Vec<usize>> = vec![Vec::new(); data.nodes.len()];

    for edge in &data.edges {
        if let (Some(&src), Some(&tgt)) = (
            id_to_idx.get(edge.source.as_str()),
            id_to_idx.get(edge.target.as_str()),
        ) {
            successors[src].push(tgt);
            predecessors[tgt].push(src);
        }
    }

    // Step 1: Layer assignment via longest path from sources
    let layers = assign_layers(&successors, &predecessors, data.nodes.len());

    // Step 2: Group nodes by layer
    let max_layer = layers.iter().copied().max().unwrap_or(0);
    let mut layer_nodes: Vec<Vec<usize>> = vec![Vec::new(); max_layer + 1];
    for (idx, &layer) in layers.iter().enumerate() {
        layer_nodes[layer].push(idx);
    }

    // Step 3: Order nodes within layers (median heuristic, one pass)
    order_by_median(&mut layer_nodes, &predecessors);

    // Step 4: Assign coordinates — spacing adapts to node sizes
    let (layer_spacing, node_spacing, padding) = derive_spacing(data);

    let mut layout_nodes = Vec::with_capacity(data.nodes.len());
    let mut bbox = BBox {
        min_x: f64::MAX,
        min_y: f64::MAX,
        max_x: f64::MIN,
        max_y: f64::MIN,
    };

    for (layer_idx, nodes_in_layer) in layer_nodes.iter().enumerate() {
        let y = padding + layer_idx as f64 * layer_spacing;
        let total_width: f64 = nodes_in_layer
            .iter()
            .map(|&idx| data.nodes[idx].width)
            .sum::<f64>()
            + (nodes_in_layer.len().saturating_sub(1)) as f64 * node_spacing;

        let mut x = padding + (500.0 - total_width) / 2.0; // Center in a 500px viewport
        if x < padding {
            x = padding;
        }

        for (order, &idx) in nodes_in_layer.iter().enumerate() {
            let node = &data.nodes[idx];
            let cx = x + node.width / 2.0;
            let cy = y + node.height / 2.0;

            layout_nodes.push(LayoutNode {
                id: node.id.clone(),
                label: node.label.clone(),
                node_type: node.node_type.clone(),
                badge: node.badge.clone(),
                x: cx,
                y: cy,
                width: node.width,
                height: node.height,
                layer: layer_idx,
                order,
            });

            // Update bounding box
            let left = cx - node.width / 2.0;
            let right = cx + node.width / 2.0;
            let top = cy - node.height / 2.0;
            let bottom = cy + node.height / 2.0;

            if left < bbox.min_x {
                bbox.min_x = left;
            }
            if right > bbox.max_x {
                bbox.max_x = right;
            }
            if top < bbox.min_y {
                bbox.min_y = top;
            }
            if bottom > bbox.max_y {
                bbox.max_y = bottom;
            }

            x += node.width + node_spacing;
        }
    }

    // Add padding to bounding box
    bbox.min_x -= padding;
    bbox.min_y -= padding;
    bbox.max_x += padding;
    bbox.max_y += padding;

    (layout_nodes, bbox)
}

/// Assigns layers using longest path from source nodes.
fn assign_layers(successors: &[Vec<usize>], predecessors: &[Vec<usize>], n: usize) -> Vec<usize> {
    let mut layers = vec![0usize; n];
    let mut visited = vec![false; n];

    // Find source nodes (no predecessors)
    let sources: Vec<usize> = (0..n).filter(|&i| predecessors[i].is_empty()).collect();

    // BFS from sources, assigning layers
    let mut queue = std::collections::VecDeque::new();
    for &src in &sources {
        queue.push_back(src);
        visited[src] = true;
    }

    // If no sources found (cycle or single disconnected nodes), start from 0
    if queue.is_empty() {
        for (i, seen) in visited.iter_mut().enumerate() {
            if !*seen {
                queue.push_back(i);
                *seen = true;
            }
        }
    }

    while let Some(node) = queue.pop_front() {
        for &succ in &successors[node] {
            let new_layer = layers[node] + 1;
            if new_layer > layers[succ] {
                layers[succ] = new_layer;
            }
            if !visited[succ] {
                visited[succ] = true;
                queue.push_back(succ);
            }
        }
    }

    layers
}

/// Orders nodes within each layer using median of predecessor positions.
fn order_by_median(layer_nodes: &mut [Vec<usize>], predecessors: &[Vec<usize>]) {
    // Build position lookup from previous layer ordering
    for layer_idx in 1..layer_nodes.len() {
        // Build position map from previous layer
        let prev_positions: HashMap<usize, usize> = layer_nodes[layer_idx - 1]
            .iter()
            .enumerate()
            .map(|(pos, &node)| (node, pos))
            .collect();

        // Compute median for each node in current layer
        let mut medians: Vec<(usize, f64)> = layer_nodes[layer_idx]
            .iter()
            .map(|&node| {
                let pred_pos: Vec<f64> = predecessors[node]
                    .iter()
                    .filter_map(|&p| prev_positions.get(&p).map(|&pos| pos as f64))
                    .collect();

                let median = if pred_pos.is_empty() {
                    f64::MAX // No predecessors = keep at end
                } else {
                    let mid = pred_pos.len() / 2;
                    let mut sorted = pred_pos;
                    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                    sorted[mid]
                };

                (node, median)
            })
            .collect();

        medians.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        layer_nodes[layer_idx] = medians.into_iter().map(|(node, _)| node).collect();
    }
}

/// Computes a horizontal (left-to-right) layered layout.
///
/// Layers progress along the x-axis; nodes within a layer are spread on the y-axis.
/// Node width/height are preserved — only positions change.
#[must_use]
pub fn compute_layout_horizontal(data: &GraphData) -> (Vec<LayoutNode>, BBox) {
    if data.nodes.is_empty() {
        return (Vec::new(), BBox::default());
    }

    // Reuse layer assignment and ordering from the vertical algorithm.
    let node_ids: Vec<&str> = data.nodes.iter().map(|n| n.id.as_str()).collect();
    let id_to_idx: std::collections::HashMap<&str, usize> = node_ids
        .iter()
        .enumerate()
        .map(|(i, id)| (*id, i))
        .collect();

    let mut successors: Vec<Vec<usize>> = vec![Vec::new(); data.nodes.len()];
    let mut predecessors: Vec<Vec<usize>> = vec![Vec::new(); data.nodes.len()];
    for edge in &data.edges {
        if let (Some(&src), Some(&tgt)) = (
            id_to_idx.get(edge.source.as_str()),
            id_to_idx.get(edge.target.as_str()),
        ) {
            successors[src].push(tgt);
            predecessors[tgt].push(src);
        }
    }

    let layers = assign_layers(&successors, &predecessors, data.nodes.len());
    let max_layer = layers.iter().copied().max().unwrap_or(0);
    let mut layer_nodes: Vec<Vec<usize>> = vec![Vec::new(); max_layer + 1];
    for (idx, &layer) in layers.iter().enumerate() {
        layer_nodes[layer].push(idx);
    }
    order_by_median(&mut layer_nodes, &predecessors);

    // Horizontal spacing: layers along x, nodes within layer along y.
    // Derive from node dimensions — wider nodes get more column spacing.
    let (_, _, padding) = derive_spacing(data);
    let max_w = data.nodes.iter().map(|n| n.width).fold(0.0_f64, f64::max);
    let max_h = data.nodes.iter().map(|n| n.height).fold(0.0_f64, f64::max);
    let col_spacing = (max_w * 1.4).max(200.0); // x gap between layer centres
    let row_spacing = (max_h * 1.4).max(70.0); // y gap between nodes in the same layer

    let mut layout_nodes: Vec<LayoutNode> = data
        .nodes
        .iter()
        .map(|n| LayoutNode {
            id: n.id.clone(),
            label: n.label.clone(),
            node_type: n.node_type.clone(),
            badge: n.badge.clone(),
            x: 0.0,
            y: 0.0,
            width: n.width,
            height: n.height,
            layer: 0,
            order: 0,
        })
        .collect();

    for (layer_idx, nodes_in_layer) in layer_nodes.iter().enumerate() {
        let x = padding + layer_idx as f64 * col_spacing;

        // Total height of this column
        let total_h: f64 = nodes_in_layer
            .iter()
            .map(|&i| data.nodes[i].height)
            .sum::<f64>()
            + (nodes_in_layer.len().saturating_sub(1)) as f64 * row_spacing;

        let mut y = padding - total_h / 2.0 + 300.0; // centre around 300 px

        for &idx in nodes_in_layer {
            let h = data.nodes[idx].height;
            layout_nodes[idx].x = x;
            layout_nodes[idx].y = y + h / 2.0;
            layout_nodes[idx].layer = layer_idx;
            y += h + row_spacing;
        }
    }

    let bbox = compute_bbox(&layout_nodes);
    (layout_nodes, bbox)
}

/// Computes a radial/circular layout.
///
/// Nodes are placed in concentric rings based on their layer assignment.
/// Layer 0 (roots) go in the center, subsequent layers on larger circles.
#[must_use]
pub fn compute_layout_radial(data: &GraphData) -> (Vec<LayoutNode>, BBox) {
    if data.nodes.is_empty() {
        return (Vec::new(), BBox::default());
    }

    // Reuse Sugiyama layer assignment
    let (layered_nodes, _) = compute_layout(data);

    // Group by layer
    let max_layer = layered_nodes.iter().map(|n| n.layer).max().unwrap_or(0);
    let mut by_layer: Vec<Vec<usize>> = vec![Vec::new(); max_layer + 1];
    for (i, node) in layered_nodes.iter().enumerate() {
        by_layer[node.layer].push(i);
    }

    let ring_spacing = 150.0;
    let center_x = 400.0;
    let center_y = 400.0;

    let mut result: Vec<LayoutNode> = layered_nodes;

    for (layer_idx, indices) in by_layer.iter().enumerate() {
        if layer_idx == 0 && indices.len() == 1 {
            // Single root node at center
            result[indices[0]].x = center_x;
            result[indices[0]].y = center_y;
            continue;
        }
        let radius = if layer_idx == 0 {
            0.0
        } else {
            layer_idx as f64 * ring_spacing
        };
        let count = indices.len();
        for (i, &idx) in indices.iter().enumerate() {
            let angle =
                2.0 * std::f64::consts::PI * i as f64 / count as f64 - std::f64::consts::FRAC_PI_2; // start from top
            result[idx].x = center_x + radius * angle.cos();
            result[idx].y = center_y + radius * angle.sin();
        }
    }

    let bbox = compute_bbox(&result);
    (result, bbox)
}

/// Computes the bounding box from positioned nodes.
fn compute_bbox(nodes: &[LayoutNode]) -> BBox {
    let mut bbox = BBox {
        min_x: f64::MAX,
        min_y: f64::MAX,
        max_x: f64::MIN,
        max_y: f64::MIN,
    };
    for node in nodes {
        let left = node.x - node.width / 2.0;
        let right = node.x + node.width / 2.0;
        let top = node.y - node.height / 2.0;
        let bottom = node.y + node.height / 2.0;
        if left < bbox.min_x {
            bbox.min_x = left;
        }
        if right > bbox.max_x {
            bbox.max_x = right;
        }
        if top < bbox.min_y {
            bbox.min_y = top;
        }
        if bottom > bbox.max_y {
            bbox.max_y = bottom;
        }
    }
    bbox.min_x -= MIN_PADDING;
    bbox.min_y -= MIN_PADDING;
    bbox.max_x += MIN_PADDING;
    bbox.max_y += MIN_PADDING;
    bbox
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{GraphEdge, GraphNode};

    fn make_node(id: &str) -> GraphNode {
        GraphNode {
            id: id.to_string(),
            label: id.to_string(),
            node_type: "test".to_string(),
            badge: None,
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 40.0,
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
    fn test_empty_graph() {
        let data = GraphData {
            nodes: vec![],
            edges: vec![],
        };
        let (nodes, bbox) = compute_layout(&data);
        assert!(nodes.is_empty());
        assert_eq!(bbox.width(), 0.0);
    }

    #[test]
    fn test_single_node() {
        let data = GraphData {
            nodes: vec![make_node("A")],
            edges: vec![],
        };
        let (nodes, _bbox) = compute_layout(&data);
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].layer, 0);
    }

    #[test]
    fn test_linear_chain() {
        let data = GraphData {
            nodes: vec![make_node("A"), make_node("B"), make_node("C")],
            edges: vec![make_edge("A", "B"), make_edge("B", "C")],
        };
        let (nodes, _bbox) = compute_layout(&data);
        assert_eq!(nodes.len(), 3);

        let a = nodes.iter().find(|n| n.id == "A").expect("A");
        let b = nodes.iter().find(|n| n.id == "B").expect("B");
        let c = nodes.iter().find(|n| n.id == "C").expect("C");

        assert_eq!(a.layer, 0);
        assert_eq!(b.layer, 1);
        assert_eq!(c.layer, 2);
        assert!(a.y < b.y, "A should be above B");
        assert!(b.y < c.y, "B should be above C");
    }

    #[test]
    fn test_no_overlapping_nodes() {
        let data = GraphData {
            nodes: vec![
                make_node("A"),
                make_node("B"),
                make_node("C"),
                make_node("D"),
            ],
            edges: vec![
                make_edge("A", "C"),
                make_edge("A", "D"),
                make_edge("B", "C"),
                make_edge("B", "D"),
            ],
        };
        let (nodes, _bbox) = compute_layout(&data);

        // Check no overlaps within same layer
        let mut by_layer: HashMap<usize, Vec<&LayoutNode>> = HashMap::new();
        for node in &nodes {
            by_layer.entry(node.layer).or_default().push(node);
        }

        for (_layer, layer_nodes) in &by_layer {
            for i in 0..layer_nodes.len() {
                for j in (i + 1)..layer_nodes.len() {
                    let a = layer_nodes[i];
                    let b = layer_nodes[j];
                    let a_right = a.x + a.width / 2.0;
                    let b_left = b.x - b.width / 2.0;
                    let no_overlap = a_right <= b_left || {
                        let b_right = b.x + b.width / 2.0;
                        let a_left = a.x - a.width / 2.0;
                        b_right <= a_left
                    };
                    assert!(no_overlap, "Nodes {} and {} overlap", a.id, b.id);
                }
            }
        }
    }
}
