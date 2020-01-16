#[macro_use]
extern crate clap;

use chrono::prelude::*;
use git2::Repository;
use image::{ImageBuffer, ImageFormat};
use itertools::izip;
use libraw::{Color, XTransPixelMap};
use std::fs::File;
use std::io::Write;
use std::{env, fs};

fn main() {
    let matches = clap_app!(blitz =>
        (version: "1.0")
        (author: "Fabian Tamp (https://capnfabs.net/contact)")
        (about: "Does awesome things")
        (@arg CONFIG: -c --config +takes_value "Sets a custom config file")
        (@arg INPUT: +required "Sets the input file to use")
        (@arg debug: -d ... "Sets the level of debugging information")
        (@subcommand test =>
            (about: "controls testing features")
            (version: "1.3")
            (author: "Someone E. <someone_else@other.com>")
            (@arg verbose: -v --verbose "Print test information verbosely")
        )
    )
    .get_matches();

    let preview_filename = "/tmp/thumb.jpg";
    let home = env::var("HOME").unwrap();
    let utc: DateTime<Utc> = Utc::now();
    let raw_preview_filename = &format!(
        "{0}/Downloads/render-{1}-rev{2}.tiff",
        home,
        utc.format("%F-%H%M%S"),
        &git_sha_descriptor()[..7],
    );
    let file = libraw::RawFile::open(matches.value_of("INPUT").unwrap()).unwrap();
    println!("Opened file: {:?}", file);
    dump_to_file(preview_filename, file.get_jpeg_thumbnail()).unwrap();
    let preview = render_raw_preview(&file);
    println!("Saving");
    preview
        .save_with_format(raw_preview_filename, ImageFormat::TIFF)
        .unwrap();
    let metadata = fs::metadata(raw_preview_filename).unwrap();
    // Set readonly so that I don't accidentally save over it later.
    let mut p = metadata.permissions();
    p.set_readonly(true);
    fs::set_permissions(raw_preview_filename, p).unwrap();
    println!("Done saving");
    dump_details(&file);
    open_preview(raw_preview_filename)
}

fn large_array_str<T>(array: &[T]) -> String
where
    T: ToString,
{
    array
        .into_iter()
        .map(|x| x.to_string())
        .collect::<Vec<String>>()
        .join(",")
}

fn git_sha_descriptor() -> String {
    let exepath = std::env::current_exe().unwrap();
    let repo = match Repository::discover(exepath.parent().unwrap()) {
        Ok(repo) => repo,
        Err(e) => panic!("failed to open: {}", e),
    };
    let head = match repo.head() {
        Ok(val) => val,
        Err(e) => panic!(e),
    };
    let commit = head.peel_to_commit().unwrap();
    commit.id().to_string()
}

fn dump_details(img: &libraw::RawFile) {
    let c = img.colordata();

    //println!("curve [{}]", large_array_str(&c.curve));
    // https://github.com/LibRaw/LibRaw/blob/master/src/preprocessing/subtract_black.cpp
    //println!("cblack [{}]", large_array_str(&c.cblack));
    println!("black {:?}", c.black);
    println!("data_maximum {:?}", c.data_maximum);
    println!("maximum {:?}", c.maximum);
    println!("linear_max {:?}", c.linear_max);
    println!("fmaximum {:?}", c.fmaximum);
    println!("fnorm {:?}", c.fnorm);
    println!("white {:?}", c.white);
    // white balance coeefficients, e.g. [584.0, 302.0, 546.0, 0.0]
    println!("cam_mul {:?}", c.cam_mul);
    //"White balance coefficients for daylight (daylight balance). Either read from file, or calculated on the basis of file data, or taken from hardcoded constants."
    println!("pre_mul {:?}", c.pre_mul);
    println!("cmatrix {:?}", c.cmatrix);
    println!("ccm {:?}", c.ccm);
    println!("rgb_cam {:?}", c.rgb_cam);
    println!("cam_xyz {:?}", c.cam_xyz);
    println!("flash_used {:?}", c.flash_used);
    println!("canon_ev {:?}", c.canon_ev);
    //println!("model2 {:?}", c.model2);
    //println!("UniqueCameraModel {:?}", c.UniqueCameraModel);
    //println!("LocalizedCameraModel {:?}", c.LocalizedCameraModel);
    println!("profile {:?}", c.profile);
    println!("profile_length {:?}", c.profile_length);
    println!("black_stat {:?}", c.black_stat);
    //pub dng_color: [libraw_dng_color_t; 2usize],
    //pub dng_levels: libraw_dng_levels_t,
    println!("baseline_exposure {:?}", c.baseline_exposure);
    //println!("WB_Coeffs {:?}", c.WB_Coeffs);
    //println!("WBCT_Coeffs {:?}", c.WBCT_Coeffs);
    // phase1?
    //pub P1_color: [libraw_P1_color_t; 2usize],
}

fn dump_to_file(filename: &str, data: &[u8]) -> std::io::Result<()> {
    let mut file = File::create(filename)?;
    println!("Writing {} bytes to {}", data.len(), filename);
    file.write_all(data)?;
    Ok(())
}

