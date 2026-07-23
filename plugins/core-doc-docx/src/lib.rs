use std::any::Any;
use ufc_ir::document::*;
use ufc_plugin_api::*;
use std::io::Read;

pub struct DocxPlugin;
impl DocxPlugin { pub fn new() -> Self { Self } }

impl ConverterPlugin for DocxPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            id: "core-docx".to_string(), version: semver::Version::new(1, 0, 0),
            api_version: semver::Version::new(1, 0, 0), author: "UFC Core Team".to_string(),
            license: "MIT".to_string(), description: "DOCX document decoder and encoder".to_string(),
            input_formats: vec![FormatId::new("application/vnd.openxmlformats-officedocument.wordprocessingml.document", &["docx"], "DOCX")],
            output_formats: vec![FormatId::new("application/vnd.openxmlformats-officedocument.wordprocessingml.document", &["docx"], "DOCX")],
            capabilities: Capabilities { metadata: MetadataSupport::ReadWrite, structure: StructureSupport::Hierarchical,
                embedded_assets: EmbeddedAssetSupport::ExtractAndEmbed, color_spaces: vec![],
                max_dimension: None, max_bit_depth: None, supports_animation: false,
                supports_transparency: false, supports_multi_page: true },
            dependencies: vec![], priority: 100, fidelity_score: 85,
            known_limitations: vec!["Complex macros and OLE objects not supported".to_string()],
            sandbox_mode: SandboxMode::Wasm,
        }
    }

    fn probe(&self, input: &FileReader) -> Result<ProbeResult, PluginError> {
        // DOCX is a ZIP file; check for [Content_Types].xml
        let header = input.read_slice(0, 4).map_err(|e| PluginError::IoError(e.to_string()))?;
        if header.len() >= 4 && header[0] == 0x50 && header[1] == 0x4B {
            // Check extension
            let ext = input.path().extension().map(|e| e.to_string_lossy().to_lowercase());
            if ext.as_deref() == Some("docx") {
                return Ok(ProbeResult { confidence: 100,
                    detected_format: FormatId::new("application/vnd.openxmlformats-officedocument.wordprocessingml.document", &["docx"], "DOCX"),
                    format_version: None, estimated_size: Some(input.size()), warnings: vec![] });
            }
        }
        Err(PluginError::UnsupportedFormat("Not a DOCX file".to_string()))
    }

    fn decode(&self, input: &FileReader, _config: &DecodeConfig, progress: &ProgressCallback) -> Result<Box<dyn Any + Send + Sync>, PluginError> {
        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(0.0));
        let data = input.read_all().map_err(|e| PluginError::IoError(e.to_string()))?;
        let cursor = std::io::Cursor::new(&data);
        let mut archive = zip::ZipArchive::new(cursor).map_err(|e| PluginError::DecodeFailed(format!("ZIP error: {}", e)))?;

        let mut ir = DocumentIR::new();

        // Read document.xml
        let doc_xml = if let Ok(mut file) = archive.by_name("word/document.xml") {
            let mut content = String::new();
            file.read_to_string(&mut content).map_err(|e| PluginError::IoError(e.to_string()))?;
            content
        } else {
            return Err(PluginError::DecodeFailed("No document.xml found in DOCX".to_string()));
        };

        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(50.0));

        // Parse XML to extract text
        parse_docx_xml(&doc_xml, &mut ir);

        // Try to read core properties for metadata
        if let Ok(mut file) = archive.by_name("docProps/core.xml") {
            let mut content = String::new();
            if file.read_to_string(&mut content).is_ok() {
                parse_docx_metadata(&content, &mut ir);
            }
        }

        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(100.0));
        Ok(Box::new(ir))
    }

    fn encode(&self, ir: &(dyn Any + Send + Sync), output: &FileWriter, _config: &EncodeConfig, progress: &ProgressCallback) -> Result<ConversionOutput, PluginError> {
        progress.update(ProgressState::new(ConversionPhase::Encoding).with_percent(0.0));
        let doc_ir = ir.downcast_ref::<DocumentIR>().ok_or_else(|| PluginError::InvalidInput("Expected DocumentIR".to_string()))?;

        // Create a minimal DOCX file
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buf);
            let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

            // [Content_Types].xml
            zip.start_file("[Content_Types].xml", options).map_err(|e| PluginError::IoError(e.to_string()))?;
            zip.write_all(CONTENT_TYPES.as_bytes()).map_err(|e| PluginError::IoError(e.to_string()))?;

            // _rels/.rels
            zip.start_file("_rels/.rels", options).map_err(|e| PluginError::IoError(e.to_string()))?;
            zip.write_all(RELS.as_bytes()).map_err(|e| PluginError::IoError(e.to_string()))?;

            // word/_rels/document.xml.rels
            zip.start_file("word/_rels/document.xml.rels", options).map_err(|e| PluginError::IoError(e.to_string()))?;
            zip.write_all(DOC_RELS.as_bytes()).map_err(|e| PluginError::IoError(e.to_string()))?;

            // word/document.xml
            let mut doc_xml = String::from(DOC_HEADER);
            for block in &doc_ir.content {
                block_to_docx_xml(block, &mut doc_xml);
            }
            doc_xml.push_str(DOC_FOOTER);
            zip.start_file("word/document.xml", options).map_err(|e| PluginError::IoError(e.to_string()))?;
            zip.write_all(doc_xml.as_bytes()).map_err(|e| PluginError::IoError(e.to_string()))?;

            // word/styles.xml
            zip.start_file("word/styles.xml", options).map_err(|e| PluginError::IoError(e.to_string()))?;
            zip.write_all(STYLES_XML.as_bytes()).map_err(|e| PluginError::IoError(e.to_string()))?;

            zip.finish().map_err(|e| PluginError::IoError(e.to_string()))?;
        }

        let docx_bytes = buf.into_inner();
        output.write_all(&docx_bytes).map_err(|e| PluginError::IoError(e.to_string()))?;
        let checksum = blake3::hash(&docx_bytes).to_hex().to_string();
        progress.update(ProgressState::new(ConversionPhase::Encoding).with_percent(100.0));
        Ok(ConversionOutput { bytes_written: docx_bytes.len() as u64, checksum, warnings: vec![], fidelity_estimate: 85 })
    }

    fn cancel(&self) -> Result<(), PluginError> { Ok(()) }
}

