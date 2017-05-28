extern crate image;

use std::path::Path;

use image::{GenericImage, DynamicImage};
use image::FilterType::Gaussian;

pub struct Distil {
    img: DynamicImage,
    max_sample_count: u32,
}

impl Distil {
    pub fn new(&self) {
        let scaled_image = self.scale_image();
        println!("dimensions: {:?}", scaled_image.dimensions());
    }

    // Proportionally scales the image to a size where the total number of pixels
    // does not exceed `max_sample_count`.
    fn scale_image(&self) -> DynamicImage {
        let mut img = self.img.clone();
        let (width, height) = img.dimensions();

        if width * height > self.max_sample_count {
            let (width, height) = (width as f32, height as f32);
            let ratio = width / height;

            let scaled_width = (ratio * (self.max_sample_count as f32)).sqrt() as u32;

            img = img.resize(scaled_width, height as u32, Gaussian);
        }

        img
    }
}

fn main() {
    let file = "/Users/elliot/dev/distil/test/sample-1.jpg";
    let img = image::open(&Path::new(&file)).unwrap();

    Distil::new(&Distil {
        img: img,
        max_sample_count: 1000,
    });
}
