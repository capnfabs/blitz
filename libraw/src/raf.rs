use crate::tiff;
use crate::tiff::IfdEntry;
use memmap::Mmap;
use nom::bytes::streaming::{tag, take};
use nom::combinator::all_consuming;
use nom::error::ParseError;
use nom::lib::std::collections::HashMap;
use nom::multi::count;
use nom::number::complete::{be_u16, be_u32};
use nom::sequence::tuple;
use nom::IResult;
use std::fs::File;
use std::path::Path;

type I<'a> = &'a [u8];

quick_error! {
    #[derive(Debug)]
    pub enum RafError {
        Io(err: std::io::Error) {
            from()
        }
        // I don't know how to capture nom:Err here, so we're stuck with this.
        Unknown {
        }
    }
}

#[derive(Debug, Copy, Clone)]
struct Header<'a> {
    model: &'a str,
    fw_version: &'a str,
}

fn str_from_fixed_len_buf(input: I) -> &str {
    if let Some(idx) = input.iter().position(|&elem| elem == 0) {
        std::str::from_utf8(&input[0..idx]).unwrap()
    } else {
        std::str::from_utf8(input).unwrap()
    }
}

fn header(input: I) -> IResult<I, Header> {
    let res = tuple((
        tag("FUJIFILMCCD-RAW 0201FF129502"),
        // Camera Model Name.
        take(32u8),
        // Camera Firmware Version. 8 bytes?? I dunno. At least 4.
        take(8u8),
        // A bunch of zeros
        count(tag(b"\0"), 16),
    ))(input)?;
    let (more, (_, model, fw_version, _)) = res;
    let model = str_from_fixed_len_buf(model);
    let fw_version = str_from_fixed_len_buf(fw_version);
    Ok((more, Header { model, fw_version }))
}

#[derive(Copy, Clone, Debug)]
struct OffsetLength {
    offset: u32,
    length: u32,
}

#[derive(Debug)]
struct Offsets {
    jpeg: OffsetLength,
    metadata: OffsetLength,
    raw: OffsetLength,
}

fn offset_size(input: I) -> IResult<I, OffsetLength> {
    let (i, (offset, length)) = tuple((be_u32, be_u32))(input)?;
    Ok((i, OffsetLength { offset, length }))
}

fn offset_sizes(input: I) -> IResult<I, Offsets> {
    let (i, v) = count(offset_size, 3)(input)?;
    Ok((
        i,
        Offsets {
            jpeg: v[0],
            metadata: v[1],
            raw: v[2],
        },
    ))
}

impl OffsetLength {
    fn apply(self, input: &[u8]) -> &[u8] {
        let start = self.offset as usize;
        let end = (self.offset + self.length) as usize;
        &input[start..end]
    }
}

type Height = u16;
type Width = u16;

#[derive(Debug)]
enum Tag<'a> {
    XTransMapping(&'a [u8]), //6x6 grid with the Xtrans mapping, 0-1-2s represent colors
    HeightWidthSensor(Height, Width),
    CropTopLeft(Height, Width), // Crop Top Left? According to Exiftool. Unclear what this is in reference to
    HeightWidthCrop(Height, Width), // Raw Image cropped Size"
    HeightWidthCrop2(Height, Width), // ???
    HeightWidthCrop3(Height, Width), // ???
    AspectRatio(u16, u16),      // u16 / u16 (height / width)
    RAFData(&'a [u8]),
    Unknown(u16, &'a [u8]),
}

fn parse_tag<'a, E: ParseError<&'a [u8]>>(code: u16, data: &'a [u8]) -> Result<Tag, nom::Err<E>> {
    let res = match code {
        0x0131 => Tag::XTransMapping(data),
        0x0100 => {
            let (_, (h, w)) = all_consuming(tuple((be_u16, be_u16)))(data)?;
            Tag::HeightWidthSensor(h, w)
        }
        0x0110 => {
            let (_, (h, w)) = all_consuming(tuple((be_u16, be_u16)))(data)?;
            Tag::CropTopLeft(h, w)
        } // Possibly Crop Top left? I get 21x16.
        0x0111 => {
            let (_, (h, w)) = all_consuming(tuple((be_u16, be_u16)))(data)?;
            Tag::HeightWidthCrop(h, w)
        }
        0x0112 => {
            let (_, (h, w)) = all_consuming(tuple((be_u16, be_u16)))(data)?;
            Tag::HeightWidthCrop2(h, w)
        }
        0x0113 => {
            let (_, (h, w)) = all_consuming(tuple((be_u16, be_u16)))(data)?;
            Tag::HeightWidthCrop3(h, w)
        }
        0x0115 => {
            let (_, (y, x)) = all_consuming(tuple((be_u16, be_u16)))(data)?;
            Tag::AspectRatio(y, x)
        }
        /*
        0x0130 => Tag::Unknown3,
        0x0141 => Tag::Unknown4,
        0x9650 => Tag::Unknown5, // dcraw: apparently something exposure related? midpointshift?
        */
        0xC000 => Tag::RAFData(data),
        other => Tag::Unknown(other, data),
    };
    Ok(res)
}

fn metadata_internal_tag(input: I) -> IResult<I, Tag> {
    let (input, (tag_num, len)) = tuple((be_u16, be_u16))(input)?;
    let (input, data) = take(len)(input)?;
    let tag = parse_tag(tag_num, data)?;
    Ok((input, tag))
}

type ImgMeta<'a> = Vec<Tag<'a>>;

fn parse_metadata(input: I) -> IResult<I, ImgMeta> {
    let (i, (_, meta_items_count)) = tuple((
        // I'm assuming this is offset+length again??
        tag(&hexlower!("0000")),
        be_u16,
    ))(input)?;

    let (i, meta) = count(metadata_internal_tag, meta_items_count as usize)(i)?;
    Ok((i, meta))
}

#[derive(Debug)]
pub struct ParsedRafFile<'a> {
    header: Header<'a>,
    jpg_preview: &'a [u8],
    // This is in the middle RAF section
    metadata: ImgMeta<'a>,
    raw: &'a [u8],
}

#[derive(Debug)]
struct RenderData {
    width: Width,
    height: Height,
    bit_depth: u16,
    black_levels: Vec<u16>,
    white_bal: WhiteBalCoefficients,
}

#[derive(Debug)]
struct Grid<'a, T: Copy> {
    data: &'a [T],
    width: Width,
}

