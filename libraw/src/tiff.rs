use itertools::Itertools;
use nom::bytes::streaming::{tag, take};
use nom::combinator::map;
use nom::multi::count;
use nom::number::complete::{le_i32, le_u16, le_u32};
use nom::sequence::tuple;
use nom::IResult;
use std::convert::TryInto;
use std::marker::PhantomData;
use tristate::TriState;

pub type I<'a> = &'a [u8];

pub struct TiffFile<'a> {
    pub ifds: Vec<Ifd<'a>>,
    data: &'a [u8],
}

#[derive(Debug, PartialEq)]
pub struct IfdEntry<'a> {
    pub tag: u16,
    pub field_type: FieldType,
    pub count: u32,
    // This one is wild and requires some explanation:
    // TODO: document
    pub value_offset: &'a [u8; 4],
}

#[derive(Debug)]
pub struct TypedIfdEntry<'a, T> {
    pub tag: u16,
    pub field_type: FieldType,
    pub value: &'a [T],
    // private thing just to prevent instantiation
    p: PhantomData<T>,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum FieldType {
    Byte,
    Ascii,
    Short,
    Long,
    Rational,
    SByte,
    Undefined,
    SShort,
    SLong,
    SRational,
    Float,
    Double,
    Unknown(u16),
}

impl FieldType {
    fn type_size(&self) -> Option<usize> {
        match self {
            FieldType::Byte => Some(1),
            FieldType::Ascii => Some(1),
            FieldType::Short => Some(2),
            FieldType::Long => Some(4),
            FieldType::Rational => Some(8),
            FieldType::SByte => Some(1),
            FieldType::Undefined => Some(1),
            FieldType::SShort => Some(2),
            FieldType::SLong => Some(4),
            FieldType::SRational => Some(8),
            FieldType::Float => Some(4),
            FieldType::Double => Some(8),
            FieldType::Unknown(_) => None,
        }
    }

    fn debug_repr(&self, data: &[u8]) -> String {
        match self {
            FieldType::Byte | FieldType::SByte | FieldType::Undefined | FieldType::Unknown(_) => {
                // Treat as bytes
                let mut data = hex::encode(data);
                data.make_ascii_uppercase();
                let data = data
                    .as_bytes()
                    .chunks(64)
                    .map(|chunk| {
                        chunk
                            .chunks(8)
                            .map(|chunk| std::str::from_utf8(chunk).unwrap())
                            .join(" ")
                    })
                    .join("\n");
                data
            }
            FieldType::Ascii => {
                // Treat as string
                String::from_utf8(data.to_vec()).unwrap()
            }
            FieldType::Short
            | FieldType::Long
            | FieldType::SShort
            | FieldType::SLong
            | FieldType::Float
            | FieldType::Double
            | FieldType::SRational
            | FieldType::Rational => {
                let chunks = data.chunks_exact(self.type_size().unwrap());
                assert_eq!(chunks.remainder().len(), 0);
                if chunks.len() > 1 {
                    format!("[{}]", chunks.map(|x| self.debug_repr_single(x)).join(", "))
                } else {
                    chunks
                        .map(|x| self.debug_repr_single(x))
                        .exactly_one()
                        .unwrap()
                }
            }
        }
    }

    fn debug_repr_single(&self, data: &[u8]) -> String {
        // Might not need this assertion, because we effectively do it in
        // the try_intos.
        assert_eq!(self.type_size().unwrap(), data.len());
        match self {
            FieldType::Short => format!("{}", u16::from_le_bytes(data.try_into().unwrap())),
            FieldType::Long => format!("{}", u32::from_le_bytes(data.try_into().unwrap())),
            FieldType::Rational => {
                let first = u32::from_le_bytes(data[..4].try_into().unwrap());
                let second = u32::from_le_bytes(data[4..].try_into().unwrap());
                format!("{}/{}", first, second)
            }
            FieldType::SShort => format!("{}", i16::from_le_bytes(data.try_into().unwrap())),
            FieldType::SLong => format!("{}", i32::from_le_bytes(data.try_into().unwrap())),
            FieldType::SRational => {
                let first = i32::from_le_bytes(data[..4].try_into().unwrap());
                let second = i32::from_le_bytes(data[4..].try_into().unwrap());
                format!("{}/{}", first, second)
            }
            FieldType::Float => format!("{}", f32::from_le_bytes(data.try_into().unwrap())),
            FieldType::Double => format!("{}", f64::from_le_bytes(data.try_into().unwrap())),
            _ => unreachable!(),
        }
    }
}

