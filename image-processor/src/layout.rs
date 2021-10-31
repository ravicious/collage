use image::RgbImage;
use petgraph::{
    dot::{Config, Dot},
    graph::NodeIndex,
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

#[derive(Serialize, Deserialize, Debug)]
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
    pub fn new(images: &'a [RgbImage]) -> Self {
        // TODO: Add estimated capacity with .with_capacity instead of .new
        let graph = LayoutGraph::new();
        let canvas_dimensions = Self::calculate_random_canvas_dimensions(images);
        let mut layout = Layout {
            graph,
            canvas_dimensions,
        };
        let mut rng = rand::thread_rng();
        let mut random_images = images.choose_multiple(&mut rng, images.len());

        layout.graph.add_node(NodeLabel::Internal(rand::random()));

        // Total number of internal nodes must be equal to <number of images> - 1 and we already
        // added one internal node.
        for _ in 0..images.len() - 2 {
            let random_index = layout.random_index_of_node_with_less_than_two_children();
            layout.add_node(random_index, NodeLabel::Internal(rand::random()));
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

        // Add images as leafs to nodes with less than two children, starting from the first added node
        // to the last one.
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
                        // This can happen if all internal nodes have the same node label or there's just a
                        // single internal node.
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
        let new_width = width + rng.gen_range(-width + 1, 2 * self.canvas_dimensions.height as i64);
        self.canvas_dimensions.width = new_width as u32;
    }

    pub fn randomize_height<R>(&mut self, rng: &mut R)
    where
        R: Rng + Sized,
    {
        let height = self.canvas_dimensions.height as i64;
        let new_height =
            height + rng.gen_range(-height + 1, 2 * self.canvas_dimensions.width as i64);
        self.canvas_dimensions.height = new_height as u32;
    }

    fn calculate_random_canvas_dimensions(images: &'a [RgbImage]) -> Dimensions {
        let mut rng = rand::thread_rng();
        let len_for_width = rng.gen_range(1, images.len() + 1);
        let len_for_height = rng.gen_range(1, images.len() + 1);
        let width = images
            .choose_multiple(&mut rng, len_for_width)
            .map(|i| i.width())
            .sum();
        let height = images
            .choose_multiple(&mut rng, len_for_height)
            .map(|i| i.height())
            .sum();

        Dimensions { width, height }
    }

    fn random_index_of_node_with_less_than_two_children(&self) -> NodeIndex {
        let mut rng = rand::thread_rng();
        self.indexes_of_nodes_with_less_than_two_children()
            .choose(&mut rng)
            .unwrap()
    }

    fn indexes_of_nodes_with_less_than_two_children(&self) -> impl Iterator<Item = NodeIndex> + '_ {
        self.graph
            .node_indices()
            .filter(move |idx| self.graph.edges(*idx).count() < 2)
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

    pub fn internal_nodes(&self) -> impl Iterator<Item = LayoutNode> + '_ {
        self.graph
            .node_indices()
            .filter(|idx| self.graph.edges(*idx).count() == 2)
            .map(|idx| self.at_index(idx))
    }

    pub fn leaf_nodes(&self) -> impl Iterator<Item = LayoutNode> + '_ {
        self.graph
            .externals(Direction::Outgoing)
            .map(move |index| LayoutNode::new(self, index))
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
        // When thinking about binary trees, we typically think that the child that was added first is
        // on the left, so we have to position the children accordingly here.
        try {
            let right = iterator.next().map(|index| self.at_index(index))?;
            let left = iterator.next().map(|index| self.at_index(index))?;

            (left, right)
        }
    }

    fn parent(&self, node: &LayoutNode) -> Option<LayoutNode> {
        self.graph
            .neighbors_directed(node.index, Direction::Incoming)
            .next()
            .map(|index| LayoutNode::new(self, index))
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
}

impl PartialEq for Layout<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.canvas_dimensions == other.canvas_dimensions && graph_eq(&self.graph, &other.graph)
    }
}

// Taken from https://github.com/petgraph/petgraph/issues/199#issuecomment-484077775
fn graph_eq<N, E, Ty, Ix>(
    a: &petgraph::Graph<N, E, Ty, Ix>,
    b: &petgraph::Graph<N, E, Ty, Ix>,
) -> bool
where
    N: PartialEq,
    E: PartialEq,
    Ty: petgraph::EdgeType,
    Ix: petgraph::graph::IndexType + PartialEq,
{
    let a_ns = a.raw_nodes().iter().map(|n| &n.weight);
    let b_ns = b.raw_nodes().iter().map(|n| &n.weight);
    let a_es = a
        .raw_edges()
        .iter()
        .map(|e| (e.source(), e.target(), &e.weight));
    let b_es = b
        .raw_edges()
        .iter()
        .map(|e| (e.source(), e.target(), &e.weight));
    a_ns.eq(b_ns) && a_es.eq(b_es)
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

#[cfg(test)]
mod tests {
    use super::*;

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

        assert_eq!(layout_1, layout_2);
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

        assert_eq!(layout_1, layout_2);
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

        assert_eq!(layout_1, layout_2);
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
        let blueprint = LayoutBlueprint {
            graph_representation: vec![(String::from("V"), vec![1]), (String::from("H"), vec![])],
            width: 10,
            height: 10,
        };
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

        assert_eq!(Ok(expected_layout), layout_from_blueprint);
    }

    // Since we only have two internal nodes, we know that if pass one of them to
    // `swap_with_random_node`, the other one will be the only other internal node. Thus we can
    // write a test case.
    #[test]
    fn swap_random_pair_of_internal_nodes() {
        let blueprint = LayoutBlueprint {
            graph_representation: vec![(String::from("V"), vec![1]), (String::from("H"), vec![])],
            width: 10,
            height: 10,
        };
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
        let blueprint = LayoutBlueprint {
            graph_representation: vec![(String::from("V"), vec![])],
            width: 10,
            height: 10,
        };
        let images = vec![RgbImage::new(1, 1), RgbImage::new(2, 2)];
        let mut layout = Layout::from_blueprint(&blueprint, &images).unwrap();

        layout.swap_with_random_node(&mut rand::thread_rng(), NodeIndex::new(1));

        assert_eq!(Some(&images[1]), layout.at_index(NodeIndex::new(1)).image());
        assert_eq!(Some(&images[0]), layout.at_index(NodeIndex::new(2)).image());
    }

    #[test]
    fn fall_back_to_swapping_leaf_nodes_if_all_internal_nodes_have_the_same_label() {
        let blueprint = LayoutBlueprint {
            graph_representation: vec![(String::from("V"), vec![1]), (String::from("V"), vec![])],
            width: 10,
            height: 10,
        };
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
        let blueprint = LayoutBlueprint {
            graph_representation: vec![(String::from("V"), vec![])],
            width: 10,
            height: 10,
        };
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
}
