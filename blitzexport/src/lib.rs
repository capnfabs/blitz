use blitz::render::{render_raw, render_raw_with_settings};
use blitz::render_settings;
use libc::c_char;
use libraw::raf::{ParsedRafFile, RafFile};
use std::ffi::CStr;

pub struct RawRenderer<'a> {
    file: RafFile,
    parsed: Option<ParsedRafFile<'a>>,
}

impl<'a> RawRenderer<'a> {
    pub fn new(filename: &str) -> Self {
        let file = RafFile::open(filename).unwrap();
        RawRenderer { file, parsed: None }
    }

    fn ensure_parsed(&'a mut self) -> &ParsedRafFile {
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

#[no_mangle]
pub extern "C" fn raw_renderer_new(filename: *const c_char) -> *mut RawRenderer<'static> {
    let c_str = unsafe {
        assert!(!filename.is_null());

        CStr::from_ptr(filename)
    };
    Box::into_raw(Box::new(RawRenderer::new(c_str.to_str().unwrap())))
}

#[no_mangle]
pub extern "C" fn raw_renderer_free(ptr: *mut RawRenderer) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        Box::from_raw(ptr);
    }
}

#[repr(C)]
pub struct Buffer {
    data: *mut u8,
    len: usize,
}

impl Buffer {
    fn from_byte_vec(byte_vec: Vec<u8>) -> Buffer {
        let mut buf = byte_vec.into_boxed_slice();
        let data = buf.as_mut_ptr();
        let len = buf.len();
        std::mem::forget(buf);
        Buffer { data, len }
    }
}

#[repr(C)]
pub struct RenderSettings {
    tone_curve: [f32; 5],
    exposure_basis: f32,
}

const TONE_CURVE_CONST: f32 = 2.0;

impl RenderSettings {
    fn to_blitz_settings(&self) -> render_settings::RenderSettings {
        render_settings::RenderSettings {
            tone_curve: self
                .tone_curve
                .iter()
                .copied()
                .map(|x| TONE_CURVE_CONST.powf(x))
                .collect(),
            exposure_basis: TONE_CURVE_CONST.powf(self.exposure_basis),
        }
    }
}

#[no_mangle]
pub extern "C" fn raw_renderer_get_preview(ptr: *mut RawRenderer) -> Buffer {
    let renderer = unsafe {
        assert!(!ptr.is_null());
        &mut *ptr
    };
    // this is a copy
    return Buffer::from_byte_vec(renderer.file.parse_preview().unwrap().to_vec());
}

#[no_mangle]
pub extern "C" fn raw_renderer_render_image(ptr: *mut RawRenderer) -> Buffer {
    let renderer = unsafe {
        assert!(!ptr.is_null());
        &mut *ptr
    };
    return Buffer::from_byte_vec(render_raw(renderer.ensure_parsed()).into_vec());
}

#[no_mangle]
pub extern "C" fn raw_renderer_render_with_settings(
    ptr: *mut RawRenderer,
    settings: RenderSettings,
) -> Buffer {
    let renderer = unsafe {
        assert!(!ptr.is_null());
        &mut *ptr
    };
    return Buffer::from_byte_vec(
        render_raw_with_settings(renderer.ensure_parsed(), &settings.to_blitz_settings())
            .into_vec(),
    );
}

#[no_mangle]
pub extern "C" fn free_buffer(buf: Buffer) {
    let s = unsafe { std::slice::from_raw_parts_mut(buf.data, buf.len) };
    let s = s.as_mut_ptr();
    unsafe {
        Box::from_raw(s);
    }
}
