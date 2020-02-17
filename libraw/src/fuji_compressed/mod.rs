// Still WIP
#![allow(unused_variables)]

mod bytecounter;
mod compress;
mod evenodd;
mod inflate;
mod process_common;
mod sample;
mod zip_with_offset;

use nom::bytes::complete::take;
use nom::bytes::streaming::tag;
use nom::combinator::map;
use nom::multi::count;
use nom::number::complete::{be_u16, be_u32, be_u8};
use nom::sequence::tuple;
use nom::IResult;

pub use compress::compress;

#[derive(Debug)]
struct FujiCompressedHeader {
    version: u8,
    // Don't really know what this means
    raw_type: u8,
    // Bits per pixel
    raw_bits: u8,
    // Height of each vertical stripe == the height of the image
    raw_height: u16,
    // This is the width of the image once you "round up" such that you can fit an even number of blocks in.
    raw_rounded_width: u16,
    // Width of the actual image from the sensor, excluding the extra rounding for block processing. Always <= raw_rounded_width.
    raw_width: u16,
    // The width of each block.
    // fuji_block_width / block_size depending on location in Libraw
    block_width: u16,
    // The number of blocks. They're all arranged side-by-side. h_blocks_in_row
    num_blocks: u8,
    // _Appears_ to be the number of lines of sensor pattern. For Xtrans, which is 6x6 repeating, this is == raw_height / 6.
    // maybe should be called lines_per_block?
    total_lines: u16,
}

type I<'a> = &'a [u8];

fn parse_fuji_header(input: I) -> IResult<I, FujiCompressedHeader> {
    map(
        tuple((
            tag(b"\x49\x53"),
            be_u8,
            be_u8,
            be_u8,
            be_u16,
            be_u16,
            be_u16,
            be_u16,
            be_u8,
            be_u16,
        )),
        |(
            _tag,
            version,
            raw_type,
            raw_bits,
            raw_height,
            raw_rounded_width,
            raw_width,
            block_size,
            blocks_in_row,
            total_lines,
        )| FujiCompressedHeader {
            version,
            raw_type,
            raw_bits,
            raw_height,
            raw_rounded_width,
            raw_width,
            block_width: block_size,
            num_blocks: blocks_in_row,
            total_lines,
        },
    )(input)
}

fn block_sizes(input: I, num_blocks: u8) -> IResult<I, Vec<u32>> {
    count(be_u32, num_blocks as usize)(input)
}

/// Given i, which should be input positioned at the first block, and block_sizes, the size of each block,
/// retrieves those blocks.
fn read_blocks<'a>(input: I<'a>, block_sizes: &[u32]) -> IResult<I<'a>, Vec<&'a [u8]>> {
    let mut i = input;
    let mut blocks = Vec::with_capacity(block_sizes.len());
    for size in block_sizes {
        let (next, block) = take(*size)(i)?;
        i = next;
        blocks.push(block);
    }
    Ok((i, blocks))
}

fn decode_block(block: I) -> Vec<u16> {
    vec![]
}

fn join_blocks(blocks: &[&[u16]]) -> Vec<u16> {
    vec![]
}

pub fn load_fuji_compressed(input: &[u8]) -> IResult<I, Vec<u16>> {
    let i = input;
    let (i, header) = parse_fuji_header(i)?;
    // TODO: build quantisation tables
    let (i, block_sizes) = block_sizes(i, header.num_blocks)?;
    let (i, blocks) = read_blocks(i, &block_sizes)?;
    // I dunno, this feels like fighting the borrow checker a lot
    let decoded_blocks: Vec<Vec<u16>> = blocks.iter().map(|b| decode_block(b)).collect();
    let block_refs: Vec<&[u16]> = decoded_blocks.iter().map(|x| x.as_slice()).collect();
    let output = join_blocks(&block_refs);

    println!("Compressed: {:#?}", header);
    Ok((input, output))
}
