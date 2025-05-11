// Source: orig_src/core/src/com/unciv/ui/screens/pickerscreens/PromotionTree.kt

use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use crate::game::unit::Unit;
use crate::game::promotion::Promotion;

/// A tree structure representing the promotion hierarchy for a unit
pub struct PromotionTree {
    pub root: Rc<PromotionNode>,
    pub nodes: HashMap<String, Rc<PromotionNode>>,
    pub unit: Rc<Unit>,
}

/// A node in the promotion tree
pub struct PromotionNode {
    pub promotion: Rc<Promotion>,
    pub children: Vec<Rc<PromotionNode>>,
    pub prerequisites: Vec<Rc<PromotionNode>>,
    pub is_pickable: bool,
    pub is_selected: bool,
    pub is_adopted: bool,
}

impl PromotionTree {
    /// Creates a new promotion tree for the given unit
    pub fn new(unit: Rc<Unit>) -> Self {
        let mut tree = Self {
            root: Rc::new(PromotionNode {
                promotion: Rc::new(Promotion::default()),
                children: Vec::new(),
                prerequisites: Vec::new(),
                is_pickable: false,
                is_selected: false,
                is_adopted: false,
            }),
            nodes: HashMap::new(),
            unit,
        };
        tree.build_tree();
        tree
    }

    /// Builds the promotion tree structure
    fn build_tree(&mut self) {
        // Clear existing nodes
        self.nodes.clear();
        self.root.children.clear();
        self.root.prerequisites.clear();

        // Add root node
        self.nodes.insert("root".to_string(), self.root.clone());

        // Add all promotions as nodes
        for promotion in &self.unit.promotions {
            let node = Rc::new(PromotionNode {
                promotion: promotion.clone(),
                children: Vec::new(),
                prerequisites: Vec::new(),
                is_pickable: self.unit.can_pick_promotion(promotion),
                is_selected: false,
                is_adopted: self.unit.has_promotion(promotion),
            });
            self.nodes.insert(promotion.name.clone(), node.clone());
        }

        // Build relationships between nodes
        for node in self.nodes.values() {
            if node.promotion.name == "root" {
                continue;
            }

            // Add prerequisites
            for prereq_name in &node.promotion.prerequisites {
                if let Some(prereq_node) = self.nodes.get(prereq_name) {
                    node.prerequisites.push(prereq_node.clone());
                }
            }

            // Add to parent's children
            if let Some(parent_name) = &node.promotion.parent {
                if let Some(parent_node) = self.nodes.get(parent_name) {
                    parent_node.children.push(node.clone());
                }
            } else {
                // If no parent, add to root
                self.root.children.push(node.clone());
            }
        }
    }

    /// Gets a node by promotion name
    pub fn get_node(&self, promotion_name: &str) -> Option<&Rc<PromotionNode>> {
        self.nodes.get(promotion_name)
    }

    /// Gets all nodes that are prerequisites for the given promotion
    pub fn get_prerequisites(&self, promotion_name: &str) -> HashSet<&Rc<PromotionNode>> {
        let mut prerequisites = HashSet::new();
        if let Some(node) = self.get_node(promotion_name) {
            self.collect_prerequisites(node, &mut prerequisites);
        }
        prerequisites
    }

    /// Recursively collects all prerequisites for a node
    fn collect_prerequisites(&self, node: &Rc<PromotionNode>, prerequisites: &mut HashSet<&Rc<PromotionNode>>) {
        for prereq in &node.prerequisites {
            prerequisites.insert(prereq);
            self.collect_prerequisites(prereq, prerequisites);
        }
    }

    /// Gets all nodes that are children of the given promotion
    pub fn get_children(&self, promotion_name: &str) -> HashSet<&Rc<PromotionNode>> {
        let mut children = HashSet::new();
        if let Some(node) = self.get_node(promotion_name) {
            self.collect_children(node, &mut children);
        }
        children
    }

    /// Recursively collects all children for a node
    fn collect_children(&self, node: &Rc<PromotionNode>, children: &mut HashSet<&Rc<PromotionNode>>) {
        for child in &node.children {
            children.insert(child);
            self.collect_children(child, children);
        }
    }

    /// Updates the pickable state of all nodes
    pub fn update_pickable_states(&mut self) {
        for node in self.nodes.values() {
            if node.promotion.name != "root" {
                node.is_pickable = self.unit.can_pick_promotion(&node.promotion);
            }
        }
    }
}