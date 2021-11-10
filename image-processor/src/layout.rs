use image::RgbImage;
use itertools::Itertools;
use petgraph::{
    dot::{Config, Dot},
    graph::NodeIndex,
    visit::Bfs,
    Direction, Graph,
};
use rand::{
    distributions::{Distribution, Standard},
    prelude::SliceRandom,
    seq::IteratorRandom,
    Rng,
};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::ptr;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct LayoutBlueprint {
    graph_representation: Vec<(String, Vec<usize>)>,
    width: u32,
    height: u32,
}

#[derive(Debug, Clone)]
pub struct Layout<'a> {
    graph: LayoutGraph<'a>,
    pub canvas_dimensions: Dimensions,
}

pub type LayoutGraph<'a> = Graph<NodeLabel<'a>, ()>;

#[derive(PartialEq, Clone, Copy)]
pub enum NodeLabel<'a> {
    Internal(SliceDirection),
    Leaf(&'a RgbImage),
}
use NodeLabel::*;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum SliceDirection {
    Vertical,
    Horizontal,
}
use SliceDirection::*;

impl Distribution<SliceDirection> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> SliceDirection {
        match rng.gen::<bool>() {
            true => SliceDirection::Vertical,
            false => SliceDirection::Horizontal,
        }
    }
}

impl std::fmt::Debug for NodeLabel<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use NodeLabel::*;
        match self {
            Leaf(image) => f
                .debug_tuple("Image")
                .field(&image.width())
                .field(&image.height())
                .finish(),
            Internal(a) => {
                write!(f, "{:?}", a)
            }
        }
    }
}

