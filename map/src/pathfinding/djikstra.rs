use std::collections::{BinaryHeap, HashMap};
use crate::map::{GameMap, TileId};
use crate::pathfinding::node_priority::NodePriority;

pub fn dijkstra(map: &GameMap, start: TileId, end: TileId) -> Option<Vec<TileId>> {
    let start_idx = *map.id_to_index.get(&start)?;
    let end_idx = *map.id_to_index.get(&end)?;
    let mut distances = HashMap::new();
    let mut previous = HashMap::new();
    let mut queue = BinaryHeap::new();

    distances.insert(start_idx, 0.0);
    queue.push(NodePriority { node: start_idx, priority: 0.0 });

    while let Some(NodePriority { node: current, .. }) = queue.pop() {
        if current == end_idx {
            let mut path = Vec::new();
            let mut curr = current;
            while let Some(&prev) = previous.get(&curr) {
                // convert NodeIndex back to TileId
                path.push(map.index_to_id[&curr]);
                curr = prev;
            }
            path.push(start); // include the start node
            path.reverse();
            return Some(path);
        }
        let current_distance = distances[&current];
        // Get neighbors of current as TileId
        let curr_tile_id = map.index_to_id[&current];
        for neighbor in map.get_neighbors(&curr_tile_id) {
            let neighbor_idx = *map.id_to_index.get(&neighbor)?;
            let distance = current_distance + 1.0;
            if let Some(&existing_distance) = distances.get(&neighbor_idx) {
                if existing_distance <= distance {
                    continue;
                }
            }
            distances.insert(neighbor_idx, distance);
            previous.insert(neighbor_idx, current);
            queue.push(NodePriority { node: neighbor_idx, priority: distance });
        }
    }
    None
}