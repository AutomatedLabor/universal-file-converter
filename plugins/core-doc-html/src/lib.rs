use std::any::Any;
use ufc_ir::document::*;
use ufc_plugin_api::*;

pub struct HtmlPlugin;
impl HtmlPlugin { pub fn new() -> Self { Self } }

impl ConverterPlugin for HtmlPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            id: "core-html".to_string(), version: semver::Version::new(1, 0, 0),
            api_version: semver::Version::new(1, 0, 0), author: "UFC Core Team".to_string(),
            license: "MIT".to_string(), description: "HTML document decoder and encoder".to_string(),
            input_formats: vec![FormatId::new("text/html", &["html", "htm"], "HTML")],
            output_formats: vec![FormatId::new("text/html", &["html", "htm"], "HTML")],
            capabilities: Capabilities { metadata: MetadataSupport::ReadWrite, structure: StructureSupport::Hierarchical,
                embedded_assets: EmbeddedAssetSupport::ExtractAndEmbed, color_spaces: vec![],
                max_dimension: None, max_bit_depth: None, supports_animation: false,
                supports_transparency: false, supports_multi_page: false },
            dependencies: vec![], priority: 90, fidelity_score: 95, known_limitations: vec![],
            sandbox_mode: SandboxMode::Wasm,
        }
    }

    fn probe(&self, input: &FileReader) -> Result<ProbeResult, PluginError> {
        let header = input.read_slice(0, 100).map_err(|e| PluginError::IoError(e.to_string()))?;
        let text = String::from_utf8_lossy(&header).to_lowercase();
        if text.contains("<!doctype") || text.contains("<html") || text.contains("<head") {
            Ok(ProbeResult { confidence: 90, detected_format: FormatId::new("text/html", &["html", "htm"], "HTML"),
                format_version: None, estimated_size: Some(input.size()), warnings: vec![] })
        } else { Err(PluginError::UnsupportedFormat("Not an HTML file".to_string())) }
    }

    fn decode(&self, input: &FileReader, _config: &DecodeConfig, progress: &ProgressCallback) -> Result<Box<dyn Any + Send + Sync>, PluginError> {
        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(0.0));
        let data = input.read_all().map_err(|e| PluginError::IoError(e.to_string()))?;
        let html = String::from_utf8_lossy(&data);
        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(30.0));

        let mut ir = DocumentIR::new();
        ir.metadata.title = scraper::Html::parse_document(&html)
            .select(&scraper::Selector::parse("title").unwrap())
            .next()
            .map(|el| el.text().collect::<String>());

        // Parse HTML structure
        let doc = scraper::Html::parse_document(&html);
        let body_selector = scraper::Selector::parse("body").unwrap();
        let body = doc.select(&body_selector).next().unwrap_or(doc.root_element());

        for child in body.children() {
            parse_html_element(&child, &mut ir.content);
        }
        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(100.0));
        Ok(Box::new(ir))
    }

    fn encode(&self, ir: &(dyn Any + Send + Sync), output: &FileWriter, _config: &EncodeConfig, progress: &ProgressCallback) -> Result<ConversionOutput, PluginError> {
        progress.update(ProgressState::new(ConversionPhase::Encoding).with_percent(0.0));
        let doc_ir = ir.downcast_ref::<DocumentIR>().ok_or_else(|| PluginError::InvalidInput("Expected DocumentIR".to_string()))?;
        let mut html = String::from("<!DOCTYPE html>\n<html>\n<head>\n");
        if let Some(title) = &doc_ir.metadata.title {
            html.push_str(&format!("<title>{}</title>\n", title));
        }
        html.push_str("</head>\n<body>\n");
        blocks_to_html(&doc_ir.content, &mut html);
        html.push_str("</body>\n</html>\n");
        let bytes = html.as_bytes();
        output.write_all(bytes).map_err(|e| PluginError::IoError(e.to_string()))?;
        let checksum = blake3::hash(bytes).to_hex().to_string();
        Ok(ConversionOutput { bytes_written: bytes.len() as u64, checksum, warnings: vec![], fidelity_estimate: 95 })
    }

    fn cancel(&self) -> Result<(), PluginError> { Ok(()) }
}

