use libraw::Color;
use num::Num;

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Pixel<U>
where
    U: Num,
{
    pub red: U,
    pub green: U,
    pub blue: U,
}

impl Pixel<f32> {
    pub fn to_rgb(&self) -> image::Rgb<u8> {
        image::Rgb([
            (self.red * 255.0) as u8,
            (self.green * 255.0) as u8,
            (self.blue * 255.0) as u8,
        ])
    }
}

impl Pixel<u16> {
    pub fn to_rgb(&self) -> image::Rgb<u8> {
        image::Rgb([
            (self.red >> 8) as u8,
            (self.green >> 8) as u8,
            (self.blue >> 8) as u8,
        ])
    }

    #[allow(dead_code)]
    pub fn only(&self, color: Color) -> Self {
        match color {
            Color::Red => Pixel {
                red: self.red,
                green: 0,
                blue: 0,
            },
            Color::Green => Pixel {
                red: 0,
                green: self.green,
                blue: 0,
            },
            Color::Blue => Pixel {
                red: 0,
                green: 0,
                blue: self.blue,
            },
        }
    }
}