impl<'a> Layout<'a> {
    pub fn new<R>(images: &'a [RgbImage], rng: &mut R) -> Self
    where
        R: Rng + Sized,
    {
        if images.len() < 2 {
            // The internal graph representation is a full binary tree, so it can't have less than
            // two images.
            panic!("Attempted to create a layout with less than two images");
        }
        // According to the property of full binary trees, a full binary tree with N leaf nodes
        // must have (N - 1) internal nodes, hence (N * 2 - 1) nodes total.
        //
        // Each internal node has exactly two edges.
        let nodes_count = images.len() * 2 - 1;
        let edges_count = (images.len() - 1) * 2;
        let graph = LayoutGraph::with_capacity(nodes_count, edges_count);
        let canvas_dimensions = Self::calculate_random_canvas_dimensions(images, rng);
        let mut layout = Layout {
            graph,
            canvas_dimensions,
        };
        let mut random_images = images.choose_multiple(rng, images.len());

        layout.graph.add_node(NodeLabel::Internal(rand::random()));

        if images.len() > 2 {
            // Total number of internal nodes must be equal to <number of images> - 1 and we
            // already added one internal node.
            for _ in 0..images.len() - 2 {
                let random_index = layout.random_index_of_node_with_less_than_two_children(rng);
                layout.add_node(random_index, NodeLabel::Internal(rand::random()));
            }
        }

        let indexes_of_nodes_with_less_than_two_children: Vec<NodeIndex> = layout
            .indexes_of_nodes_with_less_than_two_children()
            .collect();

        for index in indexes_of_nodes_with_less_than_two_children {
            while layout.node_has_less_than_two_children(index) {
                layout.add_node(
                    index,
                    NodeLabel::Leaf(random_images.next().expect("Ran out of images")),
                );
            }
        }

        layout
    }

    // The blueprint's graph representation in the form of Vec<(String, Vec<usize>)> shows how the
    // internal nodes are laid out:
    //
    //    * The first element of the tuple indicates slice direction, "V" or "H".
    //    * The second element is an array of indices to child nodes from the same main Vec.
    //
    // So, a graph representation of this form in JavaScript…
    //
    //     [
    //       ["V",  [1, 2]],
    //       ["H"], [ ]],
    //       ["V"], [ ]],
    //     ]
    //
    // …represents a graph which looks like this:
    //
    //          ┌───┐
    //          │ V │
    //          └───┘
    //            │
    //      ┌─────┴─────┐
    //      ▼           ▼
    //    ┌───┐       ┌───┐
    //    │ H │       │ V │
    //    └───┘       └───┘
    //
    // Then the images are sequentially added as leaf nodes to any internal node that has less than
    // two children, starting from the first added node to the last added node.
    pub fn from_blueprint(
        blueprint: &LayoutBlueprint,
        images: &'a [RgbImage],
    ) -> Result<Self, String> {
        let graph = LayoutGraph::new();
        let canvas_dimensions = Dimensions {
            width: blueprint.width,
            height: blueprint.height,
        };
        let mut graph_indices: Vec<NodeIndex> =
            Vec::with_capacity(blueprint.graph_representation.len());
        let mut layout = Layout {
            graph,
            canvas_dimensions,
        };

        // Add internal nodes from the blueprint.
        for (label_code, _) in blueprint.graph_representation.iter() {
            let label = match label_code.as_str() {
                "V" => Internal(Vertical),
                "H" => Internal(Horizontal),
                _ => {
                    return Err(format!(
                        "Unknown label in graph representation: {:?}",
                        label_code
                    ));
                }
            };

            graph_indices.push(layout.graph.add_node(label))
        }

        // Add edges between internal nodes based on the blueprint.
        for (parent_i, (_, child_indices)) in blueprint.graph_representation.iter().enumerate() {
            for child_i in child_indices {
                layout
                    .graph
                    .update_edge(graph_indices[parent_i], graph_indices[*child_i], ());
            }
        }

        // Add images as leafs to nodes with less than two children, starting from the first added
        // node to the last one.
        let indexes_of_nodes_with_less_than_two_children: Vec<NodeIndex> = layout
            .indexes_of_nodes_with_less_than_two_children()
            .collect();
        let mut images_iter = images.iter();

        for index in indexes_of_nodes_with_less_than_two_children {
            while layout.node_has_less_than_two_children(index) {
                let image = images_iter.next().ok_or("Ran out of images")?;
                layout.add_node(index, NodeLabel::Leaf(image));
            }
        }

        Ok(layout)
    }

    pub fn to_blueprint(&self) -> LayoutBlueprint {
        let blueprint = self.subtree_to_blueprint(self.root_node().index);

        LayoutBlueprint {
            graph_representation: blueprint,
            width: self.canvas_dimensions.width,
            height: self.canvas_dimensions.height,
        }
    }

    fn subtree_to_blueprint(&self, index: NodeIndex) -> Vec<(String, Vec<usize>)> {
        let mut blueprint_with_node_indices = vec![];

        for node in self.logical_subtree_bfs_iter(index) {
            if let Internal(_) = node.node_label() {
                let children = node.children().unwrap();

                blueprint_with_node_indices
                    .push((node.index, vec![children.0.index, children.1.index]));
            }
        }

        let mut blueprint = Vec::with_capacity(blueprint_with_node_indices.len());

        for (index, children_indices) in &blueprint_with_node_indices {
            let label = match self.graph[*index] {
                Internal(Vertical) => "V".to_string(),
                Internal(Horizontal) => "H".to_string(),
                _ => {
                    unreachable!();
                }
            };
            let children = children_indices
                .iter()
                .filter_map(|child_index| {
                    if let Internal(_) = self.graph[*child_index] {
                        Some(
                            blueprint_with_node_indices
                                .iter()
                                .position(|(i, _)| i == child_index)
                                .unwrap_or_else(|| {
                                    panic!("{:?} not found in blueprint", child_index)
                                }),
                        )
                    } else {
                        None
                    }
                })
                .collect();

            blueprint.push((label, children));
        }

        blueprint
    }

    pub fn aspect_ratio(&self) -> f64 {
        self.root_node().aspect_ratio()
    }

    pub fn dimensions(&self) -> (u32, u32) {
        self.root_node().dimensions().to_tuple()
    }

    // Smaller value is better.
    pub fn cost(&self) -> f64 {
        let number_of_images = self.leaf_nodes().count() as f64;

        number_of_images * self.scale_factor() + self.coverage_of_canvas_area()
    }

    // Previous implementation of the cost function, useful for comparing new results to old ones.
    pub fn old_cost(&self) -> f64 {
        let number_of_images = self.leaf_nodes().count() as f64;

        self.scale_factor() + number_of_images * self.coverage_of_canvas_area()
    }

    fn coverage_of_canvas_area(&self) -> f64 {
        1.0 - self
            .leaf_nodes()
            .map(|leaf_node| {
                leaf_node.dimensions().size() as f64 / self.canvas_dimensions.size() as f64
            })
            .sum::<f64>()
    }

    fn scale_factor(&self) -> f64 {
        self.leaf_nodes()
            .map(|leaf_node| {
                let original_image_size =
                    Dimensions::from_tuple(leaf_node.image().unwrap().dimensions()).size() as f64;
                let scaled_image_size = leaf_node.dimensions().size() as f64;

                (scaled_image_size - original_image_size).abs() / original_image_size
            })
            .sum::<f64>()
    }

    pub fn swap_random_node_pair<R>(&mut self, rng: &mut R)
    where
        R: Rng + Sized,
    {
        let random_node_index = self.graph.node_indices().choose(rng).unwrap();
        self.swap_with_random_node(rng, random_node_index);
    }

    fn swap_with_random_node<R>(&mut self, rng: &mut R, random_node_index: NodeIndex)
    where
        R: Rng + Sized,
    {
        let random_node_label = self.graph[random_node_index];

        let other_node_index = match random_node_label {
            NodeLabel::Internal(_) => {
                match self
                    .internal_nodes()
                    .filter(|node| *node.node_label() != random_node_label)
                    .choose(rng)
                {
                    Some(node) => node.index,
                    None => {
                        // This can happen if all internal nodes have the same node label or
                        // there's just a single internal node.
                        //
                        // In this situation we fall back to swapping leaf nodes, as we don't want
                        // to have a mutation that does nothing.
                        let random_leaf_node_index = self.leaf_nodes().choose(rng).unwrap().index;
                        self.swap_with_random_node(rng, random_leaf_node_index);
                        return;
                    }
                }
            }
            NodeLabel::Leaf(_) => {
                self.leaf_nodes()
                    .filter(|node| node.index != random_node_index)
                    .choose(rng)
                    .unwrap()
                    .index
            }
        };
        let other_node_label = self.graph[other_node_index];

        let (a, b) = self
            .graph
            .index_twice_mut(random_node_index, other_node_index);
        *a = other_node_label;
        *b = random_node_label;
    }

    pub fn randomize_width<R>(&mut self, rng: &mut R)
    where
        R: Rng + Sized,
    {
        let width = self.canvas_dimensions.width as i64;
        let new_width =
            width + rng.gen_range(-width + 1..=(2 * self.canvas_dimensions.height as i64));
        self.canvas_dimensions.width = new_width as u32;
    }

    pub fn randomize_height<R>(&mut self, rng: &mut R)
    where
        R: Rng + Sized,
    {
        let height = self.canvas_dimensions.height as i64;
        let new_height =
            height + rng.gen_range(-height + 1..=(2 * self.canvas_dimensions.width as i64));
        self.canvas_dimensions.height = new_height as u32;
    }

    pub fn randomize_dimensions_by_equal_factor<R>(&mut self, rng: &mut R)
    where
        R: Rng + Sized,
    {
        let factor = rng.gen_range(0.5..=1.5);
        self.canvas_dimensions.height = (self.canvas_dimensions.height as f64 * factor) as u32;
        self.canvas_dimensions.width = (self.canvas_dimensions.width as f64 * factor) as u32;
    }

    fn calculate_random_canvas_dimensions<R>(images: &'a [RgbImage], rng: &mut R) -> Dimensions
    where
        R: Rng + Sized,
    {
        let len_for_width = rng.gen_range(1..=images.len());
        let len_for_height = rng.gen_range(1..=images.len());
        let width = images
            .choose_multiple(rng, len_for_width)
            .map(|i| i.width())
            .sum();
        let height = images
            .choose_multiple(rng, len_for_height)
            .map(|i| i.height())
            .sum();

        Dimensions { width, height }
    }

    fn random_index_of_node_with_less_than_two_children<R>(&self, rng: &mut R) -> NodeIndex
    where
        R: Rng + Sized,
    {
        self.indexes_of_nodes_with_less_than_two_children()
            .choose(rng)
            .unwrap()
    }

    fn indexes_of_nodes_with_less_than_two_children(&self) -> impl Iterator<Item = NodeIndex> + '_ {
        self.graph.node_indices().filter(move |idx| {
            let is_internal = matches!(self.graph[*idx], Internal(_));

            is_internal && self.graph.edges(*idx).count() < 2
        })
    }

    fn node_has_less_than_two_children(&self, idx: NodeIndex) -> bool {
        self.graph.edges(idx).count() < 2
    }

    fn add_node(&mut self, parent_idx: NodeIndex, node_label: NodeLabel<'a>) -> NodeIndex {
        let idx = self.graph.add_node(node_label);
        self.graph.update_edge(parent_idx, idx, ());
        idx
    }

    fn root_node(&self) -> LayoutNode {
        let index = self.graph.externals(Direction::Incoming).next().unwrap();

        LayoutNode::new(self, index)
    }

    pub fn internal_nodes(&self) -> impl Iterator<Item = LayoutNode> + '_ + Clone {
        self.graph
            .node_indices()
            .filter(|idx| self.graph.edges(*idx).count() == 2)
            .map(|idx| self.at_index(idx))
    }