fn parse_html_element(node: &scraper::node::Node, blocks: &mut Vec<Block>) {
    use scraper::Node;
    match node {
        Node::Element(el) => {
            let tag = el.name();
            match tag {
                "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                    let level = tag[1..].parse::<u8>().unwrap_or(1);
                    let text: String = el.text().collect();
                    blocks.push(Block::Heading(Heading {
                        level, style_id: None,
                        runs: vec![InlineRun::Text(TextRun::plain(&text))],
                        id: el.attr("id").map(|s| s.to_string()),
                    }));
                }
                "p" => {
                    let text: String = el.text().collect();
                    blocks.push(Block::Paragraph(Paragraph {
                        style_id: None,
                        runs: vec![InlineRun::Text(TextRun::plain(&text))],
                    }));
                }
                "ul" | "ol" => {
                    let list_type = if tag == "ul" { ListType::Unordered } else { ListType::Ordered };
                    let mut items = Vec::new();
                    for child in el.children() {
                        if let Node::Element(li) = child.value() {
                            if li.name() == "li" {
                                let text: String = child.text().collect();
                                items.push(ListItem {
                                    content: vec![Block::Paragraph(Paragraph {
                                        style_id: None,
                                        runs: vec![InlineRun::Text(TextRun::plain(&text))],
                                    })],
                                    level: 0,
                                });
                            }
                        }
                    }
                    blocks.push(Block::List(List { list_type, items }));
                }
                "pre" | "code" => {
                    let text: String = el.text().collect();
                    blocks.push(Block::CodeBlock(CodeBlock { language: None, code: text }));
                }
                "blockquote" => {
                    let text: String = el.text().collect();
                    blocks.push(Block::BlockQuote(vec![Block::Paragraph(Paragraph {
                        style_id: None,
                        runs: vec![InlineRun::Text(TextRun::plain(&text))],
                    })]));
                }
                "hr" => blocks.push(Block::HorizontalRule),
                "img" => {
                    if let Some(src) = el.attr("src") {
                        blocks.push(Block::Image(ImageRef {
                            resource_id: src.to_string(),
                            alt_text: el.attr("alt").map(|s| s.to_string()),
                            width: el.attr("width").and_then(|s| s.parse().ok()),
                            height: el.attr("height").and_then(|s| s.parse().ok()),
                        }));
                    }
                }
                _ => {
                    for child in el.children() {
                        parse_html_element(&child, blocks);
                    }
                }
            }
        }
        Node::Text(text) => {
            let t = text.trim();
            if !t.is_empty() {
                blocks.push(Block::Paragraph(Paragraph {
                    style_id: None,
                    runs: vec![InlineRun::Text(TextRun::plain(t))],
                }));
            }
        }
        _ => {}
    }
}

fn blocks_to_html(blocks: &[Block], html: &mut String) {
    for block in blocks {
        match block {
            Block::Heading(h) => {
                let runs_text: String = h.runs.iter().filter_map(|r| match r {
                    InlineRun::Text(t) => Some(t.text.as_str()),
                    _ => None,
                }).collect();
                html.push_str(&format!("<h{0}>{1}</h{0}>\n", h.level, runs_text));
            }
            Block::Paragraph(p) => {
                let runs_text: String = p.runs.iter().filter_map(|r| match r {
                    InlineRun::Text(t) => Some(t.text.as_str()),
                    _ => None,
                }).collect();
                html.push_str(&format!("<p>{}</p>\n", runs_text));
            }
            Block::List(l) => {
                let tag = match l.list_type { ListType::Ordered => "ol", ListType::Unordered => "ul" };
                html.push_str(&format!("<{}>\n", tag));
                for item in &l.items {
                    html.push_str("<li>");
                    blocks_to_html(&item.content, html);
                    html.push_str("</li>\n");
                }
                html.push_str(&format!("</{}>\n", tag));
            }
            Block::CodeBlock(c) => {
                html.push_str(&format!("<pre><code>{}</code></pre>\n", c.code));
            }
            Block::BlockQuote(blocks) => {
                html.push_str("<blockquote>\n");
                blocks_to_html(blocks, html);
                html.push_str("</blockquote>\n");
            }
            Block::Image(img) => {
                html.push_str(&format!("<img src=\"{}\"", img.resource_id));
                if let Some(alt) = &img.alt_text { html.push_str(&format!(" alt=\"{}\"", alt)); }
                html.push_str(" />\n");
            }
            Block::HorizontalRule => html.push_str("<hr />\n"),
            Block::PageBreak => html.push_str("<div style=\"page-break-after: always;\"></div>\n"),
            _ => {}
        }
    }
}