impl From<u16> for FieldType {
    fn from(val: u16) -> Self {
        match val {
            1 => FieldType::Byte,
            2 => FieldType::Ascii,
            3 => FieldType::Short,
            4 => FieldType::Long,
            5 => FieldType::Rational,
            6 => FieldType::SByte,
            7 => FieldType::Undefined,
            8 => FieldType::SShort,
            9 => FieldType::SLong,
            10 => FieldType::SRational,
            11 => FieldType::Float,
            12 => FieldType::Double,
            val => FieldType::Unknown(val),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Rational(u32, u32);

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct SRational(i32, i32);

pub trait Parseable: Sized {
    fn type_matches(t: FieldType) -> bool;
    fn parse(input: I, count: usize) -> Vec<Self>;
}

impl Parseable for u32 {
    fn type_matches(t: FieldType) -> bool {
        match t {
            FieldType::Long => true,
            _ => false,
        }
    }

    fn parse(input: &[u8], c: usize) -> Vec<Self> {
        let res: IResult<I, Vec<u32>> = count(le_u32, c)(input);
        let (_, val) = res.unwrap();
        val
    }
}

impl Parseable for SRational {
    fn type_matches(t: FieldType) -> bool {
        t == FieldType::SRational
    }

    fn parse(input: &[u8], c: usize) -> Vec<Self> {
        let res: IResult<I, Vec<SRational>> =
            count(map(tuple((le_i32, le_i32)), |(a, b)| SRational(a, b)), c)(input);
        let (_, val) = res.unwrap();
        val
    }
}

impl<'a> IfdEntry<'a> {
    pub fn value_byte_size(&self) -> Option<usize> {
        let item_size = self.field_type.type_size()?;
        Some(item_size * self.count as usize)
    }

    pub fn value_inlined(&self) -> TriState {
        match self.value_byte_size() {
            Some(size_) if size_ > 4 => TriState::No,
            Some(_) => TriState::Yes,
            None => TriState::Unknown,
        }
    }

    fn parse<T: Parseable>(&self, input: I) -> Option<Vec<T>> {
        if !T::type_matches(self.field_type) {
            return None;
        }
        Some(T::parse(input, self.count as usize))
    }

    pub fn val_u32(&self) -> Option<u32> {
        // Should probably do errors if this isn't right, rather than asserting
        match self.field_type {
            FieldType::Byte | FieldType::Short | FieldType::Long | FieldType::Unknown(_) => {}
            _ => return None,
        };
        if self.count != 1 {
            return None;
        }
        Some(u32::from_le_bytes(*self.value_offset))
    }

    pub fn val_as_offset(&self) -> Option<usize> {
        if self.value_inlined() == TriState::Yes {
            None
        } else {
            // We don't use usize directly because the size changes on
            // different platforms, and as per TIFF spec this is u32.
            Some(u32::from_le_bytes(*self.value_offset) as usize)
        }
    }

    fn load_from_offset<T: Parseable>(&self, input: I) -> Option<Vec<T>> {
        let offset = self.val_as_offset()?;
        self.parse(&input[offset..])
    }
}

impl<'a> TiffFile<'a> {
    pub fn load_offset_data<T: Parseable>(&self, ifd_entry: &IfdEntry<'a>) -> Option<Vec<T>> {
        ifd_entry.load_from_offset(self.data)
    }

    pub fn data_for_ifd_entry(&self, ifd_entry: &'a IfdEntry) -> &'a [u8] {
        let byte_size = ifd_entry.count as usize * ifd_entry.field_type.type_size().unwrap_or(1);
        if byte_size <= 4 {
            &ifd_entry.value_offset[0..byte_size]
        } else {
            let start = ifd_entry.val_as_offset().unwrap();
            let end = start + byte_size;
            &self.data[start..end]
        }
    }

    pub fn debug_value_for_ifd_entry(&self, ifd: &IfdEntry) -> String {
        // the unwrap_or effectively treats Unknowns as 1
        let data = self.data_for_ifd_entry(ifd);
        ifd.field_type.debug_repr(data)
    }
}

pub type Ifd<'a> = Vec<IfdEntry<'a>>;

fn ifd_entry(input: I) -> IResult<I, IfdEntry> {
    map(
        tuple((le_u16, le_u16, le_u32, take(4usize))),
        |(tag, field_type, count, value_offset)| IfdEntry {
            tag,
            field_type: FieldType::from(field_type),
            count,
            // No idea if this is a good idea; maybe it results in a lot of copies?
            value_offset: value_offset.try_into().unwrap(),
        },
    )(input)
}

pub fn parse_ifd(input: I) -> IResult<I, (Ifd, Option<usize>)> {
    let (input, num_fields) = le_u16(input)?;
    tuple((
        count(ifd_entry, num_fields as usize),
        map(le_u32, |x| if x != 0 { Some(x as usize) } else { None }),
    ))(input)
}

pub fn parse_tiff(input: I) -> IResult<I, TiffFile> {
    let (_i, (_tag, first_ifd_offset)) = tuple((tag(b"II*\0"), le_u32))(input)?;
    let mut ifds = Vec::new();
    let mut ifd_offset = first_ifd_offset as usize;
    loop {
        // relative to base of TIFF file
        let ifd_input = &input[(ifd_offset as usize)..];
        let (_, (ifd, next_ifd)) = parse_ifd(ifd_input)?;
        ifds.push(ifd);
        if let Some(ifd) = next_ifd {
            ifd_offset = ifd;
        } else {
            break;
        }
    }

    Ok((input, TiffFile { ifds, data: input }))
}

#[cfg(test)]
mod tests {
    use crate::tiff::{parse_ifd, parse_tiff, FieldType, IfdEntry};

    #[test]
    fn test_raf_tiff_header() {
        let data = include_bytes!("../res/6281.tiff.dat");
        let result = parse_tiff(data);
        assert!(result.is_ok());
        let (_, result) = result.unwrap();
        assert_eq!(result.ifds.len(), 1);
        let ifd = &result.ifds[0];
        assert_eq!(ifd.len(), 1);
        let ifde = &ifd[0];
        assert_eq!(ifde.tag, 0xF000);
        assert_eq!(ifde.field_type, FieldType::Unknown(13));
        assert_eq!(ifde.count, 1);
        assert_eq!(ifde.value_offset, b"\x1A\0\0\0");
    }

    /// This method returns the value encoded as big-endian. Note that what we're
    /// testing is often little-endian! But in this case, we want to be able to copy-paste hex
    /// bytes and prefix with 0x, so we use big-endian no matter what.
    fn h(val: u32) -> [u8; 4] {
        val.to_be_bytes()
    }

    #[test]
    fn test_fuji_tiff_block() {
        let data = include_bytes!("../res/6281_fuji_custom_ifd.tiff.dat");
        let result = parse_ifd(data);
        let (_, (ifd, next)) = result.unwrap();

        // no more IFDs
        assert_eq!(next, None);
        // Should be 16 thingies
        assert_eq!(ifd.len(), 16);
        // Three boring ones
        assert_eq!(
            ifd[0],
            IfdEntry {
                tag: 0xF001,
                field_type: FieldType::Long,
                count: 1,
                value_offset: &h(0x10180000),
            }
        );
        assert_eq!(
            ifd[1],
            IfdEntry {
                tag: 0xF002,
                field_type: FieldType::Long,
                count: 1,
                value_offset: &h(0xC00F0000),
            }
        );
        assert_eq!(
            ifd[2],
            IfdEntry {
                tag: 0xF003,
                field_type: FieldType::Long,
                count: 1,
                value_offset: &h(0x0E000000),
            }
        );
        // Different count
        assert_eq!(
            ifd[9],
            IfdEntry {
                tag: 0xF00A,
                field_type: FieldType::Long,
                count: 36,
                // This is an offset
                value_offset: &h(0xE0000000),
            }
        );
        // Different count + type
        assert_eq!(
            ifd[10],
            IfdEntry {
                tag: 0xF00B,
                field_type: FieldType::SRational,
                count: 23,
                // This one too?
                value_offset: &h(0x70010000),
            }
        );
    }
}
