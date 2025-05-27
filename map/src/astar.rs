use std::collections::{BinaryHeap, HashMap, HashSet};
use std::cmp::Ordering;
use crate::position::Position;
use crate::tile::Tile;

/// Represents a tile with its priority in the A* algorithm
#[derive(Debug, Clone)]
pub struct TilePriority {
    /// The tile
    pub tile: Tile,
    /// The priority value (lower is higher priority)
    pub priority: f32,
}

impl PartialEq for TilePriority {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}

impl Eq for TilePriority {}

impl PartialOrd for TilePriority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // Reverse ordering so that lower values have higher priority
        other.priority.partial_cmp(&self.priority)
    }
}

impl Ord for TilePriority {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering so that lower values have higher priority
        other.priority.partial_cmp(&self.priority).unwrap_or(Ordering::Equal)
    }
}

/// AStar is an implementation of the A* search algorithm, commonly used for finding the shortest path
/// in a weighted graph.
///
/// The algorithm maintains a priority queue of paths while exploring the graph, expanding paths in
/// order of their estimated total cost from the start node to the goal node, factoring in both the
/// cost so far and an estimated cost (heuristic) to the goal.
///
/// # Arguments
/// * `starting_point` - The initial tile where the search begins.
/// * `predicate` - A function that determines if a tile should be considered for further exploration.
///                 For instance, it might return `true` for passable tiles and `false` for obstacles.
/// * `cost` - A function that takes two tiles (fromTile, toTile) as input and returns the cost
///            of moving from 'fromTile' to 'toTile' as a Float. This allows for flexible cost
///            calculations based on different criteria, such as distance, terrain, or other
///            custom logic defined by the user.
/// * `heuristic` - A function that estimates the cost from a given tile to the goal. For the A*
///                 algorithm to guarantee the shortest path, this heuristic must be admissible,
///                 meaning it should never overestimate the actual cost to reach the goal.
///                 You can set this to `|_, _| 0.0` for Dijkstra's algorithm.
///
/// # Example
/// ```
/// let unit: MapUnit = ...;
/// let a_star_search = AStar::new(
///     start_tile,
///     |tile| tile.is_passable(),
///     |from, to| MovementCost::get_movement_cost_between_adjacent_tiles(unit, from, to),
///     |from, to| <custom heuristic>
/// );
///
/// let path = a_star_search.find_path(goal_tile);
/// ```
pub struct AStar {
    /// The starting point of the search
    pub starting_point: Tile,
    /// Maximum number of tiles to search
    pub max_size: usize,
    /// Cache for storing the costs
    cost_cache: HashMap<(Tile, Tile), f32>,
    /// Frontier priority queue for managing the tiles to be checked
    tiles_to_check: BinaryHeap<TilePriority>,
    /// A map where each tile reached during the search points to its parent tile
    tiles_reached: HashMap<Tile, Tile>,
    /// A map holding the cumulative cost to reach each tile
    cumulative_tile_cost: HashMap<Tile, f32>,
    /// Function that determines if a tile should be considered
    predicate: Box<dyn Fn(&Tile) -> bool>,
    /// Function that calculates the cost between two tiles
    cost: Box<dyn Fn(&Tile, &Tile) -> f32>,
    /// Function that estimates the cost from a tile to the goal
    heuristic: Box<dyn Fn(&Tile, &Tile) -> f32>,
}

impl AStar {
    /// Creates a new AStar instance
    pub fn new<F, G, H>(
        starting_point: Tile,
        predicate: F,
        cost: G,
        heuristic: H,
    ) -> Self
    where
        F: Fn(&Tile) -> bool + 'static,
        G: Fn(&Tile, &Tile) -> f32 + 'static,
        H: Fn(&Tile, &Tile) -> f32 + 'static,
    {
        let mut tiles_to_check = BinaryHeap::new();
        let mut tiles_reached = HashMap::new();
        let mut cumulative_tile_cost = HashMap::new();

        tiles_to_check.push(TilePriority {
            tile: starting_point.clone(),
            priority: 0.0,
        });
        tiles_reached.insert(starting_point.clone(), starting_point.clone());
        cumulative_tile_cost.insert(starting_point.clone(), 0.0);

        Self {
            starting_point,
            max_size: usize::MAX,
            cost_cache: HashMap::new(),
            tiles_to_check,
            tiles_reached,
            cumulative_tile_cost,
            predicate: Box::new(predicate),
            cost: Box::new(cost),
            heuristic: Box::new(heuristic),
        }
    }

