#![feature(try_blocks)]

mod image_for_processing;
pub mod layout;
mod orientation;
pub mod renderer;
mod utils;

use crate::image_for_processing::{ImageForProcessing, PageOrientation::*};
use crate::layout::Layout;
use image::{GenericImage, RgbImage};
use wasm_bindgen::prelude::*;
use web_sys::console;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen(start)]
pub fn setup() {
  utils::set_panic_hook();
}

#[wasm_bindgen]
pub fn process_images(image_arrays: Vec<js_sys::Uint8Array>) -> Vec<u8> {
  let mut images: Vec<RgbImage> = image_arrays
    .into_iter()
    .enumerate()
    .map(|(i, image_array)| {
      console::group_1(&format!("processing image {}", i + 1).into());
      console::time();
      let result = array_to_image(&image_array.to_vec());
      console::time_end();
      console::group_end();
      result
    })
    .collect();

  let target;

  if images.len() > 2 {
    console::time_with_label("generating random layout");
    let mut layout = Layout::new(&images);
    let mut layout_cost = layout.cost();
    for _ in 0..100_000 {
      let new_layout = Layout::new(&images);
      let new_layout_cost = new_layout.cost();

      if new_layout_cost < layout_cost {
        layout = new_layout;
        layout_cost = new_layout_cost;
      }
    }
    console::time_end_with_label("generating random layout");

    console::time_with_label("rendering layout");
    target = renderer::render_layout(&layout);
    console::time_end_with_label("rendering layout");

    console::group_collapsed_1(&"Layout debug".into());
    console::log_1(&format!("{:?}", layout.dot()).into());
    console::log_1(&format!("Canvas dimensions: {:?}", layout.canvas_dimensions).into());
    console::log_1(&format!("Dimensions: {:?}", layout.dimensions()).into());
    console::log_1(&format!("Cost: {}", layout.cost()).into());
    console::group_end();
  } else if let ([image1, image2], _) = images.split_at_mut(2) {
    console::time_with_label("combining two images");

    target = match (image1.page_orientation(), image2.page_orientation()) {
      (Landscape, Landscape) => make_portrait(image1, image2),
      _ => make_landscape(image1, image2),
    };

    console::time_end_with_label("combining two images");
  } else {
    panic!("Less than two images received")
  }

  console::time_with_label("encoding end result");
  let mut jpg_buffer: Vec<u8> = vec![];
  let mut jpg_encoder = image::jpeg::JpegEncoder::new(&mut jpg_buffer);
  jpg_encoder.encode_image(&target).unwrap();
  console::time_end_with_label("encoding end result");

  jpg_buffer
}

fn make_landscape(image1: &mut RgbImage, image2: &mut RgbImage) -> RgbImage {
  if image1.dimensions() != image2.dimensions() {
    console::time_with_label("fit height");
    fit_height(image1, image2);
    console::time_end_with_label("fit height");
  }

  let mut target = RgbImage::new(
    image1.width() + image2.width(),
    image1.height().max(image2.height()),
  );

  target.copy_from(image1, 0, 0).unwrap();
  target.copy_from(image2, image1.width(), 0).unwrap();

  target
}

fn make_portrait(image1: &mut RgbImage, image2: &mut RgbImage) -> RgbImage {
  if image1.dimensions() != image2.dimensions() {
    console::time_with_label("fit width");
    fit_width(image1, image2);
    console::time_end_with_label("fit width");
  }

  let mut target = RgbImage::new(
    image1.width().max(image2.width()),
    image1.height() + image2.height(),
  );

  target.copy_from(image1, 0, 0).unwrap();
  target.copy_from(image2, 0, image1.height()).unwrap();

  target
}

fn array_to_image(array: &[u8]) -> RgbImage {
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