    pub fn leaf_nodes(&self) -> impl Iterator<Item = LayoutNode> + '_ + Clone {
        self.graph
            .externals(Direction::Outgoing)
            .map(move |index| LayoutNode::new(self, index))
    }

    pub fn crossover_random_subtrees<R>(&mut self, other: &mut Self, rng: &mut R)
    where
        R: Rng + Sized,
    {
        let subtrees = match self.subtree_pairs(other).choose(rng) {
            Some(value) => value,
            None => return,
        };
        let subtree_indexes = (subtrees.0.index, subtrees.1.index);

        self.crossover_subtrees(other, subtree_indexes);
    }

    // From the paper:
    //
    //     (…) swapping two subtrees each consisting of one I node and two L nodes is equivalent to
    //     swapping the labels of the two I nodes. Therefore, for the crossover, we were only
    //     interested in subtrees with at least three L nodes.
    fn subtrees(&self) -> impl Iterator<Item = Subtree> + '_ + Clone {
        self.internal_nodes().filter_map(|node| {
            let mut bfs = Bfs::new(&self.graph, node.index);
            let mut leaf_node_count: usize = 0;

            while let Some(index) = bfs.next(&self.graph) {
                if let Leaf(_) = self.graph[index] {
                    leaf_node_count += 1
                }
            }

            if leaf_node_count >= 3 {
                Some(Subtree::new(self, node.index, leaf_node_count))
            } else {
                None
            }
        })
    }

    fn subtree_pairs(
        &self,
        other: &'a Self,
    ) -> impl Iterator<Item = (Subtree, Subtree)> + '_ + Clone {
        let self_subtrees = self.subtrees();
        let other_subtrees = other.subtrees();

        self_subtrees
            .cartesian_product(other_subtrees)
            .filter(|(subtree, other_subtree)| {
                subtree.leaf_node_count == other_subtree.leaf_node_count
            })
    }

    fn crossover_subtrees(&mut self, other: &mut Self, subtrees: (NodeIndex, NodeIndex)) {
        let self_original = self.clone();

        self.swap_subtree(other, subtrees.0, subtrees.1);
        other.swap_subtree(&self_original, subtrees.1, subtrees.0);
    }

    fn swap_subtree(&mut self, other: &Self, self_index: NodeIndex, other_index: NodeIndex) {
        let subtree_blueprint = other.subtree_to_blueprint(other_index);

        let index_to_children = subtree_blueprint
            .iter()
            .map(|(label, children)| {
                let slice_direction = match label.as_str() {
                    "V" => Vertical,
                    "H" => Horizontal,
                    _ => unreachable!(),
                };

                (self.graph.add_node(Internal(slice_direction)), children)
            })
            .collect::<Vec<_>>();

        for (parent_node_index, children_indices_in_blueprint) in index_to_children.iter() {
            for child_index_in_blueprint in children_indices_in_blueprint.iter() {
                let child_index = index_to_children[*child_index_in_blueprint].0;
                self.graph.update_edge(*parent_node_index, child_index, ());
            }
        }

        // Fill leaf nodes in new subtree.
        let mut self_subtree_leaf_indices = vec![];
        let mut self_subtree_internal_indices = vec![];
        let indexes_of_nodes_with_less_than_two_children = self
            .indexes_of_nodes_with_less_than_two_children()
            .collect::<Vec<_>>();

        for node in self.logical_subtree_bfs_iter(self_index) {
            match node.node_label() {
                Leaf(_) => {
                    self_subtree_leaf_indices.push(node.index);
                }
                Internal(_) => {
                    self_subtree_internal_indices.push(node.index);
                }
            }
        }

        let mut leaf_indices_iter = self_subtree_leaf_indices.iter();

        for index in indexes_of_nodes_with_less_than_two_children {
            while self.node_has_less_than_two_children(index) {
                self.graph.update_edge(
                    index,
                    *leaf_indices_iter.next().expect("Ran out of leaf nodes"),
                    (),
                );
            }
        }

        // Connect new subtree to parent node.
        let subtree_root = self.at_index(self_index);
        let parent = subtree_root.parent();
        let parent_index = parent.map(|parent| parent.index);
        let subtree_root_side = parent.and_then(|parent| parent.child_side(&subtree_root));

        if parent.is_some() {
            self.graph
                .update_edge(parent_index.unwrap(), index_to_children[0].0, ());
        }

        // Remove leftover old nodes.
        self.graph = self.graph.filter_map(
            |index, weight| {
                if self_subtree_internal_indices.contains(&index) {
                    None
                } else {
                    Some(*weight)
                }
            },
            // We want to keep all valid edges and they all have the weight of (), so let's just
            // pass it here.
            |_, _| Some(()),
        );

        // If the node we were replacing was originally on the left side, we now need to swap the
        // children of the parent. That's because children are listed in the order of the creation
        // of the edges, so without it our new node would end up as the right child.
        //
        // This needs to be done after removing the original node, as most methods on Layout assume
        // that each internal node has two children.
        if let Some(ChildSide::Left) = subtree_root_side {
            self.swap_order_of_children(parent_index.unwrap());
        }
    }

    // For debugging the graph in Graphviz.
    pub fn dot(&self) -> Dot<'_, &Graph<NodeLabel<'_>, ()>> {
        Dot::with_config(&self.graph, &[Config::EdgeNoLabel])
    }

    fn children(&self, node: &LayoutNode) -> Option<(LayoutNode, LayoutNode)> {
        let mut iterator = self.graph.neighbors(node.index);

        // As petgraph's docs say:
        //
        //     For a Directed graph, neighbors are listed in reverse order of their addition to the
        //     graph, so the most recently added edge’s neighbor is listed first.
        //
        // https://docs.rs/petgraph/0.6.0/petgraph/graph/struct.Graph.html#method.neighbors_directed
        //
        // When thinking about binary trees, we typically think that the child that was added first
        // is on the left, so we have to position the children accordingly here.
        try {
            let right = iterator.next().map(|index| self.at_index(index))?;
            let left = iterator.next().map(|index| self.at_index(index))?;

            (left, right)
        }
    }

    fn parent(&self, node: &LayoutNode) -> Option<LayoutNode> {
        self.parent_index(node.index)
            .map(|index| LayoutNode::new(self, index))
    }

    // For situations where we just need the index alone and don't want the immutable borrow that
    // LayoutNode acquires.
    fn parent_index(&self, index: NodeIndex) -> Option<NodeIndex> {
        self.graph
            .neighbors_directed(index, Direction::Incoming)
            .next()
    }

    fn swap_order_of_children(&mut self, node_index: NodeIndex) {
        let children = self.at_index(node_index).children().unwrap();
        let child_0 = children.0.index;
        let child_1 = children.1.index;
        let child_0_edge = self.graph.find_edge(node_index, child_0).unwrap();
        let child_1_edge = self.graph.find_edge(node_index, child_1).unwrap();

        self.graph.remove_edge(child_1_edge);
        self.graph.remove_edge(child_0_edge);
        self.graph.update_edge(node_index, child_1, ());
        self.graph.update_edge(node_index, child_0, ());
    }

    // Returns a line of parents of the node, up to the root node.
    fn ancestors(&'a self, node: &'a LayoutNode<'a>) -> VecDeque<LayoutNode<'a>> {
        let mut queue = VecDeque::new();
        let mut next = node.parent();

        while let Some(node) = next {
            let parent = node.parent();
            queue.push_front(node);
            next = parent;
        }

        queue
    }

    // Returns the given node along with its line of parents, up to the root node.
    fn lineage(&'a self, node: &'a LayoutNode<'a>) -> VecDeque<LayoutNode<'a>> {
        let mut ancestors = self.ancestors(node);
        ancestors.push_back(self.at_index(node.index));
        ancestors
    }

    fn node_label(&self, node: &LayoutNode) -> &NodeLabel {
        self.graph.node_weight(node.index).unwrap()
    }

    fn at_index(&'a self, index: NodeIndex) -> LayoutNode<'a> {
        LayoutNode::new(self, index)
    }

    // Logical as in it uses the `children` method to traverse the graph. `children` is also what
    // the renderer uses to render the layout.
    fn logical_bfs_iter(&self) -> LogicalBfs {
        // Edge case in tests.
        if self.graph.node_count() == 0 {
            LogicalBfs::empty(self)
        } else {
            LogicalBfs::new(self, self.root_node().index)
        }
    }

    fn logical_subtree_bfs_iter(&self, index: NodeIndex) -> LogicalBfs {
        LogicalBfs::new(self, index)
    }

    fn logical_eq(&'a self, other: &'a Layout<'a>) -> Result<(), LogicalEqError<'a>> {
        use LogicalEqError::*;

        if self.canvas_dimensions != other.canvas_dimensions {
            return Err(DifferentCanvasDimensions);
        }

        // This should only happen if the graph ends up having some dangling nodes.
        if self.graph.node_count() != other.graph.node_count() {
            return Err(UnevenNumberOfNodes);
        }

        // This should only happen if the graph ends up having some dangling nodes.
        if self.graph.edge_count() != other.graph.edge_count() {
            return Err(UnevenNumberOfEdges);
        }

        let mut self_iter = self.logical_bfs_iter().peekable();
        let mut other_iter = other.logical_bfs_iter().peekable();
        let self_root_node = self_iter.peek();
        let other_root_node = other_iter.peek();

        if let (Some(self_root_node), Some(other_root_node)) = (self_root_node, other_root_node) {
            let self_root_node_label = self_root_node.node_label();
            let other_root_node_label = other_root_node.node_label();

            if self_root_node_label != other_root_node_label {
                return Err(RootNodesHaveDifferentLabels(
                    *self_root_node_label,
                    *other_root_node_label,
                ));
            }
        }

        loop {
            match (self_iter.next(), other_iter.next()) {
                (Some(self_node), Some(other_node)) => {
                    let self_children_node_labels = self_node
                        .children()
                        .map(|c| (*c.0.node_label(), *c.1.node_label()));
                    let other_children_node_labels = other_node
                        .children()
                        .map(|c| (*c.0.node_label(), *c.1.node_label()));

                    if self_children_node_labels != other_children_node_labels {
                        return Err(ChildrenHaveDifferentLabels(
                            (self_node, self_children_node_labels),
                            (other_node, other_children_node_labels),
                        ));
                    }
                }
                (None, None) => {
                    return Ok(());
                }
                _ => {
                    // Since we already checked that the number of nodes and edges is equal, this
                    // shouldn't happen unless we have a bug which completely screws up internal
                    // graph structure.
                    unreachable!();
                }
            }
        }
    }
}

#[derive(Debug)]
enum LogicalEqError<'a> {
    DifferentCanvasDimensions,
    UnevenNumberOfNodes,
    UnevenNumberOfEdges,
    RootNodesHaveDifferentLabels(NodeLabel<'a>, NodeLabel<'a>),
    ChildrenHaveDifferentLabels(
        (LayoutNode<'a>, Option<(NodeLabel<'a>, NodeLabel<'a>)>),
        (LayoutNode<'a>, Option<(NodeLabel<'a>, NodeLabel<'a>)>),
    ),
}

impl PartialEq for Layout<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.logical_eq(other).is_ok()
    }
}

#[derive(Clone, Copy)]
pub struct LayoutNode<'a> {
    pub index: NodeIndex,
    layout: &'a Layout<'a>,
}

impl std::fmt::Debug for LayoutNode<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("LayoutNode")
            .field("index", &self.index)
            .field("node_label", &self.node_label())
            .finish()
    }
}

