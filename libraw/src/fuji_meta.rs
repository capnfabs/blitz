use crate::raf::RafFile;
use crate::tiff;
use crate::tiff::{parse_tiff_with_options, IfdEntry, MAKERNOTES_TAG_ID};
use itertools::Itertools;
use num_traits::FromPrimitive;
use std::collections::HashMap;
use std::error::Error;

#[derive(FromPrimitive, ToPrimitive, Copy, Clone, Debug)]
pub enum FocusPriority {
    ShutterRelease = 1,
    Focus = 2,
}

#[derive(FromPrimitive, ToPrimitive, Copy, Clone, Debug)]
pub enum FocusMode {
    Manual = 0,
    SingleAuto = 1,
    ContinuousAuto = 2,
    AfsAuto = 0x11, //?? don't know what this means
}

#[derive(FromPrimitive, ToPrimitive, Copy, Clone, Debug)]
pub enum AutofocusAreaMode {
    SinglePoint = 0,
    Zone = 1,
    WideTracking = 2,
}

#[derive(Clone, Debug)]
pub struct FocusInfo {
    afs_priority: FocusPriority,
    afc_priority: FocusPriority,
    focus_mode: FocusMode,
    focus_region_mode: AutofocusAreaMode,
    focus_area_point_size: u16,
    focus_area_zone_size: u16,
}

fn get_field_val(hm: &HashMap<u16, &IfdEntry>, tag: u16) -> Result<u32, String> {
    let entry = hm.get(&tag).ok_or(format!("Missing tag 0x{:4X}", tag))?;
    let val = entry.val_u32().ok_or("Invalid 0x102b")?;
    Ok(val)
}

pub fn load_focus_info(raf_file: &RafFile) -> Result<FocusInfo, Box<dyn Error + '_>> {
    println!("Loading focus info");
    let exif_bytes = raf_file.file_parts()?.jpeg_exif_tiff;
    println!("Got exif");
    let (_, tiff) = tiff::parse_tiff_with_options(exif_bytes, b"II*\0", true)?;
    let makernotes = tiff
        .all_fields()
        .filter(|tag| tag.tag == MAKERNOTES_TAG_ID)
        .exactly_one()
        .ok()
        .ok_or("Couldn't find exactly one MakerNotes field.")?;
    let makernotes_content = tiff.data_for_ifd_entry(makernotes);
    let (_, makernotes_tiff) =
        parse_tiff_with_options(&makernotes_content, b"FUJIFILM", false).unwrap();

    let hm: HashMap<u16, &IfdEntry> = makernotes_tiff
        .all_fields()
        .map(|val| (val.tag, val))
        .collect();
    println!("Got hashmap: {:?}", hm);
    let priority = get_field_val(&hm, 0x102b)?;
    let settings = get_field_val(&hm, 0x102d)?;
    let afc = get_field_val(&hm, 0x102e)?;
    let afs_priority = FromPrimitive::from_u32(priority & 0x000F).ok_or(format!(
        "Focus Priority {} Mapped to Unknown Value",
        priority
    ))?;
    let afc_priority = FromPrimitive::from_u32((priority & 0x00F0) >> 4).ok_or(format!(
        "Focus Priority {} Mapped to Unknown Value",
        priority
    ))?;
    let focus_mode =
        FromPrimitive::from_u32(settings & 0xFF).ok_or("Focus Mode Mapped to Unknown Value")?;
    let focus_region_mode = FromPrimitive::from_u32((settings & 0xFF00) >> 8)
        .ok_or("Focus Region Mode Mapped to Unknown Value")?;
    let focus_area_point_size = ((settings & 0xF000) >> 12) as u16;
    // TODO: check this
    let focus_area_zone_size = ((settings & 0xF0000) >> 16) as u16;

    // TODO: focus pixels. Requires some refactoring.

    Ok(FocusInfo {
        afs_priority,
        afc_priority,
        focus_mode,
        focus_region_mode,
        focus_area_point_size,
        focus_area_zone_size,
    })
}
