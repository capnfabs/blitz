use chrono::prelude::*;
use clap::{App, Arg};
use directories::UserDirs;
use git2::Repository;
use image::{ImageBuffer, ImageFormat};
use itertools::Itertools;
use libraw::raf::{ParsedRafFile, RafFile};
use libraw::util::{DataGrid, Grid, Offset, Position, Size};
use libraw::{util, Color, RawFile};
use num_traits::{Num, Unsigned};
use ordered_float::NotNan;
use std::cmp::min;
use std::collections::HashSet;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

fn main() {
    let matches = App::new("Blitz")
        .arg(Arg::with_name("render").short("r").long("render"))
        .arg(Arg::with_name("INPUT").required(true).index(1))
        .get_matches();

    let input = matches.value_of("INPUT").unwrap();
    let render = matches.occurrences_of("render") == 1;

    load_and_maybe_render_native(input, render);
    load_and_maybe_render_libraw(input, render);
}

fn load_and_maybe_render_native(img_file: &str, render: bool) {
    println!("Loading RAW data: native");
    let file = RafFile::open(img_file).unwrap();
    println!("Opened file: {:?}", file);
    println!("Parsing...");
    let details = file.parse_raw().unwrap();

    println!("Parsed.");

    if !render {
        return;
    }

    let raw_preview_filename = get_output_path("native");
    let preview = render_raw_preview_native(&details);
    println!("Saving");
    preview
        .save_with_format(&raw_preview_filename, ImageFormat::TIFF)
        .unwrap();
    let metadata = fs::metadata(&raw_preview_filename).unwrap();
    // Set readonly so that I don't accidentally save over it later.
    let mut p = metadata.permissions();
    p.set_readonly(true);
    fs::set_permissions(&raw_preview_filename, p).unwrap();
    println!("Done saving");
    open_preview(&raw_preview_filename);
}

fn load_and_maybe_render_libraw(img_file: &str, render: bool) {
    println!("Loading RAW data: libraw");
    let file = RawFile::open(img_file).unwrap();
    println!("Opened file: {:?}", file);
    dump_details(&file);

    if !render {
        return;
    }
    let raw_preview_filename = get_output_path("libraw");
    println!("Rendering...");
    let preview = render_raw_preview(&file);
    println!("Saving");
    preview
        .save_with_format(&raw_preview_filename, ImageFormat::TIFF)
        .unwrap();
    let metadata = fs::metadata(&raw_preview_filename).unwrap();
    // Set readonly so that I don't accidentally save over it later.
    let mut p = metadata.permissions();
    p.set_readonly(true);
    fs::set_permissions(&raw_preview_filename, p).unwrap();
    println!("Done saving");
    open_preview(&raw_preview_filename);
}

fn get_output_path(label: &str) -> PathBuf {
    let ud = UserDirs::new().unwrap();
    let download_dir = ud.download_dir().unwrap();
    let utc: DateTime<Utc> = Utc::now();
    let filename = format!(
        "render-{0}-rev{1}-{2}.tiff",
        utc.format("%F-%H%M%S"),
        &git_sha_descriptor()[..7],
        label,
    );
    download_dir.join(filename)
}

#[allow(dead_code)]
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
    // [0,0,0,0,6,6,1022,1022,1022,1022,1022,1022,1022,1022,1022,1022,1022,1022,1022,1022,1022,1022,1022,1022,1022,1022,1022,1022,1022,1022,1022,1022,1022,1022,1022,1022,1022,1022,1022,1022,1022,1022,0,0,0....]
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

fn open_preview<P>(filename: P)
where
    P: AsRef<Path>,
{
    use std::process::Command;

    Command::new("open")
        .arg(filename.as_ref().as_os_str())
        .spawn()
        .expect("Failed to start");
}

const DBG_CROP_FACTOR: u32 = 1;

struct Pixel<U>
where
    U: Num + Unsigned,
{
    red: U,
    green: U,
    blue: U,
}

