use crate::config::AppConfig;
use crate::detector::FormatDetector;
use crate::error::CoreError;
use crate::integrity::IntegrityChecker;
use crate::queue::{ConversionQueue, QueueItem, QueueStats};
use crate::router::{ConversionPath, ConversionRouter};
use crate::state::{HistoryEntry, StateManager};
use crate::temp_manager::TempManager;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use ufc_plugin_api::{
    ConverterPlugin, DecodeConfig, EncodeConfig, FormatId, PluginManifest, ProgressCallback,
    ProgressState, ConversionPhase,
};

/// Events emitted by the orchestrator during conversion.
#[derive(Debug, Clone)]
pub enum OrchestratorEvent {
    /// A new item was added to the queue.
    ItemQueued { item_id: uuid::Uuid },
    /// Detection completed for an item.
    Detected { item_id: uuid::Uuid, format: FormatId },
    /// Conversion started for an item.
    ConversionStarted { item_id: uuid::Uuid, path: ConversionPath },
    /// Progress update for an active conversion.
    Progress { item_id: uuid::Uuid, progress: ProgressState },
    /// Conversion completed successfully.
    Completed { item_id: uuid::Uuid, bytes_written: u64, checksum: String },
    /// Conversion failed.
    Failed { item_id: uuid::Uuid, error: String },
    /// Conversion was cancelled.
    Cancelled { item_id: uuid::Uuid },
    /// All conversions completed.
    QueueFinished { stats: QueueStats },
}

/// The main conversion orchestrator.
///
/// Coordinates format detection, routing, plugin execution, and state management.
pub struct Orchestrator {
    config: AppConfig,
    detector: FormatDetector,
    router: ConversionRouter,
    queue: ConversionQueue,
    state: StateManager,
    temp: TempManager,
    integrity: IntegrityChecker,
    plugins: Vec<Arc<dyn ConverterPlugin>>,
    event_tx: Option<mpsc::UnboundedSender<OrchestratorEvent>>,
}

impl Orchestrator {
    pub fn new(config: AppConfig) -> Result<Self, CoreError> {
        let state_path = crate::state::default_state_path();
        let state = StateManager::new(state_path);
        let temp = TempManager::new(config.temp_dir.clone())?;
        let integrity = IntegrityChecker::new(config.verify_output);

        Ok(Self {
            config: config.clone(),
            detector: FormatDetector::new(),
            router: ConversionRouter::new(),
            queue: ConversionQueue::new(crate::queue::QueueConfig {
                max_concurrent: config.max_concurrent,
                max_memory_per_conversion: config.max_memory_per_conversion,
                max_total_memory: config.max_total_memory,
                auto_retry: config.auto_retry,
                max_retries: config.max_retries,
                verify_output: config.verify_output,
                overwrite_existing: config.overwrite_existing,
            }),
            state,
            temp,
            integrity,
            plugins: Vec::new(),
            event_tx: None,
        })
    }

    /// Set the event sender for receiving orchestrator events.
    pub fn set_event_sender(&mut self, tx: mpsc::UnboundedSender<OrchestratorEvent>) {
        self.event_tx = Some(tx);
    }

    /// Register a plugin with the orchestrator.
    pub fn register_plugin(&mut self, plugin: Arc<dyn ConverterPlugin>) {
        let manifest = plugin.manifest();
        self.router.register_plugin(manifest);
        self.plugins.push(plugin);
    }

    /// Add a file to the conversion queue.
    pub fn enqueue(
        &mut self,
        input: PathBuf,
        output: PathBuf,
        target_mime: &str,
    ) -> Result<uuid::Uuid, CoreError> {
        let item = QueueItem::new(input, output, target_mime.to_string());
        let id = item.id;
        self.queue.push(item);
        self.emit(OrchestratorEvent::ItemQueued { item_id: id });
        Ok(id)
    }

    /// Add multiple files for batch conversion.
    pub fn enqueue_batch(
        &mut self,
        inputs: Vec<PathBuf>,
        output_dir: &Path,
        target_ext: &str,
        target_mime: &str,
    ) -> Result<Vec<uuid::Uuid>, CoreError> {
        let mut ids = Vec::new();
        for input in inputs {
            let stem = input.file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let output = output_dir.join(format!("{}.{}", stem, target_ext));
            let id = self.enqueue(input, output, target_mime)?;
            ids.push(id);
        }
        Ok(ids)
    }

    /// Process all items in the queue (runs concurrently).
    pub async fn process_queue(&mut self) -> Result<QueueStats, CoreError> {
        // Save state before processing
        self.state.save().ok();

        while self.queue.can_start_more() {
            if let Some(item) = self.queue.next_pending() {
                let item_id = item.id;
                let input_path = item.input_path.clone();
                let output_path = item.output_path.clone();
                let target_format = item.target_format.clone();

                // Take the item out of the queue for processing
                {
                    let mut item = self.queue.get_mut(item_id).unwrap();
                    item.start();
                }

                // Process the conversion
                let result = self.convert_single(&input_path, &output_path, &target_format).await;

                // Extract result data before touching self again
                let (status, bytes_written, checksum, error_msg, source_format, duration_ms) = {
                    let item = self.queue.get_mut(item_id).unwrap();
                    let source_fmt = item.source_format.clone().unwrap_or_default();
                    let dur = item.duration().map(|d| d.num_milliseconds() as u64);
                    match &result {
                        Ok((bw, cs)) => {
                            item.complete(*bw, cs.clone());
                            ("completed", Some(*bw), Some(cs.clone()), None, source_fmt, dur)
                        }
                        Err(e) => {
                            let msg = e.to_string();
                            item.fail(msg.clone());
                            ("failed", None, None, Some(msg), source_fmt, dur)
                        }
                    }
                };

                // Now emit events and save history (no longer borrowing self.queue)
                if status == "completed" {
                    self.emit(OrchestratorEvent::Completed {
                        item_id,
                        bytes_written: bytes_written.unwrap(),
                        checksum: checksum.clone().unwrap(),
                    });
                    self.state.add_history(HistoryEntry {
                        input_path, output_path, source_format: source_format.clone(), target_format,
                        success: true, bytes_written, duration_ms, timestamp: chrono::Utc::now(), error: None,
                    });
                } else {
                    self.emit(OrchestratorEvent::Failed {
                        item_id,
                        error: error_msg.clone().unwrap(),
                    });
                    self.state.add_history(HistoryEntry {
                        input_path, output_path, source_format: source_format.clone(), target_format,
                        success: false, bytes_written: None, duration_ms, timestamp: chrono::Utc::now(),
                        error: error_msg,
                    });
                }
            } else {
                break;
            }
        }

        let stats = self.queue.stats();
        self.emit(OrchestratorEvent::QueueFinished { stats: stats.clone() });
        self.state.save().ok();
        Ok(stats)
    }

