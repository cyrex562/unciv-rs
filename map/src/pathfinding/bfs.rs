use crate::map::{GameMap, TileId};
use std::collections::{HashMap, HashSet, VecDeque};

/// Performs a breadth-first search (BFS) on the map starting from `starting_point`.
/// Returns a tuple with the reached tiles and the parent map for path reconstruction.
pub fn do_bfs<F>(
    map: &GameMap,
    starting_point: TileId,
    predicate: F,
    max_size: usize,
    destination: Option<TileId>,
) -> (HashSet<TileId>, HashMap<TileId, TileId>)
where
    F: Fn(&GameMap, &TileId) -> bool,
{
    let mut tiles_to_check = VecDeque::with_capacity(37);
    let mut tiles_reached: HashMap<TileId, TileId> = HashMap::new();

    tiles_to_check.push_back(starting_point);
    tiles_reached.insert(starting_point, starting_point);

    while let Some(current) = tiles_to_check.pop_front() {
        if tiles_reached.len() >= max_size {
            break;
        }
        if let Some(dest) = destination {
            if current == dest {
                break;
            }
        }
        for neighbor in map.get_neighbors(&current) {
            if !tiles_reached.contains_key(&neighbor) && predicate(map, &neighbor) {
                tiles_reached.insert(neighbor, current);
                tiles_to_check.push_back(neighbor);
            }
        }
    }
    (tiles_reached.keys().cloned().collect(), tiles_reached)
}

/// Reconstructs the path from `destination` back to `starting_point` using the parent map.
pub fn bfs_get_path_to(
    starting_point: TileId,
    destination: TileId,
    parent_map: &HashMap<TileId, TileId>,
) -> Vec<TileId> {
    let mut path = Vec::new();
    let mut current = destination;
    while let Some(parent) = parent_map.get(&current) {
        path.push(current);
        if current == starting_point {
            break;
        }
        current = *parent;
    }
    path
}

/// Checks if BFS has ended (no more tiles to check)
pub fn bfs_has_ended(tiles_to_check: &VecDeque<TileId>) -> bool {
    tiles_to_check.is_empty()
}

/// Checks if a tile has been reached
pub fn bfs_has_reached_tile(tile: &TileId, tiles_reached: &HashMap<TileId, TileId>) -> bool {
    tiles_reached.contains_key(tile)
}

pub fn do_bfs_wrapper(
    map: &GameMap,
    starting_point: TileId,
    predicate: Box<dyn Fn(&GameMap, &TileId) -> bool>,
) -> Vec<TileId> {
    let (reached, _) = do_bfs(map, starting_point, predicate, usize::MAX, None);
    reached.into_iter().collect()
}
