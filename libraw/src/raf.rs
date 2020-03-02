use crate::raf::EncodingType::{Compressed, Uncompressed, Unknown};
use crate::raf::Tag::XTransMapping;
use crate::tiff::{IfdEntry, SRational};
use crate::{fuji_compressed, tiff, Color};
use itertools::Itertools;
use memmap::Mmap;
use nom::bytes::streaming::{tag, take};
use nom::combinator::all_consuming;
use nom::error::ParseError;
use nom::lib::std::collections::HashMap;
use nom::multi::count;
use nom::number::complete::{be_u16, be_u32, le_u16};
use nom::sequence::tuple;
use nom::IResult;
use std::fmt::Debug;
use std::fs::File;
use std::path::Path;

type I<'a> = &'a [u8];

type Width = u16;
type Height = u16;

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

pub struct FileParts<'a> {
    pub jpeg: &'a [u8],
    pub jpeg_exif_tiff: &'a [u8],
    pub metadata: &'a [u8],
    pub raw: &'a [u8],
}

// TODO: this isn't polished or resilient; maybe I should use a library for this.
fn find_exif_tiff(jpeg_data: &[u8]) -> IResult<I, &[u8]> {
    let (i, (_tag, length, _tag2, _exif_version)) =
        tuple((tag(b"\xFF\xD8\xFF\xE1"), be_u16, tag(b"Exif"), be_u16))(jpeg_data)?;
    Ok((i, &i[..(length as usize - 2)]))
}

impl<'a> FileParts<'a> {
    fn from_offsets(data: &'a [u8], offsets: &Offsets) -> FileParts<'a> {
        let jpeg_data = offsets.jpeg.apply(data);
        // TODO: this is in a gross spot, and we shouldn't call unwrap.
        let (_, exif_tiff) = find_exif_tiff(jpeg_data).unwrap();
        FileParts {
            jpeg: jpeg_data,
            jpeg_exif_tiff: exif_tiff,
            metadata: offsets.metadata.apply(data),
            raw: offsets.raw.apply(data),
        }
    }
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

#[derive(Debug)]
pub enum Tag<'a> {
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
        0x0130 => Tag::Unknown3, Consistently 0C0C0C0C on every photo from my camera / samples
        0x0141 => Tag::Unknown4, Consistently 0x000E002A on every photo from my camera (14, 42)
        // dcraw: apparently something exposure related? midpointshift?
        // On *some* files (7371, 7375, 7723) these are FF540064.
        // On everything else, they're FFB80064.
        // That's a 100 different in the 2nd value.
        // Might be that dynamic range setting?
        0x9650 => Tag::Unknown5,
        */
        // This is a big block that nobody except Adobe knows how to parse.
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
    pub metadata: ImgMeta<'a>,
    tiffish: TiffishData,
}

impl<'a> ParsedRafFile<'a> {
    pub fn render_info(&self) -> RenderInfo {
        // Oh boy
        let mut xtrans: Vec<Color> = self
            .metadata
            .iter()
            .filter_map(|it| match it {
                XTransMapping(val) => Some(*val),
                _ => None,
            })
            .exactly_one()
            .unwrap()
            .iter()
            .map(|num| Color::from(*num as i8).unwrap())
            .collect();
        // This is _backwards_ in the file.
        xtrans.reverse();
        RenderInfo {
            width: self.tiffish.width,
            height: self.tiffish.height,
            bit_depth: self.tiffish.bit_depth,
            black_levels: self.tiffish.black_levels.clone(),
            white_bal: self.tiffish.white_bal.clone(),
            xtrans_mapping: xtrans,
            raw_data: &self.tiffish.raw_data,
        }
    }

    pub fn vignette_attenuation(&self) -> &[SRational] {
        &self.tiffish.vignette_attenuation
    }
}

#[derive(Debug)]
struct TiffishData {
    width: Width,
    height: Height,
    bit_depth: u16,
    black_levels: Vec<u16>,
    white_bal: WhiteBalCoefficients,
    vignette_attenuation: Vec<SRational>,
    raw_data: Vec<u16>,
}

pub struct RenderInfo<'a> {
    pub width: Width,
    pub height: Height,
    pub bit_depth: u16,
    pub black_levels: Vec<u16>,
    pub white_bal: WhiteBalCoefficients,
    pub xtrans_mapping: Vec<Color>,
    pub raw_data: &'a Vec<u16>,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct WhiteBalCoefficients {
    pub red: u16,
    pub green: u16,
    pub blue: u16,
}

// This almost _certainly_ doesn't represent the full spectrum of options.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum EncodingType {
    Compressed,
    Uncompressed,
    Unknown(u32),
}

impl From<u32> for EncodingType {
    fn from(val: u32) -> Self {
        match val {
            136 => Uncompressed,
            142 => Compressed,
            val => Unknown(val),
        }
    }
}