impl PartialEq for LayoutNode<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && ptr::eq(self.layout, other.layout)
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Dimensions {
    pub width: u32,
    pub height: u32,
}

impl Dimensions {
    pub fn from_tuple(tuple: (u32, u32)) -> Dimensions {
        Dimensions {
            width: tuple.0,
            height: tuple.1,
        }
    }

    pub fn to_tuple(self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub fn size(&self) -> u32 {
        self.width * self.height
    }
}

impl<'a> LayoutNode<'a> {
    pub fn new(layout: &'a Layout<'a>, index: NodeIndex) -> Self {
        LayoutNode { layout, index }
    }

    pub fn aspect_ratio(&self) -> f64 {
        use NodeLabel::*;
        use SliceDirection::*;

        match self.node_label() {
            Leaf(image) => image.width() as f64 / image.height() as f64,
            Internal(direction) => match direction {
                Vertical => {
                    let children = self.children().unwrap();

                    children.0.aspect_ratio() + children.1.aspect_ratio()
                }
                Horizontal => {
                    let children = self.children().unwrap();

                    1.0 / (1.0 / children.0.aspect_ratio() + 1.0 / children.1.aspect_ratio())
                }
            },
        }
    }

    pub fn dimensions(&self) -> Dimensions {
        let aspect_ratio = self.aspect_ratio();
        let parent_dimensions = self
            .parent()
            .map(|n| n.dimensions())
            .unwrap_or(self.layout.canvas_dimensions);

        let width = parent_dimensions
            .width
            .min((aspect_ratio * parent_dimensions.height as f64) as u32);
        let height = (width as f64 / aspect_ratio) as u32;

        Dimensions { width, height }
    }

    pub fn height(&self) -> u32 {
        self.dimensions().height
    }

    pub fn width(&self) -> u32 {
        self.dimensions().width
    }

    pub fn node_label(&self) -> &'a NodeLabel<'a> {
        self.layout.node_label(self)
    }

