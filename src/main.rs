extern crate exoquant;
extern crate image;

use std::path::Path;

use exoquant::*;
use exoquant::optimizer::Optimizer;
use exoquant::{Color, Histogram};
use image::FilterType::Gaussian;
use image::{imageops, ImageBuffer, GenericImage, DynamicImage, Rgba, Pixel};

static N_QUANTIZE: usize = 100;

pub struct Distil {
    img: DynamicImage,
    max_sample_count: u32,
}

impl Distil {
    pub fn new(&self) {
        let scaled_image = self.scale_image();

        let pixels = get_pixels(scaled_image);
        let histogram = get_histogram(pixels);

        let colorspace = SimpleColorSpace::default();
        let mut quantizer = Quantizer::new(&histogram, &colorspace);

        while quantizer.num_colors() < N_QUANTIZE {
            quantizer.step();
        }

        let palette = quantizer.colors(&colorspace);

        let optimizer = optimizer::KMeans;
        let palette = optimizer.optimize_palette(&colorspace, &palette, &histogram, 16);
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

fn get_pixels(img: DynamicImage) -> Vec<Color> {
    let mut pixels = Vec::new();

    for (_, _, px) in img.pixels() {
        let rgba = px.to_rgba();

        if has_transparency(&rgba) {
            continue;
        }

        let rgba = Color::new(rgba[0], rgba[1], rgba[2], rgba[3]);
        pixels.push(rgba);
    }

    pixels
}

// Creates a histogram that counts the number of times each color occurs in the
// input image.
fn get_histogram(pixels: Vec<Color>) -> Histogram {
    let mut histogram = Histogram::new();

    histogram.extend(pixels);

    histogram
}

fn has_transparency(rgba: &Rgba<u8>) -> bool {
    let alpha_channel = rgba[3];

    alpha_channel != 255
}

fn main() {
    let file = "/Users/elliot/dev/distil/test/sample-1.jpg";
    let img = image::open(&Path::new(&file)).unwrap();

    Distil::new(&Distil {
        img: img,
        max_sample_count: 5000,
    });
}
