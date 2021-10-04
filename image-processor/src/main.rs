use image::RgbImage;
use image_processor::layout::Layout;

// Test code for debugging layout creation. Output can be copied to the clipboard and used with
// Graphviz.
fn main() {
    let mut images = vec![];

    for i in 0..6 {
        images.push(RgbImage::new(i + 1, 1));
    }

    let layout = Layout::new(&images);

    println!("{:?}", layout.dot());
    println!("Aspect ratio: {}", layout.aspect_ratio());
}
