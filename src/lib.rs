extern crate color_quant;
extern crate delta_e;
extern crate image;
extern crate itertools;
extern crate lab;
#[macro_use]
extern crate quick_error;

use std::collections::BTreeMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use color_quant::NeuQuant;
use delta_e::DE2000;
use image::FilterType::Gaussian;
use image::{DynamicImage, GenericImage, guess_format, ImageBuffer, ImageFormat, imageops, Pixel,
            Rgb, Rgba};
use itertools::Itertools;
use lab::Lab;

static MAX_SAMPLE_COUNT: u32 = 1000;
static NQ_SAMPLE_FACTION: i32 = 10;
static NQ_PALETTE_SIZE: usize = 256;
static MIN_BLACK: u8 = 8;
static MAX_WHITE: u8 = 247;
static MIN_DISTANCE_FOR_UNIQUENESS: f32 = 10.0;

quick_error! {
    #[derive(Debug)]
    pub enum DistilError {
        /// Produced when Distil fails to parse the passed path.
        Io(path: String, err: image::ImageError) {
            display("Distil failed to parse the passed image: {}", err)
        }

        /// Produced when the image passed isn't a JPEG or a PNG.
        UnsupportedFormat {
            display("The passed image isn't a JPEG or a PNG")
        }

        /// Produced when Distil can't find any "interesting" colours in a passed image. Colours
        /// are deemed "interesting" if they fall between RGB(8, 8, 8) and RGB(247, 247, 247).
        Uninteresting {
            display("The passed image does not contain any interesting colours")
        }
    }
}

/// Represents a distilled image.
#[derive(Debug, Clone)]
pub struct Distil {
    /// `colors` contains all of the RGB values the image was distilled down
    /// into organised from most-frequent to least-frequent.
    pub colors: Vec<[u8; 3]>,

    /// `color_count` maps the index of each color in `colors` to the total
    /// number of colors that were distilled down into that same color from a
    /// palette of 256.
    ///
    /// It can be used, for example, to weight a colors importance when
    /// distilling multiple palettes into one.
    pub color_count: BTreeMap<usize, usize>,
}

impl Distil {
    /// `from_path_str` takes a path to an image which exists locally on the
    /// system and `Distil`s it.
    ///
    /// ## Example
    ///
    /// ```
    /// use distil::Distil;
    ///
    /// let path_str = "/Users/elliot/dev/distil/images/img-1.jpg";
    ///
    /// if let Ok(distilled) = Distil::from_path_str(path_str) {
    ///     // Do something with the returned `Distil` struct…
    /// }
    /// ```
    pub fn from_path_str(path_str: &str) -> Result<Distil, DistilError> {
        let path = Path::new(&path_str);
        Distil::from_path(&path)
    }

    /// `from_path` takes a `&Path` to an image which exists locally on the
    /// system and `Distil`s it.
    ///
    /// ## Example
    ///
    /// ```
    /// use std::path::Path;
    /// use distil::Distil;
    ///
    /// let path = Path::new("/Users/elliot/dev/distil/images/img-1.jpg");
    ///
    /// if let Ok(distilled) = Distil::from_path(&path) {
    ///     // Do something with the returned `Distil` struct…
    /// }
    /// ```
    pub fn from_path(path: &Path) -> Result<Distil, DistilError> {
        let image_format = get_image_format(&path)?;

        is_supported_format(image_format)?;

        match image::open(path) {
            Ok(img) => return Distil::new(img),
            Err(err) => return Err(DistilError::Io(format!("{:?}", path), err)),
        }
    }

    fn new(img: DynamicImage) -> Result<Distil, DistilError> {
        let scaled_img = scale_img(img);

        match quantize(scaled_img) {
            Ok(quantized_img) => {
                let color_count = count_colors_as_lab(quantized_img);
                let palette = remove_similar_colors(color_count);

                Ok(distil_palette(palette))
            }
            Err(err) => return Err(err),
        }
    }

