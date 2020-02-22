use crate::common::Pixel;
use libraw::util::datagrid::{DataGrid, Offset, Position};
use libraw::Color;
use num_traits::Unsigned;
use std::marker::PhantomData;

const CHECK_ORDER: [Offset; 5] = [
    Offset(0, 0),
    Offset(0, 1),
    Offset(1, 0),
    Offset(-1, 0),
    Offset(0, -1),
];

pub trait Demosaic<T: Copy + Unsigned> {
    fn demosaic(img_grid: &DataGrid<T>, mapping: &DataGrid<Color>, x: u16, y: u16) -> Pixel<T>;
}

fn offset_for_color(mapping: &DataGrid<Color>, color: Color, pos: Position) -> Position {
    for candidate_pos in CHECK_ORDER.iter().map(|offset| pos + *offset) {
        if mapping.at(candidate_pos) == color {
            return candidate_pos;
        }
    }
    let Position(a, b) = pos;
    if a == 0 || b == 0 {
        // The edges are kinda messed up, so just return the original position
        pos
    } else {
        panic!("Shouldn't get here")
    }
}

fn find_offsets(mapping: &DataGrid<Color>, pos: Position) -> [Position; 3] {
    // Ok so, every pixel has every color within one of the offsets from it.
    // This doesn't apply on edges but we're going to just ignore edges until we
    // figure out if the basic technique works.
    [
        offset_for_color(mapping, Color::Red, pos),
        offset_for_color(mapping, Color::Green, pos),
        offset_for_color(mapping, Color::Blue, pos),
    ]
}

#[allow(dead_code)]
pub struct Nearest(PhantomData<u16>);
#[allow(dead_code)]
pub struct Passthru(PhantomData<u16>);

impl Demosaic<u16> for Nearest {
    fn demosaic(img_grid: &DataGrid<u16>, mapping: &DataGrid<Color>, x: u16, y: u16) -> Pixel<u16> {
        let x = x as usize;
        let y = y as usize;
        let pixel = Position(x, y);
        let offsets = find_offsets(&mapping, pixel);
        Pixel {
            red: img_grid.at(offsets[Color::Red.idx()]),
            green: img_grid.at(offsets[Color::Green.idx()]),
            blue: img_grid.at(offsets[Color::Blue.idx()]),
        }
    }
}

impl Demosaic<u16> for Passthru {
    fn demosaic(img_grid: &DataGrid<u16>, mapping: &DataGrid<Color>, x: u16, y: u16) -> Pixel<u16> {
        let pos = Position(x as usize, y as usize);
        let v = img_grid.at(pos);
        let color = mapping.at(pos);
        match color {
            Color::Red => Pixel {
                red: v,
                blue: 0,
                green: 0,
            },
            Color::Green => Pixel {
                red: 0,
                blue: 0,
                green: v,
            },
            Color::Blue => Pixel {
                red: 0,
                blue: v,
                green: 0,
            },
        }
    }
}
