// Still WIP
#![allow(unused_variables)]

mod bytecounter;
mod compress;
mod evenodd;
mod inflate;
mod process_common;
mod sample;
mod zip_with_offset;

use hex;
use nom::bytes::complete::take;
use nom::bytes::streaming::tag;
use nom::combinator::map;
use nom::multi::count;
use nom::number::complete::{be_u16, be_u32, be_u8};
use nom::sequence::tuple;
use nom::IResult;

pub use compress::compress;
use std::io::Cursor;

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

pub fn load_fuji_compressed(input: &[u8]) -> IResult<I, Vec<u16>> {
    let i = input;
    let (i, header) = parse_fuji_header(i)?;
    // TODO: build quantisation tables
    let (i, block_sizes) = block_sizes(i, header.num_blocks)?;
    let (i, blocks) = read_blocks(i, &block_sizes)?;
    println!("Compressed: {:#?}", header);
    println!("Blocks: {:#?}", block_sizes);
    println!("block 1 first 20: {:?}", hex::encode(&blocks[1][0..20]));
    let blocks = blocks.iter().map(|x| Cursor::new(x));
    let output = inflate::inflate(blocks, &inflate::make_color_map());
    Ok((input, output))
}
