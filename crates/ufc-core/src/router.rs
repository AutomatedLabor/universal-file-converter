use std::collections::HashMap;
use ufc_plugin_api::{FormatId, PluginManifest};
use crate::error::CoreError;

/// Conversion path from source format to target format.
#[derive(Debug, Clone)]
pub struct ConversionPath {
    /// Plugin ID to use for decoding
    pub decode_plugin: String,
    /// Plugin ID to use for encoding
    pub encode_plugin: String,
    /// Intermediate representation type to use
    pub ir_type: String,
    /// Estimated fidelity (min of decode + encode plugin fidelity scores)
    pub fidelity_estimate: u8,
    /// Number of conversion steps
    pub steps: u32,
}

/// Routes conversion requests to the appropriate plugins.
///
/// Uses a DAG-based approach where:
/// - Nodes are formats (identified by `FormatId`)
/// - Edges are available plugins
/// - Paths go through intermediate representations
pub struct ConversionRouter {
    /// Registered plugin manifests
    plugins: Vec<PluginManifest>,
    /// (source_mime, target_mime) → list of conversion paths
    route_cache: HashMap<(String, String), Vec<ConversionPath>>,
}

impl ConversionRouter {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
            route_cache: HashMap::new(),
        }
    }

    /// Register a plugin manifest for routing.
    pub fn register_plugin(&mut self, manifest: PluginManifest) {
        self.route_cache.clear(); // invalidate cache
        self.plugins.push(manifest);
    }

    /// Remove a plugin by ID.
    pub fn unregister_plugin(&mut self, plugin_id: &str) {
        self.plugins.retain(|p| p.id != plugin_id);
        self.route_cache.clear();
    }

    /// Find the best conversion path from source to target format.
    pub fn find_path(&mut self, source: &FormatId, target: &FormatId) -> Result<ConversionPath, CoreError> {
        // Check cache first
        let key = (source.mime.clone(), target.mime.clone());
        if let Some(paths) = self.route_cache.get(&key) {
            if let Some(best) = paths.first() {
                return Ok(best.clone());
            }
        }

        // Find all decoder plugins for source format
        let decoders: Vec<&PluginManifest> = self.plugins.iter()
            .filter(|p| p.input_formats.iter().any(|f| f.mime == source.mime))
            .collect();

        // Find all encoder plugins for target format
        let encoders: Vec<&PluginManifest> = self.plugins.iter()
            .filter(|p| p.output_formats.iter().any(|f| f.mime == target.mime))
            .collect();

        if decoders.is_empty() {
            return Err(CoreError::NoPluginFound(format!("No decoder for {}", source.mime)));
        }
        if encoders.is_empty() {
            return Err(CoreError::NoPluginFound(format!("No encoder for {}", target.mime)));
        }

        // Build paths: for each (decoder, encoder) pair, create a path
        let mut paths: Vec<ConversionPath> = Vec::new();
        for decoder in &decoders {
            for encoder in &encoders {
                // Determine IR type based on the format category
                let ir_type = determine_ir_type(source, target);
                let fidelity = std::cmp::min(decoder.fidelity_score, encoder.fidelity_score);

                paths.push(ConversionPath {
                    decode_plugin: decoder.id.clone(),
                    encode_plugin: encoder.id.clone(),
                    ir_type,
                    fidelity_estimate: fidelity,
                    steps: 1,
                });
            }
        }

        // Sort by fidelity (highest first), then by plugin priority
        paths.sort_by(|a, b| {
            b.fidelity_estimate.cmp(&a.fidelity_estimate)
                .then_with(|| {
                    let a_priority = self.plugins.iter().find(|p| p.id == a.decode_plugin).map(|p| p.priority).unwrap_or(0);
                    let b_priority = self.plugins.iter().find(|p| p.id == b.decode_plugin).map(|p| p.priority).unwrap_or(0);
                    b_priority.cmp(&a_priority)
                })
        });

        if paths.is_empty() {
            return Err(CoreError::UnsupportedConversion {
                source: source.mime.clone(),
                target: target.mime.clone(),
            });
        }

        let best = paths[0].clone();
        self.route_cache.insert(key, paths);
        Ok(best)
    }

    /// List all supported conversions.
    pub fn supported_conversions(&self) -> Vec<(FormatId, FormatId)> {
        let mut conversions = Vec::new();
        for decoder in &self.plugins {
            for encoder in &self.plugins {
                for input in &decoder.input_formats {
                    for output in &encoder.output_formats {
                        if input.mime != output.mime {
                            conversions.push((input.clone(), output.clone()));
                        }
                    }
                }
            }
        }
        conversions.sort_by(|a, b| a.0.mime.cmp(&b.0.mime).then(a.1.mime.cmp(&b.1.mime)));
        conversions.dedup_by(|a, b| a.0.mime == b.0.mime && a.1.mime == b.1.mime);
        conversions
    }

    /// Get all registered plugin manifests.
    pub fn plugins(&self) -> &[PluginManifest] {
        &self.plugins
    }

    /// Find plugins that can decode a given format.
    pub fn find_decoders(&self, format: &FormatId) -> Vec<&PluginManifest> {
        self.plugins.iter()
            .filter(|p| p.input_formats.iter().any(|f| f.mime == format.mime))
            .collect()
    }

    /// Find plugins that can encode a given format.
    pub fn find_encoders(&self, format: &FormatId) -> Vec<&PluginManifest> {
        self.plugins.iter()
            .filter(|p| p.output_formats.iter().any(|f| f.mime == format.mime))
            .collect()
    }
}

