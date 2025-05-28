use std::collections::HashMap;
use petgraph::Graph;
use petgraph::graph::NodeIndex;
use crate::tile::Tile;

pub type TileId = (i32, i32);


pub struct GameMap {
    pub(crate) graph: Graph<TileId, ()>,            // Topology
    pub(crate) id_to_index: HashMap<TileId, NodeIndex>,       // Coord to graph node
    pub(crate) index_to_id: HashMap<NodeIndex, TileId>,       // Optional
    tiles: HashMap<TileId, Tile>                  // Actual tile data
}