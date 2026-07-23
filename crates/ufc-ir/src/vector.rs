use crate::traits::{IntermediateRepresentation, ValidationError, ValidationSeverity};
use semver::Version;
use serde::{Deserialize, Serialize};

/// Vector Graphics Intermediate Representation.
///
/// Covers: SVG, EPS, AI (limited)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorIR {
    pub version: Version,
    pub width: f64,
    pub height: f64,
    pub viewBox: Option<ViewBox>,
    pub elements: Vec<VectorElement>,
    pub defs: Vec<Def>,
    pub metadata: VectorMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VectorElement {
    Path(Path),
    Rect(Rect),
    Circle(Circle),
    Ellipse(Ellipse),
    Line(Line),
    Polyline(Polyline),
    Polygon(Polygon),
    Text(TextElement),
    Group(Group),
    Image(VectorImageRef),
    Use(UseRef),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Path {
    pub d: String,
    pub fill: Option<Paint>,
    pub stroke: Option<Paint>,
    pub stroke_width: Option<f64>,
    pub opacity: Option<f64>,
    pub transform: Option<Transform>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub rx: Option<f64>,
    pub ry: Option<f64>,
    pub fill: Option<Paint>,
    pub stroke: Option<Paint>,
    pub stroke_width: Option<f64>,
    pub opacity: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Circle {
    pub cx: f64,
    pub cy: f64,
    pub r: f64,
    pub fill: Option<Paint>,
    pub stroke: Option<Paint>,
    pub stroke_width: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ellipse {
    pub cx: f64,
    pub cy: f64,
    pub rx: f64,
    pub ry: f64,
    pub fill: Option<Paint>,
    pub stroke: Option<Paint>,
    pub stroke_width: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Line {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
    pub stroke: Option<Paint>,
    pub stroke_width: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Polyline {
    pub points: Vec<(f64, f64)>,
    pub fill: Option<Paint>,
    pub stroke: Option<Paint>,
    pub stroke_width: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Polygon {
    pub points: Vec<(f64, f64)>,
    pub fill: Option<Paint>,
    pub stroke: Option<Paint>,
    pub stroke_width: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextElement {
    pub x: f64,
    pub y: f64,
    pub content: String,
    pub font_family: Option<String>,
    pub font_size: Option<f64>,
    pub font_weight: Option<u16>,
    pub fill: Option<Paint>,
    pub transform: Option<Transform>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub elements: Vec<VectorElement>,
    pub transform: Option<Transform>,
    pub opacity: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorImageRef {
    pub href: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UseRef {
    pub href: String,
    pub x: Option<f64>,
    pub y: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Paint {
    None,
    Color(String),
    Rgb(u8, u8, u8),
    Rgba(u8, u8, u8, u8),
    Url(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Transform {
    Matrix([f64; 6]),
    Translate(f64, f64),
    Scale(f64, f64),
    Rotate(f64),
    SkewX(f64),
    SkewY(f64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Def {
    LinearGradient {
        id: String,
        stops: Vec<GradientStop>,
        x1: f64, y1: f64, x2: f64, y2: f64,
    },
    RadialGradient {
        id: String,
        stops: Vec<GradientStop>,
        cx: f64, cy: f64, r: f64,
    },
    ClipPath {
        id: String,
        elements: Vec<VectorElement>,
    },
    Mask {
        id: String,
        elements: Vec<VectorElement>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradientStop {
    pub offset: f64,
    pub color: String,
    pub opacity: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorMetadata {
    pub title: Option<String>,
    pub description: Option<String>,
    pub creator: Option<String>,
}

impl VectorIR {
    pub fn new(width: f64, height: f64) -> Self {
        Self {
            version: crate::api_version(),
            width,
            height,
            viewBox: None,
            elements: Vec::new(),
            defs: Vec::new(),
            metadata: VectorMetadata {
                title: None,
                description: None,
                creator: None,
            },
        }
    }
}

impl IntermediateRepresentation for VectorIR {
    fn version(&self) -> Version { self.version.clone() }
    fn ir_type(&self) -> &'static str { "Vector" }
    fn memory_usage(&self) -> u64 {
        (self.elements.len() as u64 * 512) + 4096
    }
    fn validate(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        if self.width <= 0.0 || self.height <= 0.0 {
            errors.push(ValidationError {
                field: "dimensions".into(),
                message: "Width and height must be > 0".into(),
                severity: ValidationSeverity::Error,
            });
        }
        errors
    }
    fn to_json(&self) -> Result<String, serde_json::Error> { serde_json::to_string_pretty(self) }
    fn is_empty(&self) -> bool { self.elements.is_empty() }
}
