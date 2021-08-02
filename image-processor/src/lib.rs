mod image_for_processing;
mod orientation;
mod utils;

use crate::image_for_processing::{ImageForProcessing, PageOrientation::*};
use image::{GenericImage, RgbImage};
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

fn make_landscape(mut image1: RgbImage, mut image2: RgbImage) -> RgbImage {
    if image1.dimensions() != image2.dimensions() {
        console::time_with_label("fit height");
        fit_height(&mut image1, &mut image2);
        console::time_end_with_label("fit height");
    }

    let mut target = RgbImage::new(
        image1.width() + image2.width(),
        image1.height().max(image2.height()),
    );

    target.copy_from(&image1, 0, 0).unwrap();
    target.copy_from(&image2, image1.width(), 0).unwrap();

    target
}

fn make_portrait(mut image1: RgbImage, mut image2: RgbImage) -> RgbImage {
    if image1.dimensions() != image2.dimensions() {
        console::time_with_label("fit width");
        fit_width(&mut image1, &mut image2);
        console::time_end_with_label("fit width");
    }

    let mut target = RgbImage::new(
        image1.width().max(image2.width()),
        image1.height() + image2.height(),
    );

    target.copy_from(&image1, 0, 0).unwrap();
    target.copy_from(&image2, 0, image1.height()).unwrap();

    target
}

fn array_to_image(array: Vec<u8>) -> RgbImage {
    orientation::fix_if_needed(array)
}

fn fit_height(image1: &mut RgbImage, image2: &mut RgbImage) {
    use std::cmp::Ordering::*;
    let taller;
    let shorter;

    match image1.height().cmp(&image2.height()) {
        Greater => {
            taller = image1;
            shorter = image2;
        }
        _ => {
            taller = image2;
            shorter = image1;
        }
    }

    // Scale the taller image down so that it has the same height as the shorter one.
    let new_width = taller.width() * shorter.height() / taller.height();
    *taller = image::imageops::resize(
        taller,
        new_width,
        shorter.height(),
        image::imageops::FilterType::Lanczos3,
    );
}

fn fit_width(image1: &mut RgbImage, image2: &mut RgbImage) {
    use std::cmp::Ordering::*;
    let wider;
    let narrower;

    match image1.width().cmp(&image2.width()) {
        Greater => {
            wider = image1;
            narrower = image2;
        }
        _ => {
            wider = image2;
            narrower = image1;
        }
    }

    // Scale the wider image down so that it has the same width as the narrower one.
    let new_height = wider.height() * narrower.width() / wider.width();
    *wider = image::imageops::resize(
        wider,
        narrower.width(),
        new_height,
        image::imageops::FilterType::Lanczos3,
    );
}