fn parse_docx_xml(xml: &str, ir: &mut DocumentIR) {
    // Simple XML text extraction
    let mut in_text = false;
    let mut current_text = String::new();
    let mut chars = xml.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '<' {
            // Read tag
            let mut tag = String::new();
            for ch in chars.by_ref() {
                if ch == '>' { break; }
                tag.push(ch);
            }
            if tag.starts_with("w:t") || tag.starts_with("w:t ") {
                in_text = true;
            } else if tag.starts_with("/w:t") {
                in_text = false;
            } else if tag.starts_with("/w:p") {
                if !current_text.is_empty() {
                    ir.content.push(Block::Paragraph(Paragraph {
                        style_id: None,
                        runs: vec![InlineRun::Text(TextRun::plain(&current_text))],
                    }));
                    current_text.clear();
                }
            } else if tag.starts_with("/w:body") {
                break;
            }
        } else if in_text {
            current_text.push(ch);
        }
    }
    if !current_text.is_empty() {
        ir.content.push(Block::Paragraph(Paragraph {
            style_id: None,
            runs: vec![InlineRun::Text(TextRun::plain(&current_text))],
        }));
    }
}

fn parse_docx_metadata(xml: &str, ir: &mut DocumentIR) {
    // Extract title and author from core.xml
    let mut chars = xml.chars().peekable();
    let mut in_title = false;
    let mut in_creator = false;
    let mut title = String::new();
    let mut author = String::new();

    while let Some(ch) = chars.next() {
        if ch == '<' {
            let mut tag = String::new();
            for ch in chars.by_ref() {
                if ch == '>' { break; }
                tag.push(ch);
            }
            if tag.contains("dc:title") && !tag.starts_with('/') { in_title = true; }
            else if tag.contains("/dc:title") { in_title = false; }
            else if tag.contains("dc:creator") && !tag.starts_with('/') { in_creator = true; }
            else if tag.contains("/dc:creator") { in_creator = false; }
        } else {
            if in_title { title.push(ch); }
            if in_creator { author.push(ch); }
        }
    }
    if !title.is_empty() { ir.metadata.title = Some(title); }
    if !author.is_empty() { ir.metadata.author = Some(author); }
}

fn block_to_docx_xml(block: &Block, xml: &mut String) {
    match block {
        Block::Paragraph(p) => {
            xml.push_str("<w:p>");
            for run in &p.runs {
                if let InlineRun::Text(t) = run {
                    xml.push_str(&format!("<w:r><w:t>{}</w:t></w:r>", xml_escape(&t.text)));
                }
            }
            xml.push_str("</w:p>");
        }
        Block::Heading(h) => {
            xml.push_str(&format!("<w:p><w:pPr><w:pStyle w:val=\"Heading{}\"/></w:pPr>", h.level));
            for run in &h.runs {
                if let InlineRun::Text(t) = run {
                    xml.push_str(&format!("<w:r><w:t>{}</w:t></w:r>", xml_escape(&t.text)));
                }
            }
            xml.push_str("</w:p>");
        }
        Block::CodeBlock(c) => {
            xml.push_str(&format!("<w:p><w:r><w:t>{}</w:t></w:r></w:p>", xml_escape(&c.code)));
        }
        Block::Image(img) => {
            xml.push_str(&format!("<w:p><w:r><w:t>[Image: {}]</w:t></w:r></w:p>", img.resource_id));
        }
        Block::PageBreak => {
            xml.push_str("<w:p><w:r><w:br w:type=\"page\"/></w:r></w:p>");
        }
        _ => {}
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

const CONTENT_TYPES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
  <Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>
</Types>"#;

const RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#;

const DOC_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
</Relationships>"#;

const DOC_HEADER: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body>"#;

const DOC_FOOTER: &str = r#"</w:body></w:document>"#;

const STYLES_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:style w:type="paragraph" w:default="1" w:styleId="Normal">
    <w:name w:val="Normal"/>
  </w:style>
  <w:style w:type="paragraph" w:styleId="Heading1">
    <w:name w:val="heading 1"/>
    <w:pPr><w:outlineLvl w:val="0"/></w:pPr>
    <w:rPr><w:b/><w:sz w:val="28"/></w:rPr>
  </w:style>
  <w:style w:type="paragraph" w:styleId="Heading2">
    <w:name w:val="heading 2"/>
    <w:pPr><w:outlineLvl w:val="1"/></w:pPr>
    <w:rPr><w:b/><w:sz w:val="24"/></w:rPr>
  </w:style>
  <w:style w:type="paragraph" w:styleId="Heading3">
    <w:name w:val="heading 3"/>
    <w:pPr><w:outlineLvl w:val="2"/></w:pPr>
    <w:rPr><w:b/><w:sz w:val="20"/></w:rPr>
  </w:style>
</w:styles>"#;