    /// Convert a single file.
    async fn convert_single(
        &mut self,
        input: &Path,
        output: &Path,
        target_mime: &str,
    ) -> Result<(u64, String), CoreError> {
        // 1. Read file header for format detection
        let header = read_header(input, 16)?;
        let source_format = self.detector.detect(input, &header)?;

        // 2. Find conversion path
        let target_format = FormatId::new(target_mime, &[], target_mime);
        let path = self.router.find_path(&source_format, &target_format)?;

        // 3. Find the decoder and encoder plugins
        let decoder = self.find_plugin(&path.decode_plugin)
            .ok_or_else(|| CoreError::NoPluginFound(path.decode_plugin.clone()))?;
        let encoder = self.find_plugin(&path.encode_plugin)
            .ok_or_else(|| CoreError::NoPluginFound(path.encode_plugin.clone()))?;

        // 4. Create temp directory for this conversion
        let session_dir = self.temp.create_session_dir()?;

        // 5. Decode: source → IR
        let (progress, _cancel) = ProgressCallback::new(|_state| {});
        let file_size = std::fs::metadata(input).map(|m| m.len()).unwrap_or(0);
        let reader = ufc_plugin_api::FileReader::new(
            Box::new(std::fs::File::open(input)?),
            input.to_path_buf(),
            file_size,
        );

        let decode_config = DecodeConfig {
            max_memory_bytes: self.config.max_memory_per_conversion,
            ..Default::default()
        };

        let ir = decoder.decode(&reader, &decode_config, &progress)?;

        // 6. Encode: IR → target
        let writer = ufc_plugin_api::FileWriter::new(
            Box::new(std::fs::File::create(output)?),
            output.to_path_buf(),
        );

        let encode_config = EncodeConfig {
            max_memory_bytes: self.config.max_memory_per_conversion,
            preserve_metadata: true,
            ..Default::default()
        };

        let result = encoder.encode(ir.as_ref(), &writer, &encode_config, &progress)?;

        // 7. Verify integrity
        if self.config.verify_output {
            let actual_checksum = self.integrity.checksum_file(output)?;
            if actual_checksum != result.checksum {
                return Err(CoreError::IntegrityCheckFailed {
                    expected: result.checksum,
                    actual: actual_checksum,
                });
            }
        }

        // 8. Cleanup
        self.temp.cleanup(&session_dir).ok();

        Ok((result.bytes_written, result.checksum))
    }

    /// Detect the format of a file.
    pub fn detect_format(&self, path: &Path) -> Result<FormatId, CoreError> {
        let header = read_header(path, 16)?;
        self.detector.detect(path, &header)
    }

    /// Get supported conversions.
    pub fn supported_conversions(&self) -> Vec<(FormatId, FormatId)> {
        self.router.supported_conversions()
    }

    /// Get a reference to the queue.
    pub fn queue(&self) -> &ConversionQueue {
        &self.queue
    }

    /// Get a mutable reference to the queue.
    pub fn queue_mut(&mut self) -> &mut ConversionQueue {
        &mut self.queue
    }

    /// Get a reference to the state manager.
    pub fn state(&self) -> &StateManager {
        &self.state
    }

    /// Cancel a specific item.
    pub fn cancel_item(&mut self, id: uuid::Uuid) -> bool {
        if let Some(item) = self.queue.get_mut(id) {
            item.cancel();
            self.emit(OrchestratorEvent::Cancelled { item_id: id });
            true
        } else {
            false
        }
    }

    /// Cancel all items.
    pub fn cancel_all(&mut self) {
        self.queue.cancel_all();
    }

    /// Pause all items.
    pub fn pause_all(&mut self) {
        self.queue.pause_all();
    }

    /// Resume all items.
    pub fn resume_all(&mut self) {
        self.queue.resume_all();
    }

    /// Find a plugin by ID.
    fn find_plugin(&self, id: &str) -> Option<Arc<dyn ConverterPlugin>> {
        self.plugins.iter().find(|p| p.manifest().id == id).cloned()
    }

    /// Emit an event to the event channel.
    fn emit(&self, event: OrchestratorEvent) {
        if let Some(tx) = &self.event_tx {
            let _ = tx.send(event);
        }
    }
}

/// Read the first N bytes of a file for format detection.
fn read_header(path: &Path, n: usize) -> Result<Vec<u8>, CoreError> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut header = vec![0u8; n];
    let bytes_read = file.read(&mut header)?;
    header.truncate(bytes_read);
    Ok(header)
}