impl<'a, T: Copy> Grid<'a, T> {
    pub fn at(&self, x: Width, y: Height) -> T {
        self.data[(y * self.width) as usize + x as usize]
    }
}

#[derive(Debug)]
struct WhiteBalCoefficients {
    red: u16,
    green: u16,
    blue: u16,
}

fn parse_tiffish(raw: &[u8]) -> IResult<I, RenderData> {
    let (_, tiff) = tiff::parse_tiff(raw)?;
    let ifd_block = &tiff.ifds[0][0];
    let (_, (ifd, next)) = tiff::parse_ifd(&raw[(ifd_block.val_u32().unwrap() as usize)..])?;
    assert!(next.is_none());

    let hm: HashMap<u16, &IfdEntry> = ifd.iter().map(|item| (item.tag, item)).collect();
    let width = hm[&61441].val_u32().unwrap() as Width;
    let height = hm[&61442].val_u32().unwrap() as Height;
    let bit_depth = hm[&61443].val_u32().unwrap() as u16;
    // _Maybe_ data offset + length for compressed?
    // Pretty sure this is data offset
    let _img_data_offset = hm[&61447].val_u32().unwrap();
    // 20743472 is this number, it's very large. 449024 is where the TIFF starts
    // 20743472 + 449024 = 21192496 ... is in middle of data, + 2048 is end of file.
    // it's the length of the compressed section.
    let _img_data_length = hm[&61448].val_u32().unwrap();
    // Back to unknown, it's 142 which could mean _anything_.
    let _49 = hm[&61449].val_u32().unwrap();
    // Maybe black levels or something, there's 36 longs
    let black_levels: Vec<u32> = tiff.load_offset_data(hm[&61450]).unwrap();
    let black_levels: Vec<u16> = black_levels.iter().map(|x| *x as u16).collect();

    // Note that tag 61454 had the same values on all the files I tested -
    // not sure what the difference is. DCRAW uses '54 and not '53.
    let wb: Vec<u32> = tiff.load_offset_data(hm[&61453]).unwrap();
    let wb = WhiteBalCoefficients {
        red: wb[0] as u16,
        green: wb[1] as u16,
        blue: wb[2] as u16,
    };

    // '51, '52, '55, '56 all look like some kind of curve.
    // The first number looks like x/y axis lengths, then x positions, then y positions.

    Ok((
        raw,
        RenderData {
            width,
            height,
            bit_depth,
            black_levels,
            white_bal: wb,
        },
    ))
}

fn parse_all(input: I) -> IResult<I, ParsedRafFile> {
    let (_, (header, offsets)) = tuple((header, offset_sizes))(input)?;
    let jpg_preview = offsets.jpeg.apply(input);
    let metadata = offsets.metadata.apply(input);
    let raw = offsets.raw.apply(input);
    let (_, wump) = parse_tiffish(raw)?;
    let (i, metadata) = parse_metadata(metadata)?;
    Ok((
        i,
        ParsedRafFile {
            header,
            jpg_preview,
            metadata,
            raw,
        },
    ))
}

#[derive(Debug)]
pub struct RafFile {
    file: File,
    mmap: Mmap,
}

impl RafFile {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<RafFile, RafError> {
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file) }?;
        Ok(RafFile { file, mmap })
    }

    pub fn parse_raw(&self) -> Result<ParsedRafFile, RafError> {
        let result = parse_all(&self.mmap);
        match result {
            Ok((_, parsed)) => Ok(parsed),
            Err(e) => Err(RafError::Unknown),
        }
    }
}
