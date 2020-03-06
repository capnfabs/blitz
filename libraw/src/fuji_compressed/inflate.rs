use crate::fuji_compressed::process_common::{
    collect_carry_lines, compute_weighted_average_even, flatten, grad_and_weighted_avg_even,
    grad_and_weighted_avg_odd, is_interpolated, load_even_coefficients, split_at, PROCESS, UNSET,
};
use crate::fuji_compressed::sample::{Grad, Gradients, Sample};
use crate::fuji_compressed::zip_with_offset::zip_with_offset;
use crate::util::bitreader::BitReader;
use crate::util::colored::Colored;
use crate::util::datagrid::{DataGrid, Position, Size};
use crate::Color;
use crate::Color::{Blue, Green, Red};
use itertools::Itertools;
use std::io;

use ndarray::{Array2, ArrayViewMut1, ArrayViewMut2, Axis, ShapeBuilder};
use rayon::prelude::*;
use std::io::Cursor;
use std::iter::repeat;

static VERTICAL: Axis = Axis(1);
static HORIZONTAL: Axis = Axis(0);

// TODO: this should be moved to a testing utilities file.
pub fn make_color_map() -> DataGrid<'static, Color> {
    DataGrid::wrap(
        &[
            Green, Green, Red, Green, Green, Blue, Green, Green, Blue, Green, Green, Red, Blue,
            Red, Green, Red, Blue, Green, Green, Green, Blue, Green, Green, Red, Green, Green, Red,
            Green, Green, Blue, Red, Blue, Green, Blue, Red, Green,
        ],
        Size(6, 6),
    )
}

pub fn inflate(
    img_size: Size,
    stripe_width: usize,
    blocks: Vec<Cursor<&[u8]>>,
    color_map: &DataGrid<Color>,
) -> Vec<u16> {
    let Size(img_width, img_height) = img_size;

    let output = vec![0; img_width * img_height];
    let mut mg = Array2::from_shape_vec((img_width, img_height).set_f(true), output).unwrap();
    //println!("MG shape 0x1 {}x{}", mg.len_of(Axis(0)), mg.len_of(Axis(1)));
    let mut chunks = mg.axis_chunks_iter_mut(Axis(0), stripe_width).collect_vec();
    chunks
        //.par_iter_mut()
        .iter_mut()
        .zip(blocks)
        .enumerate()
        .for_each(|(_block_num, (stripe, block))| {
            inflate_stripe(block, color_map, stripe_width, stripe);
        });
    mg.into_raw_vec()
}

pub fn inflate_stripe<T: io::Read>(
    reader: T,
    color_map: &DataGrid<Color>,
    // It _is_ possible to get this from output.size(), but it breaks in the
    // case of the rightmost stripe, which is skinnier for output but
    // decompresses using the same size.
    stripe_width: usize,
    output: &mut ndarray::ArrayViewMut2<u16>,
) {
    /*println!(
        "Stripe shape 0x1 {}x{}",
        output.len_of(Axis(0)),
        output.len_of(Axis(1))
    );*/
    let mut r: BitReader<_> = BitReader::new(reader);

    // As per Xtrans matrix, there's a max of 4 green pixels out of every 6, so
    // we need 512 slots for every line of 768 pixels (for example)
    let required_capacity = stripe_width * 4 / 6;

    let mut prev_lines = {
        let zeros = vec![0u16; required_capacity];
        Colored::new(
            vec![zeros.clone(), zeros.clone()],
            vec![zeros.clone(), zeros.clone()],
            vec![zeros.clone(), zeros.clone()],
        )
    };

    let mut gradients = ([[Grad::default(); 41]; 3], [[Grad::default(); 41]; 3]);

    let stripe_height = output.len_of(Axis(1));
    let num_lines = stripe_height / 6;

    for line in 0..num_lines {
        let results = inflate_line(&mut r, &mut gradients, &prev_lines);
        prev_lines = collect_carry_lines(&results);
        copy_line_to_xtrans(
            color_map,
            &mut output.slice_mut(s![.., line * 6..(line + 1) * 6]),
            results,
        )
    }
}