fn open_preview(filename: &str) {
    use std::process::Command;

    Command::new("open")
        .arg(filename)
        .spawn()
        .expect("Failed to start");
}

const DBG_CROP_FACTOR: u32 = 1;

fn render_raw_preview(img: &libraw::RawFile) -> image::RgbImage {
    let sizes = img.img_params();
    println!("Loading RAW data");
    let img_data = img.load_raw_data();
    println!("Done loading; rendering");
    let mapping = img.xtrans_pixel_mapping();

    let buf = ImageBuffer::from_fn(
        sizes.raw_width as u32 / DBG_CROP_FACTOR,
        sizes.raw_height as u32 / DBG_CROP_FACTOR,
        |x, y| {
            render(
                x,
                y,
                sizes.raw_width as u32,
                sizes.raw_height as u32,
                img_data,
                &mapping,
                img.colordata(),
            )
        },
    );
    println!("Done rendering");
    buf
}

struct BlackValues<'a> {
    cdata: &'a libraw::libraw_colordata_t,
}

impl<'a> BlackValues<'a> {
    fn wrap(cdata: &'a libraw::libraw_colordata_t) -> BlackValues<'a> {
        BlackValues { cdata }
    }

    fn black_val(&self, x: u32, y: u32, color: libraw::Color) -> u16 {
        let (black_width, black_height) = (self.cdata.cblack[4], self.cdata.cblack[5]);
        let (black_x, black_y) = (x % black_width, y % black_height);
        let idx = (black_y * (black_width) + black_x) as usize;
        (self.cdata.black + self.cdata.cblack[6 + idx] + self.cdata.cblack[color.idx()]) as u16
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
struct Offset {
    x: i8,
    y: i8,
}

const CHECK_ORDER: [Offset; 5] = [
    Offset { x: 0, y: 0 },
    Offset { x: 0, y: 1 },
    Offset { x: 1, y: 0 },
    Offset { x: -1, y: 0 },
    Offset { x: 0, y: -1 },
];

fn offset_for_color(mapping: &XTransPixelMap, color: Color, x: usize, y: usize) -> Offset {
    for offset in CHECK_ORDER.iter() {
        if mapping[((x as i32 + offset.x as i32).rem_euclid(6)) as usize]
            [((y as i32 + offset.y as i32).rem_euclid(6)) as usize]
            == color
        {
            return offset.clone();
        }
    }
    panic!("Shouldn't get here")
}

// Returns one offset per color.
fn find_offsets(mapping: &XTransPixelMap, x: usize, y: usize) -> [Offset; 3] {
    // Ok so, every pixel has every color within one of the offsets from it.
    // This doesn't apply on edges but we're going to just ignore edges until we
    // figure out if the basic technique works.
    [
        offset_for_color(mapping, Color::Red, x, y),
        offset_for_color(mapping, Color::Green, x, y),
        offset_for_color(mapping, Color::Blue, x, y),
    ]
}

fn pixel_idx(x: u32, y: u32, width: u32, height: u32, offset: Offset) -> usize {
    let mut offset_y = y as i32 + offset.y as i32;
    if offset_y < 0 {
        offset_y = 0;
    }
    let mut offset_x = x as i32 + offset.x as i32;
    if offset_x < 0 {
        offset_x = 0;
    }
    let idx = (offset_y as u32 * (width as u32) + offset_x as u32) as usize;
    if idx < (width * height) as usize {
        idx
    } else {
        0
    }
}

fn render(
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    data: &[u16],
    mapping: &libraw::XTransPixelMap,
    colors: &libraw::libraw_colordata_t,
) -> image::Rgb<u8> {
    let scale = 255.0 / (colors.maximum as f32);
    let black_values = BlackValues::wrap(colors);

    //let black = black_values.black_val(x, y, color);
    let offsets = find_offsets(mapping, x as usize, y as usize);
    let r_idx = pixel_idx(x, y, width, height, offsets[Color::Red.idx()]);
    let g_idx = pixel_idx(x, y, width, height, offsets[Color::Green.idx()]);
    let b_idx = pixel_idx(x, y, width, height, offsets[Color::Blue.idx()]);
    // TODO: Skipping black subtraction for now
    // TODO: this is a matrix multiplication, but I don't want to deal with that right now.
    let r_contrib: Vec<f32> = colors.rgb_cam[Color::Red.idx()]
        .iter()
        .map(|x| x * data[r_idx] as f32)
        .collect();
    let g_contrib: Vec<f32> = colors.rgb_cam[Color::Green.idx()]
        .iter()
        .map(|x| x * data[g_idx] as f32)
        .collect();
    let b_contrib: Vec<f32> = colors.rgb_cam[Color::Blue.idx()]
        .iter()
        .map(|x| x * data[b_idx] as f32)
        .collect();

    let vals: Vec<f32> = izip!(r_contrib, g_contrib, b_contrib)
        .map(|(r, g, b)| r + g + b)
        .collect();

    //let cmap = color.multipliers();
    image::Rgb([
        (vals[0] * scale) as u8,
        (vals[1] * scale) as u8,
        (vals[2] * scale) as u8,
    ])
}
