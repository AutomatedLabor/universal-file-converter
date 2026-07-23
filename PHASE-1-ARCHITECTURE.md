# Universal File Converter — Phase 1: Architecture & Design

---

## 1. REQUIREMENTS ANALYSIS

### 1.1 Format Priority Confirmation

The proposed tiering is sound. Refined with rationale:

| Tier | Categories | Rationale |
|------|-----------|-----------|
| 1 | Documents, Images, Audio, Video, Archives | Highest daily-driver demand; mature open-source codec libraries exist |
| 2 | eBooks, Office, Vector Graphics, Fonts, Structured Data | Professional workflows; some require complex parsing (DOCX, EPUB) |
| 3 | Subtitles, CAD, 3D, Databases, Config/Logs, Source Code | Niche but valuable; lower fidelity expectations from users |

**Adjustment:** Move Structured Data (CSV/JSON/XML/HTML/Markdown) from Tier 2 to Tier 1. These formats are trivially parseable, universally needed, and serve as excellent first plugin targets for validating the IR pipeline.

### 1.2 Top 5 Technical Risks & Mitigations

| # | Risk | Severity | Mitigation |
|---|------|----------|------------|
| 1 | **Format fidelity loss** — Complex formats (DOCX, PDF, PSD) have undocumented behaviors | High | Golden-file test suite per format; fidelity score per plugin; user-visible capability declarations |
| 2 | **Plugin sandboxing overhead** — Process isolation adds latency; WASM has limited system access | High | Benchmark both approaches early (Milestone 1); hybrid model: WASM for simple formats, process isolation for complex ones needing native libs |
| 3 | **Native dependency hell** — FFmpeg, ImageMagick, LibreOffice, Poppler have version/platform quirks | High | Bundle static builds; containerize build CI; fallback to pure-Rust implementations where possible (image, pdf crate) |
| 4 | **Memory pressure on large files** — Video/RAW images can be GB-scale | Medium | Streaming pipeline design from day 1; configurable chunk sizes; memory-mapped I/O for reads; backpressure signals in the conversion graph |
| 5 | **IR design brittleness** — An IR that's too narrow forces lossy conversions; too broad becomes unmaintainable | Medium | Design IRs per domain (not one universal IR); version each IR schema; explicit lossy/lossless annotations on every field |

### 1.3 Assumptions

| # | Assumption | Justification |
|---|-----------|---------------|
| 1 | Target platforms: Windows 10+, macOS 13+, Ubuntu 22.04+ | Covers >95% of professional desktop users |
| 2 | Max single file size: 10 GB | Beyond this, users should use specialized tools; streaming handles this |
| 3 | No real-time conversion (sub-second latency not required) | File conversion is inherently batch-oriented |
| 4 | Users can install system-level dependencies if needed | Some codecs (H.265, AV1) may require runtime libs |
| 5 | Plugin authors are at least semi-technical | Plugin SDK targets Rust developers; GUI plugin install is Tier 3 |

---

## 2. ARCHITECTURE DESIGN

### 2.1 High-Level System Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                          UI Layer (Tauri)                           │
│  ┌──────────┐ ┌──────────┐ ┌───────────┐ ┌──────────┐ ┌─────────┐ │
│  │ DropZone │ │ Queue    │ │ Progress  │ │ Settings │ │ Plugin  │ │
│  │          │ │ Manager  │ │ Tracker   │ │          │ │ Manager │ │
│  └────┬─────┘ └────┬─────┘ └─────┬─────┘ └────┬─────┘ └────┬────┘ │
│       └─────────────┴─────────────┴─────────────┴────────────┘     │
│                              │ IPC (Tauri commands)                 │
├──────────────────────────────┼──────────────────────────────────────┤
│                     Core Engine (Rust)                              │
│  ┌───────────────────────────┼──────────────────────────────────┐   │
│  │                    Orchestrator                               │   │
│  │  ┌──────────┐ ┌──────────┼──────────┐ ┌───────────────────┐ │   │
│  │  │ Format   │ │ Conversion          │ │ State Manager     │ │   │
│  │  │ Detector │ │ Router  │          │ │ (queue, progress, │ │   │
│  │  └──────────┘ │         │          │ │  pause/resume)    │ │   │
│  │               │  ┌──────┴──────┐   │ └───────────────────┘ │   │
│  │               │  │ DAG Solver  │   │                       │   │
│  │               │  │ (shortest   │   │ ┌───────────────────┐ │   │
│  │               │  │  IR path)   │   │ │ Integrity Checker │ │   │
│  │               │  └──────┬──────┘   │ │ (checksums)       │ │   │
│  │               │         │          │ └───────────────────┘ │   │
│  │               └─────────┼──────────┘                       │   │
│  └─────────────────────────┼──────────────────────────────────┘   │
│                            │                                       │
│  ┌─────────────────────────┼──────────────────────────────────┐   │
│  │                 Plugin Host                                  │   │
│  │  ┌─────────┐  ┌────────┴────────┐  ┌────────────────────┐ │   │
│  │  │ Registry│  │ Execution Engine │  │ Sandbox Manager   │ │   │
│  │  │         │  │ (WASM / Process) │  │ (resource limits, │ │   │
│  │  │         │  │                  │  │  I/O restrictions) │ │   │
│  │  └─────────┘  └─────────────────┘  └────────────────────┘ │   │
│  └────────────────────────────────────────────────────────────┘   │
│                            │                                       │
│  ┌─────────────────────────┼──────────────────────────────────┐   │
│  │                    IR Layer                                  │   │
│  │  ┌──────┐ ┌──────┐ ┌───┴───┐ ┌──────┐ ┌──────┐ ┌──────┐ │   │
│  │  │ Doc  │ │ Image│ │ Audio │ │ Video│ │Vector│ │Table │ │   │
│  │  │ IR   │ │ IR   │ │ IR    │ │ IR   │ │ IR   │ │ IR   │ │   │
│  │  └──────┘ └──────┘ └───────┘ └──────┘ └──────┘ └──────┘ │   │
│  └────────────────────────────────────────────────────────────┘   │
│                                                                    │
│  ┌────────────────────────────────────────────────────────────┐   │
│  │                  Infrastructure                             │   │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────┐ │   │
│  │  │ Temp File│ │ Logging  │ │ Config   │ │ Error        │ │   │
│  │  │ Manager  │ │ & Audit  │ │ Store    │ │ Recovery     │ │   │
│  │  └──────────┘ └──────────┘ └──────────┘ └──────────────┘ │   │
│  └────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
```

### 2.2 Data Flow: Single File Conversion

```
User drops file
      │
      ▼
