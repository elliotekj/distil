extern crate color_quant;
extern crate delta_e;
extern crate image;
extern crate itertools;
extern crate lab;

use std::collections::BTreeMap;
use std::fs::File;
use std::path::Path;

use color_quant::NeuQuant;
use delta_e::DE2000;
use image::FilterType::Gaussian;
use image::{imageops, ImageBuffer, GenericImage, DynamicImage, Rgba, Rgb, Pixel};
use itertools::Itertools;
use lab::Lab;

static MAX_SAMPLE_COUNT: u32 = 1000;
static NQ_SAMPLE_FACTION: i32 = 10;
static NQ_PALETTE_SIZE: usize = 256;
static MIN_BLACK: u8 = 8;
static MAX_WHITE: u8 = 247;
static MIN_DISTANCE_FOR_UNIQUENESS: f32 = 10.0;

pub struct Distil;

impl Distil {
    pub fn new(img: DynamicImage, palette_size: u8) {
        let scaled_img = scale_img(img);
        let quantized_img = quantize(scaled_img);
        let color_histogram = get_histogram(quantized_img);
        let colors_as_lab = to_lab(color_histogram);
        let palette = remove_similar_colors(colors_as_lab);

        output_palette_as_img(palette, palette_size);
    }
}

// Proportionally scales the image to a size where the total number of pixels
// does not exceed `MAX_SAMPLE_COUNT`.
fn scale_img(mut img: DynamicImage) -> DynamicImage {
    let (width, height) = img.dimensions();

    if width * height > MAX_SAMPLE_COUNT {
        let (width, height) = (width as f32, height as f32);
        let ratio = width / height;

        let scaled_width = (ratio * (MAX_SAMPLE_COUNT as f32)).sqrt() as u32;

        img = img.resize(scaled_width, height as u32, Gaussian);
    }

    img
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

        if has_transparency(&rgba) || is_black(&rgba) || is_white(&rgba) {
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

    histogram_vec.sort_by(|&(_, a), &(_, b)| b.cmp(&a));

    histogram_vec
}

fn has_transparency(rgba: &Rgba<u8>) -> bool {
    let alpha_channel = rgba[3];

    alpha_channel != 255
}

fn is_black(rgba: &Rgba<u8>) -> bool {
    rgba[0] < MIN_BLACK && rgba[1] < MIN_BLACK && rgba[2] < MIN_BLACK
}

fn is_white(rgba: &Rgba<u8>) -> bool {
    rgba[0] > MAX_WHITE && rgba[1] > MAX_WHITE && rgba[2] > MAX_WHITE
}

fn to_lab(histogram: Vec<(Rgb<u8>, usize)>) -> Vec<(Lab, usize)> {
    histogram.iter()
        .fold(Vec::with_capacity(histogram.len()),
              |mut acc, &(color, count)| {
                  acc.push((Lab::from_rgb(&[color[0], color[1], color[2]]), count));
                  acc
              })
}

fn remove_similar_colors(palette: Vec<(Lab, usize)>) -> Vec<(Lab, usize)> {
    let mut additions = Vec::new();
    let mut refined_palette: Vec<(Lab, usize)> = Vec::new();

    for &(lab_x, count_x) in palette.iter() {
        let mut is_similar = false;

        for (i, &(lab_y, _)) in refined_palette.iter().enumerate() {
            let delta = DE2000::new(lab_x.into(), lab_y.into());

            if delta < MIN_DISTANCE_FOR_UNIQUENESS {
                additions.push((i, count_x));
                is_similar = true;
                break;
            }
        }

        if !is_similar {
            refined_palette.push((lab_x, count_x));
        }
    }

    for &(i, count) in &additions {
        refined_palette[i].1 += count;
    }

    refined_palette.sort_by(|&(_, a), &(_, b)| a.cmp(&b));

    refined_palette
}

fn output_palette_as_img(palette: Vec<(Lab, usize)>, palette_size: u8) {
    let colors_img_width;

    if palette.len() < palette_size as usize {
        colors_img_width = 80 * palette.len();
    } else {
        colors_img_width = 80 * palette_size as usize;
    }

    let mut colors_img_buf = ImageBuffer::<Rgb<u8>, Vec<u8>>::new(colors_img_width as u32, 80);

    for (i, &(color, _)) in palette.iter().enumerate() {
        let x_offset = (80 * i) as u32;
        let mut sub_img = imageops::crop(&mut colors_img_buf, x_offset, 0, 80, 80);
        let as_rgb = Lab::to_rgb(&color);
        let rgb = Rgb::from_channels(as_rgb[0], as_rgb[1], as_rgb[2], 255);

        for (_, _, px) in sub_img.pixels_mut() {
            px.data = rgb.data;
        }

        if i == palette_size as usize - 1 {
            break;
        }
    }

    let filename = format!("fout.png");
    let ref mut fout = File::create(&Path::new(&filename)).unwrap();
    let _ = image::ImageRgb8(colors_img_buf).save(fout, image::PNG);
}

fn main() {
    let file = "/Users/elliot/dev/distil/test/sample-3.png";
    let img = image::open(&Path::new(&file)).unwrap();

    Distil::new(img, 8);
}
