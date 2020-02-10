mod fuji_compressed;
pub mod raf;
mod tiff;
pub mod util;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

#[macro_use]
extern crate quick_error;

#[macro_use]
extern crate derivative;

#[macro_use]
extern crate data_encoding_macro;

pub use libraw_sys::libraw_colordata_t;

use libraw_sys::*;
use std::ffi::{CStr, CString};
use std::os::raw::c_int;
use std::path::Path;
use std::slice;

quick_error! {
    #[derive(Debug)]
    pub enum RawError {
        NotFound {}
        GenericError(message: &'static str) {}
        LibraryError {}
    }
}

type Result<T> = std::result::Result<T, RawError>;

#[derive(Debug)]
pub struct RawFile {
    libraw: *mut libraw_data_t,

    img_params: ImgParams,
}

#[derive(Debug)]
pub struct ImgParams {
    pub raw_height: u32,
    pub raw_width: u32,
    pub image_height: u32,
    pub image_width: u32,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Color {
    Red,
    Green,
    Blue,
}

impl Color {
    pub fn idx(&self) -> usize {
        match self {
            Color::Red => 0,
            Color::Green => 1,
            Color::Blue => 2,
        }
    }
    // TODO: make this generic in numbers
    pub fn from(val: i8) -> Option<Color> {
        match val {
            0 => Some(Color::Red),
            1 => Some(Color::Green),
            2 => Some(Color::Blue),
            _ => None,
        }
    }

    // TODO does this belong here?
    pub fn multipliers(&self) -> [u16; 3] {
        match self {
            Color::Red => [1, 0, 0],
            Color::Green => [0, 1, 0],
            Color::Blue => [0, 0, 1],
        }
    }
}

#[allow(dead_code)] // We'll use this eventually instead of the other thing
struct XTransMapping<'a> {
    data: &'a [u8],
    sensor_width: usize,
}

#[allow(dead_code)]
impl<'a> XTransMapping<'a> {
    pub fn color_at(&self, x: usize, y: usize) -> Color {
        Color::from(self.data[y * self.sensor_width + x] as i8).unwrap()
    }
}

pub type XTransPixelMap = [[Color; 6]; 6];

impl Drop for RawFile {
    fn drop(&mut self) {
        unsafe {
            libraw_close(self.libraw);
        }
    }
}

fn with_err_conversion<F>(mut f: F) -> Result<()>
where
    F: FnMut() -> c_int,
{
    match f() {
        0 => Ok(()),
        code => {
            // safe because these are all constants in the libraw library.
            let err = unsafe { CStr::from_ptr(libraw_strerror(code)) };
            Err(RawError::GenericError(err.to_str().unwrap()))
        }
    }
}

// TODO: the error-handling story in general is pretty bad - libraw has a bunch
// of errors that mean "I gave up and your object is no longer valid" and we
// don't handle those super well. See https://www.libraw.org/docs/API-datastruct.html#LibRaw_errors
// TODO: some of these methods probably need to receive &mut self.
impl RawFile {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<RawFile> {
        let libraw = unsafe { libraw_init(0) };
        if libraw.is_null() {
            return Err(RawError::LibraryError);
        }
        // TODO: this is only legit if Path is valid unicode.
        let img_path = path.as_ref().to_str().unwrap();
        let cstr = CString::new(img_path).unwrap();
        with_err_conversion(|| unsafe { libraw_open_file(libraw, cstr.as_ptr()) })?;

        // this is _not_ safe to call multiple times so just do it once at init 🙃
        with_err_conversion(|| unsafe { libraw_unpack(libraw) }).unwrap();

        let (rheight, rwidth, iheight, iwidth) = unsafe {
            (
                (libraw_get_raw_height(libraw) as u32),
                (libraw_get_raw_width(libraw) as u32),
                (libraw_get_iheight(libraw) as u32),
                (libraw_get_iwidth(libraw) as u32),
            )
        };

        Ok(RawFile {
            libraw,
            // TODO: maybe this can be loaded directly from the pointer
            img_params: ImgParams {
                image_height: iheight,
                image_width: iwidth,
                raw_height: rheight,
                raw_width: rwidth,
            },
        })
    }

    pub fn img_params(&self) -> &ImgParams {
        &self.img_params
    }

    // maybe TODO: we definitely don't need to create a new one of these every
    // time, we can have XTransPixelMap just wrap the existing struct with a
    // lifetime equal to that of the RawFile.
    pub fn xtrans_pixel_mapping(&self) -> XTransPixelMap {
        let orig = unsafe { &(*self.libraw).idata.xtrans_abs };
        let mut colors = [[Color::Red; 6]; 6];
        for r in 0..6 {
            for c in 0..6 {
                colors[r][c] = Color::from(orig[r][c]).unwrap();
            }
        }
        colors
    }

    pub fn colordata(&self) -> &libraw_colordata_t {
        unsafe { &(*self.libraw).color }
    }

    pub fn get_jpeg_thumbnail(&self) -> &[u8] {
        // this is _not_ safe to call multiple times.
        // TODO: fix this / figure out an API.
        with_err_conversion(|| unsafe { libraw_unpack_thumb(self.libraw) }).unwrap();
        let thumb = unsafe { (*self.libraw).thumbnail };

        let format_code = &thumb.tformat;

        #[allow(non_upper_case_globals)]
        match *format_code {
            LibRaw_thumbnail_formats_LIBRAW_THUMBNAIL_JPEG => (),
            _ => panic!("Expected JPEG thumbnail format"),
        }
        println!("size: {}", thumb.tlength);

        unsafe { slice::from_raw_parts(thumb.thumb as *const u8, thumb.tlength as usize) }
    }

    pub fn raw_data(&self) -> &[u16] {
        let sizes = unsafe { (*self.libraw) }.rawdata.sizes;
        let num_shorts = sizes.raw_width as usize * sizes.raw_height as usize;
        unsafe { slice::from_raw_parts((*self.libraw).rawdata.raw_image, num_shorts) }
    }
}