    /// Export the distilled color palette as a PNG.
    ///
    /// ## Example
    ///
    /// ```
    /// use std::path::Path;
    /// use distil::Distil;
    ///
    /// let path_str = "/Users/elliot/dev/distil/images/img-1.jpg";
    /// let output_str = "img-1-palette.png";
    /// let palette_size = 5;
    ///
    /// if let Ok(distilled) = Distil::from_path_str(path_str) {
    ///     distilled.as_img(&Path::new(output_str), palette_size);
    /// }
    /// ```
    pub fn as_img(&self, out_path: &Path, palette_size: u8) {
        let colors_img_width;

        if self.colors.len() < palette_size as usize {
            colors_img_width = 80 * self.colors.len();
        } else {
            colors_img_width = 80 * palette_size as usize;
        }

        let mut colors_img_buf = ImageBuffer::<Rgb<u8>, Vec<u8>>::new(colors_img_width as u32, 80);

        for (i, color) in self.colors.iter().enumerate() {
            let x_offset = (80 * i) as u32;
            let mut sub_img = imageops::crop(&mut colors_img_buf, x_offset, 0, 80, 80);
            let rgb = Rgb::from_channels(color[0], color[1], color[2], 255);

            for (_, _, px) in sub_img.pixels_mut() {
                px.data = rgb.data;
            }

            if i == palette_size as usize - 1 {
                break;
            }
        }

        if let Ok(ref mut fout) = File::create(&out_path) {
            let _ = image::ImageRgb8(colors_img_buf).save(fout, image::PNG);
        };
    }
}

fn get_image_format(path: &Path) -> Result<ImageFormat, DistilError> {
    if let Ok(mut file) = File::open(path) {
        let mut file_buffer = [0; 16];
        let _ = file.read(&mut file_buffer);

        if let Ok(format) = guess_format(&file_buffer) {
            return Ok(format);
        }
    }

    Err(DistilError::UnsupportedFormat)
}

fn is_supported_format(format: ImageFormat) -> Result<(), DistilError> {
    match format {
        ImageFormat::PNG | ImageFormat::JPEG => {
            return Ok(());
        }
        _ => {
            return Err(DistilError::UnsupportedFormat);
        }
    }
}

/// Proportionally scales the passed image to a size where its total number of
/// pixels does not exceed the value of `MAX_SAMPLE_COUNT`.
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

/// Uses the NeuQuant quantization algorithm to reduce the passed image to a
/// palette of `NQ_PALETTE_SIZE` colors.
///
/// Note: NeuQuant is designed to produce images with between 64 and 256
/// colors. As such, `NQ_PALETTE_SIZE`'s value should be kept within those
/// bounds.
fn quantize(img: DynamicImage) -> Result<Vec<Rgb<u8>>, DistilError> {
    match get_pixels(img) {
        Ok(pixels) => {
            let quantized = NeuQuant::new(NQ_SAMPLE_FACTION, NQ_PALETTE_SIZE, &pixels);

            Ok(quantized.color_map_rgb()
                .iter()
                .chunks(3)
                .into_iter()
                .map(|rgb_iter| {
                    let rgb_slice: Vec<u8> = rgb_iter.cloned().collect();
                    Rgb::from_slice(&rgb_slice).clone()
                })
                .collect())
        }
        Err(err) => Err(err),
    }
}

/// Processes each of the pixels in the passed image, filtering out any that are
/// transparent or too light / dark to be interesting, then returns a `Vec` of the
/// `Rgba` channels of "interesting" pixels which is intended to be fed into
/// `NeuQuant`.
fn get_pixels(img: DynamicImage) -> Result<Vec<u8>, DistilError> {
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

    if pixels.len() == 0 {
        return Err(DistilError::Uninteresting);
    }

    Ok(pixels)
}

/// Checks if the passed pixel is opaque or not.
fn has_transparency(rgba: &Rgba<u8>) -> bool {
    let alpha_channel = rgba[3];

    alpha_channel != 255
}

/// Checks if the passed pixel is too dark to be interesting.
fn is_black(rgba: &Rgba<u8>) -> bool {
    rgba[0] < MIN_BLACK && rgba[1] < MIN_BLACK && rgba[2] < MIN_BLACK
}

/// Checks if the passed pixel is too light to be interesting.
fn is_white(rgba: &Rgba<u8>) -> bool {
    rgba[0] > MAX_WHITE && rgba[1] > MAX_WHITE && rgba[2] > MAX_WHITE
}

