use petgraph::prelude::*;
use petgraph::visit::{IntoNeighbors, NodeIndexable};
use petgraph::Graph;
use std::collections::{BinaryHeap, HashMap};
use std::cmp::Ordering;
use crate::map::GameMap;

/// A node with its priority for the priority queue
#[derive(Debug, Clone, PartialEq)]
struct NodePriority {
    node: NodeIndex,
    priority: f32,
}

impl Eq for NodePriority {
    fn assert_receiver_is_total_eq(&self) {}
}

impl Ord for NodePriority {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Reverse ordering for min-heap behavior (lower priority values come first)
        // Also handle NaN values by considering them greater
        other.priority.partial_cmp(&self.priority)
            .unwrap_or(std::cmp::Ordering::Greater)
    }
}

impl PartialOrd for NodePriority {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}



/// A* pathfinding for petgraph-based maps
/// Returns a vector of TileIds representing the path from start to goal, or empty if no path found
pub fn astar_petgraph<F, H>(
    map: &GameMap,
    start: &crate::map::TileId,
    goal: &crate::map::TileId,
    mut cost: F,
    mut heuristic: H,
) -> Vec<crate::map::TileId>
where
    F: FnMut(&crate::map::TileId, &crate::map::TileId) -> f32,
    H: FnMut(&crate::map::TileId, &crate::map::TileId) -> f32,
{
    let graph = &map.graph;
    let id_to_index = &map.id_to_index;
    let index_to_id = &map.index_to_id;

    let start_idx = match id_to_index.get(start) {
        Some(idx) => *idx,
        None => return vec![],
    };
    let goal_idx = match id_to_index.get(goal) {
        Some(idx) => *idx,
        None => return vec![],
    };

    let mut open_set = BinaryHeap::new();
    open_set.push(NodePriority { node: start_idx, priority: 0.0 });

    let mut came_from: HashMap<NodeIndex, NodeIndex> = HashMap::new();
    let mut g_score: HashMap<NodeIndex, f32> = HashMap::new();
    g_score.insert(start_idx, 0.0);

    let mut f_score: HashMap<NodeIndex, f32> = HashMap::new();
    f_score.insert(start_idx, heuristic(start, goal));

    while let Some(NodePriority { node: current, .. }) = open_set.pop() {
        if current == goal_idx {
            // Reconstruct path
            let mut path = vec![current];
            let mut curr = current;
            while let Some(&prev) = came_from.get(&curr) {
                path.push(prev);
                curr = prev;
            }
            path.reverse();
            // Convert NodeIndex to TileId
            return path.iter().filter_map(|idx| index_to_id.get(idx)).cloned().collect();
        }
        for neighbor in graph.neighbors(current) {
            let from_id = match index_to_id.get(&current) {
                Some(id) => id,
                None => continue,
            };
            let to_id = match index_to_id.get(&neighbor) {
                Some(id) => id,
                None => continue,
            };
            let tentative_g_score = g_score.get(&current).unwrap_or(&f32::INFINITY) + cost(from_id, to_id);
            if tentative_g_score < *g_score.get(&neighbor).unwrap_or(&f32::INFINITY) {
                came_from.insert(neighbor, current);
                g_score.insert(neighbor, tentative_g_score);
                let f = tentative_g_score + heuristic(to_id, goal);
                f_score.insert(neighbor, f);
                open_set.push(NodePriority { node: neighbor, priority: f });
            }
        }
    }
    vec![] // No path found
}