use libraw::Color;
use num_traits::{Num, Unsigned};

pub struct Pixel<U>
where
    U: Num + Unsigned,
{
    pub red: U,
    pub green: U,
    pub blue: U,
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
