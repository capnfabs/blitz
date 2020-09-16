mod render_settings;
mod structs;

use crate::structs::ImageAndHistogram;
use blitz::diagnostics::histogram::ToHistogram;
use blitz::render::{render_raw, render_raw_with_settings};
use libc::c_char;
use render_settings::RenderSettings;
use std::ffi::CStr;
use structs::{Buffer, RawRenderer};

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
) -> ImageAndHistogram {
    let renderer = unsafe {
        assert!(!ptr.is_null());
        &mut *ptr
    };

    let img = render_raw_with_settings(renderer.ensure_parsed(), &settings.to_blitz_settings());
    println!("Computing histograms");
    let histo = img.histogram();

    ImageAndHistogram {
        img: Buffer::from_byte_vec(img.into_vec()),
        histogram: Buffer::from_byte_vec(histo.to_img(256, 128).into_vec()),
    }
}

#[no_mangle]
pub extern "C" fn free_buffer(buf: Buffer) {
    // do this explicitly so the containing method doesn't get erased.
    drop(buf)
}