fn parse_tiffish(raw: &[u8]) -> IResult<I, TiffishData> {
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
    let img_byte_offset = hm[&61447].val_u32().unwrap() as usize;
    // 20743472 is this number, it's very large. 449024 is where the TIFF starts
    // 20743472 + 449024 = 21192496 ... is in middle of data, + 2048 is end of file.
    // it's the length (in bytes) of the data section.
    let img_byte_count = hm[&61448].val_u32().unwrap() as usize;
    let img_num_u16 = img_byte_count / 2;
    let img_encoding_type = EncodingType::from(hm[&61449].val_u32().unwrap());

    println!(
        "img is at {} and length {} with encoding {:?}",
        img_byte_offset, img_byte_count, img_encoding_type
    );

    let black_levels: Vec<u32> = tiff.load_offset_data(hm[&61450]).unwrap();
    let black_levels: Vec<u16> = black_levels.iter().map(|x| *x as u16).collect();
    println!("Black levels: {:#?}", black_levels);

    // No idea what this one is either; it's 8 numbers, looks wb related
    // because _52[0] and _52[4] == _53[0].
    let _52: Vec<u32> = tiff.load_offset_data(hm[&61452]).unwrap();
    println!("52: {:#?}", _52);

    // Note that tag 61454 had the same values on all the files I tested -
    // not sure what the difference is. DCRAW uses '54 and not '53.
    // Alright, on my COMPRESSED RAW FILE test (2827), '54 and '53 were the same
    // values. On the uncompressed test (6281) they're different, and '53 isn't right.
    let wb: Vec<u32> = tiff.load_offset_data(hm[&61454]).unwrap();
    let wb = WhiteBalCoefficients {
        // The order here in the RAF file is green, red, blue.
        // TODO: maybe this is similar to how TIFF does it?
        red: wb[1] as u16,
        green: wb[0] as u16,
        blue: wb[2] as u16,
    };

    let img_bytes = &raw[img_byte_offset..(img_byte_offset + img_byte_count)];

    let decode_result = match img_encoding_type {
        Compressed => fuji_compressed::load_fuji_compressed(img_bytes),
        _ => all_consuming(count(le_u16, img_num_u16))(img_bytes),
    };

    let (_, img_data) = decode_result?;

    // '51, '55, '56 all look like some kind of curve.
    // The first number looks like x/y axis lengths, then x positions, then y positions.
    let _51: Vec<SRational> = tiff.load_offset_data(hm[&61451]).unwrap();
    let _55: Vec<SRational> = tiff.load_offset_data(hm[&61455]).unwrap();
    let vignette_attentuation: Vec<SRational> = tiff.load_offset_data(hm[&61456]).unwrap();
    println!("51: {:?}", _51);
    println!("55: {:?}", _55);

    Ok((
        raw,
        TiffishData {
            width,
            height,
            bit_depth,
            black_levels,
            white_bal: wb,
            raw_data: img_data,
            vignette_attenuation: vignette_attentuation,
        },
    ))
}

fn parse_only_metadata(input: I) -> IResult<I, ImgMeta> {
    let (_, (_, offsets)) = tuple((header, offset_sizes))(input)?;
    let metadata = offsets.metadata.apply(input);
    let (i, metadata) = parse_metadata(metadata)?;
    Ok((i, metadata))
}

fn parse_all(input: I) -> IResult<I, ParsedRafFile> {
    let (_, (header, offsets)) = tuple((header, offset_sizes))(input)?;
    let jpg_preview = offsets.jpeg.apply(input);
    let metadata = offsets.metadata.apply(input);
    let raw = offsets.raw.apply(input);
    let (_, tiffish) = parse_tiffish(raw)?;
    let (i, metadata) = parse_metadata(metadata)?;
    Ok((
        i,
        ParsedRafFile {
            header,
            jpg_preview,
            metadata,
            tiffish,
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

    pub fn parse_meta(&self) -> Result<ImgMeta, RafError> {
        let result = parse_only_metadata(&self.mmap);
        match result {
            Ok((_, parsed)) => Ok(parsed),
            Err(_) => Err(RafError::Unknown),
        }
    }

    pub fn parse_raw(&self) -> Result<ParsedRafFile, RafError> {
        let result = parse_all(&self.mmap);
        match result {
            Ok((_, parsed)) => Ok(parsed),
            Err(_) => Err(RafError::Unknown),
        }
    }

    pub fn file_parts(&self) -> Result<FileParts, RafError> {
        let result = tuple((header, offset_sizes))(&self.mmap);
        match result {
            Ok((_, (_, offsets))) => Ok(FileParts::from_offsets(&self.mmap, &offsets)),
            Err(_) => Err(RafError::Unknown),
        }
    }
}