┌─────────────┐     ┌──────────────┐     ┌─────────────┐
│ Format      │────▶│ Conversion   │────▶│ Plugin Host  │
│ Detector    │     │ Router       │     │              │
│             │     │              │     │  ┌─────────┐ │
│ - magic     │     │ - lookup src │     │  │ Decode  │ │
│   bytes     │     │   format     │     │  │ Plugin  │ │
│ - extension │     │ - lookup tgt │     │  └────┬────┘ │
│ - MIME      │     │   format     │     │       │ IR   │
│             │     │ - find path  │     │  ┌────▼────┐ │
│             │     │   via DAG    │     │  │ Encode  │ │
│             │     │              │     │  │ Plugin  │ │
│             │     │ Path:        │     │  └─────────┘ │
│             │     │ src→IR→tgt   │     │              │
└─────────────┘     └──────────────┘     └──────┬───────┘
                                                │
                                                ▼
                                         ┌─────────────┐
                                         │ Integrity   │
                                         │ Checker     │
                                         │             │
                                         │ - checksum  │
                                         │ - validate  │
                                         │   output    │
                                         └──────┬──────┘
                                                │
                                                ▼
                                         ┌─────────────┐
                                         │ Output      │
                                         │ (with       │
                                         │  metadata)  │
                                         └─────────────┘
```

### 2.3 Plugin System Design

#### 2.3.1 Plugin Interface Contract

```rust
// === Core Types ===

/// Unique identifier for a format (e.g., "image/png", "document/pdf")
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FormatId {
    /// MIME type (primary identifier)
    pub mime: String,
    /// Common file extensions (e.g., ["png", "apng"])
    pub extensions: Vec<String>,
    /// Human-readable name
    pub display_name: String,
}

