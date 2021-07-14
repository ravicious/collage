mod image_for_processing;
mod orientation;
mod utils;

use crate::image_for_processing::{ImageForProcessing, PageOrientation::*};
use image::flat::{FlatSamples, SampleLayout};
use image::{GenericImage, Rgb, RgbImage};
use wasm_bindgen::prelude::*;
use web_sys::console;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub fn setup() {
    utils::set_panic_hook();
}

#[wasm_bindgen]
pub fn process_images(image_array1: Vec<u8>, image_array2: Vec<u8>) -> Vec<u8> {
    console::time_with_label("loading first image");
    let image1 = array_to_image(image_array1);
    console::time_end_with_label("loading first image");

    console::time_with_label("loading second image");
    let image2 = array_to_image(image_array2);
    console::time_end_with_label("loading second image");

    console::time_with_label("combining images");

    let target = match (image1.page_orientation(), image2.page_orientation()) {
        (Landscape, Landscape) => make_portrait(image1, image2),
        _ => make_landscape(image1, image2),
    };

    let mut jpg_buffer: Vec<u8> = vec![];
    let mut jpg_encoder = image::jpeg::JpegEncoder::new(&mut jpg_buffer);
    jpg_encoder.encode_image(&target).unwrap();

    console::time_end_with_label("combining images");

    jpg_buffer
}

fn make_landscape(image1: RgbImage, image2: RgbImage) -> RgbImage {
    let mut target = RgbImage::new(
        image1.width() + image2.width(),
        image1.height().max(image2.height()),
    );

    if image1.dimensions() != image2.dimensions() {
        fill_background(&mut target);
    }

    target.copy_from(&image1, 0, 0).unwrap();
    target.copy_from(&image2, image1.width(), 0).unwrap();

    target
}

fn make_portrait(image1: RgbImage, image2: RgbImage) -> RgbImage {
    let mut target = RgbImage::new(
        image1.width().max(image2.width()),
        image1.height() + image2.height(),
    );

    if image1.dimensions() != image2.dimensions() {
        fill_background(&mut target);
    }

    target.copy_from(&image1, 0, 0).unwrap();
    target.copy_from(&image2, 0, image1.height()).unwrap();

    target
}

fn array_to_image(array: Vec<u8>) -> RgbImage {
    orientation::fix_if_needed(array)
}

fn fill_background(image: &mut RgbImage) {
    console::time_with_label("filling background");

    let white_buffer = FlatSamples {
        samples: &[0xff],
        layout: SampleLayout {
            channels: 3,
            channel_stride: 0,
            width: image.width(),
            width_stride: 0,
            height: image.height(),
            height_stride: 0,
        },
        color_hint: None,
    };
    let white_bg = white_buffer.as_view::<Rgb<u8>>().unwrap();
    image.copy_from(&white_bg, 0, 0).unwrap();

    console::time_end_with_label("filling background");
}
