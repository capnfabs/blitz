#[derive(Debug, Clone, Copy)]
pub enum TagPath {
    PreviewJpeg,
    PreviewExif,
    PreviewExifMakerNotes,
    Raw,
}

const NOEXIST: &'static str = "SENTINEL_NOEXIST";

pub fn label_for_tag(context: TagPath, tag_id: u16) -> Option<&'static str> {
    let func = match context {
        TagPath::PreviewJpeg => label_for_jpeg_tag,
        TagPath::PreviewExif => label_for_exif_field,
        TagPath::PreviewExifMakerNotes => label_for_maker_notes,
        TagPath::Raw => label_for_raw,
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
        // -----------
        // !!SubIFD from tag F000!!
        0xF001 => "FujiRafWidth",
        0xF002 => "FujiRafHeight",
        0xF003 => "FujiRafBitsPerPixel",
        0xF004 => "", // ??
        0xF005 => "", // ?
        0xF006 => "", // ?
        0xF007 => "FujiRafRawDataOffset",
        0xF008 => "FujiRafRawDataLength", // Bytes
        0xF009 => "FujiRafRawEncodingType",
        0xF00A => "FujiRafBlackLevelPattern",
        0xF00B => "??FujiRafSomeUnidentifiedCurve51", // ??
        0xF00C => "[Maybe]FujiRafColorCalibration",
        0xF00D => "[Maybe]FujiRafWhiteBalCoefficients1",
        0xF00E => "[Maybe]FujiRafWhiteBalCoefficients2",
        0xF00F => "??FujiRafSomeUnidentifiedCurve55",
        0xF010 => "FujiRafVignetteProfile",
        _ => NOEXIST,
    }
}

fn label_for_maker_notes(id: u16) -> &'static str {
    "TODO"
}

fn label_for_jpeg_tag(id: u16) -> &'static str {
    match id {
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
        0xC4A5 => "", // ??
        _ => NOEXIST,
    }
}

fn label_for_exif_field(id: u16) -> &'static str {
    match id {
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
        _ => NOEXIST,
    }
}
