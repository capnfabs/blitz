use libraw_sys::*;
use std::ffi::{CString, CStr};
use std::slice;
use libc::c_int;
use image::{GenericImage, ImageBuffer};

#[derive(Debug)]
pub struct RawFile {
    filename: CString,
    libraw: *mut libraw_data_t,
}

/// TODO: document. Should this be usize?
const DBG_CROP_FACTOR: u32 = 1;
// can't remember if this is right or not.
const BITS_PER_PIXEL: usize = 14;

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

pub fn test_drive() {
    unsafe {
        let libraw = libraw_init(0);
        let filename = CString::new("/Users/fabian/Pictures/2018/2018-12-02/ROFL6244.RAF").unwrap();
        libraw_open_file(libraw, filename.as_ptr());
        libraw_unpack_thumb(libraw);
        libraw_unpack(libraw);
        println!("{:?}", libraw);
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
        // this is safe to call multiple times.
        unpack_thumb(self.libraw).unwrap();
        let thumb = unsafe {
            &(*self.libraw).thumbnail
        };

        let format_code = &thumb.tformat;
        match *format_code {
            LibRaw_thumbnail_formats_LIBRAW_THUMBNAIL_JPEG => (),
            _ => panic!("Expected JPEG thumbnail format"),
        }
        println!("size: {}", thumb.tlength);

        unsafe {
            slice::from_raw_parts(thumb.thumb as *mut u8, thumb.tlength as usize)
        }
    }

    pub fn render_raw_preview(&self) -> image::RgbImage {
        let sizes = unsafe {(*self.libraw).sizes};
        let img_data = self.load_raw_data();
        let mapping = unsafe {(*self.libraw).idata.xtrans_abs};

        let img = ImageBuffer::from_fn(
            sizes.raw_width as u32 / DBG_CROP_FACTOR,
            sizes.raw_height as u32 / DBG_CROP_FACTOR,
            |x, y| {
                let idx = y*(sizes.width as u32) + x;
                // TODO: this should be a generic call to some kind of demosaic algorithm.
                let pixel = RawFile::map_x_trans(x, y, sizes.raw_width as u32, sizes.raw_height as u32, img_data, mapping);
                pixel
            }
        );
        img
    }

    fn map_x_trans(x: u32, y: u32, width: u32, _height: u32, data: &[u16], mapping: [[i8; 6]; 6]) -> image::Rgb<u8> {
        let idx = (y*(width as u32) + x) as usize;
        // TODO: 8 is the target per-channel size here, encode this with generics probably.
        let val = (data[idx] >> (BITS_PER_PIXEL - 8)) as u8;
        match mapping[x as usize % 6][y as usize % 6] {
            0 => image::Rgb([val, 0, 0]), // red
            1 => image::Rgb([0, val >> 1, 0]), // green
            2 => image::Rgb([0, 0, val]), // blue
            _ => panic!("Got unexpected value in xtrans mapping"),
        }
    }

    fn load_raw_data(&self) -> (&[u16]) {
        with_err_conversion(|| unsafe {
            libraw_unpack(self.libraw)
        }).unwrap();
        let sizes = unsafe {(*self.libraw).sizes};
        // convert to u32 to avoid overflow
        let num_shorts = sizes.raw_width as usize * sizes.raw_height as usize;
        // TODO: actually render
        let img_data = unsafe {
            slice::from_raw_parts((*self.libraw).rawdata.raw_image, num_shorts)
        };
        img_data
    }
}

impl Drop for RawFile {
    fn drop(&mut self) {
        println!("Dropping!");
        unsafe {
            // TODO: unclear if we need libraw_recycle here too.
            libraw_recycle(self.libraw);
            libraw_close(self.libraw);
        }
    }
}



#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
