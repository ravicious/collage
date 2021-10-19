use image::RgbImage;
use image_processor::{layout::Layout, renderer};

// Test code for debugging layout creation. Output can be copied to the clipboard and used with
// Graphviz.
fn main() {
    let mut images = vec![];

    for i in 0..6 {
        images.push(RgbImage::new(i * 100 + 100, 1000));
    }

    let layout = Layout::new(&images);

    println!("{:?}", layout.dot());
    println!("Canvas dimensions: {:?}", layout.canvas_dimensions);
    println!("Dimensions: {:?}", layout.dimensions());
    println!();

    // let leaf_node = layout.leaf_nodes().last().unwrap();

    // println!("leaf node: {:?}", leaf_node);
    // println!();

    // for ancestor in leaf_node.ancestors() {
    //     println!("parent: {:?}", ancestor);
    //     println!("children: {:?}", ancestor.children());
    //     println!();
    // }

    println!("Rendering layout");
    println!();

    renderer::render_layout(&layout);
}
