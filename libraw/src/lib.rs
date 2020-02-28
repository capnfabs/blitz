#![allow(clippy::just_underscores_and_digits, clippy::too_many_arguments)]

pub mod fuji_compressed;
pub mod raf;
pub mod tiff;
pub mod util;

#[macro_use]
extern crate quick_error;

#[macro_use]
extern crate derivative;

#[macro_use]
extern crate data_encoding_macro;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Color {
    Red,
    Green,
    Blue,
}

impl Color {
    pub fn idx(self) -> usize {
        match self {
            Color::Red => 0,
            Color::Green => 1,
            Color::Blue => 2,
        }
    }
    // TODO: make this generic in numbers
    pub fn from(val: i8) -> Option<Color> {
        match val {
            0 => Some(Color::Red),
            1 => Some(Color::Green),
            2 => Some(Color::Blue),
            _ => None,
        }
    }

    // TODO does this belong here?
    pub fn multipliers(self) -> [u16; 3] {
        match self {
            Color::Red => [1, 0, 0],
            Color::Green => [0, 1, 0],
            Color::Blue => [0, 0, 1],
        }
    }

    pub fn letter(self) -> &'static str {
        match self {
            Color::Red => "R",
            Color::Green => "G",
            Color::Blue => "B",
        }
    }
}
