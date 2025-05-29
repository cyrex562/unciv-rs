use crate::tile::Tile;
use petgraph::graph::NodeIndex;
use petgraph::Graph;
use std::collections::HashMap;

pub type TileId = (i32, i32);

pub struct GameMap {
    pub graph: Graph<TileId, ()>,                // Topology
    pub id_to_index: HashMap<TileId, NodeIndex>, // Coord to graph node
    pub index_to_id: HashMap<NodeIndex, TileId>, // Optional
    tiles: HashMap<TileId, Tile>,                       // Actual tile data
}

impl GameMap {
    /// Returns all neighboring tile positions of the given tile position.
    ///
    /// Neighbors are defined as the 8 tiles that surround the given tile.
    /// Tiles that are not on the map are ignored.
    ///
    /// The returned vector is sorted in the order of the directions array
    /// which is: North, East, South, West, Northeast, Southeast, Southwest,
    /// Northwest.
    pub fn get_neighbors(&self, p0: &TileId) -> Vec<TileId> {
        let mut neighbors = Vec::new();
        let directions = [
            (0, 1),   // North
            (1, 0),   // East
            (0, -1),  // South
            (-1, 0),  // West
            (1, 1),   // Northeast
            (1, -1),  // Southeast
            (-1, -1), // Southwest
            (-1, 1),  // Northwest
        ];

        for &(dx, dy) in &directions {
            let neighbor_id = (p0.0 + dx, p0.1 + dy);
            if self.tiles.contains_key(&neighbor_id) {
                neighbors.push(neighbor_id);
            }
        }
        neighbors
    }
}

impl GameMap {
    /// Creates a new empty `GameMap`.
    ///
    /// Returns a `GameMap` with all its fields initialized to empty containers.
    pub fn new() -> Self {
        GameMap {
            graph: Graph::new(),
            id_to_index: HashMap::new(),
            index_to_id: HashMap::new(),
            tiles: HashMap::new(),
        }
    }

    /// Adds a tile to the map at the given coordinates and associates it with
    /// the given `Tile` data.
    ///
    /// # Arguments
    ///
    /// * `tile_id` - The coordinates of the tile to add.
    /// * `tile` - The `Tile` data to associate with the tile.
    pub fn add_tile(&mut self, tile_id: TileId, tile: Tile) {
        let node_index = self.graph.add_node(tile_id);
        self.id_to_index.insert(tile_id, node_index);
        self.index_to_id.insert(node_index, tile_id);
        self.tiles.insert(tile_id, tile);
    }

    /// Returns a reference to the tile at the given coordinates if it exists,
    /// or `None` otherwise.
    pub fn get_tile(&self, tile_id: &TileId) -> Option<&Tile> {
        self.tiles.get(tile_id)
    }
}
