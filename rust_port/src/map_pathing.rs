use std::collections::{HashMap, HashSet, VecDeque};
use std::f32;
use crate::civilization::Civilization;
use crate::map::mapunit::MapUnit;
use crate::map::tile::Tile;
use crate::utils::Log;

/// A* pathfinding implementation for finding optimal paths between tiles
struct AStar<'a> {
    start: &'a Tile,
    predicate: Box<dyn Fn(&Tile) -> bool + 'a>,
    cost: Box<dyn Fn(&Tile, &Tile) -> f32 + 'a>,
    heuristic: Box<dyn Fn(&Tile, &Tile) -> f32 + 'a>,
    open_set: HashSet<&'a Tile>,
    closed_set: HashSet<&'a Tile>,
    came_from: HashMap<&'a Tile, &'a Tile>,
    g_score: HashMap<&'a Tile, f32>,
    f_score: HashMap<&'a Tile, f32>,
}

impl<'a> AStar<'a> {
    /// Creates a new A* pathfinding instance
    fn new(
        start: &'a Tile,
        predicate: impl Fn(&Tile) -> bool + 'a,
        cost: impl Fn(&Tile, &Tile) -> f32 + 'a,
        heuristic: impl Fn(&Tile, &Tile) -> f32 + 'a,
    ) -> Self {
        let mut g_score = HashMap::new();
        let mut f_score = HashMap::new();
        g_score.insert(start, 0.0);
        f_score.insert(start, heuristic(start, start));

        let mut open_set = HashSet::new();
        open_set.insert(start);

        Self {
            start,
            predicate: Box::new(predicate),
            cost: Box::new(cost),
            heuristic: Box::new(heuristic),
            open_set,
            closed_set: HashSet::new(),
            came_from: HashMap::new(),
            g_score,
            f_score,
        }
    }

    /// Checks if the A* search has ended (no path found)
    fn has_ended(&self) -> bool {
        self.open_set.is_empty()
    }

    /// Checks if a specific tile has been reached
    fn has_reached_tile(&self, tile: &Tile) -> bool {
        self.closed_set.contains(tile)
    }

    /// Gets the current size of the search
    fn size(&self) -> usize {
        self.open_set.len() + self.closed_set.len()
    }

    /// Performs the next step of the A* algorithm
    fn next_step(&mut self) {
        if self.open_set.is_empty() {
            return;
        }

        // Find the tile with the lowest f_score in the open set
        let current = self.open_set
            .iter()
            .min_by(|a, b| {
                let a_score = self.f_score.get(**a).unwrap_or(&f32::INFINITY);
                let b_score = self.f_score.get(**b).unwrap_or(&f32::INFINITY);
                a_score.partial_cmp(b_score).unwrap_or(std::cmp::Ordering::Equal)
            })
            .copied()
            .unwrap();

        if current == self.start {
            self.open_set.remove(current);
            self.closed_set.insert(current);
            return;
        }

        self.open_set.remove(current);
        self.closed_set.insert(current);

        // Process neighbors
        for neighbor in current.neighbors.iter() {
            if self.closed_set.contains(neighbor) {
                continue;
            }

            if !(self.predicate)(neighbor) {
                continue;
            }

            let tentative_g_score = self.g_score.get(current).unwrap_or(&f32::INFINITY) + (self.cost)(current, neighbor);

            if !self.open_set.contains(neighbor) {
                self.open_set.insert(neighbor);
            } else if tentative_g_score >= *self.g_score.get(neighbor).unwrap_or(&f32::INFINITY) {
                continue;
            }

            self.came_from.insert(neighbor, current);
            self.g_score.insert(neighbor, tentative_g_score);
            self.f_score.insert(neighbor, tentative_g_score + (self.heuristic)(neighbor, self.start));
        }
    }

    /// Gets the path from the start tile to a specific tile
    fn get_path_to(&self, tile: &Tile) -> Vec<&Tile> {
        let mut path = Vec::new();
        let mut current = tile;

        while self.came_from.contains_key(current) {
            path.push(current);
            current = self.came_from[current];
        }
        path.push(self.start);

        path
    }
}

/// Map pathfinding functionality for finding paths between tiles
pub struct MapPathing;

impl MapPathing {
    /// Calculates the preferred movement cost for road construction
    ///
    /// We prefer the worker to prioritize paths connected by existing roads.
    /// If a tile has a road, but the civ has the ability to upgrade it to a railroad,
    /// we consider it to be a railroad for pathing since it will be upgraded.
    /// Otherwise, we set every tile to have equal value since building a road on any
    /// of them makes the original movement cost irrelevant.
    fn road_preferred_movement_cost(unit: &MapUnit, _from: &Tile, to: &Tile) -> f32 {
        // has_road_connection accounts for civs that treat jungle/forest as roads
        // Ignore road over river penalties.
        if to.has_road_connection(&unit.civ, false) || to.has_railroad_connection(false) {
            return 0.5;
        }

        1.0
    }

