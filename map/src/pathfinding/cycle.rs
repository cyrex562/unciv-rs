use crate::map::{GameMap, TileId};

/// Detects whether a cycle exists in the graph starting from `start`.
///
/// This implements a breadth-first search (BFS) algorithm to traverse the graph
/// and detect a cycle. A cycle is detected if a neighbor of a node is seen
/// before the node is popped from the queue.
///
/// `false` is returned if no cycle is detected.
pub fn detect_cycle(map: &GameMap, start: TileId) -> bool { 
    use std::collections::HashSet;
    use std::collections::VecDeque;

    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    
    queue.push_back(start);
    visited.insert(start);

    while let Some(current) = queue.pop_front() {
        for neighbor in map.get_neighbors(&current) {
            if visited.contains(&neighbor) {
                // Cycle detected
                return true;
            }
            if !visited.contains(&neighbor) {
                visited.insert(neighbor);
                queue.push_back(neighbor);
            }
        }
    }
    
    false
}