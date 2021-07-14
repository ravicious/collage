mod orientation;
mod utils;

use image::flat::{FlatSamples, SampleLayout};
use image::{GenericImage, Rgb, RgbImage};
use wasm_bindgen::prelude::*;

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
    let image1 = array_to_image(image_array1);
    let image2 = array_to_image(image_array2);

    let mut target = RgbImage::new(
        image1.width() + image2.width(),
        image1.height().max(image2.height()),
    );

    // Fill target image with white color.
    // TODO: Don't do this if both images have the same size. Could be optimize further but that's
    // not important for now.
    let white_buffer = FlatSamples {
        samples: &[0xff],
        layout: SampleLayout {
            channels: 3,
            channel_stride: 0,
            width: target.width(),
            width_stride: 0,
            height: target.height(),
            height_stride: 0,
        },
        color_hint: None,
    };
    let white_bg = white_buffer.as_view::<Rgb<u8>>().unwrap();
    target.copy_from(&white_bg, 0, 0).unwrap();

    target.copy_from(&image1, 0, 0).unwrap();
    target.copy_from(&image2, image1.width(), 0).unwrap();

    let mut jpg_buffer: Vec<u8> = vec![];
    let mut jpg_encoder = image::jpeg::JpegEncoder::new(&mut jpg_buffer);
    jpg_encoder.encode_image(&target).unwrap();

    jpg_buffer
}

fn array_to_image(array: Vec<u8>) -> RgbImage {
    orientation::fix_if_needed(array)
}
