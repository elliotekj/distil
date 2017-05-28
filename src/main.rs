extern crate color_quant;
extern crate image;
extern crate itertools;

use std::collections::BTreeMap;
use std::fs::File;
use std::path::Path;

use color_quant::NeuQuant;
use image::FilterType::Gaussian;
use image::{imageops, ImageBuffer, GenericImage, DynamicImage, Rgba, Rgb, Pixel};
use itertools::Itertools;

static NQ_SAMPLE_FACTION: i32 = 10;
static NQ_PALETTE_SIZE: usize = 256;
static MIN_DISTANCE: f32 = 10.0;

pub struct Distil {
    img: DynamicImage,
    max_sample_count: u32,
}

impl Distil {
    pub fn new(&self) {
        let scaled_img = self.scale_img();
        let quantized_img = quantize(scaled_img);
        let color_histogram = get_histogram(quantized_img);
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
}

// Reduce the image's color palette down to 256 colors.
fn quantize(img: DynamicImage) -> Vec<Rgb<u8>> {
    let pixels = get_pixels(img);
    let quantized = NeuQuant::new(NQ_SAMPLE_FACTION, NQ_PALETTE_SIZE, &pixels);

    quantized.color_map_rgb()
        .iter()
        .chunks(3)
        .into_iter()
        .map(|rgb_iter| {
            let rgb_slice: Vec<u8> = rgb_iter.cloned().collect();
            Rgb::from_slice(&rgb_slice).clone()
        })
        .collect()
}

fn get_pixels(img: DynamicImage) -> Vec<u8> {
    let mut pixels = Vec::new();

    for (_, _, px) in img.pixels() {
        let rgba = px.to_rgba();

        if has_transparency(&rgba) {
            continue;
        }

        for channel in px.channels() {
            pixels.push(*channel);
        }
    }

    pixels
}

// Creates a histogram that counts the number of times each color occurs in the
// input image.
fn get_histogram(pixels: Vec<Rgb<u8>>) -> Vec<(Rgb<u8>, usize)> {
    let histogram_map = pixels.iter()
        .fold(BTreeMap::new(), |mut acc, px| {
            *acc.entry(px.channels()).or_insert(0) += 1;
            acc
        });

    let mut histogram_vec = histogram_map.iter()
        .fold(Vec::new(), |mut acc, (color, count)| {
            acc.push((Rgb::from_slice(&color).to_owned(), *count as usize));
            acc
        });

    histogram_vec.sort_by(|&(_, a), &(_, b)| a.cmp(&b));

    histogram_vec
}

fn has_transparency(rgba: &Rgba<u8>) -> bool {
    let alpha_channel = rgba[3];

    alpha_channel != 255
}

// fn output_palette_as_img(palette: Vec<Color>) {
//     let colors_img_width = 32 * palette.len() as u32;
//     let mut colors_img_buf = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(colors_img_width, 32);

//     for (i, color) in palette.iter().enumerate() {
//         let x_offset = (32 * i) as u32;
//         let mut sub_img = imageops::crop(&mut colors_img_buf, x_offset, 0, 32, 32);
//         let rgba = Rgba::from_channels(color.r, color.g, color.b, color.a);

//         for (_, _, px) in sub_img.pixels_mut() {
//             px.data = rgba.data;
//         }
//     }

//     let filename = format!("fout.png");
//     let ref mut fout = File::create(&Path::new(&filename)).unwrap();
//     let _ = image::ImageRgba8(colors_img_buf).save(fout, image::PNG);
// }

fn main() {
    let file = "/Users/elliot/dev/distil/test/sample-1.jpg";
    let img = image::open(&Path::new(&file)).unwrap();

    Distil::new(&Distil {
        img: img,
        max_sample_count: 5000,
    });
}