fn copy_line_to_xtrans(
    color_map: &DataGrid<Color>,
    output: &mut ArrayViewMut2<u16>,
    results: Colored<Vec<Vec<u16>>>,
) {
    /*println!(
        "Copy line to xtrans shape 0x1 {}x{}",
        output.len_of(Axis(0)),
        output.len_of(Axis(1))
    );*/
    for row_idx in 0..6 {
        let (r, g, b) = results.split();
        let line_colors = Colored::new(&r[row_idx / 2], &g[row_idx], &b[row_idx / 2]);
        map_contiguous_colors_to_xtrans(
            &mut output.column_mut(row_idx),
            &line_colors,
            &color_map.subgrid(Position(0, row_idx), Size(6, 1)),
        );
    }
}

fn map_contiguous_colors_to_xtrans(
    output_row: &mut ArrayViewMut1<u16>,
    colors: &Colored<&Vec<u16>>,
    row_color_map: &DataGrid<Color>,
) {
    //println!("len output_row {}", output_row.len());
    for (pos, val) in output_row.iter_mut().enumerate() {
        // TODO: extract into a method, also used in map_xtrans_to_contiguous_colors
        let squashed_idx = (((pos as i32 - 1) * 2).div_euclid(3) + 1) as usize;
        let color = row_color_map.at(Position(pos, 0));
        *val = colors[color][squashed_idx];
    }
}

pub trait ValueTarget {
    fn write_val(&mut self, value: u16);
}

fn inflate_line<R: io::Read>(
    reader: &mut BitReader<R>,
    gradients: &mut (Gradients, Gradients),
    carry_results: &Colored<Vec<Vec<u16>>>,
) -> Colored<Vec<Vec<u16>>> {
    let mut colors = Colored::new(
        vec![vec![UNSET; 512]; 3],
        vec![vec![UNSET; 512]; 6],
        vec![vec![UNSET; 512]; 3],
    );
    for (color_a, color_b, grad_set_idx) in &PROCESS {
        let ca_even = repeat(color_a).zip((0..512).step_by(2));
        let ca_odd = repeat(color_a).zip((0..512).skip(1).step_by(2));
        let cb_even = repeat(color_b).zip((0..512).step_by(2));
        let cb_odd = repeat(color_b).zip((0..512).skip(1).step_by(2));
        // This starts processing the odd entries after the first 4 even entries are processed.
        let zipped = zip_with_offset(ca_even.zip_eq(cb_even), 0, ca_odd.zip_eq(cb_odd), 4)
            .map(|(a, b)| (flatten(a), flatten(b)));

        for ((ca_even, cb_even), (ca_odd, cb_odd)) in zipped {
            for thing in &[ca_even, cb_even, ca_odd, cb_odd] {
                if let Some(((color, row), idx)) = thing {
                    let value = if is_interpolated(*color, *row, *idx) {
                        interpolate_value(&colors, &carry_results, *row, *color, *idx)
                    } else {
                        compute_value_and_update_gradients(
                            reader,
                            &colors,
                            gradients,
                            &carry_results,
                            *row,
                            *color,
                            *idx,
                            *grad_set_idx,
                        )
                    };
                    colors[*color][*row][*idx] = value;
                }
            }
        }
    }
    colors
}

fn interpolate_value(
    colors: &Colored<Vec<Vec<u16>>>,
    carry_results: &Colored<Vec<Vec<u16>>>,
    row_idx: usize,
    color: Color,
    idx: usize,
) -> u16 {
    assert_eq!(idx % 2, 0);
    let carry_results = &carry_results[color];
    let cdata = &colors[color];
    let (rprevprev, rprev) = match row_idx {
        0 => (carry_results[0].as_slice(), carry_results[1].as_slice()),
        1 => (carry_results[1].as_slice(), cdata[0].as_slice()),
        row => (cdata[row - 2].as_slice(), cdata[row - 1].as_slice()),
    };
    let ec = load_even_coefficients(rprev, rprevprev, idx);
    let weighted_average = compute_weighted_average_even(ec);
    assert!(weighted_average < (1 << 14));
    weighted_average
}

