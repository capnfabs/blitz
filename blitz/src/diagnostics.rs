use image::{DynamicImage, ImageBuffer, ImageOutputFormat, Luma};
use std::io::Write;

use crate::histo;
use histogram;
use iterm2::download_file;
use resvg::Options;

pub trait TermImage {
    fn draw_to<W: std::io::Write>(&self, writer: &mut W);
    fn display(&self) {
        let mut buf: Vec<u8> = Vec::new();
        self.draw_to(&mut buf);
        download_file(&[("inline", "1")], &buf).unwrap();
        println!()
    }
}

impl TermImage for DynamicImage {
    fn draw_to<W: std::io::Write>(&self, writer: &mut W) {
        self.write_to(writer, ImageOutputFormat::Png).unwrap();
    }
}

impl TermImage for svg::Document {
    fn draw_to<W: std::io::Write>(&self, writer: &mut W) {
        let mut buf: Vec<u8> = Vec::new();
        write!(&mut buf, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",).unwrap();
        svg::write(&mut buf, self).unwrap();
        let backend = resvg::default_backend();
        let options = Options::default();
        let tree = resvg::usvg::Tree::from_data(&buf, &options.usvg).unwrap();
        let mut img = backend.render_to_image(&tree, &options).unwrap();
        let sz = tree.svg_node().size;
        let rgba = img.make_rgba_vec();
        let data = ImageBuffer::from_vec(sz.width() as u32, sz.height() as u32, rgba).unwrap();
        let x = DynamicImage::ImageRgba8(data);
        x.draw_to(writer);
    }
}

pub fn render_tone_curve(h: &histogram::Histogram, width: u32, height: u32) -> impl TermImage {
    let max = h.maximum().unwrap() as u32;
    let step = 100.0 / ((width - 1) as f64);

    let mut buf = ImageBuffer::new(width, height);
    for bar in 0..width {
        let val = h.percentile(bar as f64 * step).unwrap() as f32 / max as f32;
        for i in ((height as f32 * (1.0 - val)) as u32)..height {
            buf[(bar, i)] = Luma([255u8]);
        }
    }
    DynamicImage::ImageLuma8(buf)
}

pub fn render_histogram(h: &histo::Histo, height: usize, width: usize) -> impl TermImage {
    use svg::node::element::path::Data;
    use svg::node::element::Path;
    use svg::node::element::Rectangle;
    use svg::Document;
    let mut data = Data::new().move_to((0, height));
    let mut x_pos = 0;
    let max_bucket = h.iter().map(|bucket| bucket.count).max().unwrap();
    for bucket in h.iter() {
        let bucket_height = height * bucket.count / max_bucket;
        data = data.line_to((x_pos, bucket_height));
        x_pos += 1;
    }
    data = data.line_to((x_pos, height));

    let path = Path::new()
        .set("fill", "grey")
        .set("stroke", "black")
        .set("stroke-width", 1)
        .set("d", data);

    let document = Document::new()
        .set("viewBox", (0, 0, width, height))
        .add(path)
        .add(
            Rectangle::new()
                .set("width", width)
                .set("height", height)
                .set("stroke", "black")
                .set("fill", "none")
                .set("stroke-width", 1),
        );

    document
}
