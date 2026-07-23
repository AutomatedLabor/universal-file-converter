use std::any::Any;
use ufc_ir::document::*;
use ufc_plugin_api::*;
use pulldown_cmark::{Parser, Event, Tag, Options};

pub struct MarkdownPlugin;
impl MarkdownPlugin { pub fn new() -> Self { Self } }

impl ConverterPlugin for MarkdownPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            id: "core-markdown".to_string(), version: semver::Version::new(1, 0, 0),
            api_version: semver::Version::new(1, 0, 0), author: "UFC Core Team".to_string(),
            license: "MIT".to_string(), description: "Markdown document decoder and encoder".to_string(),
            input_formats: vec![FormatId::new("text/markdown", &["md", "markdown"], "Markdown")],
            output_formats: vec![FormatId::new("text/markdown", &["md", "markdown"], "Markdown")],
            capabilities: Capabilities { metadata: MetadataSupport::ReadOnly, structure: StructureSupport::Hierarchical,
                embedded_assets: EmbeddedAssetSupport::Extract, color_spaces: vec![],
                max_dimension: None, max_bit_depth: None, supports_animation: false,
                supports_transparency: false, supports_multi_page: false },
            dependencies: vec![], priority: 100, fidelity_score: 100, known_limitations: vec![],
            sandbox_mode: SandboxMode::Wasm,
        }
    }

    fn probe(&self, input: &FileReader) -> Result<ProbeResult, PluginError> {
        let ext = input.path().extension().map(|e| e.to_string_lossy().to_lowercase());
        match ext.as_deref() {
            Some("md") | Some("markdown") => Ok(ProbeResult {
                confidence: 80, detected_format: FormatId::new("text/markdown", &["md", "markdown"], "Markdown"),
                format_version: None, estimated_size: Some(input.size()), warnings: vec![] }),
            _ => Err(PluginError::UnsupportedFormat("Not a Markdown file".to_string())),
        }
    }

    fn decode(&self, input: &FileReader, _config: &DecodeConfig, progress: &ProgressCallback) -> Result<Box<dyn Any + Send + Sync>, PluginError> {
        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(0.0));
        let data = input.read_all().map_err(|e| PluginError::IoError(e.to_string()))?;
        let md = String::from_utf8_lossy(&data);
        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(30.0));

        let mut ir = DocumentIR::new();
        let parser = Parser::new_ext(&md, Options::all());
        let mut current_runs: Vec<InlineRun> = Vec::new();
        let mut in_paragraph = false;

        for event in parser {
            match event {
                Event::Start(Tag::Heading(level, _, _)) => {
                    if in_paragraph && !current_runs.is_empty() {
                        ir.content.push(Block::Paragraph(Paragraph { style_id: None, runs: std::mem::take(&mut current_runs) }));
                        in_paragraph = false;
                    }
                    let _ = level; // will collect text in next events
                }
                Event::End(Tag::Heading(level, _, _)) => {
                    ir.content.push(Block::Heading(Heading {
                        level: level as u8, style_id: None,
                        runs: std::mem::take(&mut current_runs), id: None,
                    }));
                }
                Event::Start(Tag::Paragraph) => { in_paragraph = true; }
                Event::End(Tag::Paragraph) => {
                    ir.content.push(Block::Paragraph(Paragraph { style_id: None, runs: std::mem::take(&mut current_runs) }));
                    in_paragraph = false;
                }
                Event::Start(Tag::List(ordered)) => {
                    let _ = ordered;
                }
                Event::End(Tag::List(_)) => {}
                Event::Start(Tag::Item) => {}
                Event::End(Tag::Item) => {}
                Event::Start(Tag::CodeBlock(_)) => {}
                Event::End(Tag::CodeBlock(_)) => {}
                Event::Start(Tag::BlockQuote(_)) => {}
                Event::End(Tag::BlockQuote(_)) => {}
                Event::Text(text) => {
                    current_runs.push(InlineRun::Text(TextRun::plain(&text)));
                }
                Event::Code(code) => {
                    current_runs.push(InlineRun::Text(TextRun::plain(&code)));
                }
                Event::SoftBreak => {
                    current_runs.push(InlineRun::LineBreak);
                }
                Event::HardBreak => {
                    current_runs.push(InlineRun::LineBreak);
                }
                Event::Rule => {
                    ir.content.push(Block::HorizontalRule);
                }
                _ => {}
            }
        }
        if !current_runs.is_empty() {
            ir.content.push(Block::Paragraph(Paragraph { style_id: None, runs: current_runs }));
        }
        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(100.0));
        Ok(Box::new(ir))
    }

    fn encode(&self, ir: &(dyn Any + Send + Sync), output: &FileWriter, _config: &EncodeConfig, progress: &ProgressCallback) -> Result<ConversionOutput, PluginError> {
        progress.update(ProgressState::new(ConversionPhase::Encoding).with_percent(0.0));
        let doc_ir = ir.downcast_ref::<DocumentIR>().ok_or_else(|| PluginError::InvalidInput("Expected DocumentIR".to_string()))?;
        let mut md = String::new();
        blocks_to_markdown(&doc_ir.content, &mut md, 0);
        let bytes = md.as_bytes();
        output.write_all(bytes).map_err(|e| PluginError::IoError(e.to_string()))?;
        let checksum = blake3::hash(bytes).to_hex().to_string();
        Ok(ConversionOutput { bytes_written: bytes.len() as u64, checksum, warnings: vec![], fidelity_estimate: 100 })
    }

    fn cancel(&self) -> Result<(), PluginError> { Ok(()) }
}

