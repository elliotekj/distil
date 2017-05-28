extern crate exoquant;
extern crate image;
extern crate itertools;

use std::path::Path;
use std::fs::File;

use exoquant::*;
use exoquant::optimizer::Optimizer;
use exoquant::{Color, Histogram};
use image::FilterType::Gaussian;
use image::{imageops, ImageBuffer, GenericImage, DynamicImage, Rgba, Rgb, Pixel};
use itertools::Itertools;

static N_QUANTIZE: usize = 100;

pub struct Distil {
    img: DynamicImage,
    max_sample_count: u32,
}

impl Distil {
    pub fn new(&self) {
        let scaled_img = self.scale_img();
        let quantized_img = self.quantize(scaled_img);
        let color_vec = to_color_vec(quantized_img);
        let color_histogram = get_histogram(color_vec);
        let sorted_histogram = sort_histogram(color_histogram);

        // let quantized_image_rgb_values = self.get_rgb_values(quantized_image);

        // let color_histogram = self.get_histogram(quantized_image_rgb_values);
    }

    // Proportionally scales the image to a size where the total number of pixels
    // does not exceed `max_sample_count`.
    fn scale_img(&self) -> DynamicImage {
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

    // Reduce the image's color palette down to `N_QUANTIZE` colors.
    fn quantize(&self, img: DynamicImage) -> Vec<u8> {
        let pixels = get_pixels(img.clone());
        let histogram = get_histogram(pixels.clone());

        let colorspace = SimpleColorSpace::default();
        let mut quantizer = Quantizer::new(&histogram, &colorspace);

        while quantizer.num_colors() < N_QUANTIZE {
            quantizer.step();
        }

        let palette = quantizer.colors(&colorspace);
        let palette = optimizer::KMeans.optimize_palette(&colorspace, &palette, &histogram, 16);

        let ditherer = ditherer::FloydSteinberg::checkered();
        let remapper = Remapper::new(&palette, &colorspace, &ditherer);

        // output_palette_as_img(palette);

        remapper.remap(&pixels, img.dimensions().1 as usize)
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

fn sort_histogram(histogram: Histogram) -> Vec<(Color, usize)> {
    let mut colors_and_count: Vec<(Color, usize)> = Vec::new();

    for c in histogram.iter() {
        colors_and_count.push((c.0.to_owned(), c.1.to_owned()));
    }

    colors_and_count.sort_by(|&(_, a), &(_, b)| a.cmp(&b));

    colors_and_count
}

fn has_transparency(rgba: &Rgba<u8>) -> bool {
    let alpha_channel = rgba[3];

    alpha_channel != 255
}

fn to_color_vec(rgba_as_u8_vec: Vec<u8>) -> Vec<Color> {
    let rgba_chunks = rgba_as_u8_vec.iter().chunks(4);
    let mut colors: Vec<Color> = Vec::new();

    for chunk in &rgba_chunks {
        let rgba_slice: Vec<u8> = chunk.cloned().collect();
        colors.push(Color::new(rgba_slice[0], rgba_slice[1], rgba_slice[2], rgba_slice[3]));
    }

    colors
}

fn output_palette_as_img(palette: Vec<Color>) {
    let colors_img_width = 32 * palette.len() as u32;
    let mut colors_img_buf = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(colors_img_width, 32);

    for (i, color) in palette.iter().enumerate() {
        let x_offset = (32 * i) as u32;
        let mut sub_img = imageops::crop(&mut colors_img_buf, x_offset, 0, 32, 32);
        let rgba = Rgba::from_channels(color.r, color.g, color.b, color.a);

        for (_, _, px) in sub_img.pixels_mut() {
            px.data = rgba.data;
        }
    }

    let filename = format!("fout.png");
    let ref mut fout = File::create(&Path::new(&filename)).unwrap();
    let _ = image::ImageRgba8(colors_img_buf).save(fout, image::PNG);
}

fn main() {
    let file = "/Users/elliot/dev/distil/test/sample-1.jpg";
    let img = image::open(&Path::new(&file)).unwrap();

    Distil::new(&Distil {
        img: img,
        max_sample_count: 5000,
    });
}
