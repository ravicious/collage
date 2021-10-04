use image::RgbImage;
use petgraph::{
    dot::{Config, Dot},
    graph::NodeIndex,
    Graph,
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

pub type LayoutGraph<'a> = Graph<LayoutNode<'a>, ()>;

pub enum LayoutNode<'a> {
    Internal(SliceDirection),
    Leaf(&'a RgbImage),
}

#[derive(Debug)]
pub enum SliceDirection {
    Vertical,
    Horizontal,
}

impl Distribution<SliceDirection> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> SliceDirection {
        match rng.gen::<bool>() {
            true => SliceDirection::Vertical,
            false => SliceDirection::Horizontal,
        }
    }
}

impl std::fmt::Debug for LayoutNode<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use LayoutNode::*;
        match self {
            Leaf(image) => {
                let dimensions = image.dimensions();
                f.debug_tuple("Image")
                    .field(&dimensions.0)
                    .field(&dimensions.1)
                    .finish()
            }
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

        layout.graph.add_node(LayoutNode::Internal(rand::random()));

        // Total number of internal nodes must be equal to <number of images> - 1 and we already
        // added one internal node.
        for _ in 0..images.len() - 2 {
            let random_index = layout.random_index_of_node_with_less_than_two_children();
            layout.add_node(random_index, LayoutNode::Internal(rand::random()));
        }

        let indexes_of_nodes_with_less_than_two_children: Vec<NodeIndex> = layout
            .indexes_of_nodes_with_less_than_two_children()
            .collect();

        for index in indexes_of_nodes_with_less_than_two_children {
            while layout.node_has_less_than_two_children(index) {
                layout.add_node(
                    index,
                    LayoutNode::Leaf(random_images.next().expect("Ran out of images")),
                );
            }
        }

        layout
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

    fn add_node(&mut self, parent_idx: NodeIndex, layout_node: LayoutNode<'a>) -> NodeIndex {
        let idx = self.graph.add_node(layout_node);
        self.graph.update_edge(parent_idx, idx, ());
        idx
    }

    // For debugging the graph in Graphviz.
    pub fn dot(&self) -> Dot<'_, &Graph<LayoutNode<'_>, ()>> {
        Dot::with_config(&self.graph, &[Config::EdgeNoLabel])
    }
}
