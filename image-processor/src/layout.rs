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

pub struct Layout<'a> {
    graph: LayoutGraph<'a>,
}

pub type LayoutGraph<'a> = Graph<NodeLabel<'a>, ()>;

pub enum NodeLabel<'a> {
    Internal(SliceDirection),
    Leaf(&'a RgbImage),
}
use NodeLabel::*;

#[derive(Debug)]
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
        let mut layout = Layout { graph };
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

    pub fn aspect_ratio(&self) -> f64 {
        self.root_node().aspect_ratio()
    }

    pub fn dimensions(&self) -> (u32, u32) {
        self.root_node().dimensions()
    }

    fn random_index_of_node_with_less_than_two_children(&self) -> NodeIndex {
        let mut rng = rand::thread_rng();
        self.indexes_of_nodes_with_less_than_two_children()
            .choose(&mut rng)
            .unwrap()
    }

    fn indexes_of_nodes_with_less_than_two_children(
        &self,
    ) -> impl Iterator<Item = NodeIndex> + '_ {
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

        LayoutNode {
            index,
            layout: self,
        }
    }

    // For debugging the graph in Graphviz.
    pub fn dot(&self) -> Dot<'_, &Graph<NodeLabel<'_>, ()>> {
        Dot::with_config(&self.graph, &[Config::EdgeNoLabel])
    }

    fn children(&self, node: &LayoutNode) -> Option<(LayoutNode, LayoutNode)> {
        let mut iterator = self.graph.neighbors(node.index);

        try {
            (
                iterator.next().map(|index| self.at_index(index))?,
                iterator.next().map(|index| self.at_index(index))?,
            )
        }
    }

    fn node_label(&self, node: &LayoutNode) -> &NodeLabel {
        self.graph.node_weight(node.index).unwrap()
    }

    fn at_index(&self, index: NodeIndex) -> LayoutNode {
        LayoutNode {
            index,
            layout: self,
        }
    }
}

struct LayoutNode<'a> {
    index: NodeIndex,
    layout: &'a Layout<'a>,
}

impl LayoutNode<'_> {
    fn aspect_ratio(&self) -> f64 {
        use NodeLabel::*;
        use SliceDirection::*;

        match self.node_label() {
            Leaf(image) => f64::from(image.width()) / f64::from(image.height()),
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

    pub fn dimensions(&self) -> (u32, u32) {
        use std::cmp::Ordering::*;

        match self.node_label() {
            Leaf(image) => image.dimensions(),
            Internal(Vertical) => {
                let children = self.children().unwrap();
                let taller;
                let shorter;

                match children.0.height().cmp(&children.1.height()) {
                    Greater => {
                        taller = children.0;
                        shorter = children.1;
                    }
                    _ => {
                        taller = children.1;
                        shorter = children.0;
                    }
                }

                let new_taller_width = taller.width() * shorter.height() / taller.height();
                let width = new_taller_width + shorter.width();
                let height = shorter.height();

                (width, height)
            }
            Internal(Horizontal) => {
                let children = self.children().unwrap();
                let wider;
                let narrower;

                match children.0.width().cmp(&children.1.width()) {
                    Greater => {
                        wider = children.0;
                        narrower = children.1;
                    }
                    _ => {
                        wider = children.1;
                        narrower = children.0;
                    }
                }

                let new_wider_height = wider.height() * narrower.width() / wider.width();
                let height = new_wider_height + narrower.height();
                let width = narrower.width();

                (width, height)
            }
        }
    }

    fn height(&self) -> u32 {
        match self.node_label() {
            Leaf(image) => image.height(),
            _ => self.dimensions().1,
        }
    }

    fn width(&self) -> u32 {
        match self.node_label() {
            Leaf(image) => image.width(),
            _ => self.dimensions().0,
        }
    }

    fn node_label(&self) -> &NodeLabel {
        self.layout.node_label(self)
    }

    fn children(&self) -> Option<(LayoutNode, LayoutNode)> {
        self.layout.children(self)
    }
}