    /// Retrieves the cost of moving to a given tile, utilizing a cache to improve efficiency.
    /// If the cost for a tile is not already cached, it computes the cost using the provided cost function and stores it in the cache.
    ///
    /// # Arguments
    /// * `from` - The source tile.
    /// * `to` - The destination tile.
    ///
    /// # Returns
    /// The cost of moving between the tiles.
    fn get_cost(&mut self, from: &Position, to: &Position) -> f32 {
        // TODO: Implement a more efficient caching mechanism
        unimplemented!()
    }

    /// Continues the search process until there are no more tiles left to check.
    pub fn step_to_end(&mut self) {
        while !self.has_ended() {
            self.next_step();
        }
    }

    /// Continues the search process until either the specified destination is reached or there are no more tiles left to check.
    ///
    /// # Arguments
    /// * `destination` - The destination tile to reach.
    ///
    /// # Returns
    /// A reference to this AStar instance, allowing for method chaining.
    pub fn step_until_destination(&mut self, destination: &Tile) -> &Self {
        while !self.tiles_reached.contains_key(destination) && !self.has_ended() {
            self.next_step();
        }
        self
    }

    /// Processes one step in the A* algorithm, expanding the search from the current tile to its neighbors.
    /// It updates the search structures accordingly, considering both the cost so far and the heuristic estimate.
    ///
    /// If the maximum size is reached or no more tiles are available, this method will do nothing.
    pub fn next_step(&mut self) {
        if self.tiles_reached.len() >= self.max_size {
            self.tiles_to_check.clear();
            return;
        }

        let current_tile = match self.tiles_to_check.pop() {
            Some(tp) => tp.tile,
            None => return,
        };

        for neighbor in &current_tile.neighbors {
            let new_cost = self.cumulative_tile_cost[&current_tile] + self.get_cost(&current_tile, neighbor);

            if (self.predicate)(neighbor) &&
               (!self.cumulative_tile_cost.contains_key(neighbor) ||
                new_cost < *self.cumulative_tile_cost.get(neighbor).unwrap_or(&f32::MAX))
            {
                self.cumulative_tile_cost.insert(neighbor.clone(), new_cost);
                let priority = new_cost + (self.heuristic)(neighbor, &current_tile);
                self.tiles_to_check.push(TilePriority {
                    tile: neighbor.clone(),
                    priority,
                });
                self.tiles_reached.insert(neighbor.clone(), current_tile.clone());
            }
        }
    }

    /// Constructs a sequence representing the path from the given destination tile back to the starting point.
    /// If the destination has not been reached, the sequence will be empty.
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

    /// Checks if there are no more tiles to be checked in the search.
    ///
    /// # Returns
    /// True if the search has ended, otherwise false.
    pub fn has_ended(&self) -> bool {
        self.tiles_to_check.is_empty()
    }

    /// Determines if a specific tile has been reached during the search.
    ///
    /// # Arguments
    /// * `tile` - The tile to check.
    ///
    /// # Returns
    /// True if the tile has been reached, otherwise false.
    pub fn has_reached_tile(&self, tile: &Tile) -> bool {
        self.tiles_reached.contains_key(tile)
    }

    /// Retrieves all tiles that have been reached so far in the search.
    ///
    /// # Returns
    /// A set of tiles that have been reached.
    pub fn get_reached_tiles(&self) -> HashSet<Tile> {
        self.tiles_reached.keys().cloned().collect()
    }

    /// Provides the number of tiles that have been reached so far in the search.
    ///
    /// # Returns
    /// The count of tiles reached.
    pub fn size(&self) -> usize {
        self.tiles_reached.len()
    }
}