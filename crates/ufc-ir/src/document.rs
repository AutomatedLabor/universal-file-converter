use crate::traits::{IntermediateRepresentation, ValidationError, ValidationSeverity};
use chrono::{DateTime, Utc};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Document Intermediate Representation.
///
/// Covers: PDF, DOCX, ODT, HTML, Markdown, RTF, EPUB, plain text
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentIR {
    pub version: Version,
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
pub enum MetadataValue {
    String(String),
    Number(f64),
    Boolean(bool),
    Date(DateTime<Utc>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSize {
    pub width_pt: f64,
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

// ── Styles ──

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
pub struct CharacterStyle {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub font: Option<FontSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableStyle {
    pub id: String,
    pub name: String,
    pub borders: Option<Borders>,
    pub cell_shading: Option<Shading>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontSpec {
    pub family: String,
    pub size_pt: f64,
    pub weight: u16,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub color: Option<Color>,
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
    Left,
    Center,
    Right,
    Justify,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spacing {
    pub before_pt: f64,
    pub after_pt: f64,
    pub line_spacing: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Borders {
    pub top: Option<BorderSide>,
    pub right: Option<BorderSide>,
    pub bottom: Option<BorderSide>,
    pub left: Option<BorderSide>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BorderSide {
    pub width_pt: f64,
    pub color: Option<Color>,
    pub style: BorderLineStyle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BorderLineStyle {
    None,
    Solid,
    Dashed,
    Dotted,
    Double,
    Groove,
    Ridge,
    Inset,
    Outset,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shading {
    pub fill: Option<Color>,
    pub pattern: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NumberingRef {
    pub list_id: String,
    pub level: u8,
}

// ── Content ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Block {
    Paragraph(Paragraph),
    Heading(Heading),
    Table(Table),
    List(List),
    CodeBlock(CodeBlock),
    BlockQuote(Vec<Block>),
    Image(ImageRef),
    HorizontalRule,
    PageBreak,
    SectionBreak,
    TableOfContents(TocField),
    Custom {
        type_id: String,
        data: serde_json::Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paragraph {
    pub style_id: Option<String>,
    pub runs: Vec<InlineRun>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heading {
    pub level: u8,
    pub style_id: Option<String>,
    pub runs: Vec<InlineRun>,
    pub id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InlineRun {
    Text(TextRun),
    Link {
        runs: Vec<InlineRun>,
        href: String,
    },
    Image(ImageRef),
    FootnoteRef(String),
    Bookmark(String),
    LineBreak,
    SoftHyphen,
    Tab,
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

impl TextRun {
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style_id: None,
            bold: None,
            italic: None,
            underline: None,
            strikethrough: None,
            color: None,
            background: None,
            font: None,
            superscript: false,
            subscript: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    pub style_id: Option<String>,
    pub rows: Vec<TableRow>,
    pub column_widths: Option<Vec<f64>>,
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
    pub row_span: u32,
    pub col_span: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VerticalAlignment {
    Top,
    Center,
    Bottom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct List {
    pub list_type: ListType,
    pub items: Vec<ListItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ListType {
    Ordered,
    Unordered,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListItem {
    pub content: Vec<Block>,
    pub level: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeBlock {
    pub language: Option<String>,
    pub code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageRef {
    pub resource_id: String,
    pub alt_text: Option<String>,
    pub width: Option<f64>,
    pub height: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TocField {
    pub max_level: u8,
}

// ── Annotations ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Annotation {
    pub id: String,
    pub annotation_type: AnnotationType,
    pub content: String,
    pub target: AnnotationTarget,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnnotationType {
    Footnote,
    Endnote,
    Comment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnnotationTarget {
    Block(usize),
    Inline { block: usize, run: usize },
}

// ── Embedded resources ──

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
    Reference(String),
}

// ── Table of Contents ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TocNode {
    pub title: String,
    pub level: u8,
    pub target_id: String,
    pub children: Vec<TocNode>,
}

// ─────────────────────────────────────────────
// Constructors
// ─────────────────────────────────────────────

impl DocumentIR {
    pub fn new() -> Self {
        Self {
            version: crate::api_version(),
            metadata: DocumentMetadata {
                title: None,
                author: None,
                created: None,
                modified: None,
                language: None,
                page_size: None,
                custom: HashMap::new(),
            },
            styles: StyleSheet {
                paragraph_styles: Vec::new(),
                character_styles: Vec::new(),
                table_styles: Vec::new(),
            },
            content: Vec::new(),
            annotations: Vec::new(),
            embedded_resources: Vec::new(),
            outline: None,
        }
    }

    /// Extract plain text content from the document.
    pub fn plain_text(&self) -> String {
        let mut text = String::new();
        for block in &self.content {
            extract_block_text(block, &mut text);
            text.push('\n');
        }
        text
    }
}

fn extract_block_text(block: &Block, text: &mut String) {
    match block {
        Block::Paragraph(p) => extract_runs_text(&p.runs, text),
        Block::Heading(h) => {
            extract_runs_text(&h.runs, text);
        }
        Block::Table(t) => {
            for row in &t.rows {
                for cell in &row.cells {
                    for b in &cell.content {
                        extract_block_text(b, text);
                    }
                    text.push('\t');
                }
                text.push('\n');
            }
        }
        Block::List(l) => {
            for item in &l.items {
                for b in &item.content {
                    extract_block_text(b, text);
                }
            }
        }
        Block::CodeBlock(c) => text.push_str(&c.code),
        Block::BlockQuote(blocks) => {
            for b in blocks {
                extract_block_text(b, text);
            }
        }
        Block::Image(_) => {}
        Block::HorizontalRule | Block::PageBreak | Block::SectionBreak => {}
        Block::TableOfContents(_) => {}
        Block::Custom { .. } => {}
    }
}

fn extract_runs_text(runs: &[InlineRun], text: &mut String) {
    for run in runs {
        match run {
            InlineRun::Text(t) => text.push_str(&t.text),
            InlineRun::Link { runs, .. } => extract_runs_text(runs, text),
            InlineRun::Image(_) => {}
            InlineRun::FootnoteRef(_) => {}
            InlineRun::Bookmark(_) => {}
            InlineRun::LineBreak => text.push('\n'),
            InlineRun::SoftHyphen => {}
            InlineRun::Tab => text.push('\t'),
        }
    }
}

impl Default for DocumentIR {
    fn default() -> Self {
        Self::new()
    }
}

impl IntermediateRepresentation for DocumentIR {
    fn version(&self) -> Version {
        self.version.clone()
    }

    fn ir_type(&self) -> &'static str {
        "Document"
    }

    fn memory_usage(&self) -> u64 {
        let mut total = 0u64;
        for block in &self.content {
            total += estimate_block_size(block);
        }
        for res in &self.embedded_resources {
            total += match &res.data {
                ResourceData::Inline(d) => d.len() as u64,
                ResourceData::Reference(_) => 0,
            };
        }
        total + 4096 // metadata overhead
    }

    fn validate(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        if self.content.is_empty() {
            errors.push(ValidationError {
                field: "content".into(),
                message: "Document has no content blocks".into(),
                severity: ValidationSeverity::Warning,
            });
        }
        errors
    }

    fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    fn is_empty(&self) -> bool {
        self.content.is_empty()
    }
}

fn estimate_block_size(block: &Block) -> u64 {
    match block {
        Block::Paragraph(p) => p.runs.iter().map(estimate_run_size).sum(),
        Block::Heading(h) => h.runs.iter().map(estimate_run_size).sum(),
        Block::Table(t) => {
            t.rows
                .iter()
                .flat_map(|r| &r.cells)
                .flat_map(|c| &c.content)
                .map(estimate_block_size)
                .sum()
        }
        Block::List(l) => l
            .items
            .iter()
            .flat_map(|i| &i.content)
            .map(estimate_block_size)
            .sum(),
        Block::CodeBlock(c) => c.code.len() as u64,
        Block::BlockQuote(blocks) => blocks.iter().map(estimate_block_size).sum(),
        _ => 256,
    }
}

fn estimate_run_size(run: &InlineRun) -> u64 {
    match run {
        InlineRun::Text(t) => t.text.len() as u64 + 128,
        InlineRun::Link { runs, .. } => runs.iter().map(estimate_run_size).sum::<u64>() + 256,
        _ => 128,
    }
}
