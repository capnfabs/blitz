use chrono::prelude::*;
use clap::{App, Arg};
use directories::UserDirs;
use git2::Repository;
use image::{ImageBuffer, ImageFormat};
use itertools::Itertools;
use libraw::raf::{ParsedRafFile, RafFile};
use libraw::util::datagrid::{DataGrid, Offset, Position, Size};
use libraw::Color;
use num_traits::{Num, Unsigned};
use ordered_float::NotNan;
use std::cmp::min;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let matches = App::new("Blitz")
        .arg(Arg::with_name("render").short("r").long("render"))
        .arg(Arg::with_name("INPUT").required(true).index(1))
        .get_matches();

    let input = matches.value_of("INPUT").unwrap();
    let render = matches.occurrences_of("render") == 1;

    load_and_maybe_render_native(input, render);
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

fn render_raw_preview_native(img: &ParsedRafFile) -> image::RgbImage {
    let img = &img.render_info();

    // Change 14 bit to 16 bit.
    //let img_data: Vec<u16> = img_data.iter().copied().map(|v| v << 2).collect();

    let mapping = DataGrid::wrap(&img.xtrans_mapping, Size(6, 6));

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

    let img_grid = DataGrid::wrap(&img_data, Size(img.width as usize, img.height as usize));

    let _demosaic = |x: u16, y: u16| -> Pixel<u16> {
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
        saturating_scale(_demosaic(x as u16, y as u16), &scale_factors).to_rgb()
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
    let Position(a, b) = pos;
    if a == 0 || b == 0 {
        // The edges are kinda messed up, so just return the original position
        pos
    } else {
        panic!("Shouldn't get here")
    }
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
