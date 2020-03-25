use crate::common::Pixel;
use libraw::griditer::{FilterMap, IndexWrapped2};
use libraw::Color;
use ndarray::ArrayView2;
use num_traits::Num;
use std::marker::PhantomData;

type Offset = (i32, i32);

const CHECK_ORDER: [Offset; 5] = [(0, 0), (0, 1), (1, 0), (-1, 0), (0, -1)];

type Position = (usize, usize);

pub trait Demosaic<T: Copy + Num> {
    fn demosaic(img_grid: &ArrayView2<T>, mapping: &FilterMap, x: usize, y: usize) -> Pixel<T>;
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

fn black<T: Copy + Num>() -> Pixel<T> {
    Pixel {
        red: T::zero(),
        green: T::zero(),
        blue: T::zero(),
    }
}

impl<T: Copy + Num> Demosaic<T> for Nearest {
    fn demosaic(img_grid: &ArrayView2<T>, mapping: &FilterMap, x: usize, y: usize) -> Pixel<T> {
        let pixel = (x, y);
        // TODO: maybe inline this.
        let (width, height) = img_grid.dim();
        if x >= width - 1 || y >= height - 1 || x == 0 || y == 0 {
            return black();
        }
        let offsets = find_offsets(mapping, pixel);
        Pixel {
            red: img_grid[offsets[Color::Red.idx()]],
            green: img_grid[offsets[Color::Green.idx()]],
            blue: img_grid[offsets[Color::Blue.idx()]],
        }
    }
}

impl<T: Copy + Num> Demosaic<T> for Passthru {
    fn demosaic(img_grid: &ArrayView2<T>, mapping: &FilterMap, x: usize, y: usize) -> Pixel<T> {
        let v = img_grid[(x, y)];
        let color = mapping.index_wrapped(x as usize, y as usize);
        match color {
            Color::Red => Pixel {
                red: v,
                blue: T::zero(),
                green: T::zero(),
            },
            Color::Green => Pixel {
                red: T::zero(),
                blue: T::zero(),
                green: v,
            },
            Color::Blue => Pixel {
                red: T::zero(),
                blue: v,
                green: T::zero(),
            },
        }
    }
}
