use memmap::Mmap;
use nom::bytes::streaming::{tag, take};
use nom::multi::count;
use nom::number::complete::{be_u16, be_u32};
use nom::sequence::tuple;
use nom::IResult;
use std::fs::File;
use std::path::Path;

quick_error! {
    #[derive(Debug)]
    pub enum RafError {
        Io(err: std::io::Error) {
            from()
        }
    }
}

#[derive(Debug, Copy, Clone)]
struct Header<'a> {
    model: &'a str,
    fw_version: &'a str,
}

type I<'a> = &'a [u8];

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

#[derive(Debug)]
struct MetaTag<'a> {
    tag_num: u16,
    data: &'a [u8],
}

fn metadata_internal_tag(input: I) -> IResult<I, MetaTag> {
    let (input, (tag_num, len)) = tuple((be_u16, be_u16))(input)?;
    let (input, data) = take(len)(input)?;
    Ok((input, MetaTag { tag_num, data }))
}

type ImgMeta<'a> = Vec<MetaTag<'a>>;

fn parse_metadata(input: I) -> IResult<I, ImgMeta> {
    let (i, (_, meta_items_count)) = tuple((
        // I'm assuming this is offset+length again??
        tag(&hexlower!("0000")),
        be_u16,
    ))(input)?;

    let (i, meta) = count(metadata_internal_tag, meta_items_count as usize)(i)?;
    Ok((i, meta))
}

enum Tags {
    XTransMapping = 0x0131, //6x6 grid with the Xtrans mapping, 0-1-2s represent colors
    HeightWidthSensor = 0x0100,
    Unknown1 = 0x0110,         // Crop Top Left? According to Exiftool
    HeightWidthCrop = 0x0111,  // Raw Image cropped Size"
    HeightWidthCrop2 = 0x0112, // ???
    HeightWidthCrop3 = 0x0113, // ???
    Unknown2 = 0x0115,         // "Raw Image Aspect Ratio"
    Unknown3 = 0x0130,         // 0C0C0C0C
    Unknown4 = 0x0141,         // ???
    Unknown5 = 0x9650,         // FFB80064
    // Some of the data in here is _little endian_ flips of the Height / Width crop stuff. I don't know what's going on.
    // There's the prefix setting in there somewhere too (DSCF, ROFL on mine).
    // This contains a huge amount of data; it goes to the end of the META section.
    // According to ExifTool site, this is the RAFData. Not much info at all https://exiftool.org/TagNames/FujiFilm.html#RAFData.
    Unknown6 = 0xC000,
}

#[derive(Debug)]
struct RafFile<'a> {
    header: Header<'a>,
    jpg_preview: &'a [u8],
    metadata: ImgMeta<'a>,
    raw: &'a [u8],
}
fn parse_all(input: I) -> IResult<I, RafFile> {
    let (i, (header, offsets)) = tuple((header, offset_sizes))(input)?;
    println!("Offsets {:?}", offsets);
    let jpg_preview = offsets.jpeg.apply(input);
    let metadata = offsets.metadata.apply(input);
    let raw = offsets.raw.apply(input);
    let (i, metadata) = parse_metadata(metadata)?;
    Ok((
        i,
        RafFile {
            header,
            jpg_preview,
            metadata,
            raw,
        },
    ))
}

pub fn parse_raf<P>(path: P) -> Result<(), RafError>
where
    P: AsRef<Path>,
{
    let file = File::open(path)?;
    let mmap = unsafe { Mmap::map(&file) }?;
    let res = parse_all(&mmap);
    match res {
        Ok((_rest, info)) => {
            println!("{:?}", info.header);
            println!("{:?}", info.metadata);
        }
        Err(e) => println!("Something went wrong: {:?}", e),
    }
    Ok(())
}
