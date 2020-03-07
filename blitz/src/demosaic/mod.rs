use crate::common::Pixel;
use libraw::griditer::{FilterMap, GridRandomAccess, IndexWrapped2};
use libraw::Color;
use num::Unsigned;
use std::marker::PhantomData;

type Offset = (i32, i32);

const CHECK_ORDER: [Offset; 5] = [(0, 0), (0, 1), (1, 0), (-1, 0), (0, -1)];

type Position = (usize, usize);

pub trait Demosaic<T: Copy + Unsigned, Container: GridRandomAccess> {
    fn demosaic(img_grid: &Container, mapping: &FilterMap, x: u16, y: u16) -> Pixel<T>;
}

fn offset_for_color(mapping: &FilterMap, color: Color, pos: Position) -> Position {
    for candidate_pos in CHECK_ORDER.iter().map(|offset| {
        let x = pos.0 as i32 + offset.0;
        let y = pos.1 as i32 + offset.1;
        assert!(x >= 0);
        assert!(y >= 0);
        (x as usize, y as usize)
    }) {
        if *mapping.index_wrapped(candidate_pos.0, candidate_pos.1) == color {
            return candidate_pos;
        }
    }
    unreachable!("Should have selected a position already");
}

fn find_offsets(mapping: &FilterMap, pos: Position) -> [Position; 3] {
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

static BLACK: Pixel<u16> = Pixel {
    red: 0,
    green: 0,
    blue: 0,
};

impl<Container> Demosaic<u16, Container> for Nearest
where
    Container: GridRandomAccess,
{
    fn demosaic(img_grid: &Container, mapping: &FilterMap, x: u16, y: u16) -> Pixel<u16> {
        let x = x as usize;
        let y = y as usize;
        let pixel = (x, y);
        if x >= 6047 || y >= 4037 || x == 0 || y == 0 {
            return BLACK.clone();
        }
        let offsets = find_offsets(mapping, pixel);
        Pixel {
            red: img_grid[offsets[Color::Red.idx()]],
            green: img_grid[offsets[Color::Green.idx()]],
            blue: img_grid[offsets[Color::Blue.idx()]],
        }
    }
}

impl<Container> Demosaic<u16, Container> for Passthru
where
    Container: GridRandomAccess,
{
    fn demosaic(img_grid: &Container, mapping: &FilterMap, x: u16, y: u16) -> Pixel<u16> {
        let v = img_grid[(x as usize, y as usize)];
        let color = mapping.index_wrapped(x as usize, y as usize);
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
