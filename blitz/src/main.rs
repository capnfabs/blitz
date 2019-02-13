use libraw;
use std::fs::File;
use std::io::prelude::*;

fn main() {
    let file = libraw::RawFile::new("/Users/fabian/Pictures/2018/2018-12-02/ROFL6243.RAF".to_string()).unwrap();
    println!("Opened file: {:?}", file);
    dump_to_file("/tmp/thumb.jpg", file.get_jpeg_thumbnail()).unwrap();
    let img = file.render_raw_preview();
    img.save("/tmp/render.jpg").unwrap();
}

fn dump_to_file(filename: &str, data: &[u8]) -> std::io::Result<()> {
    let mut file = File::create(filename)?;
    println!("Writing {} bytes to {}", data.len(), filename);
    file.write_all(data)?;
    Ok(())
}