    pub fn image(&self) -> Option<&'a RgbImage> {
        if let NodeLabel::Leaf(image) = self.node_label() {
            return Some(image);
        }

        None
    }

    pub fn children(&self) -> Option<(LayoutNode<'a>, LayoutNode<'a>)> {
        self.layout.children(self)
    }

    pub fn parent(&self) -> Option<LayoutNode<'a>> {
        self.layout.parent(self)
    }

    pub fn ancestors(&'a self) -> VecDeque<LayoutNode<'a>> {
        self.layout.ancestors(self)
    }

    pub fn lineage(&'a self) -> VecDeque<LayoutNode<'a>> {
        self.layout.lineage(self)
    }

    pub fn other_child(&'a self, node: &LayoutNode<'a>) -> Option<LayoutNode<'a>> {
        let children = self.children()?;

        if node == &children.0 {
            Some(children.1)
        } else if node == &children.1 {
            Some(children.0)
        } else {
            None
        }
    }

    pub fn child_side(&self, node: &LayoutNode<'a>) -> Option<ChildSide> {
        let children = self.children()?;

        if node == &children.0 {
            Some(ChildSide::Left)
        } else if node == &children.1 {
            Some(ChildSide::Right)
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub enum ChildSide {
    Left,
    Right,
}

#[derive(Clone, Copy)]
pub struct Subtree<'a> {
    layout: &'a Layout<'a>,
    pub index: NodeIndex,
    pub leaf_node_count: usize,
}

impl PartialEq for Subtree<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
            && self.leaf_node_count == other.leaf_node_count
            && ptr::eq(self.layout, other.layout)
    }
}

impl<'a> Subtree<'a> {
    pub fn new(layout: &'a Layout<'a>, index: NodeIndex, leaf_node_count: usize) -> Self {
        Subtree {
            layout,
            index,
            leaf_node_count,
        }
    }
}

impl std::fmt::Debug for Subtree<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("Subtree")
            .field("index", &self.index)
            .field("leaf_node_count", &self.leaf_node_count)
            .finish()
    }
}

struct LogicalBfs<'a> {
    layout: &'a Layout<'a>,
    indexes_to_visit: VecDeque<NodeIndex>,
}

impl<'a> LogicalBfs<'a> {
    fn new(layout: &'a Layout<'a>, start_index: NodeIndex) -> Self {
        LogicalBfs {
            layout,
            indexes_to_visit: VecDeque::from([start_index]),
        }
    }

    fn empty(layout: &'a Layout<'a>) -> Self {
        LogicalBfs {
            layout,
            indexes_to_visit: VecDeque::new(),
        }
    }
}

impl<'a> Iterator for LogicalBfs<'a> {
    type Item = LayoutNode<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self
            .indexes_to_visit
            .pop_front()
            .map(|index| self.layout.at_index(index))?;

        if let Some((left_child, right_child)) = next.children() {
            self.indexes_to_visit.push_back(left_child.index);
            self.indexes_to_visit.push_back(right_child.index);
        }

        Some(next)
    }
}

// Auxiliary function for creating blueprints in tests.
#[cfg(test)]
fn create_blueprint_from_slice(
    dimensions: (u32, u32),
    graph_representation: &[(&str, &[usize])],
) -> LayoutBlueprint {
    let graph_representation: Vec<(String, Vec<usize>)> = graph_representation
        .iter()
        .map(|(label, indices)| (label.to_string(), indices.to_vec()))
        .collect();

    LayoutBlueprint {
        width: dimensions.0,
        height: dimensions.1,
        graph_representation,
    }
}

#[cfg(test)]
mod logical_bfs_tests {
    use super::*;

    #[test]
    fn logical_bfs_iterates_correctly_on_simple_layouts() {
        let blueprint =
            create_blueprint_from_slice((10, 10), &[("V", &[1, 2]), ("H", &[]), ("V", &[])]);
        let images = vec![
            RgbImage::new(1, 1),
            RgbImage::new(1, 2),
            RgbImage::new(1, 3),
            RgbImage::new(1, 4),
        ];
        let layout = Layout::from_blueprint(&blueprint, &images).unwrap();

        let expected_node_labels = vec![
            Internal(Vertical),
            Internal(Horizontal),
            Internal(Vertical),
            Leaf(&images[0]),
            Leaf(&images[1]),
            Leaf(&images[2]),
            Leaf(&images[3]),
        ];
        let actual_node_labels = layout
            .logical_bfs_iter()
            .map(|node| *node.node_label())
            .collect::<Vec<_>>();

        assert_eq!(expected_node_labels, actual_node_labels);
    }

    #[test]
    fn logical_eq_works_for_simple_layouts_which_are_equal() {
        let blueprint =
            create_blueprint_from_slice((10, 10), &[("V", &[1, 2]), ("H", &[]), ("V", &[])]);
        let images = vec![
            RgbImage::new(1, 1),
            RgbImage::new(1, 2),
            RgbImage::new(1, 3),
            RgbImage::new(1, 4),
        ];
        let layout = Layout::from_blueprint(&blueprint, &images).unwrap();

        assert!(layout.logical_eq(&layout).is_ok());
    }

