
mod libraw;

use libraw_sys::*;
use std::ffi::{CString, CStr};
use std::slice;
use libc::c_int;

/*
fn main() {
    let file = libraw::RawFile::new("/Users/fabian/Pictures/2018/2018-12-02/ROFL6243.RAF".to_string()).unwrap();
    println!("Opened file: {:?}", file);
    dump_to_file("/tmp/thumb.jpg", file.get_jpeg_thumbnail()).unwrap();
}*/

fn main() {
    unsafe {
        let libraw = libraw_init(0);
        let filename = CString::new("/Users/fabian/Pictures/2018/2018-12-02/ROFL6244.RAF").unwrap();
        libraw_open_file(libraw, filename.as_ptr());
        libraw_unpack_thumb(libraw);
        libraw_unpack(libraw);
        println!("{:?}", libraw);
    }
}

use std::fs::File;
use std::io::prelude::*;

fn dump_to_file(filename: &str, data: &[u8]) -> std::io::Result<()> {
    let mut file = File::create(filename)?;
    println!("Writing {} bytes to {}", data.len(), filename);
    file.write_all(data)?;
    Ok(())
}
