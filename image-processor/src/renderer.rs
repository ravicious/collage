use crate::layout::{ChildSide::*, Layout, NodeLabel::*, SliceDirection::*};
use image::{GenericImage, RgbImage};
use itertools::Itertools;
use web_sys::console;

#[derive(Debug)]
struct Point {
    x: u32,
    y: u32,
}

pub fn render_layout(layout: &Layout) -> RgbImage {
    // Canvas dimensions stored on the layout are just a side effect of how the original algorithm
    // is described in the paper. The paper assumes that the canvas size is always known upfront.
    // But in our case we want to be as big as possible without scaling the images up or down too
    // much.
    //
    // So the canvas dimensions are more like just auxiliary values that are meant to help us get to
    // the optimal layout. It's rare that the generated dimensions are 100% equal to canvas
    // dimensions. So instead of including bars of black pixels, the final image can just have the
    // actual generated dimensions.
    let (width, height) = layout.dimensions();
    let mut result = RgbImage::new(width, height);

    for internal_node in layout.internal_nodes() {
        console::log_1(
            &format!(
                "{:?}, {:?}, {:?}, {}",
                internal_node.index,
                internal_node.node_label(),
                internal_node.dimensions().to_tuple(),
                internal_node.aspect_ratio()
            )
            .into(),
        );
    }

    // For each leaf node:
    //
    // 1. collect each parent up to the root node and save its node label and calculated size
    // 2. traverse that path from the root node, calculating the position based on the size
    // 3. render the image on the canvas
    for leaf_node in layout.leaf_nodes() {
        let mut coords = Point { x: 0, y: 0 };
        for (parent, child) in leaf_node.lineage().iter().tuple_windows() {
            let other_child_dimensions = parent.other_child(child).unwrap().dimensions();
            let child_side = parent.child_side(child).unwrap();

            match (parent.node_label(), child_side) {
                (Internal(Horizontal), Right) => coords.y += other_child_dimensions.height,
                (Internal(Vertical), Right) => coords.x += other_child_dimensions.width,
                _ => {}
            }
        }

        let dimensions = leaf_node.dimensions();

        let resized_image = image::imageops::resize(
            leaf_node.image().unwrap(),
            dimensions.width,
            dimensions.height,
            image::imageops::FilterType::Lanczos3,
        );

        console::log_1(
            &format!(
                "{:?}, {:?}, {:?}, {:?}, {}",
                leaf_node.index,
                leaf_node.node_label(),
                dimensions.to_tuple(),
                coords,
                leaf_node.aspect_ratio()
            )
            .into(),
        );

        result
            .copy_from(&resized_image, coords.x, coords.y)
            .unwrap();
    }

    result
}
