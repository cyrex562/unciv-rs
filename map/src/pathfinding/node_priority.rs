use petgraph::graph::NodeIndex;

/// A node with its priority for the priority queue
#[derive(Debug, Clone, PartialEq)]
pub struct NodePriority {
    pub node: NodeIndex,
    pub priority: f32,
}

impl Eq for NodePriority {
    /// This is a no-op function that exists only to satisfy the requirements of the `Eq` trait.
    /// It is needed because the `Ord` trait requires `Eq`, but the `PartialEq` implementation
    /// provided by `PartialOrd` is not sufficient to satisfy the `Eq` trait's requirements.
    ///
    /// See [the `Eq` trait's documentation](https://doc.rust-lang.org/std/cmp/trait.Eq.html) for more information.
    fn assert_receiver_is_total_eq(&self) {}
}

impl Ord for NodePriority {
    /// Compares two `NodePriority` values by their priority.
    ///
    /// Reverses the ordering for min-heap behavior, so that lower priority values come first.
    /// Also handles NaN values by considering them greater.
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Reverse ordering for min-heap behavior (lower priority values come first)
        // Also handle NaN values by considering them greater
        other.priority.partial_cmp(&self.priority)
            .unwrap_or(std::cmp::Ordering::Greater)
    }
}

impl PartialOrd for NodePriority {
    /// Compares two `NodePriority` values by their priority.
    ///
    /// Reverses the ordering for min-heap behavior, so that lower priority values come first.
    /// Also handles NaN values by considering them greater.
    ///
    /// This function always returns `Some`, so it can be safely used with `unwrap`.
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}