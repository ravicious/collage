use exif::{In, Tag};
use image::RgbImage;
use std::io::Cursor;
use web_sys::console;

pub fn fix_if_needed(raw_image: Vec<u8>) -> RgbImage {
    let orientation = get_orientation(&raw_image);
    let needs_to_be_fixed = needs_fixing(orientation);
    console::log_1(&format!("needs to be fixed? {:?}", needs_to_be_fixed).into());
    // TODO: Actually fix the image before returning it.
    image::load_from_memory(&raw_image).unwrap().into_rgb8()
}

#[derive(Debug)]
enum OrientationError {
    ParsingError(String),
    NoExif,
    NoOrientationTag,
}

fn get_orientation(raw_image: &[u8]) -> Result<exif::Field, OrientationError> {
    let reader = exif::Reader::new();
    let mut cursor = Cursor::new(&raw_image);
    let exif_data = reader
        .read_from_container(&mut cursor)
        .map_err(|err| match err {
            exif::Error::NotFound(_) => OrientationError::NoExif,
            _ => OrientationError::ParsingError(format!("{:?}", err)),
        })?;

    exif_data
        .get_field(Tag::Orientation, In::PRIMARY)
        .cloned()
        .ok_or(OrientationError::NoOrientationTag)
}

fn needs_fixing(field_result: Result<exif::Field, OrientationError>) -> bool {
    match field_result {
        Ok(orientation) => match orientation.value.get_uint(0) {
            Some(1) => false,
            Some(2..=8) => true,
            value => {
                console::error_1(
                    &format!("Value of orientation field is invalid: {:?}", value).into(),
                );
                false
            }
        },
        Err(error) => {
            match error {
                OrientationError::NoExif => {
                    console::log_1(&"No Exif data found".into());
                }
                OrientationError::NoOrientationTag => {
                    console::log_1(&"No orientation tag found in Exif attributes".into());
                }
                _ => {
                    console::error_1(
                        &format!("Error while getting orientation from Exif: {:?}", error).into(),
                    );
                }
            }
            false
        }
    }
}