/// Maps each unique Lab color in the passed `Vec` of pixels to the total
/// number of times that color appears in the `Vec`.
fn count_colors_as_lab(pixels: Vec<Rgb<u8>>) -> Vec<(Lab, usize)> {
    let color_count_map = pixels.iter()
        .fold(BTreeMap::new(), |mut acc, px| {
            *acc.entry(px.channels()).or_insert(0) += 1;
            acc
        });

    let mut color_count_vec = color_count_map.iter()
        .fold(Vec::new(), |mut acc, (color, count)| {
            let rgb = Rgb::from_slice(&color).to_owned();
            acc.push((Lab::from_rgb(&[rgb[0], rgb[1], rgb[2]]), *count as usize));
            acc
        });

    color_count_vec.sort_by(|&(_, a), &(_, b)| b.cmp(&a));

    color_count_vec
}

fn remove_similar_colors(palette: Vec<(Lab, usize)>) -> Vec<(Lab, usize)> {
    let mut similars = Vec::new();
    let mut refined_palette: Vec<(Lab, usize)> = Vec::new();

    for &(lab_x, count_x) in palette.iter() {
        let mut is_similar = false;

        for (i, &(lab_y, _)) in refined_palette.iter().enumerate() {
            let delta = DE2000::new(lab_x.into(), lab_y.into());

            if delta < MIN_DISTANCE_FOR_UNIQUENESS {
                similars.push((i, lab_x, count_x));
                is_similar = true;
                break;
            }
        }

        if !is_similar {
            refined_palette.push((lab_x, count_x));
        }
    }

    for &(i, lab_y, count) in &similars {
        let lab_x = refined_palette[i].0;
        let (lx, ax, bx) = (lab_x.l, lab_x.a, lab_x.b);
        let (ly, ay, by) = (lab_y.l, lab_y.a, lab_y.b);

        let count_x = refined_palette[i].1 as f32;
        let count_y = count as f32;

        let balanced_l = (lx * count_x + ly * count_y) / (count_x + count_y);
        let balanced_a = (ax * count_x + ay * count_y) / (count_x + count_y);
        let balanced_b = (bx * count_x + by * count_y) / (count_x + count_y);

        refined_palette[i].0 = Lab {
            l: balanced_l,
            a: balanced_a,
            b: balanced_b,
        };

        refined_palette[i].1 += count_y as usize;
    }

    refined_palette.sort_by(|&(_, a), &(_, b)| b.cmp(&a));

    refined_palette
}

/// Organises the produced color palette into something that's useful for a
/// user.
fn distil_palette(palette: Vec<(Lab, usize)>) -> Distil {
    let mut colors = Vec::new();
    let mut color_count = BTreeMap::new();

    for (i, &(lab_color, count)) in palette.iter().enumerate() {
        colors.push(lab_color.to_rgb());
        color_count.insert(i, count);
    }

    Distil {
        colors: colors,
        color_count: color_count,
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{Distil, DistilError};

    #[test]
    fn from_path_str() {
        let path_str = "/Users/elliot/dev/distil/images/img-1.jpg";

        match Distil::from_path_str(path_str) {
            Ok(distilled) => {
                distilled.as_img(&Path::new("img-1-palette.png"), 5);
            }
            Err(err) => {
                println!("{}", err);
            }
        }
    }

    #[test]
    fn from_path() {
        let path = Path::new("/Users/elliot/dev/distil/images/img-1.jpg");

        match Distil::from_path(&path) {
            Ok(distilled) => {
                distilled.as_img(&Path::new("img-1-palette.png"), 5);
            }
            Err(err) => {
                println!("{}", err);
            }
        }
    }

    #[test]
    fn pure_white() {
        let path = Path::new("/Users/elliot/dev/distil/tests/pure-white.png");
        let distilled_err = Distil::from_path(&path).unwrap_err();

        match distilled_err {
            DistilError::Uninteresting => assert!(true),
            _ => assert!(false),
        }
    }

    #[test]
    fn pure_black() {
        let path = Path::new("/Users/elliot/dev/distil/tests/pure-black.png");
        let distilled_err = Distil::from_path(&path).unwrap_err();

        match distilled_err {
            DistilError::Uninteresting => assert!(true),
            _ => assert!(false),
        }
    }

    #[test]
    fn unsupported_format() {
        let path = Path::new("/Users/elliot/dev/distil/tests/unsupported-format.gif");
        let distilled_err = Distil::from_path(&path).unwrap_err();

        match distilled_err {
            DistilError::UnsupportedFormat => assert!(true),
            _ => assert!(false),
        }
    }
}