fn compute_value_and_update_gradients<R: io::Read>(
    reader: &mut BitReader<R>,
    colors: &Colored<Vec<Vec<u16>>>,
    gradients: &mut ([[Grad; 41]; 3], [[Grad; 41]; 3]),
    carry_results: &Colored<Vec<Vec<u16>>>,
    row_idx: usize,
    color: Color,
    idx: usize,
    grad_set: usize,
) -> u16 {
    let is_even = idx % 2 == 0;
    // Setup. Choose coefficients based on color / row etc
    let carry_results = &carry_results[color];
    let cdata = &colors[color];
    let (even_gradients, odd_gradients) = gradients;
    let (rprevprev, rprev) = match row_idx {
        0 => (carry_results[0].as_slice(), carry_results[1].as_slice()),
        1 => (carry_results[1].as_slice(), cdata[0].as_slice()),
        row => (cdata[row - 2].as_slice(), cdata[row - 1].as_slice()),
    };
    // Computation is different based on whether the index is odd or even.
    let (grad_set, (weighted_average, which_grad)) = if is_even {
        (
            &mut even_gradients[grad_set],
            grad_and_weighted_avg_even(idx, rprevprev, rprev),
        )
    } else {
        (
            &mut odd_gradients[grad_set],
            grad_and_weighted_avg_odd(idx, rprevprev, rprev, &cdata[row_idx]),
        )
    };
    let grad = &mut grad_set[which_grad.abs() as usize];
    let grad_instructs_subtraction = which_grad < 0;

    let dec_bits = grad.bit_diff() as usize;

    let sample = read_sample(reader, dec_bits).unwrap();

    let delta = sample_to_delta(sample);
    // Finally: update gradient.
    let actual_value = if grad_instructs_subtraction {
        (weighted_average as i32 - delta as i32)
    } else {
        (weighted_average as i32 + delta as i32)
    };

    grad.update_from_value(delta.abs());

    // huh, this is actually necessary.
    actual_value.rem_euclid(1 << 14) as u16
}

fn read_sample<T: io::Read>(reader: &mut BitReader<T>, lower_bits: usize) -> io::Result<Sample> {
    let upper = reader.count_continuous_0s()?;

    if upper > 40 {
        let lower = reader.read_bits(14)?;
        Ok(Sample::EntireDelta(lower as u16))
    } else {
        let lower = reader.read_bits(lower_bits)? as u16;
        Ok(Sample::SplitDelta {
            upper: upper as u16,
            lower,
            lower_bits,
        })
    }
}

fn sample_to_delta(sample: Sample) -> i32 {
    match sample {
        Sample::EntireDelta(val) => {
            let invert = (val & 0b1) == 0;
            let val = (val >> 1) as i32 + 1;
            if invert {
                -val
            } else {
                val
            }
        }
        Sample::SplitDelta {
            upper,
            lower,
            lower_bits,
        } => {
            let val = (upper << lower_bits as u16) | lower;
            let (val, invert) = split_at(val, 1);
            let val = val as i32;
            let invert = invert != 0;
            if invert {
                -(val + 1)
            } else {
                val
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::fuji_compressed::inflate::{inflate_stripe, make_color_map};
    use crate::fuji_compressed::process_common::UNSET;
    use crate::util::datagrid::{MutableDataGrid, Size};

    use itertools::Itertools;
    use std::convert::TryInto;

    const STRIPE_WIDTH: usize = 768;
    const NUM_LINES: usize = 673;

    #[test]
    fn end_to_end_stripe_inflate() {
        const UNCOMPRESSED: &[u8] = include_bytes!("testdata/DSCF2279-block0.uncompressed.bin");
        const COMPRESSED: &[u8] = include_bytes!("testdata/DSCF2279-block0.compressed.bin");

        let expected = UNCOMPRESSED
            .chunks_exact(2)
            .map(|x| u16::from_le_bytes(x.try_into().unwrap()))
            .collect_vec();

        let mut actual_data = vec![UNSET; STRIPE_WIDTH * NUM_LINES * 6];
        let mut output = MutableDataGrid::new(&mut actual_data, Size(STRIPE_WIDTH, NUM_LINES * 6));

        inflate_stripe(
            &mut COMPRESSED,
            &make_color_map(),
            STRIPE_WIDTH,
            &mut output,
        );
        assert_eq!(actual_data.len(), expected.len());
        assert_eq!(actual_data, expected.as_slice());
    }
}
