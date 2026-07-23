use crate::traits::{IntermediateRepresentation, ValidationError, ValidationSeverity};
use semver::Version;
use serde::{Deserialize, Serialize};

/// Image Intermediate Representation.
///
/// Covers: PNG, JPEG, WebP, BMP, TIFF, GIF, ICO, AVIF, HEIF, JPEG2000, QOI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageIR {
    pub version: Version,
    pub dimensions: Dimensions,
    pub color_space: ColorSpace,
    pub bit_depth: BitDepth,
    pub alpha: AlphaChannel,
    pub pixels: PixelData,
    pub metadata: ImageMetadata,
    pub layers: Option<Vec<Layer>>,
    pub animation: Option<Animation>,
    pub icc_profile: Option<Vec<u8>>,
    pub exif: Option<ExifData>,
    pub xmp: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dimensions {
    pub width: u32,
    pub height: u32,
    pub dpi_x: Option<f64>,
    pub dpi_y: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorSpace {
    Gray,
    GrayAlpha,
    Rgb,
    Rgba,
    Cmyk,
    YCbCr,
    Lab,
    Hsl,
    Hsv,
    Indexed { palette_size: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BitDepth {
    U1,
    U2,
    U4,
    U8,
    U16,
    U32,
    F16,
    F32,
}

impl BitDepth {
    pub fn bits_per_sample(&self) -> u8 {
        match self {
            Self::U1 => 1,
            Self::U2 => 2,
            Self::U4 => 4,
            Self::U8 => 8,
            Self::U16 => 16,
            Self::U32 => 32,
            Self::F16 => 16,
            Self::F32 => 32,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlphaChannel {
    None,
    Straight,
    Premultiplied,
}

/// Pixel storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PixelData {
    /// Raw interleaved pixel buffer (R,G,B,A,R,G,B,A,...)
    Raw(Vec<u8>),
    /// For very large images: tile-based storage
    Tiled {
        tile_width: u32,
        tile_height: u32,
        tiles: Vec<Tile>,
    },
    /// 16-bit per channel interleaved
    Raw16(Vec<u16>),
    /// 32-bit float per channel interleaved
    RawF32(Vec<f32>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tile {
    pub x: u32,
    pub y: u32,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageMetadata {
    pub format_name: String,
    pub format_version: Option<String>,
    pub has_transparency: bool,
    pub is_interlaced: bool,
    pub compression: Option<CompressionInfo>,
    pub color_count: Option<u32>,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionInfo {
    pub algorithm: String,
    pub level: Option<u32>,
    pub ratio: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer {
    pub name: String,
    pub visible: bool,
    pub opacity: f32,
    pub blend_mode: BlendMode,
    pub offset: (i32, i32),
    pub pixels: PixelData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
    Hue,
    Saturation,
    Color,
    Luminosity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Animation {
    pub frames: Vec<AnimationFrame>,
    pub loop_count: Option<u32>,
    pub default_delay_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationFrame {
    pub pixels: PixelData,
    pub delay_ms: u32,
    pub dispose_method: DisposeMethod,
    pub blend_method: BlendMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DisposeMethod {
    None,
    RestoreBackground,
    RestorePrevious,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BlendMethod {
    Source,
    Over,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExifData {
    pub make: Option<String>,
    pub model: Option<String>,
    pub software: Option<String>,
    pub datetime: Option<String>,
    pub exposure_time: Option<String>,
    pub f_number: Option<f64>,
    pub iso_speed: Option<u32>,
    pub focal_length: Option<f64>,
    pub orientation: Option<u16>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub gps: Option<GpsData>,
    pub custom: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpsData {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: Option<f64>,
}

// ─────────────────────────────────────────────
// Constructors
// ─────────────────────────────────────────────

impl ImageIR {
    /// Create a new ImageIR with the given dimensions and color space.
    pub fn new(width: u32, height: u32, color_space: ColorSpace, bit_depth: BitDepth) -> Self {
        Self {
            version: crate::api_version(),
            dimensions: Dimensions {
                width,
                height,
                dpi_x: None,
                dpi_y: None,
            },
            color_space,
            bit_depth,
            alpha: AlphaChannel::None,
            pixels: PixelData::Raw(Vec::new()),
            metadata: ImageMetadata {
                format_name: String::new(),
                format_version: None,
                has_transparency: false,
                is_interlaced: false,
                compression: None,
                color_count: None,
                comment: None,
            },
            layers: None,
            animation: None,
            icc_profile: None,
            exif: None,
            xmp: None,
        }
    }

    /// Get the number of channels for the current color space.
    pub fn channels(&self) -> u8 {
        match &self.color_space {
            ColorSpace::Gray => 1,
            ColorSpace::GrayAlpha => 2,
            ColorSpace::Rgb => 3,
            ColorSpace::Rgba => 4,
            ColorSpace::Cmyk => 4,
            ColorSpace::YCbCr => 3,
            ColorSpace::Lab => 3,
            ColorSpace::Hsl => 3,
            ColorSpace::Hsv => 3,
            ColorSpace::Indexed { .. } => 1,
        }
    }

    /// Get the total number of pixels.
    pub fn pixel_count(&self) -> u64 {
        self.dimensions.width as u64 * self.dimensions.height as u64
    }

    /// Get the expected raw data size in bytes (interleaved, uncompressed).
    pub fn expected_data_size(&self) -> u64 {
        let bpp = self.bit_depth.bits_per_sample() as u64 * self.channels() as u64;
        (self.pixel_count() * bpp + 7) / 8
    }
}

impl IntermediateRepresentation for ImageIR {
    fn version(&self) -> Version {
        self.version.clone()
    }

    fn ir_type(&self) -> &'static str {
        "Image"
    }

    fn memory_usage(&self) -> u64 {
        let pixel_bytes = match &self.pixels {
            PixelData::Raw(d) => d.len() as u64,
            PixelData::Raw16(d) => d.len() as u64 * 2,
            PixelData::RawF32(d) => d.len() as u64 * 4,
            PixelData::Tiled { tiles, .. } => tiles.iter().map(|t| t.data.len() as u64).sum(),
        };
        let layer_bytes = self
            .layers
            .as_ref()
            .map(|layers| {
                layers
                    .iter()
                    .map(|l| match &l.pixels {
                        PixelData::Raw(d) => d.len() as u64,
                        PixelData::Raw16(d) => d.len() as u64 * 2,
                        PixelData::RawF32(d) => d.len() as u64 * 4,
                        PixelData::Tiled { tiles, .. } => {
                            tiles.iter().map(|t| t.data.len() as u64).sum()
                        }
                    })
                    .sum::<u64>()
            })
            .unwrap_or(0);
        let anim_bytes = self
            .animation
            .as_ref()
            .map(|a| {
                a.frames
                    .iter()
                    .map(|f| match &f.pixels {
                        PixelData::Raw(d) => d.len() as u64,
                        PixelData::Raw16(d) => d.len() as u64 * 2,
                        PixelData::RawF32(d) => d.len() as u64 * 4,
                        PixelData::Tiled { tiles, .. } => {
                            tiles.iter().map(|t| t.data.len() as u64).sum()
                        }
                    })
                    .sum::<u64>()
            })
            .unwrap_or(0);
        let icc_bytes = self.icc_profile.as_ref().map(|p| p.len() as u64).unwrap_or(0);
        pixel_bytes + layer_bytes + anim_bytes + icc_bytes + 1024 // +1KB for metadata overhead
    }

    fn validate(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        if self.dimensions.width == 0 {
            errors.push(ValidationError {
                field: "dimensions.width".into(),
                message: "Width must be > 0".into(),
                severity: ValidationSeverity::Error,
            });
        }
        if self.dimensions.height == 0 {
            errors.push(ValidationError {
                field: "dimensions.height".into(),
                message: "Height must be > 0".into(),
                severity: ValidationSeverity::Error,
            });
        }
        if self.dimensions.width > 65535 || self.dimensions.height > 65535 {
            errors.push(ValidationError {
                field: "dimensions".into(),
                message: "Dimensions exceed maximum (65535x65535)".into(),
                severity: ValidationSeverity::Warning,
            });
        }
        if let Some(anim) = &self.animation {
            if anim.frames.is_empty() {
                errors.push(ValidationError {
                    field: "animation.frames".into(),
                    message: "Animation has no frames".into(),
                    severity: ValidationSeverity::Error,
                });
            }
        }

        errors
    }

    fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    fn is_empty(&self) -> bool {
        match &self.pixels {
            PixelData::Raw(d) => d.is_empty(),
            PixelData::Raw16(d) => d.is_empty(),
            PixelData::RawF32(d) => d.is_empty(),
            PixelData::Tiled { tiles, .. } => tiles.is_empty(),
        }
    }
}