impl Pixel<u16> {
    fn to_rgb(&self) -> image::Rgb<u8> {
        image::Rgb([
            (self.red >> 8) as u8,
            (self.green >> 8) as u8,
            (self.blue >> 8) as u8,
        ])
    }
}

#[allow(dead_code)]
fn only<T>(p: Pixel<T>, color: Color) -> Pixel<T>
where
    T: Unsigned + Num,
{
    match color {
        Color::Red => Pixel {
            red: p.red,
            green: T::zero(),
            blue: T::zero(),
        },
        Color::Green => Pixel {
            red: T::zero(),
            green: p.green,
            blue: T::zero(),
        },
        Color::Blue => Pixel {
            red: T::zero(),
            green: T::zero(),
            blue: p.blue,
        },
    }
}

type Axis = u32;
type Value = u16;

struct ImageLayoutIterator<'a> {
    width: Axis,
    height: Axis,
    pos: usize,
    data: &'a [Value],
}

impl<'a> Iterator for ImageLayoutIterator<'a> {
    type Item = (Axis, Axis, u16);
    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= (self.width * self.height) as usize {
            return None;
        }
        let x = self.pos as Axis % self.width;
        let y = self.pos as Axis / self.width;
        let result = self.data[self.pos];
        self.pos += 1;
        Some((x, y, result))
    }
}

#[allow(dead_code)] // Candidate for inclusion later.
fn enumerate_xy(width: Axis, height: Axis, data: &[Value]) -> ImageLayoutIterator {
    ImageLayoutIterator {
        width,
        height,
        pos: 0,
        data,
    }
}

fn render_raw_preview(img: &libraw::RawFile) -> image::RgbImage {
    let img_data = img.raw_data();

    // Change 14 bit to 16 bit.
    //let img_data: Vec<u16> = img_data.iter().copied().map(|v| v << 2).collect();

    let mapping = img.xtrans_pixel_mapping();
    let width = img.img_params().raw_width as usize;
    let height = img.img_params().raw_height as usize;

    let black_vals = BlackValues::wrap(img.colordata());

    let black_sub = |val: u16| -> u16 { val.saturating_sub(black_vals.black_val()) };

    let img_data: Vec<u16> = img_data.iter().copied().map(|v| black_sub(v)).collect();

    // hot pixel elimination through a hard-coded filter lol
    let max = img_data
        .iter()
        // TODO: this is hardcoded!
        .filter(|v| **v < 6000)
        .copied()
        .max()
        .unwrap();

    let mapping: Vec<Color> = mapping.iter().flatten().copied().collect_vec();
    let mapping = util::wrap(&mapping, Size(6, 6));
    let img_grid = util::wrap(&img_data, Size(width, height));
    let demosaic = |x: u32, y: u32| -> Pixel<u16> {
        let x = x as usize;
        let y = y as usize;
        let offsets = find_offsets_native(&mapping, Position(x, y));
        Pixel {
            red: img_grid.at(offsets[Color::Red.idx()]),
            green: img_grid.at(offsets[Color::Green.idx()]),
            blue: img_grid.at(offsets[Color::Blue.idx()]),
        }
    };

    let _passthru_demosaic = |x: Axis, y: Axis| -> Pixel<u16> {
        let pos = Position(x as usize, y as usize);
        let v = img_grid.at(pos);
        let color = mapping.at(pos);
        match color {
            Color::Red => Pixel {
                red: v,
                blue: 0,
                green: 0,
            },
            Color::Green => Pixel {
                red: 0,
                blue: 0,
                green: v,
            },
            Color::Blue => Pixel {
                red: 0,
                blue: v,
                green: 0,
            },
        }
    };

    // Compute scaling params
    println!("Overall max: {:}", max);
    // MINS   825   882   831
    // MAXS  4579 13556  4491
    // This is int scaling, so it'll be pretty crude (e.g. Green will only scale 4x, not 4.5x)
    // Camera scaling factors are 773, 302, 412. They are theoretically white balance but I don't know
    // how they work.

    // Let's do some WB.
    let cam_mul = img.colordata().cam_mul;
    let scale_factors = make_normalized_wb_coefs(cam_mul);
    println!("scale_factors: {:?}", scale_factors);
    let scale_factors: Vec<f32> = scale_factors
        .iter()
        .map(|val| val * (std::u16::MAX as f32) / max as f32)
        .collect();
    println!("scale_factors: {:?}", scale_factors);
    let scale_factors: Vec<u16> = scale_factors.iter().copied().map(|v| v as u16).collect();
    println!("scale_factors: {:?}", scale_factors);

    let buf = ImageBuffer::from_fn(
        img.img_params().raw_width / DBG_CROP_FACTOR,
        img.img_params().raw_height / DBG_CROP_FACTOR,
        |x, y| saturating_scale(_passthru_demosaic(x, y), &scale_factors).to_rgb(),
    );
    println!("Done rendering");
    buf
}

