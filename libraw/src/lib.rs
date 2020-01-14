#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

#[macro_use]
extern crate quick_error;

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
    // TODO: figure out how getters work
    img_params: ImgParams,
}

#[derive(Debug)]
pub struct ImgParams {
    pub raw_height: u32,
    pub raw_width: u32,
    pub image_height: u32,
    pub image_width: u32,
}

type XTransPixelMap = [[i8; 6]; 6];

impl Drop for RawFile {
    fn drop(&mut self) {
        unsafe {
            libraw_close(self.libraw);
        }
    }
}

fn with_err_conversion<F>(f: F) -> Result<()>
where
    F: Fn() -> c_int,
{
    unsafe {
        match f() {
            0 => Ok(()),
            code => {
                let err = CStr::from_ptr(libraw_strerror(code));
                Err(RawError::GenericError(err.to_str().unwrap()))
            }
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

    pub fn xtrans_pixel_mapping(&self) -> &XTransPixelMap {
        unsafe { &(*self.libraw).idata.xtrans_abs }
    }

    pub fn get_jpeg_thumbnail(&self) -> &[u8] {
        // this is safe to call multiple times.
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

    pub fn load_raw_data(&self) -> &[u16] {
        with_err_conversion(|| unsafe { libraw_unpack(self.libraw) }).unwrap();
        let sizes = unsafe { (*self.libraw).sizes };
        // convert to u32 to avoid overflow
        let num_shorts = sizes.raw_width as usize * sizes.raw_height as usize;
        unsafe { slice::from_raw_parts((*self.libraw).rawdata.raw_image, num_shorts) }
    }
}