/// What a plugin can preserve during conversion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {
    pub metadata: MetadataSupport,
    pub structure: StructureSupport,
    pub embedded_assets: EmbeddedAssetSupport,
    pub color_spaces: Vec<ColorSpace>,
    pub max_dimension: Option<(u32, u32)>,
    pub max_bit_depth: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetadataSupport {
    None,
    ReadOnly,
    ReadWrite,
    ReadWriteTransform, // Can adapt metadata between formats
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StructureSupport {
    Flat,           // No structure (e.g., plain text)
    Hierarchical,   // Sections, headings, nesting
    Relational,     // Tables, cross-references
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmbeddedAssetSupport {
    None,
    Extract,        // Can extract but not embed
    ExtractAndEmbed,
}

/// Plugin metadata (static declaration)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,                          // unique e.g., "core-png-decoder"
    pub version: semver::Version,
    pub api_version: semver::Version,        // plugin API compatibility
    pub author: String,
    pub license: String,
    pub description: String,
    pub input_formats: Vec<FormatId>,
    pub output_formats: Vec<FormatId>,
    pub capabilities: Capabilities,
    pub dependencies: Vec<Dependency>,
    pub priority: i32,                       // higher = preferred when multiple plugins handle same format
    pub fidelity_score: u8,                  // 0-100, self-declared
    pub known_limitations: Vec<String>,
    pub sandbox_mode: SandboxMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SandboxMode {
    Wasm,           // Runs in WASM sandbox (preferred for safety)
    Process,        // Runs in separate process (for plugins needing native libs)
    InProcess,      // Runs in main process (only for trusted core plugins)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub version_req: semver::VersionReq,
    pub optional: bool,
}

// === Plugin Trait ===

/// Every converter plugin implements this trait
pub trait ConverterPlugin: Send + Sync {
    /// Static manifest — called once at registration
    fn manifest(&self) -> PluginManifest;

    /// Probe a file to confirm it can be decoded.
    /// Returns confidence score 0-100 and detected format details.
    fn probe(&self, input: &FileReader) -> Result<ProbeResult, PluginError>;

    /// Decode source file into the appropriate IR.
    /// `progress` is a callback for streaming progress updates.
    fn decode(
        &self,
        input: &FileReader,
        config: &DecodeConfig,
        progress: &ProgressCallback,
    ) -> Result<Box<dyn IntermediateRepresentation>, PluginError>;

    /// Encode an IR into the target format.
    fn encode(
        &self,
        ir: &dyn IntermediateRepresentation,
        output: &FileWriter,
        config: &EncodeConfig,
        progress: &ProgressCallback,
    ) -> Result<ConversionOutput, PluginError>;

    /// Cancel a running conversion. Must be safe to call from any thread.
    fn cancel(&self) -> Result<(), PluginError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeResult {
    pub confidence: u8,          // 0-100
    pub detected_format: FormatId,
    pub format_version: Option<String>,
    pub estimated_size: Option<u64>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecodeConfig {
    pub max_memory_bytes: u64,
    pub prefer_speed_over_quality: bool,
    pub strip_metadata: bool,
    pub custom: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncodeConfig {
    pub quality: QualityPreset,
    pub max_memory_bytes: u64,
    pub preserve_metadata: bool,
    pub custom: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QualityPreset {
    Lossless,
    High,
    Medium,
    Low,
    Custom(HashMap<String, f64>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionOutput {
    pub bytes_written: u64,
    pub checksum: String,
    pub warnings: Vec<String>,
    pub fidelity_estimate: u8,
}

// === Progress & Cancellation ===

pub struct ProgressCallback {
    sender: tokio::sync::watch::Sender<ProgressState>,
    cancel_token: CancellationToken,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressState {
    pub phase: ConversionPhase,
    pub percent: f32,            // 0.0 - 100.0
    pub bytes_processed: u64,
    pub bytes_total: Option<u64>,
    pub elapsed: Duration,
    pub eta: Option<Duration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConversionPhase {
    Probing,
    Decoding,
    Transforming,
    Encoding,
    Verifying,
}
```

#### 2.3.2 Plugin Discovery & Registration

```
plugins/
├── manifest.toml          # Auto-generated index
├── core-png-decoder/
│   ├── plugin.wasm        # WASM plugin binary
│   └── manifest.toml      # Plugin declaration
├── core-png-encoder/
│   ├── plugin.wasm
│   └── manifest.toml
├── community-webp/
│   ├── plugin.wasm
│   └── manifest.toml
└── ffmpeg-video/          # Process-sandboxed plugin
    ├── plugin-bin          # Native binary
    └── manifest.toml
```

**Registration flow:**
1. On startup, Plugin Host scans `plugins/` directory
2. Reads each `manifest.toml`, validates `api_version` compatibility
3. Registers `(input_format, output_format) → plugin_id` in a routing table
4. For conflicts (multiple plugins for same conversion), selects by: `priority DESC, fidelity_score DESC, id ASC`

#### 2.3.3 Sandbox Architecture

```
┌───────────────────────────────────────────────┐
│                Plugin Host                     │
│                                                │
│  ┌──────────────────────────────────────────┐ │
│  │           WASM Sandbox (wasmtime)        │ │
│  │                                          │ │
│  │  ┌──────────┐  ┌──────────┐  ┌────────┐ │ │
│  │  │ Plugin A │  │ Plugin B │  │Plugin C│ │ │
│  │  │ (image)  │  │ (audio)  │  │(csv)   │ │ │
│  │  └──────────┘  └──────────┘  └────────┘ │ │
│  │                                          │ │
│  │  Capabilities:                           │ │
│  │  - Memory limit per instance (256MB def) │ │
│  │  - No filesystem access (read via host   │ │
│  │    callback, write via host callback)    │ │
│  │  - No network access                     │ │
│  │  - CPU time limit per conversion         │ │
│  │  - Panic/crash does not affect host      │ │
│  └──────────────────────────────────────────┘ │
│                                                │
│  ┌──────────────────────────────────────────┐ │
│  │        Process Sandbox (fallback)        │ │
│  │                                          │ │
│  │  ┌──────────────────────────────────────┐│ │
│  │  │ Plugin D (ffmpeg — needs native libs)││ │
│  │  │ Runs as child process                ││ │
│  │  │ Communicates via protobuf over stdin ││ │
│  │  │ Killed on timeout / resource excess  ││ │
│  │  └──────────────────────────────────────┘││ │
│  └──────────────────────────────────────────┘ │
└───────────────────────────────────────────────┘
```

### 2.4 Intermediate Representation Designs

#### 2.4.1 Document IR (DocIR)

```rust
/// Document Intermediate Representation
/// Covers: PDF, DOCX, ODT, HTML, Markdown, RTF, EPUB, plain text
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentIR {
    pub version: semver::Version,
    pub metadata: DocumentMetadata,
    pub styles: StyleSheet,
    pub content: Vec<Block>,
    pub annotations: Vec<Annotation>,
    pub embedded_resources: Vec<EmbeddedResource>,
    pub outline: Option<TocNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub created: Option<DateTime<Utc>>,
    pub modified: Option<DateTime<Utc>>,
    pub language: Option<String>,
    pub page_size: Option<PageSize>,
    pub custom: HashMap<String, MetadataValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSize {
    pub width_pt: f64,   // 1 pt = 1/72 inch
    pub height_pt: f64,
    pub margins: Margins,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Margins {
    pub top_pt: f64,
    pub right_pt: f64,
    pub bottom_pt: f64,
    pub left_pt: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleSheet {
    pub paragraph_styles: Vec<ParagraphStyle>,
    pub character_styles: Vec<CharacterStyle>,
    pub table_styles: Vec<TableStyle>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParagraphStyle {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub font: Option<FontSpec>,
    pub alignment: Option<Alignment>,
    pub spacing: Option<Spacing>,
    pub borders: Option<Borders>,
    pub shading: Option<Shading>,
    pub numbering: Option<NumberingRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontSpec {
    pub family: String,
    pub size_pt: f64,
    pub weight: u16,        // 100-900
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub color: Option<Color>,
    pub script: Option<Script>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Color {
    Rgb(u8, u8, u8),
    Rgba(u8, u8, u8, u8),
    Named(String),
    Theme(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Alignment {
    Left, Center, Right, Justify,
    Start, End, // for RTL support
}

/// Block-level content elements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Block {
    Paragraph(Paragraph),
    Heading(Heading),
    Table(Table),
    List(List),
    CodeBlock(CodeBlock),
    BlockQuote(Vec<Block>),
    Image(ImageRef),
    PageBreak,
    SectionBreak(SectionBreakType),
    TableOfContents(TocField),
    Custom { type_id: String, data: serde_json::Value },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paragraph {
    pub style_id: Option<String>,
    pub runs: Vec<InlineRun>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heading {
    pub level: u8,          // 1-6
    pub style_id: Option<String>,
    pub runs: Vec<InlineRun>,
    pub id: Option<String>, // for cross-references
}

/// Inline content within a block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InlineRun {
    Text(TextRun),
    Link { runs: Vec<InlineRun>, href: String },
    Image(ImageRef),
    FootnoteRef(String),
    EndnoteRef(String),
    Bookmark(String),
    Field(FieldType),
    LineBreak,
    PageBreak,
    Tab,
    SoftHyphen,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextRun {
    pub text: String,
    pub style_id: Option<String>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,
    pub strikethrough: Option<bool>,
    pub color: Option<Color>,
    pub background: Option<Color>,
    pub font: Option<FontSpec>,
    pub superscript: bool,
    pub subscript: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    pub style_id: Option<String>,
    pub rows: Vec<TableRow>,
    pub column_widths: Option<Vec<f64>>,
    pub merged_cells: Vec<MergedCell>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableRow {
    pub cells: Vec<TableCell>,
    pub height: Option<f64>,
    pub header_row: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableCell {
    pub content: Vec<Block>,
    pub vertical_alignment: Option<VerticalAlignment>,
    pub shading: Option<Shading>,
    pub borders: Option<Borders>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedResource {
    pub id: String,
    pub mime: String,
    pub data: ResourceData,
    pub alt_text: Option<String>,
    pub dimensions: Option<(u32, u32)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResourceData {
    Inline(Vec<u8>),
    Reference(String), // path or URI
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TocNode {
    pub title: String,
    pub level: u8,
    pub target_id: String,
    pub children: Vec<TocNode>,
}

/// Information preservation:
/// LOSSLESS: text content, basic formatting (bold/italic/underline),
///   headings, lists, tables, links, images, metadata, page size
/// LOSSY: complex table merges across pages, exact pixel-perfect layout,
///   macros, OLE objects, tracked changes, some custom XML metadata,
///   font embedding (may substitute), complex numbering restarts
```

#### 2.4.2 Image IR (ImageIR)

```rust
/// Image Intermediate Representation
/// Covers: PNG, JPEG, WebP, BMP, TIFF, GIF, ICO, AVIF, HEIF, SVG (raster),
///         PSD (flattened), RAW (via libraw), QOI, JPEG2000
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageIR {
    pub version: semver::Version,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    Indexed { palette: Vec<[u8; 3]> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BitDepth {
    U1, U2, U4, U8, U16, U32,
    F16, F32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlphaChannel {
    None,
    Straight,
    Premultiplied,
}

/// Pixel storage — chosen by the plugin based on what's most natural
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
    /// Lazy — plugin provides a reader interface instead of materializing
    Lazy {
        width: u32,
        height: u32,
        format: String,
        data_ref: String,
    },
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
    Normal, Multiply, Screen, Overlay, Darken, Lighten,
    ColorDodge, ColorBurn, HardLight, SoftLight,
    Difference, Exclusion, Hue, Saturation, Color, Luminosity,
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
    None, RestoreBackground, RestorePrevious,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BlendMethod {
    Source, Over,
}

/// Information preservation:
/// LOSSLESS: pixel data (at same bit depth), dimensions, color space,
///   ICC profiles, EXIF, XMP, animation frames, layers (when both
///   source and target support them), transparency
/// LOSSY: bit depth reduction (16→8), color space conversion (CMYK→RGB),
///   palette reduction, layer flattening, animation frame dropping,
///   compression artifacts (JPEG↔lossless), EXIF in formats that don't
///   support it
```

#### 2.4.3 Audio IR (AudioIR)

```rust
/// Audio Intermediate Representation
/// Covers: WAV, FLAC, MP3, AAC, OGG/Vorbis, Opus, WMA, AIFF, ALAC,
///         M4A, PCM raw
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioIR {
    pub version: semver::Version,
    pub format: AudioFormat,
    pub samples: SampleData,
    pub metadata: AudioMetadata,
    pub chapters: Option<Vec<Chapter>>,
    pub tags: AudioTags,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioFormat {
    pub sample_rate: u32,           // Hz (e.g., 44100, 48000, 96000)
    pub channels: ChannelLayout,
    pub bit_depth: AudioBitDepth,
    pub sample_format: SampleFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChannelLayout {
    Mono,
    Stereo,
    Surround5_1,
    Surround7_1,
    Custom(Vec<ChannelDef>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelDef {
    pub id: String,
    pub position: (f64, f64, f64), // x, y, z
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AudioBitDepth {
    U8, I16, I24, I32, F32, F64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SampleFormat {
    Integer,
    Float,
}

/// Sample storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SampleData {
    /// All samples in memory (suitable for short files)
    Interleaved(Vec<f32>),
    /// Per-channel storage
    Planar(Vec<Vec<f32>>),
    /// For large files: streaming reader
    Streaming {
        total_samples: u64,
        reader_ref: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioMetadata {
    pub duration: Duration,
    pub original_format: String,
    pub original_bitrate: Option<u32>,
    pub encoder: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioTags {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    pub year: Option<u32>,
    pub genre: Option<String>,
    pub comment: Option<String>,
    pub cover_art: Option<Vec<u8>>,
    pub replay_gain: Option<ReplayGain>,
    pub custom: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayGain {
    pub track_gain_db: f64,
    pub track_peak: f64,
    pub album_gain_db: Option<f64>,
    pub album_peak: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chapter {
    pub start: Duration,
    pub end: Duration,
    pub title: String,
}

/// Information preservation:
/// LOSSLESS: sample data (PCM), channel layout, sample rate (if same),
///   bit depth (if same), all tags, cover art, chapters, ReplayGain
/// LOSSY: sample rate conversion, bit depth reduction, channel downmix
///   (5.1→stereo), lossy codec encoding (MP3/AAC/Opus), tag format
///   differences (ID3v1 vs ID3v2 vs Vorbis Comments)
```

### 2.5 Conversion Graph & Routing

The router uses a directed acyclic graph (DAG) where:
- **Nodes** are formats (identified by `FormatId`)
- **Edges** are available plugins (weighted by priority × fidelity)
- **Paths** are conversion chains through IRs

```
Example graph (simplified):

PNG ──decode──▶ ImageIR ──encode──▶ WebP
                    │
JPEG ──decode──────┤
                    │
BMP ──decode───────┤
                    ├──encode──▶ TIFF
                    ├──encode──▶ AVIF
                    │
PSD ──decode───────┘

DOCX ──decode──▶ DocIR ──encode──▶ PDF
                    │
HTML ──decode──────┤
                    ├──encode──▶ Markdown
MD ──decode────────┤
                    ├──encode──▶ EPUB
                    │
RTF ──decode───────┘
```

**Routing algorithm:**
1. Look up all plugins that can decode source format
2. Look up all plugins that can encode target format
3. Find shortest path: `decode(src) → [IR transforms] → encode(tgt)`
4. If direct path exists (same IR), use it
5. If no direct path, look for IR-to-IR transforms
6. Score paths by: `min(plugin.fidelity_score)` along path
7. Return best path; if none found, report unsupported conversion

**Multi-step conversion example:**
```
DOCX → DocIR → PDF     (direct, 1 step through IR)
DOCX → DocIR → HTML → String → Markdown  (2-step: DocIR → HTML → text)
```

### 2.6 Error Handling Strategy

```rust
/// Error hierarchy — every error is recoverable at the orchestrator level
#[derive(Debug, thiserror::Error)]
pub enum ConversionError {
    #[error("Format detection failed: {reason}")]
    DetectionFailed { reason: String },

    #[error("Unsupported conversion: {source} → {target}")]
    UnsupportedConversion { source: FormatId, target: FormatId },

    #[error("Plugin error in {plugin_id}: {kind}")]
    PluginError {
        plugin_id: String,
        kind: PluginErrorKind,
    },

    #[error("IR validation failed: {errors:?}")]
    IrValidationFailed { errors: Vec<ValidationError> },

    #[error("Output validation failed: checksum mismatch")]
    IntegrityCheckFailed {
        expected: String,
        actual: String,
    },

    #[error("Resource limit exceeded: {resource} ({limit})")]
    ResourceLimitExceeded {
        resource: String,  // "memory", "disk", "cpu_time"
        limit: String,
    },

    #[error("Conversion cancelled by user")]
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginErrorKind {
    DecodeFailed(String),
    EncodeFailed(String),
    InvalidInput(String),
    InternalError(String),
    Timeout,
    Crashed(String),
    OutOfMemory,
}
```

**Error recovery rules:**
1. Plugin crash → kill sandbox, log error, report to user, continue with next file in batch
2. Decode failure → try alternate decoder plugin if available (lower priority fallback)
3. IR validation failure → report specific validation errors with field paths
4. Integrity check failure → delete corrupted output, report, offer retry
5. Resource limit → pause queue, report, let user adjust limits
6. All errors are logged with full context (input file, plugin, phase, stack trace)

### 2.7 State Management

```rust
/// Queue and progress state — lives in the Orchestrator, exposed to UI via IPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionQueue {
    pub items: Vec<QueueItem>,
    pub active: Vec<ActiveConversion>,
    pub completed: Vec<CompletedItem>,
    pub config: QueueConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItem {
    pub id: Uuid,
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub detected_format: Option<FormatId>,
    pub target_format: FormatId,
    pub status: QueueItemStatus,
    pub created_at: DateTime<Utc>,
    pub priority: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueueItemStatus {
    Pending,
    Detecting,
    Converting { progress: ProgressState },
    Paused,
    Completed { output: ConversionOutput },
    Failed { error: ConversionError },
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveConversion {
    pub item_id: Uuid,
    pub conversion_path: ConversionPath,
    pub progress: ProgressState,
    pub cancel_token: CancellationToken,
    pub pause_token: CancellationToken,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueConfig {
    pub max_concurrent: usize,        // default: num_cpus
    pub max_memory_per_conversion: u64, // default: 512MB
    pub max_total_memory: u64,         // default: 4GB
    pub auto_retry_on_failure: bool,
    pub max_retries: u32,
    pub verify_output: bool,           // checksum verification
    pub overwrite_existing: bool,
    pub conflict_resolution: ConflictResolution,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictResolution {
    Overwrite,
    Rename,      // append number
    Skip,
    Ask,
}
```

**Pause/Resume mechanism:**
- Each `ActiveConversion` holds a `CancellationToken` (from tokio-util)
- Plugins check `cancel_token.is_cancelled()` at natural yield points (per-row, per-frame, per-tile)
- Pause: sets a separate `pause_token` which blocks the conversion task via `pause_token.cancelled().await`
- Resume: resets the pause token
- State is persisted to disk after each status change (crash recovery)

---

## 3. TECHNOLOGY STACK JUSTIFICATION

### 3.1 Language: Rust

| Criterion | Rust | TypeScript/Node | C++ |
|-----------|------|-----------------|-----|
| **Performance** | Native, zero-cost abstractions | V8 overhead, GC pauses | Native, but manual memory mgmt |
| **Memory safety** | Guaranteed at compile time | Runtime (safe but GC) | Manual (error-prone) |
| **Plugin sandboxing** | WASM (first-class via wasmtime) | V8 isolates (limited) | Process isolation only |
| **Concurrency** | Tokio async + Send/Sync guarantees | Node async (single-threaded) | Manual threading |
| **Cross-platform** | Excellent (Tier 1: Win/Mac/Linux) | Excellent | Good but platform-specific |
| **Ecosystem** | Strong for CLI/system tools (clap, serde, tokio) | Massive npm ecosystem | Vast but fragmented |
| **Build complexity** | Medium (cargo is excellent) | Low | High (CMake, vcpkg) |
| **Long-term maintainability** | Strong type system prevents regressions | Weaker typing, dependency rot | Header hell, ABI breaks |

**Decision: Rust.**

Rationale: The three hardest problems in this project — plugin sandboxing, streaming large files without memory blowup, and cross-platform reliability — all favor Rust. WASM sandboxing via wasmtime is a first-class Rust crate. The type system prevents entire categories of bugs. Cargo is the best build system in any language. The learning curve is steeper, but this is a 15-year codebase — the investment pays off.

### 3.2 UI Framework: Tauri

| Criterion | Tauri | Electron | Native (egui/iced) |
|-----------|-------|----------|-------------------|
| **Binary size** | ~5-10 MB (uses system WebView) | ~150+ MB (bundles Chromium) | ~5-15 MB |
| **Memory usage** | Low (shared WebView) | High (Chromium per window) | Lowest |
| **Frontend flexibility** | Any web framework (React, Vue, Svelte) | Any web framework | Rust-only |
| **Native integration** | Excellent (Rust backend direct) | Good (IPC) | Excellent |
| **UI polish** | High (web ecosystem) | High | Medium (limited widget set) |
| **Dev speed** | Fast (web tooling) | Fast | Slow (custom widgets) |
| **Cross-platform** | Windows, macOS, Linux | Windows, macOS, Linux | Varies |

**Decision: Tauri.**

Rationale: Tauri gives us the best of both worlds — a Rust backend that communicates directly with our engine (zero IPC overhead for the core logic), and a web frontend for rich UI (drag-drop, progress bars, format previews). The binary size is 10-30x smaller than Electron. System WebViews on all three platforms are mature enough for our needs.

**Frontend:** Svelte (lightweight, fast, excellent for reactive UIs with less boilerplate than React).

### 3.3 WASM Sandboxing: wasmtime

| Criterion | wastime | Wasmer | V8 Isolates |
|-----------|--------|--------|-------------|
| **Maturity** | Most mature, Bytecode Alliance | Good, growing | Not designed for this |
| **WASI support** | Full WASI Preview 2 | Partial | N/A |
| **Performance** | Excellent (Cranelift JIT) | Good (multiple backends) | N/A |
| **Memory limits** | Per-instance configurable | Per-instance | Per-isolate |
| **Rust integration** | First-class | Good | Poor |
| **Hot reload** | Supported (instantiate new module) | Supported | N/A |

**Decision: wasmtime.** Bytecode Alliance backing, best WASI support, excellent Rust integration.

### 3.4 Build System & CI

| Component | Choice | Rationale |
|-----------|--------|-----------|
| **Build** | Cargo + Tauri CLI | Standard Rust tooling, excellent cross-compilation |
| **Test** | cargo test + proptest + criterion | Unit + property-based + benchmarks in one ecosystem |
| **CI** | GitHub Actions | Free for open-source, native Rust support, matrix builds for 3 platforms |
| **Fuzzing** | cargo-fuzz (libFuzzer) | Native Rust fuzzing, catches malformed input bugs |
| **Code quality** | clippy + rustfmt + cargo-audit | Linting, formatting, dependency vulnerability scanning |

### 3.5 Key Rust Crates

| Purpose | Crate | Notes |
|---------|-------|-------|
| Async runtime | tokio | Industry standard |
| Serialization | serde + serde_json + toml | Plugin manifests, IPC, config |
| CLI | clap | For CLI interface (Tier 2) |
| Image processing | image, imageproc | Pure Rust, no C deps for basic formats |
| Audio | symphonia | Pure Rust audio decoding (MP3, AAC, FLAC, etc.) |
| Video | ffmpeg-next (binding) | Process-sandboxed; too complex for pure Rust |
| PDF reading | pdf-extract or lopdf | Pure Rust PDF parsing |
| PDF writing | printpdf | Pure Rust PDF generation |
| DOCX | docx-rs | Pure Rust DOCX reading |
| HTML parsing | scraper, html2text | For HTML ↔ DocIR |
| Markdown | pulldown-cmark | CommonMark parser |
| Archives | zip, tar, flate2 | Pure Rust archive handling |
| WASM runtime | wasmtime | Plugin sandboxing |
| Crypto/hashing | blake3, sha2 | Checksum verification |
| Error handling | thiserror, anyhow | Structured errors + context |
| Progress bars | indicatif (CLI) | For CLI interface |
| UUID | uuid | Queue item IDs |
| Logging | tracing | Structured logging |
| Config | directories | Platform-specific config paths |

---

## 4. IMPLEMENTATION ROADMAP

### Milestone Overview

| # | Milestone | Complexity | Dependencies | Duration Est. |
|---|-----------|------------|--------------|---------------|
| 1 | Foundation & MVP Pipeline | L | — | 3-4 weeks |
| 2 | Core Image Plugins | M | M1 | 2-3 weeks |
| 3 | Document & Text Pipeline | L | M1 | 3-4 weeks |
| 4 | Audio Pipeline | M | M1 | 2-3 weeks |
| 5 | Video & Archive Pipeline | L | M1 | 3-4 weeks |
| 6 | Queue, Batch & State Management | M | M1-M5 | 2-3 weeks |
| 7 | Tier 2 Features | L | M1-M6 | 3-4 weeks |
| 8 | Polish, Performance & Security Audit | M | M1-M7 | 2-3 weeks |

**Total estimated effort: 20-28 weeks (5-7 months) for a small team.**

### Milestone 1: Foundation & MVP Pipeline (L)

**Goal:** End-to-end conversion working for 2-3 formats, proving the full architecture.

**Deliverables:**
- [ ] Core engine: Orchestrator, Format Detector, Conversion Router
- [ ] Plugin Host with WASM sandbox (wasmtime)
- [ ] Plugin trait and interface crate (`converter-plugin-api`)
- [ ] Image IR implementation (full data model)
- [ ] 3 plugins: PNG decoder, JPEG decoder, PNG encoder
- [ ] Conversion: JPEG → ImageIR → PNG (round-trip proves the pipeline)
- [ ] Tauri app shell with drag-and-drop zone
- [ ] Basic progress display
- [ ] CLI tool (`ufc convert input.jpg output.png`)
- [ ] Unit tests for IR, router, detector
- [ ] Integration test: JPEG → PNG golden-file test
- [ ] CI pipeline: build + test on 3 platforms

**Success criteria:**
- Drop a JPEG, get a PNG out with verified checksum
- Plugin crash does not crash the app
- Memory usage stays under 256MB for a 20MB image
- All tests pass on Windows, macOS, Linux

### Milestone 2: Core Image Plugins (M)

**Goal:** Full image format coverage for Tier 1.

**Deliverables:**
- [ ] Decoders: BMP, TIFF, GIF, WebP, ICO, AVIF
- [ ] Encoders: JPEG, WebP, BMP, TIFF, GIF, AVIF
- [ ] Animated GIF support (decode all frames, encode)
- [ ] ICC profile passthrough
- [ ] EXIF/XMP metadata preservation
- [ ] Image resize and color space conversion transforms
- [ ] Batch image conversion (100+ files)
- [ ] Golden-file tests for each format pair

**Success criteria:**
- Round-trip fidelity: pixel-perfect for lossless formats, SSIM > 0.95 for lossy
- Animated GIF → WebP animation preserves frame timing
- 100 PNG files → WebP in under 30 seconds

### Milestone 3: Document & Text Pipeline (L)

**Goal:** Document conversion covering the most common office/web formats.

**Deliverables:**
- [ ] Document IR implementation (full data model)
- [ ] Decoders: PDF, DOCX, HTML, Markdown, RTF, plain text
- [ ] Encoders: PDF, HTML, Markdown, DOCX
- [ ] Rich text formatting preservation (bold, italic, headings, lists, tables)
- [ ] Embedded image extraction and re-embedding
- [ ] Table structure preservation
- [ ] Metadata (title, author, dates) passthrough
- [ ] Golden-file tests with complex documents

**Success criteria:**
- DOCX → PDF preserves all text, basic formatting, images, and tables
- HTML → Markdown → HTML round-trip preserves semantic structure
- 50-page PDF converts without memory exceeding 512MB

### Milestone 4: Audio Pipeline (M)

**Goal:** Audio format conversion with metadata preservation.

**Deliverables:**
- [ ] Audio IR implementation
- [ ] Decoders: WAV, FLAC, MP3, AAC, OGG/Vorbis, Opus, AIFF
- [ ] Encoders: WAV, FLAC, MP3, AAC (via fdkaac), OGG/Vorbis, Opus
- [ ] Tag preservation (ID3v2, Vorbis Comments, MP4 tags)
- [ ] Cover art extraction and embedding
- [ ] Sample rate and bit depth conversion
- [ ] Channel layout conversion (5.1 → stereo)
- [ ] Batch audio conversion

**Success criteria:**
- FLAC → WAV → FLAC is bit-perfect
- MP3 → OGG preserves all tags and cover art
- 100-track album batch converts with progress tracking

### Milestone 5: Video & Archive Pipeline (L)

**Goal:** Video conversion (via ffmpeg) and archive handling.

**Deliverables:**
- [ ] Video IR implementation
- [ ] Process-sandboxed FFmpeg plugin
- [ ] Decoders: MP4, MKV, AVI, MOV, WebM, FLV
- [ ] Encoders: MP4 (H.264/H.265), WebM (VP9/AV1), MKV
- [ ] Audio track extraction and muxing
- [ ] Subtitle track handling (SRT, ASS, WebVTT)
- [ ] Chapter preservation
- [ ] Archive IR implementation
- [ ] Decoders/Encoders: ZIP, TAR, TAR.GZ, TAR.BZ2, TAR.XZ, 7Z (read)
- [ ] Archive conversion preserves directory structure and permissions

**Success criteria:**
- MP4 → WebM conversion with progress tracking
- Subtitle track preserved across container formats
- ZIP → TAR.GZ preserves file permissions and timestamps

### Milestone 6: Queue, Batch & State Management (M)

**Goal:** Production-grade queue management and state persistence.

**Deliverables:**
- [ ] Persistent conversion queue (survives app restart)
- [ ] Concurrent conversion with configurable parallelism
- [ ] Pause/resume individual conversions and entire queue
- [ ] Cancel with cleanup (partial output deletion)
- [ ] Duplicate detection (content hash)
- [ ] Conversion history with search
- [ ] Folder/recursive conversion
- [ ] Integrity verification (Blake3 checksums)

**Success criteria:**
- 1000-file batch completes without intervention
- App crash → restart → queue resumes from last checkpoint
- Pause/resume works mid-conversion for video files

### Milestone 7: Tier 2 Features (L)

**Goal:** Professional workflow features.

**Deliverables:**
- [ ] Conversion profiles (save/load named settings)
- [ ] Preview before conversion (render first page/frame)
- [ ] Keyboard shortcuts (full navigation)
- [ ] Dark mode
- [ ] CLI with full feature parity
- [ ] eBooks (EPUB, MOBI → EPUB)
- [ ] Font conversion (TTF, OTF, WOFF, WOFF2)
- [ ] Structured data: CSV ↔ JSON ↔ XML ↔ YAML
- [ ] SVG ↔ other vector formats
- [ ] Plugin management UI (install, remove, configure, update)

**Success criteria:**
- CLI can do everything the GUI can
- EPUB → PDF preserves chapters and formatting
- All keyboard shortcuts documented

### Milestone 8: Polish, Performance & Security Audit (M)

**Goal:** Production-ready release quality.

**Deliverables:**
- [ ] Performance profiling and optimization
- [ ] Memory leak detection (valgrind, heaptrack)
- [ ] Fuzz testing all decoders with 10K+ mutations
- [ ] Security audit of WASM sandbox
- [ ] Large file stress tests (10GB video, 1GB image)
- [ ] Accessibility audit (screen reader, keyboard-only)
- [ ] Auto-update mechanism
- [ ] Installer packages (MSI, DMG, AppImage, deb, rpm)
- [ ] User documentation and plugin developer guide

**Success criteria:**
- Zero crashes on 10K-file fuzz corpus
- 10GB video converts without OOM
- All security audit findings addressed
- Installers work on clean systems

### Dependency Graph

```
M1 (Foundation)
├── M2 (Images)
├── M3 (Documents)
├── M4 (Audio)
├── M5 (Video/Archives)
│
M6 (Queue/Batch) ← depends on M1-M5 having working conversions
│
M7 (Tier 2) ← depends on M6 for batch/profile features
│
M8 (Polish) ← depends on all above
```

---

## 5. PROJECT STRUCTURE

```
universal-file-converter/
├── Cargo.toml                    # Workspace root
├── Cargo.lock
├── rust-toolchain.toml           # Pin Rust version
├── .github/
│   └── workflows/
│       ├── ci.yml                # Build + test on 3 platforms
│       ├── release.yml           # Build installers on tag
│       └── fuzz.yml              # Nightly fuzz testing
├── README.md
├── LICENSE
├── CONTRIBUTING.md
├── ARCHITECTURE.md               # This document (living)
│
├── crates/
│   ├── ufc-core/                 # Core engine
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── orchestrator.rs   # Main conversion coordinator
│   │       ├── detector.rs       # Format detection (magic bytes, ext, MIME)
│   │       ├── router.rs         # Conversion path DAG solver
│   │       ├── queue.rs          # Conversion queue management
│   │       ├── state.rs          # State persistence (queue, history)
│   │       ├── integrity.rs      # Checksum verification
│   │       ├── temp_manager.rs   # Temp file lifecycle
│   │       └── config.rs         # Application configuration
│   │
│   ├── ufc-plugin-api/           # Plugin interface contract (public API)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── traits.rs         # ConverterPlugin trait
│   │       ├── types.rs          # FormatId, Capabilities, ProbeResult, etc.
│   │       ├── config.rs         # DecodeConfig, EncodeConfig
│   │       ├── progress.rs       # ProgressCallback, ProgressState
│   │       ├── error.rs          # PluginError types
│   │       └── io.rs             # FileReader, FileWriter (sandboxed I/O)
│   │
│   ├── ufc-ir/                   # Intermediate representations
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── document.rs       # DocumentIR
│   │       ├── image.rs          # ImageIR
│   │       ├── audio.rs          # AudioIR
│   │       ├── video.rs          # VideoIR
│   │       ├── vector.rs         # VectorIR
│   │       ├── table.rs          # TableIR
│   │       ├── archive.rs        # ArchiveIR
│   │       ├── mesh.rs           # Mesh3DIR
│   │       └── traits.rs         # IntermediateRepresentation trait
│   │
│   ├── ufc-host/                 # Plugin host & sandbox manager
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── registry.rs       # Plugin discovery, registration, routing
│   │       ├── wasm_sandbox.rs   # wasmtime-based WASM execution
│   │       ├── process_sandbox.rs# Child process execution (for ffmpeg etc)
│   │       ├── resource_limits.rs# Memory, CPU, disk limits per plugin
│   │       └── loader.rs         # Dynamic plugin loading
│   │
│   ├── ufc-cli/                  # CLI interface
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── commands/
│   │       │   ├── convert.rs    # Single file conversion
│   │       │   ├── batch.rs      # Batch conversion
│   │       │   ├── detect.rs     # Format detection
│   │       │   ├── list.rs       # List available formats/plugins
│   │       │   └── profile.rs    # Manage conversion profiles
│   │       └── output.rs         # Progress bars, colored output
│   │
│   └── ufc-tauri/                # Tauri desktop application
│       ├── Cargo.toml
│       ├── tauri.conf.json
│       ├── src/
│       │   ├── main.rs
│       │   ├── commands.rs       # Tauri IPC commands
│       │   ├── state.rs          # App state management
│       │   └── tray.rs           # System tray
│       └── ui/                   # Svelte frontend
│           ├── package.json
│           ├── svelte.config.js
│           ├── src/
│           │   ├── App.svelte
│           │   ├── lib/
│           │   │   ├── components/
│           │   │   │   ├── DropZone.svelte
│           │   │   │   ├── ConversionQueue.svelte
│           │   │   │   ├── ProgressCard.svelte
│           │   │   │   ├── FormatSelector.svelte
│           │   │   │   ├── Settings.svelte
│           │   │   │   ├── PluginManager.svelte
│           │   │   │   └── ConversionHistory.svelte
│           │   │   ├── stores/
│           │   │   │   ├── queue.ts
│           │   │   │   ├── settings.ts
│           │   │   │   └── plugins.ts
│           │   │   └── api/
│           │   │       └── tauri.ts   # Tauri invoke wrappers
│           │   └── routes/
│           │       ├── +page.svelte
│           │       ├── +layout.svelte
│           │       └── settings/
│           │           └── +page.svelte
│           └── static/
│               └── favicon.ico
│
├── plugins/                      # Built-in plugins
│   ├── core-image-png/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── decoder.rs
│   │       └── encoder.rs
│   ├── core-image-jpeg/
│   ├── core-image-webp/
│   ├── core-image-bmp/
│   ├── core-image-tiff/
│   ├── core-image-gif/
│   ├── core-image-avif/
│   ├── core-image-ico/
│   ├── core-doc-pdf/
│   ├── core-doc-docx/
│   ├── core-doc-html/
│   ├── core-doc-markdown/
│   ├── core-doc-rtf/
│   ├── core-audio-wav/
│   ├── core-audio-flac/
│   ├── core-audio-mp3/
│   ├── core-audio-aac/
│   ├── core-audio-vorbis/
│   ├── core-audio-opus/
│   ├── core-video-ffmpeg/        # Process-sandboxed
│   ├── core-archive-zip/
│   ├── core-archive-tar/
│   ├── core-archive-7z/
│   ├── core-struct-csv/
│   ├── core-struct-json/
│   ├── core-struct-xml/
│   └── core-struct-yaml/
│
├── tests/                        # Integration & golden-file tests
│   ├── integration/
│   │   ├── pipeline_tests.rs
│   │   ├── plugin_host_tests.rs
│   │   └── queue_tests.rs
│   ├── golden/
│   │   ├── images/               # Known input → expected output pairs
│   │   ├── documents/
│   │   ├── audio/
│   │   └── video/
│   └── fuzz/
│       ├── fuzz_image_decode.rs
│       ├── fuzz_doc_decode.rs
│       └── fuzz_audio_decode.rs
│
├── benchmarks/
│   ├── Cargo.toml
│   └── src/
│       ├── image_throughput.rs
│       ├── audio_throughput.rs
│       └── queue_concurrency.rs
│
├── docs/
│   ├── architecture.md           # This document
│   ├── plugin-development.md     # Plugin author guide
│   ├── api-reference.md          # Plugin API docs
│   ├── user-guide.md             # End-user documentation
│   └── adr/                      # Architecture Decision Records
│       ├── 001-language-choice.md
│       ├── 002-wasm-sandboxing.md
│       └── 003-ir-design.md
│
└── scripts/
    ├── build-plugins.sh          # Build all WASM plugins
    ├── package-release.sh        # Build installers for all platforms
    └── run-fuzz.sh               # Run fuzz testing suite
```

### Naming Conventions

| Element | Convention | Example |
|---------|-----------|---------|
| Crate names | `ufc-{module}` | `ufc-core`, `ufc-ir` |
| Plugin crates | `core-{category}-{format}` or `community-{name}` | `core-image-png` |
| Modules | snake_case | `conversion_router.rs` |
| Types | PascalCase | `ConversionQueue`, `FormatId` |
| Functions | snake_case | `detect_format()`, `convert_file()` |
| Constants | SCREAMING_SNAKE | `MAX_PLUGIN_MEMORY`, `DEFAULT_TIMEOUT` |
| Config keys | snake_case (TOML) | `max_concurrent = 4` |
| Test files | `*_tests.rs` or inline `#[cfg(test)]` | `pipeline_tests.rs` |
| IR versions | semver | `DocumentIR { version: "1.0.0" }` |

### Module Organization Rationale

1. **Workspace crates over monolith:** Each crate has a clear API boundary. `ufc-plugin-api` is the only crate external plugin authors depend on — it changes rarely and is versioned semantically.

2. **Separate IR crate:** IRs are shared between core, plugins, and tests. A dedicated crate avoids circular dependencies.

3. **Plugin crates are standalone:** Each plugin compiles to a `.wasm` file independently. They depend only on `ufc-plugin-api` and `ufc-ir`.

4. **CLI and GUI are thin shells:** All business logic lives in `ufc-core` and `ufc-host`. The CLI and Tauri app are presentation layers.

5. **Tests at workspace root:** Integration tests span multiple crates, so they live in a top-level `tests/` directory.

---

## Appendix A: Architecture Decision Records (Summary)

### ADR-001: Rust over TypeScript

**Context:** Need a language for the core engine that supports WASM sandboxing, high-performance file I/O, and cross-platform reliability.

**Decision:** Rust.

**Consequences:** Higher initial development cost, but superior long-term correctness, performance, and sandboxing. Plugin authors must know Rust (mitigated by clear API docs and examples).

### ADR-002: WASM Sandboxing (Hybrid Model)

**Context:** Plugins must be isolated from the host and each other. Pure WASM has limitations for complex codecs.

**Decision:** WASM (wasmtime) as default; process isolation as fallback for plugins requiring native libraries (FFmpeg, LibreOffice).

**Consequences:** Most plugins are portable `.wasm` files. Video plugins require platform-specific native binaries but are still sandboxed via process isolation.

### ADR-003: Domain-Specific IRs (Not Universal)

**Context:** A single universal IR for all formats would be impossibly complex.

**Decision:** Separate IR per domain (Document, Image, Audio, Video, Vector, Table, Archive, Mesh).

**Consequences:** Clean, focused data models. Cross-domain conversions (e.g., extracting an image from a PDF) go through the orchestrator, not through a single IR.

### ADR-004: Tauri over Electron

**Context:** Need a cross-platform GUI framework.

**Decision:** Tauri with Svelte frontend.

**Consequences:** 10-30x smaller binaries, lower memory usage. System WebView differences across platforms require testing. No access to Chrome-specific APIs (not needed for our use case).

---

## Appendix B: Format Support Matrix (Initial)

| Format | Category | Decode | Encode | Plugin Type | Notes |
|--------|----------|--------|--------|-------------|-------|
| PNG | Image | ✅ | ✅ | WASM | Pure Rust (image crate) |
| JPEG | Image | ✅ | ✅ | WASM | Pure Rust (image crate) |
| WebP | Image | ✅ | ✅ | WASM | Pure Rust (image crate) |
| BMP | Image | ✅ | ✅ | WASM | Pure Rust |
| TIFF | Image | ✅ | ✅ | WASM | Pure Rust |
| GIF | Image | ✅ | ✅ | WASM | Animation support |
| AVIF | Image | ✅ | ✅ | WASM | via ravif |
| ICO | Image | ✅ | ✅ | WASM | Pure Rust |
| PDF | Document | ✅ | ✅ | WASM | lopdf + printpdf |
| DOCX | Document | ✅ | ✅ | WASM | docx-rs |
| HTML | Document | ✅ | ✅ | WASM | scraper + markup5ever |
| Markdown | Document | ✅ | ✅ | WASM | pulldown-cmark |
| RTF | Document | ✅ | ❌ | WASM | Decode only initially |
| Plain Text | Document | ✅ | ✅ | WASM | Trivial |
| EPUB | eBook | ✅ | ✅ | WASM | zip + XHTML |
| WAV | Audio | ✅ | ✅ | WASM | hound |
| FLAC | Audio | ✅ | ✅ | WASM | symphonia |
| MP3 | Audio | ✅ | ✅ | WASM | symphonia |
| AAC | Audio | ✅ | ✅ | WASM | symphonia |
| OGG/Vorbis | Audio | ✅ | ✅ | WASM | symphonia |
| Opus | Audio | ✅ | ✅ | WASM | symphonia |
| MP4 | Video | ✅ | ✅ | Process | FFmpeg |
| MKV | Video | ✅ | ✅ | Process | FFmpeg |
| AVI | Video | ✅ | ✅ | Process | FFmpeg |
| MOV | Video | ✅ | ✅ | Process | FFmpeg |
| WebM | Video | ✅ | ✅ | Process | FFmpeg |
| ZIP | Archive | ✅ | ✅ | WASM | zip crate |
| TAR | Archive | ✅ | ✅ | WASM | tar crate |
| TAR.GZ | Archive | ✅ | ✅ | WASM | tar + flate2 |
| 7Z | Archive | ✅ | ❌ | WASM | Read only |
| CSV | Structured | ✅ | ✅ | WASM | csv crate |
| JSON | Structured | ✅ | ✅ | WASM | serde_json |
| XML | Structured | ✅ | ✅ | WASM | quick-xml |
| YAML | Structured | ✅ | ✅ | WASM | serde_yaml |
| TTF/OTF | Font | ✅ | ✅ | WASM | font-kit |
| WOFF/WOFF2 | Font | ✅ | ✅ | WASM | woff2 |
| SVG | Vector | ✅ | ✅ | WASM | resvg |
