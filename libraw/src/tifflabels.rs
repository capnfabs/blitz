#[derive(Debug, Clone, Copy)]
pub enum TagContext {
    Exif,
    ExifMakerNotes,
    FujiRaw,
}

const NOEXIST: &'static str = "SENTINEL_NOEXIST";

pub fn label_for_tag(context: TagContext, tag_id: u16) -> Option<&'static str> {
    let func = match context {
        TagContext::Exif => label_for_exif_field,
        TagContext::ExifMakerNotes => label_for_maker_notes,
        TagContext::FujiRaw => label_for_raw,
    };
    let label = func(tag_id);
    if label != NOEXIST {
        Some(label)
    } else {
        None
    }
}

fn label_for_raw(id: u16) -> &'static str {
    match id {
        0xF000 => "Fuji RAW Section Pointer",
        0xF001 => "FujiRafWidth",
        0xF002 => "FujiRafHeight",
        0xF003 => "FujiRafBitsPerPixel",
        // F004-F006 are present in my RAF files, but they're all zeros, and
        // exiftool doesn't know about them.
        0xF004 => "[reserved]",
        0xF005 => "[reserved]",
        0xF006 => "[reserved]",
        0xF007 => "FujiRafRawDataOffset",
        0xF008 => "FujiRafRawDataLength", // Bytes
        0xF009 => "FujiRafRawEncodingType",
        0xF00A => "FujiRafBlackLevelPattern",
        0xF00B => "FujiRafGeometricDistortionParams", // according to exiftool
        // These are in the form [Green Red Blue EXIF_LIGHT_SOURCE_CODE]
        // See https://www.awaresystems.be/imaging/tiff/tifftags/privateifd/exif/lightsource.html
        // Notably, 17 and 21 are Standard Light A and D65
        0xF00C => "[Maybe]FujiRafColorCalibration",
        0xF00D => "FujiRafWhiteBalCoefficentsAuto",
        // I *think* these are user-selected white bal coefficients, but I'm not sure.
        0xF00E => "FujiRafWhiteBalCoefficentsSelected",
        0xF00F => "FujiRafChromaticAberrationParams", // according to Exiftool
        0xF010 => "FujiRafVignetteProfile",
        _ => NOEXIST,
    }
}

fn label_for_maker_notes(id: u16) -> &'static str {
    match id {
        0x0000 => "Version",
        0x0010 => "Internal Serial Number",
        0x1000 => "Quality",
        0x1001 => "Sharpness",
        0x1002 => "White Balance",
        0x1003 => "Saturation",
        0x100a => "White Balance Fine Tune",
        0x100e => "Noise Reduction",
        0x1010 => "Fuji Flash Mode",
        0x1011 => "Flash Exposure Comp",
        0x1021 => "Focus Mode",
        0x1022 => "AF Mode",
        0x1023 => "Focus Pixel",
        // 0x1026 => "", ?
        // Focus Priority Settings, packed into 4 bit fields
        // Fujifilm.pm: 882
        0x102B => "Focus / Shutter Priority Settings",
        // 0x102C => "", // Unknown
        // More Focus Settings, see FujiFilm.pm:907
        0x102D => "Focus Settings",
        // FujiFilm.pm:957
        0x102E => "Continuous Autofocus Settings",
        0x1030 => "Slow Sync",
        0x1031 => "Picture Mode",
        0x1032 => "Exposure Count",
        0x1040 => "Shadow Tone",
        0x1041 => "Highlight Tone",
        0x1045 => "Lens Modulation Optimizer",
        // 0x1046 => "",
        0x1047 => "Grain Effect",
        0x1050 => "Shutter Type",
        0x1100 => "Auto Bracketing",
        0x1101 => "Sequence Number",
        0x1103 => "Drive Mode Settings",
        // 0x1200 => "",
        0x1300 => "Blur Warning",
        0x1301 => "Focus Warning",
        0x1302 => "Exposure Warning",
        // 0x1303 => "",
        // 0x1304 => "",
        // 0x1305 => "",
        0x1400 => "Dynamic Range",
        0x1401 => "Film Mode",
        0x1402 => "Dynamic Range Setting",
        0x1404 => "Min Focal Length",
        0x1405 => "Max Focal Length",
        0x1406 => "Max Aperture At Min Focal",
        0x1407 => "Max Aperture At Max Focal",
        // 0x1408 => "",
        // 0x1409 => "",
        // 0x140A => "",
        0x140b => "Auto Dynamic Range",
        0x1422 => "Image Stabilization",
        // 0x1424 => "",
        // 0x1430 => "",
        0x1431 => "Rating",
        0x1436 => "Image Generation",
        0x1438 => "Image Count",
        // 0x1439 => "",
        0x1446 => "Flicker Reduction",
        0x4100 => "Faces Detected",
        0x4200 => "Num Face Elements",
        0x0008 => "Raw Image Width",
        0x000c => "Raw Image Height",
        _ => NOEXIST,
    }
}