fn blocks_to_markdown(blocks: &[Block], md: &mut String, depth: usize) {
    for block in blocks {
        match block {
            Block::Heading(h) => {
                let prefix = "#".repeat(h.level as usize);
                let text = runs_to_text(&h.runs);
                md.push_str(&format!("{} {}\n\n", prefix, text));
            }
            Block::Paragraph(p) => {
                let text = runs_to_text(&p.runs);
                md.push_str(&format!("{}\n\n", text));
            }
            Block::List(l) => {
                for (i, item) in l.items.iter().enumerate() {
                    let indent = "  ".repeat(depth);
                    match l.list_type {
                        ListType::Ordered => md.push_str(&format!("{}{}. ", indent, i + 1)),
                        ListType::Unordered => md.push_str(&format!("{}- ", indent)),
                    }
                    for b in &item.content {
                        match b {
                            Block::Paragraph(p) => md.push_str(&runs_to_text(&p.runs)),
                            Block::List(_) => blocks_to_markdown(&[b.clone()], md, depth + 1),
                            _ => {}
                        }
                        md.push('\n');
                    }
                }
                md.push('\n');
            }
            Block::CodeBlock(c) => {
                let lang = c.language.as_deref().unwrap_or("");
                md.push_str(&format!("```{}\n{}\n```\n\n", lang, c.code));
            }
            Block::BlockQuote(blocks) => {
                let inner = { let mut s = String::new(); blocks_to_markdown(blocks, &mut s, 0); s };
                for line in inner.lines() {
                    md.push_str(&format!("> {}\n", line));
                }
                md.push('\n');
            }
            Block::Image(img) => {
                let alt = img.alt_text.as_deref().unwrap_or("");
                md.push_str(&format!("![{}]({})\n\n", alt, img.resource_id));
            }
            Block::HorizontalRule => md.push_str("---\n\n"),
            Block::PageBreak => md.push_str("\\newpage\n\n"),
            _ => {}
        }
    }
}

fn runs_to_text(runs: &[InlineRun]) -> String {
    runs.iter().map(|r| match r {
        InlineRun::Text(t) => t.text.clone(),
        InlineRun::Link { runs, href } => format!("[{}]({})", runs_to_text(runs), href),
        InlineRun::Code(code) => format!("`{}`", code),
        InlineRun::LineBreak => "  \n".to_string(),
        _ => String::new(),
    }).collect()
}
