pub trait ImageForProcessing {
    fn page_orientation(&self) -> PageOrientation;
}

impl ImageForProcessing for image::RgbImage {
    fn page_orientation(&self) -> PageOrientation {
        use std::cmp::Ordering::*;
        use PageOrientation::*;

        let width = self.width();
        let height = self.height();

        match width.cmp(&height) {
            Greater => Landscape,
            Less => Portrait,
            Equal => Square,
        }
    }
}

pub enum PageOrientation {
    Landscape,
    Portrait,
    Square,
}
