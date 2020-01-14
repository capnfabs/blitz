#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

#[macro_use]
extern crate quick_error;

use std::path::Path;
use libraw_sys::*;
use std::ffi::CString;
use std::slice;

quick_error! {
    #[derive(Debug)]
    pub enum RawError {
        NotFound {}
    }
}

type Result<T> = std::result::Result<T, RawError>;

#[derive(Debug)]
pub struct RawFile {
    libraw: * mut libraw_data_t,
    img_params: ImgParams,
}

#[derive(Debug)]
struct ImgParams {
    rheight: u32,
    rwidth: u32,
    iheight: u32,
    iwidth: u32,
}

impl Drop for RawFile {
    fn drop(&mut self) {
        unsafe {
            libraw_close(self.libraw);
        }
    }
}


impl RawFile {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<RawFile> {
        // TODO: handle null (represents an error)
        let libraw = unsafe {
            libraw_init(0)
        };
        // TODO: this is only legit if Path is valid unicode.
        let img_path = path.as_ref().to_str().unwrap();
        let cstr = CString::new(img_path).unwrap();
        let code = unsafe {
            libraw_open_file(libraw, cstr.as_ptr())
        };

        let (rheight, rwidth, iheight, iwidth) = unsafe {
            (
                (libraw_get_raw_height(libraw) as u32),
                (libraw_get_raw_width(libraw) as u32),
                (libraw_get_iheight(libraw) as u32),
                (libraw_get_iwidth(libraw) as u32),
            )
        };

        // TODO: check code
        Ok(RawFile {
            libraw,
            // TODO: maybe this can be loaded directly from the thingy?
            img_params: ImgParams {
                iheight,
                iwidth,
                rheight,
                rwidth,
            },
        })
    }

    pub fn get_jpeg_thumbnail(&self) -> &[u8] {
        // this is safe to call multiple times.
        let error = unsafe {
            libraw_unpack_thumb(self.libraw)
        };
        // TODO check code
        let thumb = unsafe {
            (*self.libraw).thumbnail
        };

        let format_code = &thumb.tformat;
        match *format_code {
            LibRaw_thumbnail_formats_LIBRAW_THUMBNAIL_JPEG => (),
            _ => panic!("Expected JPEG thumbnail format"),
        }
        println!("size: {}", thumb.tlength);

        unsafe {
            slice::from_raw_parts(thumb.thumb as *const u8, thumb.tlength as usize)
        }
    }
}