    /// Checks if a tile is valid for road path construction
    pub fn is_valid_road_path_tile(unit: &MapUnit, tile: &Tile) -> bool {
        let road_improvement = match &tile.ruleset.road_improvement {
            Some(imp) => imp,
            None => return false,
        };

        let railroad_improvement = match &tile.ruleset.railroad_improvement {
            Some(imp) => imp,
            None => return false,
        };

        if tile.is_water {
            return false;
        }

        if tile.is_impassible() {
            return false;
        }

        if !unit.civ.has_explored(tile) {
            return false;
        }

        if !tile.can_civ_pass_through(&unit.civ) {
            return false;
        }

        tile.has_road_connection(&unit.civ, false)
            || tile.has_railroad_connection(false)
            || tile.improvement_functions.can_build_improvement(road_improvement, &unit.civ)
            || tile.improvement_functions.can_build_improvement(railroad_improvement, &unit.civ)
    }

    /// Calculates the path for road construction between two tiles
    ///
    /// This function uses the A* search algorithm to find an optimal path for road
    /// construction between two specified tiles.
    ///
    /// # Arguments
    ///
    /// * `unit` - The unit that will construct the road
    /// * `start_tile` - The starting tile of the path
    /// * `end_tile` - The destination tile of the path
    ///
    /// # Returns
    ///
    /// A vector of tiles representing the path from start_tile to end_tile,
    /// or None if no valid path is found
    pub fn get_road_path(unit: &MapUnit, start_tile: &Tile, end_tile: &Tile) -> Option<Vec<&Tile>> {
        Self::get_path(
            unit,
            start_tile,
            end_tile,
            Self::is_valid_road_path_tile,
            Self::road_preferred_movement_cost,
            |_, _, _| 0.0,
        )
    }

    /// Calculates the path between two tiles
    ///
    /// This function uses the A* search algorithm to find an optimal path between
    /// two specified tiles on a game map.
    ///
    /// # Arguments
    ///
    /// * `unit` - The unit for which the path is being calculated
    /// * `start_tile` - The tile from which the pathfinding begins
    /// * `end_tile` - The destination tile for the pathfinding
    /// * `predicate` - A function that determines whether a tile can be traversed by the unit
    /// * `cost` - A function that calculates the cost of moving from one tile to another
    /// * `heuristic` - A function that estimates the cost from a given tile to the end tile
    ///
    /// # Returns
    ///
    /// A vector of tiles representing the path from the start_tile to the end_tile.
    /// Returns None if no valid path is found.
    fn get_path<F, G, H>(
        unit: &MapUnit,
        start_tile: &Tile,
        end_tile: &Tile,
        predicate: F,
        cost: G,
        heuristic: H,
    ) -> Option<Vec<&Tile>>
    where
        F: Fn(&MapUnit, &Tile) -> bool + 'static,
        G: Fn(&MapUnit, &Tile, &Tile) -> f32 + 'static,
        H: Fn(&MapUnit, &Tile, &Tile) -> f32 + 'static,
    {
        let predicate_wrapper = move |tile: &Tile| predicate(unit, tile);
        let cost_wrapper = move |from: &Tile, to: &Tile| cost(unit, from, to);
        let heuristic_wrapper = move |from: &Tile, to: &Tile| heuristic(unit, from, to);

        let mut astar = AStar::new(
            start_tile,
            predicate_wrapper,
            cost_wrapper,
            heuristic_wrapper,
        );

        loop {
            if astar.has_ended() {
                // We failed to find a path
                Log::debug(&format!("get_path failed at AStar search size {}", astar.size()));
                return None;
            }

            if !astar.has_reached_tile(end_tile) {
                astar.next_step();
                continue;
            }

            // Found a path
            let mut path = astar.get_path_to(end_tile);
            path.reverse();
            return Some(path);
        }
    }

    /// Gets the connection to the end tile
    ///
    /// This does not take into account tile movement costs.
    /// Takes in a civilization instead of a specific unit.
    ///
    /// # Arguments
    ///
    /// * `civ` - The civilization for which the connection is being calculated
    /// * `start_tile` - The starting tile of the connection
    /// * `end_tile` - The destination tile of the connection
    /// * `predicate` - A function that determines whether a tile can be traversed by the civilization
    ///
    /// # Returns
    ///
    /// A vector of tiles representing the connection from start_tile to end_tile.
    /// Returns None if no valid connection is found.
    pub fn get_connection<F>(
        civ: &Civilization,
        start_tile: &Tile,
        end_tile: &Tile,
        predicate: F,
    ) -> Option<Vec<&Tile>>
    where
        F: Fn(&Civilization, &Tile) -> bool + 'static,
    {
        let predicate_wrapper = move |tile: &Tile| predicate(civ, tile);
        let cost_wrapper = |_: &Tile, _: &Tile| 1.0;
        let heuristic_wrapper = |from: &Tile, to: &Tile| from.aerial_distance_to(to) as f32;

        let mut astar = AStar::new(
            start_tile,
            predicate_wrapper,
            cost_wrapper,
            heuristic_wrapper,
        );

        loop {
            if astar.has_ended() {
                // We failed to find a path
                Log::debug(&format!("get_connection failed at AStar search size {}", astar.size()));
                return None;
            }

            if !astar.has_reached_tile(end_tile) {
                astar.next_step();
                continue;
            }

            // Found a path
            let mut path = astar.get_path_to(end_tile);
            path.reverse();
            return Some(path);
        }
    }
}