fn label_for_exif_field(id: u16) -> &'static str {
    match id {
        0x103 => "Compression",
        0x10F => "Make",
        0x110 => "Model",
        0x112 => "Orientation", // Enum
        0x11A => "XResolution",
        0x11B => "YResolution",
        0x128 => "ResolutionUnit", // Enum
        0x131 => "Software",       // On Fuji, also firmware version
        0x132 => "DateTime",
        0x13B => "Artist",
        0x213 => "YCbCrPositioning",
        0x8298 => "Copyright",
        0x8769 => "Exif IFD Pointer",
        0xC4A5 => "?? PrintIM", // Starts with Magic String PrintIM, exiftool doesn't know what it is either
        0x829A => "ExposureTime",
        0x829D => "FNumber",
        0x8822 => "ExposureProgram",
        0x8827 => "PhotographicSensitivity",
        0x8830 => "SensitivityType",
        0x9000 => "ExifVersion",
        0x9003 => "DateTimeOriginal",
        0x9004 => "DateTimeDigitized",
        0x9101 => "ComponentsConfiguration",
        0x9102 => "CompressedBitsPerPixel",
        0x9201 => "ShutterSpeedValue",
        0x9202 => "ApertureValue",
        0x9203 => "BrightnessValue",
        0x9204 => "ExposureBiasValue",
        0x9205 => "MaxApertureValue",
        0x9207 => "MeteringMode",
        0x9208 => "LightSource",
        0x9209 => "Flash",
        0x920A => "FocalLength",
        0x927C => "MakerNote",
        0x9286 => "UserComment",
        0xA000 => "FlashpixVersion",
        0xA001 => "ColorSpace",
        0xA002 => "PixelXDimension",
        0xA003 => "PixelYDimension",
        0xA005 => "Interoperability IFD Pointer",
        0xA20E => "FocalPlaneXResolution",
        0xA20F => "FocalPlaneYResolution",
        0xA210 => "FocalPlaneResolutionUnit",
        0xA217 => "SensingMethod",
        0xA300 => "FileSource",
        0xA301 => "SceneType",
        0xA401 => "CustomRendered",
        0xA402 => "ExposureMode",
        0xA403 => "WhiteBalance",
        0xA405 => "FocalLengthIn35mmFilm",
        0xA406 => "SceneCaptureType",
        0xA40A => "Sharpness",
        0xA40C => "SubjectDistanceRange",
        0xA431 => "BodySerialNumber",
        0xA432 => "LensSpecification",
        0xA433 => "LensMake",
        0xA434 => "LensModel",
        0xA435 => "LensSerialNumber",
        // as per Exiftool, "Thumbnail Offset", but this is only true on IFD 1 in the Exif
        0x0201 => "JPEGInterchangeFormat", // from Exif spec
        // as per Exiftool, "Thumbnail Length", but this is only true on IFD 1 in the Exif
        0x0202 => "JPEGInterchangeFormatLength", // from Exif spec
        _ => NOEXIST,
    }
}