    #[test]
    fn logical_eq_works_for_simple_layouts_which_are_not_equal() {
        let blueprint =
            create_blueprint_from_slice((10, 10), &[("V", &[1, 2]), ("H", &[]), ("V", &[])]);
        let images = vec![
            RgbImage::new(1, 1),
            RgbImage::new(1, 2),
            RgbImage::new(1, 3),
            RgbImage::new(1, 4),
        ];
        let layout = Layout::from_blueprint(&blueprint, &images).unwrap();
        let other_blueprint =
            create_blueprint_from_slice((10, 10), &[("V", &[1, 2]), ("H", &[]), ("H", &[])]);
        let other_layout = Layout::from_blueprint(&other_blueprint, &images).unwrap();

        assert!(!layout.logical_eq(&other_layout).is_ok());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_core::SeedableRng;
    use rand_pcg::Pcg64;

    macro_rules! assert_logical_eq_of_layouts {
        ($left_layout:expr, $right_layout:expr) => {
            match $left_layout.logical_eq($right_layout) {
                Ok(_) => {}
                Err(LogicalEqError::ChildrenHaveDifferentLabels(
                    (left_layout_node, left_layout_children),
                    (right_layout_node, right_layout_children),
                )) => {
                    panic!(
                        "The layouts appear to have different children for a particular node.\n\n\
                        The node from the left layout and its children look like this:\n\
                        {:?}\n{:?}\n\n\
                        The node from the right layout and its children look like this:\n\
                        {:?}\n{:?}\n",
                        left_layout_node,
                        left_layout_children,
                        right_layout_node,
                        right_layout_children
                    );
                }
                Err(LogicalEqError::RootNodesHaveDifferentLabels(
                    left_layout_label,
                    right_layout_label,
                )) => {
                    panic!(
                        "The layouts have different root nodes: {:?} vs {:?}",
                        left_layout_label, right_layout_label
                    );
                }
                Err(LogicalEqError::UnevenNumberOfNodes) => {
                    panic!(
                        "The layouts have different number of nodes: {} vs {}",
                        $left_layout.graph.node_count(),
                        $right_layout.graph.node_count()
                    );
                }
                Err(LogicalEqError::UnevenNumberOfEdges) => {
                    panic!(
                        "The layouts have different number of edges: {} vs {}",
                        $left_layout.graph.edge_count(),
                        $right_layout.graph.edge_count()
                    );
                }
                Err(LogicalEqError::DifferentCanvasDimensions) => {
                    panic!(
                        "The layouts have different canvas dimensions: {:?} vs {:?}",
                        $left_layout.canvas_dimensions, $right_layout.canvas_dimensions
                    );
                }
            }
        };
    }

    #[test]
    fn comparing_layouts_with_equal_dimensions() {
        let layout_1 = Layout {
            graph: LayoutGraph::new(),
            canvas_dimensions: Dimensions::from_tuple((1, 1)),
        };
        let layout_2 = Layout {
            graph: LayoutGraph::new(),
            canvas_dimensions: Dimensions::from_tuple((1, 1)),
        };

        assert_logical_eq_of_layouts!(layout_1, &layout_2);
    }

    #[test]
    fn comparing_layouts_with_different_dimensions() {
        let layout_1 = Layout {
            graph: LayoutGraph::new(),
            canvas_dimensions: Dimensions::from_tuple((1, 1)),
        };
        let layout_2 = Layout {
            graph: LayoutGraph::new(),
            canvas_dimensions: Dimensions::from_tuple((3, 7)),
        };

        assert_ne!(layout_1, layout_2);
    }

    #[test]
    fn compare_layouts_with_the_same_single_node() {
        let mut layout_1 = Layout {
            graph: LayoutGraph::new(),
            canvas_dimensions: Dimensions::from_tuple((1, 1)),
        };
        let mut layout_2 = Layout {
            graph: LayoutGraph::new(),
            canvas_dimensions: Dimensions::from_tuple((1, 1)),
        };
        layout_1.graph.add_node(Internal(Vertical));
        layout_2.graph.add_node(Internal(Vertical));

        assert_logical_eq_of_layouts!(layout_1, &layout_2);
    }

    #[test]
    fn compare_layouts_with_different_single_node() {
        let mut layout_1 = Layout {
            graph: LayoutGraph::new(),
            canvas_dimensions: Dimensions::from_tuple((1, 1)),
        };
        let mut layout_2 = Layout {
            graph: LayoutGraph::new(),
            canvas_dimensions: Dimensions::from_tuple((1, 1)),
        };
        layout_1.graph.add_node(Internal(Vertical));
        layout_2.graph.add_node(Internal(Horizontal));

        assert_ne!(layout_1, layout_2);
    }

    #[test]
    fn compare_layouts_with_equal_leaf_nodes() {
        let mut layout_1 = Layout {
            graph: LayoutGraph::new(),
            canvas_dimensions: Dimensions::from_tuple((1, 1)),
        };
        let mut layout_2 = Layout {
            graph: LayoutGraph::new(),
            canvas_dimensions: Dimensions::from_tuple((1, 1)),
        };
        let image_1 = RgbImage::new(1, 1);
        let image_2 = RgbImage::new(2, 2);

        let root_index_1 = layout_1.graph.add_node(Internal(Vertical));
        layout_1.add_node(root_index_1, Leaf(&image_1));
        layout_1.add_node(root_index_1, Leaf(&image_2));

        let root_index_2 = layout_2.graph.add_node(Internal(Vertical));
        layout_2.add_node(root_index_2, Leaf(&image_1));
        layout_2.add_node(root_index_2, Leaf(&image_2));

        assert_logical_eq_of_layouts!(layout_1, &layout_2);
    }

    #[test]
    fn compare_layouts_with_different_leaf_nodes() {
        let mut layout_1 = Layout {
            graph: LayoutGraph::new(),
            canvas_dimensions: Dimensions::from_tuple((1, 1)),
        };
        let mut layout_2 = Layout {
            graph: LayoutGraph::new(),
            canvas_dimensions: Dimensions::from_tuple((1, 1)),
        };
        let image_1 = RgbImage::new(1, 1);
        let image_2 = RgbImage::new(2, 2);

        let root_index_1 = layout_1.graph.add_node(Internal(Vertical));
        layout_1.add_node(root_index_1, Leaf(&image_1));
        layout_1.add_node(root_index_1, Leaf(&image_2));

        // layout_2 refers to the same images, but has them in different order.
        let root_index_2 = layout_2.graph.add_node(Internal(Vertical));
        layout_2.add_node(root_index_2, Leaf(&image_2));
        layout_2.add_node(root_index_2, Leaf(&image_1));

        assert_ne!(layout_1, layout_2);
    }

    #[test]
    fn create_layout_from_blueprint() {
        // digraph {
        //     0 [ label = "Vertical"     ]
        //     1 [ label = "Horizontal"   ]
        //     2 [ label = "Image(5, 10)" ]
        //     3 [ label = "Image(2, 2)"  ]
        //     4 [ label = "Image(2, 4)"  ]
        //     0 -> 1 [ ]
        //     0 -> 2 [ ]
        //     1 -> 3 [ ]
        //     1 -> 4 [ ]
        // }
        let blueprint = create_blueprint_from_slice((10, 10), &[("V", &[1]), ("H", &[])]);
        let images = vec![
            RgbImage::new(5, 10),
            RgbImage::new(2, 2),
            RgbImage::new(2, 4),
        ];
        let layout_from_blueprint = Layout::from_blueprint(&blueprint, &images);

        // Expected layout, manually crafted.
        let graph = LayoutGraph::new();
        let canvas_dimensions = Dimensions::from_tuple((10, 10));
        let mut expected_layout = Layout {
            graph,
            canvas_dimensions,
        };
        let v_index = expected_layout.graph.add_node(Internal(Vertical));
        let h_index = expected_layout.graph.add_node(Internal(Horizontal));
        let image_0_index = expected_layout.graph.add_node(Leaf(&images[0]));
        let image_1_index = expected_layout.graph.add_node(Leaf(&images[1]));
        let image_2_index = expected_layout.graph.add_node(Leaf(&images[2]));

        expected_layout.graph.update_edge(v_index, h_index, ());
        expected_layout
            .graph
            .update_edge(v_index, image_0_index, ());
        expected_layout
            .graph
            .update_edge(h_index, image_1_index, ());
        expected_layout
            .graph
            .update_edge(h_index, image_2_index, ());

        assert_logical_eq_of_layouts!(expected_layout, layout_from_blueprint.as_ref().unwrap());
    }

    // Since we only have two internal nodes, we know that if pass one of them to
    // `swap_with_random_node`, the other one will be the only other internal node. Thus we can
    // write a test case.
    #[test]
    fn swap_random_pair_of_internal_nodes() {
        let blueprint = create_blueprint_from_slice((10, 10), &[("V", &[1]), ("H", &[])]);
        let images = vec![
            RgbImage::new(5, 10),
            RgbImage::new(2, 2),
            RgbImage::new(2, 4),
        ];
        let mut layout = Layout::from_blueprint(&blueprint, &images).unwrap();

        layout.swap_with_random_node(&mut rand::thread_rng(), NodeIndex::new(0));

        assert_eq!(
            Internal(Horizontal),
            *layout.at_index(NodeIndex::new(0)).node_label()
        );
        assert_eq!(
            Internal(Vertical),
            *layout.at_index(NodeIndex::new(1)).node_label()
        );
    }

    // Since we only have two leaf nodes, we know that if pass one of them to
    // `swap_with_random_node`, the other one will be the only other leaf node. Thus we can write a
    // test case.
    #[test]
    fn swap_random_pair_of_leaf_nodes() {
        let blueprint = create_blueprint_from_slice((10, 10), &[("V", &[])]);
        let images = vec![RgbImage::new(1, 1), RgbImage::new(2, 2)];
        let mut layout = Layout::from_blueprint(&blueprint, &images).unwrap();

        layout.swap_with_random_node(&mut rand::thread_rng(), NodeIndex::new(1));

        assert_eq!(Some(&images[1]), layout.at_index(NodeIndex::new(1)).image());
        assert_eq!(Some(&images[0]), layout.at_index(NodeIndex::new(2)).image());
    }

    #[test]
    fn fall_back_to_swapping_leaf_nodes_if_all_internal_nodes_have_the_same_label() {
        let blueprint = create_blueprint_from_slice((10, 10), &[("V", &[1]), ("V", &[])]);
        let images = vec![
            RgbImage::new(1, 1),
            RgbImage::new(1, 2),
            RgbImage::new(1, 3),
        ];
        let mut layout = Layout::from_blueprint(&blueprint, &images).unwrap();

        layout.swap_with_random_node(&mut rand::thread_rng(), NodeIndex::new(0));

        let actual_leaf_node_images: Vec<RgbImage> = layout
            .leaf_nodes()
            .map(|node| node.image().unwrap())
            .cloned()
            .collect();

        assert_ne!(images, actual_leaf_node_images);
    }

    #[test]
    fn fall_back_to_swapping_leaf_nodes_if_theres_one_internal_node() {
        let blueprint = create_blueprint_from_slice((10, 10), &[("V", &[])]);
        let images = vec![RgbImage::new(1, 1), RgbImage::new(2, 2)];
        let mut layout = Layout::from_blueprint(&blueprint, &images).unwrap();

        layout.swap_with_random_node(&mut rand::thread_rng(), NodeIndex::new(0));

        let expected_leaf_node_images = vec![&images[1], &images[0]];
        let actual_leaf_node_images: Vec<&RgbImage> = layout
            .leaf_nodes()
            .map(|node| node.image().unwrap())
            .collect();

        assert_eq!(expected_leaf_node_images, actual_leaf_node_images);
    }

    #[test]
    fn create_blueprint_from_layout() {
        let images = vec![
            RgbImage::new(1, 1),
            RgbImage::new(1, 2),
            RgbImage::new(1, 3),
        ];
        let mut layout = Layout {
            graph: LayoutGraph::new(),
            canvas_dimensions: Dimensions::from_tuple((10, 10)),
        };
        let v_index = layout.graph.add_node(Internal(Vertical));
        let h_index = layout.graph.add_node(Internal(Horizontal));
        let image_0_index = layout.graph.add_node(Leaf(&images[0]));
        let image_1_index = layout.graph.add_node(Leaf(&images[1]));
        let image_2_index = layout.graph.add_node(Leaf(&images[2]));

        layout.graph.update_edge(v_index, h_index, ());
        layout.graph.update_edge(v_index, image_0_index, ());
        layout.graph.update_edge(h_index, image_1_index, ());
        layout.graph.update_edge(h_index, image_2_index, ());

        let expected_blueprint = create_blueprint_from_slice((10, 10), &[("V", &[1]), ("H", &[])]);
        let actual_blueprint = layout.to_blueprint();

        assert_eq!(expected_blueprint, actual_blueprint);
    }

    #[test]
    fn from_and_to_blueprint_returns_same_blueprint() {
        let seed = rand::thread_rng().gen();
        // let seed = 7519943073446034360;
        let mut rng = Pcg64::seed_from_u64(seed);
        for _ in 0..100 {
            let mut images = vec![];
            for i in 0..rng.gen_range(2..10) {
                images.push(RgbImage::new(1, i + 1));
            }
            let layout = Layout::new(&images, &mut rng);
            let blueprint1 = layout.to_blueprint();
            let blueprint2 = Layout::from_blueprint(&blueprint1, &images)
                .unwrap()
                .to_blueprint();

            assert_eq!(
                blueprint1,
                blueprint2,
                "Blueprints are not matching each other\nSeed: {}\nLayout:\n{:?}",
                seed,
                layout.dot(),
            );
        }
    }

    #[test]
    fn find_subtrees() {
        let blueprint =
            create_blueprint_from_slice((10, 10), &[("V", &[1]), ("H", &[2]), ("H", &[])]);
        let images = vec![
            RgbImage::new(1, 1),
            RgbImage::new(1, 2),
            RgbImage::new(1, 3),
            RgbImage::new(1, 4),
        ];
        let layout = Layout::from_blueprint(&blueprint, &images).unwrap();

        let subtrees: Vec<Subtree> = layout.subtrees().collect();

        assert_eq!(Subtree::new(&layout, NodeIndex::new(0), 4), subtrees[0]);
        assert_eq!(Subtree::new(&layout, NodeIndex::new(1), 3), subtrees[1]);
    }

    #[test]
    fn find_pairs_in_subtrees() {
        let images = vec![
            RgbImage::new(1, 1),
            RgbImage::new(1, 2),
            RgbImage::new(1, 3),
            RgbImage::new(1, 4),
            RgbImage::new(1, 5),
        ];
        let blueprint1 = create_blueprint_from_slice(
            (10, 10),
            &[("V", &[1]), ("V", &[2]), ("V", &[3]), ("V", &[])],
        );
        let blueprint2 = create_blueprint_from_slice(
            (10, 10),
            &[("H", &[1, 2]), ("V", &[3]), ("V", &[]), ("V", &[])],
        );
        let layout_1 = Layout::from_blueprint(&blueprint1, &images).unwrap();
        let layout_2 = Layout::from_blueprint(&blueprint2, &images).unwrap();

        let expected_pairs = vec![
            (
                Subtree::new(&layout_1, NodeIndex::new(0), 5),
                Subtree::new(&layout_2, NodeIndex::new(0), 5),
            ),
            (
                Subtree::new(&layout_1, NodeIndex::new(2), 3),
                Subtree::new(&layout_2, NodeIndex::new(1), 3),
            ),
        ];

        let actual_pairs: Vec<(Subtree, Subtree)> = layout_1.subtree_pairs(&layout_2).collect();

        assert_eq!(expected_pairs, actual_pairs);
    }

    #[test]
    fn swapping_order_of_children() {
        let blueprint =
            create_blueprint_from_slice((10, 10), &[("V", &[1, 2]), ("H", &[]), ("V", &[])]);
        let images = vec![
            RgbImage::new(1, 1),
            RgbImage::new(1, 2),
            RgbImage::new(1, 3),
            RgbImage::new(1, 4),
        ];
        let mut layout = Layout::from_blueprint(&blueprint, &images).unwrap();

        layout.swap_order_of_children(NodeIndex::new(0));

        let (left_child, right_child) = layout.at_index(NodeIndex::new(0)).children().unwrap();
        assert_eq!(Internal(Vertical), *left_child.node_label());
        assert_eq!(Internal(Horizontal), *right_child.node_label());
    }

    #[test]
    fn blueprint_of_layout_with_swapped_children_leads_to_equal_layout() {
        let blueprint =
            create_blueprint_from_slice((10, 10), &[("V", &[1, 2]), ("H", &[]), ("V", &[])]);
        let images = vec![
            RgbImage::new(1, 1),
            RgbImage::new(1, 2),
            RgbImage::new(1, 3),
            RgbImage::new(1, 4),
        ];
        let mut layout = Layout::from_blueprint(&blueprint, &images).unwrap();

        layout.swap_order_of_children(NodeIndex::new(0));

        let layout_from_blueprint =
            Layout::from_blueprint(&layout.to_blueprint(), &images).unwrap();
        let (left_child, right_child) = layout_from_blueprint
            .at_index(NodeIndex::new(0))
            .children()
            .unwrap();
        assert_eq!(Internal(Vertical), *left_child.node_label());
        assert_eq!(Internal(Horizontal), *right_child.node_label());
    }

    #[test]
    fn swaping_single_subtree() {
        let images1 = vec![
            RgbImage::new(1, 1),
            RgbImage::new(1, 2),
            RgbImage::new(1, 3),
            RgbImage::new(1, 4),
            RgbImage::new(1, 5),
        ];
        let blueprint1 = create_blueprint_from_slice(
            (10, 10),
            &[("V", &[1]), ("H", &[2]), ("V", &[3]), ("H", &[])],
        );
        let blueprint2 = create_blueprint_from_slice(
            (10, 10),
            &[("H", &[1, 2]), ("H", &[3]), ("V", &[]), ("V", &[])],
        );
        let mut images2 = images1.clone();
        images2.reverse();

        let mut layout = Layout::from_blueprint(&blueprint1, &images1).unwrap();
        let layout_other = Layout::from_blueprint(&blueprint2, &images2).unwrap();

        layout.swap_subtree(&layout_other, NodeIndex::new(2), NodeIndex::new(1));

        let expected_layout_blueprint = create_blueprint_from_slice(
            (10, 10),
            &[("V", &[1]), ("H", &[2]), ("H", &[3]), ("V", &[])],
        );

        assert_eq!(expected_layout_blueprint, layout.to_blueprint());
        assert_eq!(
            images1,
            layout
                .leaf_nodes()
                .map(|n| n.image().unwrap())
                .cloned()
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn swapping_two_subtrees_keeps_logical_structure() {
        let images1 = vec![
            RgbImage::new(1, 1),
            RgbImage::new(1, 2),
            RgbImage::new(1, 3),
            RgbImage::new(1, 4),
            RgbImage::new(1, 5),
        ];
        let blueprint1 = create_blueprint_from_slice(
            (10, 10),
            &[("V", &[1]), ("H", &[2]), ("V", &[3]), ("H", &[])],
        );
        let blueprint2 = create_blueprint_from_slice(
            (10, 10),
            &[("H", &[1, 2]), ("H", &[3]), ("V", &[]), ("V", &[])],
        );
        let mut images2 = images1.clone();
        images2.reverse();

        let mut layout_1 = Layout::from_blueprint(&blueprint1, &images1).unwrap();
        let mut layout_2 = Layout::from_blueprint(&blueprint2, &images2).unwrap();

        layout_1.crossover_subtrees(&mut layout_2, (NodeIndex::new(2), NodeIndex::new(1)));

        let expected_layout_1_blueprint = create_blueprint_from_slice(
            (10, 10),
            &[("V", &[1]), ("H", &[2]), ("H", &[3]), ("V", &[])],
        );
        let expected_layout_2_blueprint = create_blueprint_from_slice(
            (10, 10),
            &[("H", &[1, 2]), ("V", &[3]), ("V", &[]), ("H", &[])],
        );
        let expected_layout_1 =
            Layout::from_blueprint(&expected_layout_1_blueprint, &images1).unwrap();
        let expected_layout_2 =
            Layout::from_blueprint(&expected_layout_2_blueprint, &images2).unwrap();

        assert_logical_eq_of_layouts!(expected_layout_1, &layout_1);
        assert_logical_eq_of_layouts!(expected_layout_2, &layout_2);
    }

    #[test]
    fn swapping_the_whole_layout_keeps_logical_structure() {
        let images = vec![
            RgbImage::new(1, 1),
            RgbImage::new(1, 2),
            RgbImage::new(1, 3),
            RgbImage::new(1, 4),
            RgbImage::new(1, 5),
        ];
        let blueprint1 = create_blueprint_from_slice(
            (10, 10),
            &[("V", &[1]), ("H", &[2]), ("V", &[3]), ("H", &[])],
        );
        let blueprint2 = create_blueprint_from_slice(
            (10, 10),
            &[("H", &[1, 2]), ("H", &[3]), ("V", &[]), ("V", &[])],
        );

        let mut layout_1 = Layout::from_blueprint(&blueprint1, &images).unwrap();
        let mut layout_2 = Layout::from_blueprint(&blueprint2, &images).unwrap();
        let expected_layout_1 = layout_2.clone();
        let expected_layout_2 = layout_1.clone();

        layout_1.crossover_subtrees(&mut layout_2, (NodeIndex::new(0), NodeIndex::new(0)));

        assert_logical_eq_of_layouts!(expected_layout_1, &layout_1);
        assert_logical_eq_of_layouts!(expected_layout_2, &layout_2);
    }
}
