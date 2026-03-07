//! Sugiyama-style layered layout algorithm.
//!
//! Implements a simplified version of the Sugiyama framework:
//! 1. Layer assignment (longest path from sources)
//! 2. Node ordering within layers (median heuristic)
//! 3. Coordinate assignment (center nodes in layer)

use std::collections::HashMap;

use crate::state::GraphData;

use super::types::{BBox, LayoutNode};

/// Spacing constants for the layout.
const LAYER_SPACING: f64 = 120.0;
const NODE_SPACING: f64 = 40.0;
const PADDING: f64 = 60.0;

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

    // Step 4: Assign coordinates
    let mut layout_nodes = Vec::with_capacity(data.nodes.len());
    let mut bbox = BBox {
        min_x: f64::MAX,
        min_y: f64::MAX,
        max_x: f64::MIN,
        max_y: f64::MIN,
    };

    for (layer_idx, nodes_in_layer) in layer_nodes.iter().enumerate() {
        let y = PADDING + layer_idx as f64 * LAYER_SPACING;
        let total_width: f64 = nodes_in_layer
            .iter()
            .map(|&idx| data.nodes[idx].width)
            .sum::<f64>()
            + (nodes_in_layer.len().saturating_sub(1)) as f64 * NODE_SPACING;

        let mut x = PADDING + (500.0 - total_width) / 2.0; // Center in a 500px viewport
        if x < PADDING {
            x = PADDING;
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

            x += node.width + NODE_SPACING;
        }
    }

    // Add padding to bounding box
    bbox.min_x -= PADDING;
    bbox.min_y -= PADDING;
    bbox.max_x += PADDING;
    bbox.max_y += PADDING;

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
        for i in 0..n {
            if !visited[i] {
                queue.push_back(i);
                visited[i] = true;
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
