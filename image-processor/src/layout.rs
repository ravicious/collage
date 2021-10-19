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
use std::collections::VecDeque;
use std::ptr;

#[derive(Debug)]
pub struct Layout<'a> {
  graph: LayoutGraph<'a>,
  pub canvas_dimensions: Dimensions,
}

pub type LayoutGraph<'a> = Graph<NodeLabel<'a>, ()>;

pub enum NodeLabel<'a> {
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

  pub fn aspect_ratio(&self) -> f64 {
    self.root_node().aspect_ratio()
  }

  pub fn dimensions(&self) -> (u32, u32) {
    self.root_node().dimensions().to_tuple()
  }

  // Smaller value is better.
  pub fn cost(&self) -> f64 {
    let number_of_images = self.leaf_nodes().count() as f64;
    let coverage_of_canvas_area = 1.0
      - self
        .leaf_nodes()
        .map(|leaf_node| {
          leaf_node.dimensions().size() as f64 / self.canvas_dimensions.size() as f64
        })
        .sum::<f64>();
    let scale_factor = self
      .leaf_nodes()
      .map(|leaf_node| {
        let original_image_size =
          Dimensions::from_tuple(leaf_node.image().unwrap().dimensions()).size() as f64;
        let scaled_image_size = leaf_node.dimensions().size() as f64;

        (scaled_image_size - original_image_size).abs() / original_image_size
      })
      .sum::<f64>();

    scale_factor + number_of_images * coverage_of_canvas_area
  }

  fn calculate_random_canvas_dimensions(images: &'a [RgbImage]) -> Dimensions {
    let mut rng = rand::thread_rng();
    let len_for_width = rng.gen_range(1..=images.len());
    let len_for_height = rng.gen_range(1..=images.len());
    let width = images
      .choose_multiple(&mut rng, len_for_width)
      .map(|i| i.width())
      .sum();
    let height = images
      .choose_multiple(&mut rng, len_for_height)
      .map(|i| i.height())
      .sum();

    // To sum dimensions from the same set of images (might produce better results):
    //   let len = rng.gen_range(1..=images.len());
    //   let random_images: Vec<&RgbImage> = images.choose_multiple(&mut rng, len).collect();
    //   let width = random_images.iter().map(|i| i.width()).sum();
    //   let height = random_images.iter().map(|i| i.height()).sum();

    Dimensions { width, height }
  }

  fn random_index_of_node_with_less_than_two_children(&self) -> NodeIndex {
    let mut rng = rand::thread_rng();
    self
      .indexes_of_nodes_with_less_than_two_children()
      .choose(&mut rng)
      .unwrap()
  }

  fn indexes_of_nodes_with_less_than_two_children(&self) -> impl Iterator<Item = NodeIndex> + '_ {
    self
      .graph
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

  pub fn leaf_nodes(&self) -> impl Iterator<Item = LayoutNode> + '_ {
    self
      .graph
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
    self
      .graph
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

pub struct LayoutNode<'a> {
  index: NodeIndex,
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

#[derive(Debug, Copy, Clone)]
pub struct Dimensions {
  pub width: u32,
  pub height: u32,
}

impl Dimensions {
  fn from_tuple(tuple: (u32, u32)) -> Dimensions {
    Dimensions {
      width: tuple.0,
      height: tuple.1,
    }
  }

  fn to_tuple(self) -> (u32, u32) {
    (self.width, self.height)
  }

  fn size(&self) -> u32 {
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
