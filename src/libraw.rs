use libraw_sys::*;
use std::ffi::{CString, CStr};
use std::slice;
use libc::c_int;

#[derive(Debug)]
pub struct RawFile {
    filename: CString,
    libraw: *mut libraw_data_t,
}

fn open_file(libraw: *mut libraw_data_t, filename: &CString) -> Result<(), & 'static str> {
    with_err_conversion(|| unsafe {
        libraw_open_file(libraw, filename.as_ptr())
    })
}

fn unpack_thumb(libraw: *mut libraw_data_t) -> Result<(), & 'static str> {
    with_err_conversion(|| unsafe {
        libraw_unpack_thumb(libraw)
    })
}

fn with_err_conversion<F>(f: F) -> Result<(), &'static str> where
    F: Fn() -> c_int {
        unsafe {
            match f() {
                0 => Ok(()),
                code => {
                let err = CStr::from_ptr(libraw_strerror(code));
                Err(err.to_str().unwrap())
            }
            }
        }
    }

impl RawFile {
    pub fn new(filename: String) -> Result<RawFile, & 'static str> {
        unsafe {
            let libraw = libraw_init(0);
            let cstr = CString::new(filename.clone()).unwrap();
            open_file(libraw, &cstr)?;
            Ok(RawFile { filename: cstr, libraw })
        }
    }

    pub fn get_jpeg_thumbnail(&self) -> &[u8] {
         unsafe {
            unpack_thumb(self.libraw).unwrap();
            let thumb = &(*self.libraw).thumbnail;
            let format_code = &thumb.tformat;
            match format_code {
                LibRaw_thumbnail_formats::LIBRAW_THUMBNAIL_JPEG => (),
                _ => panic!("Expected JPEG thumbnail format"),
            }
            println!("size: {}", thumb.tlength);
            slice::from_raw_parts(thumb.thumb as *mut u8, thumb.tlength as usize)
        }
    }
}

impl Drop for RawFile {
    fn drop(&mut self) {
        println!("Dropping!");
        unsafe {
            // TODO: unclear if we need libraw_recycle here too.
            libraw_close(self.libraw);
        }
    }
}
