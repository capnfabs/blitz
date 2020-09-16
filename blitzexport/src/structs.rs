use libraw::raf::{ParsedRafFile, RafFile};

#[repr(C)]
pub struct Buffer {
    pub data: *mut u8,
    pub len: usize,
}

#[repr(C)]
pub struct ImageAndHistogram {
    pub img: RawImage,
    pub histogram: RawImage,
}

#[repr(C)]
pub enum ImageFormat {
    Rgb,
    Rgba,
}

#[repr(C)]
pub struct RawImage {
    pub data: Buffer,
    pub width: u32,
    pub height: u32,
    pub pixel_format: ImageFormat,
}

impl RawImage {
    pub fn from_rgb_image(img: image::RgbImage) -> Self {
        let width = img.width();
        let height = img.height();
        RawImage {
            data: Buffer::from_byte_vec(img.into_vec()),
            width: width,
            height: height,
            pixel_format: ImageFormat::Rgb,
        }
    }

    pub fn from_rgba_image(img: image::RgbaImage) -> Self {
        let width = img.width();
        let height = img.height();
        RawImage {
            data: Buffer::from_byte_vec(img.into_vec()),
            width: width,
            height: height,
            pixel_format: ImageFormat::Rgba,
        }
    }
}

impl Buffer {
    fn empty() -> Buffer {
        Buffer {
            data: std::ptr::null_mut(),
            len: 0,
        }
    }

    pub fn from_byte_vec(byte_vec: Vec<u8>) -> Buffer {
        if byte_vec.is_empty() {
            // freeing Buffers for empty vecs was causing problems; this works around it.
            // TODO: understand why.
            Buffer::empty()
        } else {
            let mut buf = byte_vec.into_boxed_slice();
            let data = buf.as_mut_ptr();
            let len = buf.len();
            std::mem::forget(buf);
            println!("Supplying {} bytes at {:p}", len, data);
            Buffer { data, len }
        }
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        if !self.data.is_null() {
            let s = unsafe { std::slice::from_raw_parts_mut(self.data, self.len) };
            let s = s.as_mut_ptr();
            unsafe {
                Box::from_raw(s);
            }
        }
    }
}

pub struct RawRenderer<'a> {
    pub file: RafFile,
    parsed: Option<ParsedRafFile<'a>>,
}

impl<'a> RawRenderer<'a> {
    pub fn new(filename: &str) -> Self {
        let file = RafFile::open(filename).unwrap();
        RawRenderer { file, parsed: None }
    }

    pub fn ensure_parsed(&'a mut self) -> &ParsedRafFile {
        if self.parsed.is_none() {
            println!(
                "Parsing: {}...",
                self.file
                    .path()
                    .file_name()
                    .and_then(|x| x.to_str())
                    .unwrap()
            );
            self.parsed = Some(self.file.parse_raw().unwrap());
            println!("...done!");
        }
        self.parsed.as_ref().unwrap()
    }
}
