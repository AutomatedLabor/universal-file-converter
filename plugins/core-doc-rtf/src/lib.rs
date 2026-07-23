use std::any::Any;
use ufc_ir::document::*;
use ufc_plugin_api::*;

pub struct RtfPlugin;
impl RtfPlugin { pub fn new() -> Self { Self } }

impl ConverterPlugin for RtfPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            id: "core-rtf".to_string(), version: semver::Version::new(1, 0, 0),
            api_version: semver::Version::new(1, 0, 0), author: "UFC Core Team".to_string(),
            license: "MIT".to_string(), description: "RTF document decoder (decode-only)".to_string(),
            input_formats: vec![FormatId::new("application/rtf", &["rtf"], "RTF")],
            output_formats: vec![],
            capabilities: Capabilities { metadata: MetadataSupport::ReadOnly, structure: StructureSupport::Hierarchical,
                embedded_assets: EmbeddedAssetSupport::None, color_spaces: vec![],
                max_dimension: None, max_bit_depth: None, supports_animation: false,
                supports_transparency: false, supports_multi_page: false },
            dependencies: vec![], priority: 80, fidelity_score: 70,
            known_limitations: vec!["Encode not supported".to_string()],
            sandbox_mode: SandboxMode::Wasm,
        }
    }

    fn probe(&self, input: &FileReader) -> Result<ProbeResult, PluginError> {
        let header = input.read_slice(0, 5).map_err(|e| PluginError::IoError(e.to_string()))?;
        if header.len() >= 5 && &header[0..5] == b"{\\rtf" {
            Ok(ProbeResult { confidence: 100, detected_format: FormatId::new("application/rtf", &["rtf"], "RTF"),
                format_version: None, estimated_size: Some(input.size()), warnings: vec![] })
        } else { Err(PluginError::UnsupportedFormat("Not an RTF file".to_string())) }
    }

    fn decode(&self, input: &FileReader, _config: &DecodeConfig, progress: &ProgressCallback) -> Result<Box<dyn Any + Send + Sync>, PluginError> {
        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(0.0));
        let data = input.read_all().map_err(|e| PluginError::IoError(e.to_string()))?;
        let rtf = String::from_utf8_lossy(&data);
        let mut ir = DocumentIR::new();
        // Simple RTF text extraction
        let text = extract_rtf_text(&rtf);
        if !text.is_empty() {
            ir.content.push(Block::Paragraph(Paragraph {
                style_id: None, runs: vec![InlineRun::Text(TextRun::plain(&text))],
            }));
        }
        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(100.0));
        Ok(Box::new(ir))
    }

    fn encode(&self, _ir: &(dyn Any + Send + Sync), _output: &FileWriter, _config: &EncodeConfig, _progress: &ProgressCallback) -> Result<ConversionOutput, PluginError> {
        Err(PluginError::EncodeFailed("RTF encoding not supported".to_string()))
    }

    fn cancel(&self) -> Result<(), PluginError> { Ok(()) }
}

fn extract_rtf_text(rtf: &str) -> String {
    let mut text = String::new();
    let mut depth = 0;
    let mut in_control = false;
    let mut control_word = String::new();
    let bytes = rtf.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        let ch = bytes[i] as char;
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 && !text.ends_with('\n') { text.push('\n'); }
            }
            '\\' => {
                if i + 1 < bytes.len() && bytes[i + 1] == b'\\' {
                    text.push('\\'); i += 2; continue;
                }
                in_control = true; control_word.clear();
            }
            _ if in_control => {
                if ch.is_ascii_alphabetic() {
                    control_word.push(ch);
                } else {
                    if ch == ' ' {
                        // skip space after control word
                    } else if control_word == "par" || control_word == "line" {
                        text.push('\n');
                    } else if control_word == "tab" {
                        text.push('\t');
                    } else if control_word.starts_with('u') {
                        // Unicode character
                        if let Ok(code) = control_word[1..].parse::<i32>() {
                            if let Some(c) = char::from_u32(code as u32) {
                                text.push(c);
                            }
                        }
                    }
                    in_control = false;
                    if ch != ' ' && ch != '\\' && ch != '{' && ch != '}' {
                        // don't push control chars
                    } else {
                        i -= 1; // re-process this char
                    }
                }
            }
            _ if depth > 0 && !ch.is_ascii_control() => {
                text.push(ch);
            }
            _ => {}
        }
        i += 1;
    }
    text.trim().to_string()
}
