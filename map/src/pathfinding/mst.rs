use petgraph::Graph;
use petgraph::prelude::EdgeRef;
use petgraph::visit::{IntoEdges, NodeCount};
use crate::map::TileId;

/// Minimum Spanning Tree (MST) implementation using Kruskal's algorithm
pub fn mst(graph: &Graph<TileId, f32>) -> Vec<(TileId, TileId, f32)> {
    let mut mst = Vec::new();
    let mut disjoint_set = DisjointSet::new(graph.node_count());

    let mut edges = graph.edge_references().collect::<Vec<_>>();
    edges.sort_by(|a, b| a.weight().partial_cmp(&b.weight()).unwrap());

    for edge in edges {
        let u = edge.source().index();
        let v = edge.target().index();
        let w = *edge.weight();
        if disjoint_set.find(u) != disjoint_set.find(v) {
            mst.push((graph.node_weight(edge.source()).cloned().unwrap(),
                      graph.node_weight(edge.target()).cloned().unwrap(),
                      w));
            disjoint_set.union(u, v);
        }
    }

    mst
}

struct DisjointSet {
    parent: Vec<usize>,
}

impl DisjointSet {
    fn new(n: usize) -> DisjointSet {
        DisjointSet {
            parent: (0..n).collect(),
        }
    }

    fn find(&mut self, x: usize) -> usize {
        if self.parent[x] != x {
            self.parent[x] = self.find(self.parent[x]);
        }
        self.parent[x]
    }

    fn union(&mut self, x: usize, y: usize) {
        let x_root = self.find(x);
        let y_root = self.find(y);
        if x_root != y_root {
            self.parent[x_root] = y_root;
        }
    }
}