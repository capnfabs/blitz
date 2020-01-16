#[macro_use]
extern crate clap;

use chrono::prelude::*;
use git2::Repository;
use image::ImageBuffer;
use std::env;
use std::fs::File;
use std::io::Write;

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
        "{0}/Downloads/render-{1}-rev{2}.jpg",
        home,
        utc.format("%F-%H%M%S"),
        &git_sha_descriptor()[..7],
    );
    let file = libraw::RawFile::open(matches.value_of("INPUT").unwrap()).unwrap();
    println!("Opened file: {:?}", file);
    dump_to_file(preview_filename, file.get_jpeg_thumbnail()).unwrap();
    let preview = render_raw_preview(&file);
    println!("Saving");
    preview.save(raw_preview_filename).unwrap();
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
const BITS_PER_PIXEL: u16 = 14;

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
                mapping,
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

enum Color {
    Red,
    Green,
    Blue,
}

impl Color {
    fn idx(&self) -> usize {
        match self {
            Color::Red => 0,
            Color::Green => 1,
            Color::Blue => 2,
        }
    }
    // TODO: make this generic
    fn from(val: i8) -> Option<Color> {
        match val {
            0 => Some(Color::Red),
            1 => Some(Color::Green),
            2 => Some(Color::Blue),
            _ => None,
        }
    }

    fn multipliers(&self) -> [u16; 3] {
        match self {
            Color::Red => [1, 0, 0],
            Color::Green => [0, 1, 0],
            Color::Blue => [0, 0, 1],
        }
    }
}

impl<'a> BlackValues<'a> {
    fn wrap(cdata: &'a libraw::libraw_colordata_t) -> BlackValues<'a> {
        BlackValues { cdata }
    }

    fn black_val(&self, x: u32, y: u32, color: &Color) -> u16 {
        let (black_width, black_height) = (self.cdata.cblack[4], self.cdata.cblack[5]);
        let (black_x, black_y) = (x % black_width, y % black_height);
        let idx = (black_y * (black_width) + black_x) as usize;
        (self.cdata.black + self.cdata.cblack[6 + idx] + self.cdata.cblack[color.idx()]) as u16
    }
}

fn render(
    x: u32,
    y: u32,
    width: u32,
    _height: u32,
    data: &[u16],
    mapping: &[[i8; 6]; 6],
    colors: &libraw::libraw_colordata_t,
) -> image::Rgb<u8> {
    let scale = 255.0 / (colors.maximum as f32);
    let black_values = BlackValues::wrap(colors);
    let idx = (y * (width as u32) + x) as usize;

    // TODO: 8 is the target per-channel size here, encode this with generics probably.
    let color = Color::from(mapping[x as usize % 6][y as usize % 6]).unwrap();
    let black = black_values.black_val(x, y, &color);
    let val = (data[idx] - black) as f32;
    //let cmap = colors.rgb_cam[color.idx()];
    let cmap = color.multipliers();
    image::Rgb([
        (val * cmap[0] as f32 * scale) as u8,
        (val * cmap[1] as f32 * scale) as u8,
        (val * cmap[2] as f32 * scale) as u8,
    ])
}