fn render_raw_preview_native(img: &ParsedRafFile) -> image::RgbImage {
    let img = &img.render_info();

    // Change 14 bit to 16 bit.
    //let img_data: Vec<u16> = img_data.iter().copied().map(|v| v << 2).collect();

    let mapping = util::wrap(&img.xtrans_mapping, Size(6, 6));

    // Should fix this lol
    let black_sub = |val: u16| -> u16 { val.saturating_sub(1022) };

    let img_data: Vec<u16> = img.raw_data.iter().copied().map(|v| black_sub(v)).collect();

    // hot pixel elimination through a hard-coded filter lol
    let max = img_data
        .iter()
        // TODO: this is hardcoded!
        .filter(|v| **v < 6000)
        .copied()
        .max()
        .unwrap();

    let img_grid = util::wrap(&img_data, Size(img.width as usize, img.height as usize));

    let _demosaic = |x: u32, y: u32| -> Pixel<u16> {
        let x = x as usize;
        let y = y as usize;
        let pixel = Position(x, y);
        let offsets = find_offsets_native(&mapping, pixel);
        Pixel {
            red: img_grid.at(offsets[Color::Red.idx()]),
            green: img_grid.at(offsets[Color::Green.idx()]),
            blue: img_grid.at(offsets[Color::Blue.idx()]),
        }
    };

    let _passthru_demosaic = |x: u16, y: u16| -> Pixel<u16> {
        let pos = Position(x as usize, y as usize);
        let v = img_grid.at(pos);
        let color = mapping.at(pos);
        match color {
            Color::Red => Pixel {
                red: v,
                blue: 0,
                green: 0,
            },
            Color::Green => Pixel {
                red: 0,
                blue: 0,
                green: v,
            },
            Color::Blue => Pixel {
                red: 0,
                blue: v,
                green: 0,
            },
        }
    };

    // Compute scaling params
    println!("Overall max: {:}", max);
    // This is int scaling, so it'll be pretty crude (e.g. Green will only scale 4x, not 4.5x)
    // Camera scaling factors are 773, 302, 412. They are theoretically white balance but I don't know
    // how they work.

    // Let's do some WB.
    let wb = img.white_bal;
    let scale_factors =
        make_normalized_wb_coefs([wb.red as f32, wb.green as f32, wb.blue as f32, 0.0]);
    println!("scale_factors: {:?}", scale_factors);
    let scale_factors: Vec<f32> = scale_factors
        .iter()
        .map(|val| val * (std::u16::MAX as f32) / max as f32)
        .collect();
    println!("scale_factors: {:?}", scale_factors);
    let scale_factors: Vec<u16> = scale_factors.iter().copied().map(|v| v as u16).collect();
    println!("scale_factors: {:?}", scale_factors);

    let buf = ImageBuffer::from_fn(img.width as u32, img.height as u32, |x, y| {
        saturating_scale(_passthru_demosaic(x as u16, y as u16), &scale_factors).to_rgb()
    });
    println!("Done rendering");
    buf
}

