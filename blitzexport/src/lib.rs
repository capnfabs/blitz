use libc::c_char;
use libraw::raf::RafFile;
use std::ffi::CStr;

pub struct RawRenderer {
    file: RafFile,
}

impl RawRenderer {
    pub fn new(filename: &str) -> Self {
        let file = RafFile::open(filename).unwrap();
        RawRenderer { file }
    }
}

#[no_mangle]
pub extern "C" fn raw_renderer_new(filename: *const c_char) -> *mut RawRenderer {
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

#[no_mangle]
pub extern "C" fn raw_renderer_get_preview(ptr: *mut RawRenderer) -> Buffer {
    let renderer = unsafe {
        assert!(!ptr.is_null());
        &mut *ptr
    };
    let content = renderer.file.parse_preview().unwrap().to_vec();
    let mut buf = content.into_boxed_slice();
    let data = buf.as_mut_ptr();
    let len = buf.len();
    Buffer { data, len }
}

#[no_mangle]
pub extern "C" fn free_buffer(buf: Buffer) {
    let s = unsafe { std::slice::from_raw_parts_mut(buf.data, buf.len) };
    let s = s.as_mut_ptr();
    unsafe {
        Box::from_raw(s);
    }
}