impl Default for ConversionRouter {
    fn default() -> Self {
        Self::new()
    }
}

/// Determine the IR type based on source and target format categories.
fn determine_ir_type(source: &FormatId, target: &FormatId) -> String {
    let mime = &source.mime;
    if mime.starts_with("image/") {
        "Image".to_string()
    } else if mime.starts_with("audio/") || mime.starts_with("application/ogg") {
        "Audio".to_string()
    } else if mime.starts_with("video/") {
        "Video".to_string()
    } else if mime.starts_with("text/") || mime.starts_with("application/pdf")
        || mime.starts_with("application/rtf")
        || mime.starts_with("application/vnd.openxmlformats")
    {
        "Document".to_string()
    } else if mime.starts_with("application/zip") || mime.starts_with("application/gzip")
        || mime.starts_with("application/x-tar") || mime.starts_with("application/x-7z")
    {
        "Archive".to_string()
    } else if mime.starts_with("application/json") || mime.starts_with("text/csv")
        || mime.starts_with("application/xml") || mime.starts_with("application/x-yaml")
    {
        "Table".to_string()
    } else if mime.starts_with("font/") {
        "Font".to_string()
    } else {
        "Unknown".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_manifest(id: &str, inputs: Vec<FormatId>, outputs: Vec<FormatId>) -> PluginManifest {
        PluginManifest {
            id: id.to_string(),
            version: semver::Version::new(1, 0, 0),
            api_version: semver::Version::new(1, 0, 0),
            author: "test".to_string(),
            license: "MIT".to_string(),
            description: "test".to_string(),
            input_formats: inputs,
            output_formats: outputs,
            capabilities: ufc_plugin_api::Capabilities {
                metadata: ufc_plugin_api::MetadataSupport::ReadWrite,
                structure: ufc_plugin_api::StructureSupport::Flat,
                embedded_assets: ufc_plugin_api::EmbeddedAssetSupport::None,
                color_spaces: vec![],
                max_dimension: None,
                max_bit_depth: None,
                supports_animation: false,
                supports_transparency: false,
                supports_multi_page: false,
            },
            dependencies: vec![],
            priority: 0,
            fidelity_score: 90,
            known_limitations: vec![],
            sandbox_mode: ufc_plugin_api::SandboxMode::Wasm,
        }
    }

    #[test]
    fn test_find_path() {
        let mut router = ConversionRouter::new();

        let png = FormatId::new("image/png", &["png"], "PNG");
        let jpeg = FormatId::new("image/jpeg", &["jpg", "jpeg"], "JPEG");

        router.register_plugin(make_manifest(
            "png-decoder",
            vec![png.clone()],
            vec![],
        ));
        router.register_plugin(make_manifest(
            "jpeg-encoder",
            vec![],
            vec![jpeg.clone()],
        ));

        let path = router.find_path(&png, &jpeg).unwrap();
        assert_eq!(path.decode_plugin, "png-decoder");
        assert_eq!(path.encode_plugin, "jpeg-encoder");
        assert_eq!(path.ir_type, "Image");
    }

    #[test]
    fn test_no_decoder() {
        let mut router = ConversionRouter::new();
        let png = FormatId::new("image/png", &["png"], "PNG");
        let jpeg = FormatId::new("image/jpeg", &["jpg", "jpeg"], "JPEG");

        let result = router.find_path(&png, &jpeg);
        assert!(result.is_err());
    }
}
