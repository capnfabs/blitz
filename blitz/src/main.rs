#[macro_use]
extern crate clap;

use image::ImageBuffer;
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
    let raw_preview_filename = "/tmp/render.jpg";
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

fn dump_details(img: &libraw::RawFile) {
    let c = img.colordata();
    //println!("cblack {:?}", c.cblack);
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
    println!("baseline_exposure {:?}", c.baseline_exposure);
    //println!("WB_Coeffs {:?}", c.WB_Coeffs);
    //println!("WBCT_Coeffs {:?}", c.WBCT_Coeffs);
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
            // TODO: this should be a generic call to some kind of demosaic algorithm.
            let pixel = map_x_trans(
                x,
                y,
                sizes.raw_width as u32,
                sizes.raw_height as u32,
                img_data,
                mapping,
                &img.colordata().rgb_cam,
            );
            pixel
        },
    );
    println!("Done rendering");
    buf
}

fn map_x_trans(
    x: u32,
    y: u32,
    width: u32,
    _height: u32,
    data: &[u16],
    mapping: &[[i8; 6]; 6],
    rgb_cam: &[[f32; 4usize]; 3usize],
) -> image::Rgb<u8> {
    let idx = (y * (width as u32) + x) as usize;
    // TODO: 8 is the target per-channel size here, encode this with generics probably.
    let val = (data[idx] >> (BITS_PER_PIXEL - 8)) as f32;
    let color = mapping[x as usize % 6][y as usize % 6] as usize;
    // lo-fi matrix transpose
    let colors = [rgb_cam[0][color], rgb_cam[1][color], rgb_cam[2][color]];
    image::Rgb([
        (val * colors[0]) as u8,
        (val * colors[1]) as u8,
        (val * colors[2]) as u8,
    ])
}
