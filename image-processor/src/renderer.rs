use crate::layout::{ChildSide::*, Layout, NodeLabel::*, SliceDirection::*};
use image::{GenericImage, RgbImage};
use itertools::Itertools;

#[derive(Debug)]
struct Point {
  x: u32,
  y: u32,
}

pub fn render_layout(layout: &Layout) -> RgbImage {
  // For each leaf node:
  //
  // 1. collect each parent up to the root node and save its node label and calculated size
  // 2. traverse that path from the root node, calculating the position based on the size
  // 3. render the image on the canvas

  let mut result = RgbImage::new(
    layout.canvas_dimensions.width,
    layout.canvas_dimensions.height,
  );

  for leaf_node in layout.leaf_nodes() {
    let mut coords = Point { x: 0, y: 0 };
    for (parent, child) in leaf_node.lineage().iter().tuple_windows() {
      let other_child = parent.other_child(child).unwrap();
      let child_side = parent.child_side(child).unwrap();

      match (parent.node_label(), child_side) {
        (Internal(Horizontal), Right) => coords.y += other_child.height(),
        (Internal(Vertical), Right) => coords.x += other_child.width(),
        _ => {}
      }
    }

    let resized_image = image::imageops::resize(
      leaf_node.image().unwrap(),
      leaf_node.width(),
      leaf_node.height(),
      image::imageops::FilterType::Lanczos3,
    );

    result
      .copy_from(&resized_image, coords.x, coords.y)
      .unwrap();
  }

  result
}