fn saturating_scale(p: Pixel<u16>, scale_factors: &[u16]) -> Pixel<u16> {
    Pixel {
        red: min(p.red as u32 * scale_factors[0] as u32, std::u16::MAX as u32) as u16,
        green: min(
            p.green as u32 * scale_factors[1] as u32,
            std::u16::MAX as u32,
        ) as u16,
        blue: min(
            p.blue as u32 * scale_factors[2] as u32,
            std::u16::MAX as u32,
        ) as u16,
    }
}

struct BlackValues {
    black: u16,
}

impl BlackValues {
    fn wrap(cdata: &libraw::libraw_colordata_t) -> BlackValues {
        // Check black levels are all the same for the optimised version
        let (black_width, black_height) = (cdata.cblack[4] as usize, cdata.cblack[5] as usize);
        let distinct_black_levels: HashSet<u32> = cdata.cblack[6..(6 + black_width * black_height)]
            .iter()
            .copied()
            .collect();
        assert_eq!(distinct_black_levels.len(), 1);
        let distinct_black_levels: HashSet<u32> = cdata.cblack[0..4].iter().copied().collect();
        assert_eq!(distinct_black_levels.len(), 1);
        let black = (cdata.cblack[0] + cdata.cblack[6]) as u16;
        BlackValues { black }
    }

    fn black_val(&self) -> u16 {
        self.black
    }
}

const CHECK_ORDER: [Offset; 5] = [
    Offset(0, 0),
    Offset(0, 1),
    Offset(1, 0),
    Offset(-1, 0),
    Offset(0, -1),
];

fn offset_for_color_native(mapping: &DataGrid<Color>, color: Color, pos: Position) -> Position {
    for candidate_pos in CHECK_ORDER.iter().map(|offset| pos + *offset) {
        if mapping.at(candidate_pos) == color {
            return candidate_pos;
        }
    }
    panic!("Shouldn't get here")
}

fn find_offsets_native(mapping: &DataGrid<Color>, pos: Position) -> [Position; 3] {
    // Ok so, every pixel has every color within one of the offsets from it.
    // This doesn't apply on edges but we're going to just ignore edges until we
    // figure out if the basic technique works.
    [
        offset_for_color_native(mapping, Color::Red, pos),
        offset_for_color_native(mapping, Color::Green, pos),
        offset_for_color_native(mapping, Color::Blue, pos),
    ]
}

/// Returns whitebalance coefficients normalized such that the smallest is 1
fn make_normalized_wb_coefs(coefs: [f32; 4]) -> [f32; 3] {
    println!("coefs {:?}", coefs);
    let minval = coefs
        .iter()
        .cloned()
        .filter(|v| *v != 0.0)
        .map_into::<NotNan<f32>>()
        .min()
        .unwrap()
        .into_inner();
    println!("coefs min {:?}", minval);
    [coefs[0] / minval, coefs[1] / minval, coefs[2] / minval]
}

#[allow(dead_code)]
fn saturating_downcast(val: f32) -> u8 {
    if val.is_sign_negative() {
        0
    } else if (val as u16) > std::u8::MAX as u16 {
        255
    } else {
        val as u8
    }
}

#[allow(dead_code)] // Debug method
fn dump_sample(label: &str, pixels: Vec<Pixel<u16>>) {
    // these are different from the C++
    let width = 6048;
    let _height = 4038;
    let crop_width = 512;
    let crop_height = 512;
    let start_col = 3834;
    let start_row = 1168 + 6;

    let filename = format!("/tmp/rust_{}.ppm", label);
    let mut file = File::create(filename).unwrap();
    write!(file, "P3\n{} {}\n16384\n", crop_width, crop_height).unwrap();
    for row in start_row..(start_row + crop_height) {
        for col in start_col..(start_col + crop_width) {
            let pixel = &pixels[row * width + col];
            write!(file, "{} {} {}\n", pixel.red, pixel.green, pixel.blue).unwrap();
        }
    }
}
