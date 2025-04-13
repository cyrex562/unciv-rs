use std::collections::{HashMap, HashSet, VecDeque};
use crate::map::tile::Tile;

/// BFS (Breadth-First Search) is an implementation of the breadth-first search algorithm,
/// commonly used for finding the shortest path or connected tiles in a graph.
///
/// The algorithm explores all neighboring tiles at the present depth before moving on to tiles at the next depth level.
///
/// # Arguments
/// * `starting_point` - The initial tile where the search begins.
/// * `predicate` - A function that determines if a tile should be considered for further exploration.
///                 For instance, it might return `true` for passable tiles and `false` for obstacles.
///
/// # Example
/// ```
/// let bfs_search = BFS::new(
///     start_tile,
///     |tile| tile.is_passable(),
/// );
///
/// let path = bfs_search.get_path_to(goal_tile);
/// ```
pub struct BFS {
    /// The starting point of the search
    pub starting_point: Tile,
    /// Maximum number of tiles to search
    pub max_size: usize,
    /// Remaining tiles to check
    tiles_to_check: VecDeque<Tile>,
    /// Each tile reached points to its parent tile, where we got to it from
    tiles_reached: HashMap<Tile, Tile>,
    /// Function that determines if a tile should be considered
    predicate: Box<dyn Fn(&Tile) -> bool>,
}

impl BFS {
    /// Creates a new BFS instance
    pub fn new<F>(
        starting_point: Tile,
        predicate: F,
    ) -> Self
    where
        F: Fn(&Tile) -> bool + 'static,
    {
        let mut tiles_to_check = VecDeque::with_capacity(37); // needs resize at distance 4
        let mut tiles_reached = HashMap::new();

        tiles_to_check.push_back(starting_point.clone());
        tiles_reached.insert(starting_point.clone(), starting_point.clone());

        Self {
            starting_point,
            max_size: usize::MAX,
            tiles_to_check,
            tiles_reached,
            predicate: Box::new(predicate),
        }
    }

    /// Process fully until there's nowhere left to check
    pub fn step_to_end(&mut self) {
        while !self.has_ended() {
            self.next_step();
        }
    }

    /// Process until either destination is reached or there's nowhere left to check
    ///
    /// # Arguments
    /// * `destination` - The destination tile to reach.
    ///
    /// # Returns
    /// A reference to this BFS instance, allowing for method chaining.
    pub fn step_until_destination(&mut self, destination: &Tile) -> &Self {
        while !self.tiles_reached.contains_key(destination) && !self.has_ended() {
            self.next_step();
        }
        self
    }

    /// Process one tile-to-search, fetching all neighbors not yet touched
    /// and adding those that fulfill the predicate to the reached set
    /// and to the yet-to-be-processed set.
    ///
    /// Will do nothing when has_ended returns true
    ///
    /// # Returns
    /// The Tile that was checked, or None if there was nothing to do
    pub fn next_step(&mut self) -> Option<Tile> {
        if self.tiles_reached.len() >= self.max_size {
            self.tiles_to_check.clear();
            return None;
        }

        let current = match self.tiles_to_check.pop_front() {
            Some(tile) => tile,
            None => return None,
        };

        for neighbor in &current.neighbors {
            if !self.tiles_reached.contains_key(neighbor) && (self.predicate)(neighbor) {
                self.tiles_reached.insert(neighbor.clone(), current.clone());
                self.tiles_to_check.push_back(neighbor.clone());
            }
        }

        Some(current)
    }

    /// Returns a vector from the destination back to the starting_point, including both,
    /// or empty if destination has not been reached
    ///
    /// # Arguments
    /// * `destination` - The destination tile to trace the path to.
    ///
    /// # Returns
    /// A vector of tiles representing the path from the destination to the starting point.
    pub fn get_path_to(&self, destination: &Tile) -> Vec<Tile> {
        let mut path = Vec::new();
        let mut current_node = destination.clone();

        loop {
            let parent = match self.tiles_reached.get(&current_node) {
                Some(p) => p.clone(),
                None => break, // destination is not in our path
            };

            path.push(current_node.clone());

            if current_node == self.starting_point {
                break;
            }

            current_node = parent;
        }

        path
    }

    /// Returns true if there are no more tiles to check
    ///
    /// # Returns
    /// True if the search has ended, otherwise false.
    pub fn has_ended(&self) -> bool {
        self.tiles_to_check.is_empty()
    }

    /// Returns true if the tile has been reached
    ///
    /// # Arguments
    /// * `tile` - The tile to check.
    ///
    /// # Returns
    /// True if the tile has been reached, otherwise false.
    pub fn has_reached_tile(&self, tile: &Tile) -> bool {
        self.tiles_reached.contains_key(tile)
    }

    /// Returns all tiles reached so far
    ///
    /// # Returns
    /// A set of tiles that have been reached.
    pub fn get_reached_tiles(&self) -> HashSet<Tile> {
        self.tiles_reached.keys().cloned().collect()
    }

    /// Returns number of tiles reached so far
    ///
    /// # Returns
    /// The count of tiles reached.
    pub fn size(&self) -> usize {
        self.tiles_reached.len()
    }
}