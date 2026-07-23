use std::collections::HashMap;
use std::sync::Arc;
use ufc_plugin_api::{ConverterPlugin, FormatId, PluginManifest};

/// Plugin registry that manages all loaded plugins.
///
/// Handles plugin discovery, registration, conflict resolution,
/// and lookup by format.
pub struct PluginRegistry {
    plugins: Vec<Arc<dyn ConverterPlugin>>,
    /// (mime, direction) → sorted plugin indices
    decode_index: HashMap<String, Vec<usize>>,
    encode_index: HashMap<String, Vec<usize>>,
}

/// Direction of conversion (decode or encode).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Decode,
    Encode,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
            decode_index: HashMap::new(),
            encode_index: HashMap::new(),
        }
    }

    /// Register a plugin.
    pub fn register(&mut self, plugin: Arc<dyn ConverterPlugin>) {
        let manifest = plugin.manifest();
        let index = self.plugins.len();

        // Index by input formats (for decoding)
        for format in &manifest.input_formats {
            self.decode_index
                .entry(format.mime.clone())
                .or_insert_with(Vec::new)
                .push(index);
        }

        // Index by output formats (for encoding)
        for format in &manifest.output_formats {
            self.encode_index
                .entry(format.mime.clone())
                .or_insert_with(Vec::new)
                .push(index);
        }

        self.plugins.push(plugin);
        tracing::info!("Registered plugin: {}", manifest.id);
    }

    /// Remove a plugin by ID.
    pub fn unregister(&mut self, plugin_id: &str) -> bool {
        if let Some(index) = self.plugins.iter().position(|p| p.manifest().id == plugin_id) {
            self.plugins.remove(index);
            // Rebuild indices
            self.rebuild_indices();
            true
        } else {
            false
        }
    }

    /// Get a plugin by ID.
    pub fn get(&self, plugin_id: &str) -> Option<&Arc<dyn ConverterPlugin>> {
        self.plugins.iter().find(|p| p.manifest().id == plugin_id)
    }

    /// Get all plugins that can handle a given format in the specified direction.
    pub fn plugins_for_format(&self, format: &FormatId, direction: Direction) -> Vec<&Arc<dyn ConverterPlugin>> {
        let index = match direction {
            Direction::Decode => &self.decode_index,
            Direction::Encode => &self.encode_index,
        };

        index
            .get(&format.mime)
            .map(|indices| {
                indices
                    .iter()
                    .filter_map(|&i| self.plugins.get(i))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get the best plugin for a format (highest priority × fidelity).
    pub fn best_plugin(&self, format: &FormatId, direction: Direction) -> Option<&Arc<dyn ConverterPlugin>> {
        let mut candidates = self.plugins_for_format(format, direction);
        candidates.sort_by(|a, b| {
            let ma = a.manifest();
            let mb = b.manifest();
            mb.priority
                .cmp(&ma.priority)
                .then(mb.fidelity_score.cmp(&ma.fidelity_score))
        });
        candidates.into_iter().next()
    }

    /// Get all registered plugin manifests.
    pub fn manifests(&self) -> Vec<PluginManifest> {
        self.plugins.iter().map(|p| p.manifest()).collect()
    }

    /// Get the number of registered plugins.
    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }

    /// List all supported formats (input + output).
    pub fn supported_formats(&self) -> (Vec<FormatId>, Vec<FormatId>) {
        let mut inputs = Vec::new();
        let mut outputs = Vec::new();
        for plugin in &self.plugins {
            let manifest = plugin.manifest();
            inputs.extend(manifest.input_formats);
            outputs.extend(manifest.output_formats);
        }
        inputs.sort_by(|a, b| a.mime.cmp(&b.mime));
        inputs.dedup_by(|a, b| a.mime == b.mime);
        outputs.sort_by(|a, b| a.mime.cmp(&b.mime));
        outputs.dedup_by(|a, b| a.mime == b.mime);
        (inputs, outputs)
    }

    /// Rebuild the format indices after plugin removal.
    fn rebuild_indices(&mut self) {
        self.decode_index.clear();
        self.encode_index.clear();
        for (index, plugin) in self.plugins.iter().enumerate() {
            let manifest = plugin.manifest();
            for format in &manifest.input_formats {
                self.decode_index
                    .entry(format.mime.clone())
                    .or_insert_with(Vec::new)
                    .push(index);
            }
            for format in &manifest.output_formats {
                self.encode_index
                    .entry(format.mime.clone())
                    .or_insert_with(Vec::new)
                    .push(index);
            }
        }
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ufc_plugin_api::*;

    struct MockPlugin {
        manifest: PluginManifest,
    }

    impl ConverterPlugin for MockPlugin {
        fn manifest(&self) -> PluginManifest { self.manifest.clone() }
        fn probe(&self, _: &FileReader) -> Result<ProbeResult, PluginError> {
            unimplemented!()
        }
        fn decode(&self, _: &FileReader, _: &DecodeConfig, _: &ProgressCallback) -> Result<Box<dyn std::any::Any + Send + Sync>, PluginError> {
            unimplemented!()
        }
        fn encode(&self, _: &(dyn std::any::Any + Send + Sync), _: &FileWriter, _: &EncodeConfig, _: &ProgressCallback) -> Result<ConversionOutput, PluginError> {
            unimplemented!()
        }
        fn cancel(&self) -> Result<(), PluginError> { Ok(()) }
    }

    fn make_mock(id: &str, inputs: Vec<FormatId>, outputs: Vec<FormatId>) -> Arc<dyn ConverterPlugin> {
        Arc::new(MockPlugin {
            manifest: PluginManifest {
                id: id.to_string(),
                version: semver::Version::new(1, 0, 0),
                api_version: semver::Version::new(1, 0, 0),
                author: "test".to_string(),
                license: "MIT".to_string(),
                description: "test".to_string(),
                input_formats: inputs,
                output_formats: outputs,
                capabilities: Capabilities::default(),
                dependencies: vec![],
                priority: 0,
                fidelity_score: 90,
                known_limitations: vec![],
                sandbox_mode: SandboxMode::InProcess,
            },
        })
    }

    #[test]
    fn test_registry_lookup() {
        let mut registry = PluginRegistry::new();
        let png = FormatId::new("image/png", &["png"], "PNG");
        let jpeg = FormatId::new("image/jpeg", &["jpg"], "JPEG");

        registry.register(make_mock("png-dec", vec![png.clone()], vec![]));
        registry.register(make_mock("jpeg-enc", vec![], vec![jpeg.clone()]));

        let decoders = registry.plugins_for_format(&png, Direction::Decode);
        assert_eq!(decoders.len(), 1);

        let encoders = registry.plugins_for_format(&jpeg, Direction::Encode);
        assert_eq!(encoders.len(), 1);

        assert_eq!(registry.len(), 2);
    }
}
