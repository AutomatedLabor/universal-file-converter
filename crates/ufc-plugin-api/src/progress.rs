use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Callback for reporting conversion progress and checking cancellation.
///
/// Plugins call `progress.update(state)` periodically to report progress.
/// Plugins call `progress.is_cancelled()` at natural yield points to check
/// if the user has requested cancellation.
#[derive(Clone)]
pub struct ProgressCallback {
    update_fn: Arc<dyn Fn(ProgressState) + Send + Sync>,
    cancelled: Arc<AtomicBool>,
}

impl ProgressCallback {
    pub fn new(update_fn: impl Fn(ProgressState) + Send + Sync + 'static) -> (Self, CancellationHandle) {
        let cancelled = Arc::new(AtomicBool::new(false));
        let handle = CancellationHandle {
            cancelled: cancelled.clone(),
        };
        let callback = Self {
            update_fn: Arc::new(update_fn),
            cancelled,
        };
        (callback, handle)
    }

    /// Report updated progress state to the host.
    pub fn update(&self, state: ProgressState) {
        (self.update_fn)(state);
    }

    /// Check if the user has requested cancellation.
    /// Plugins should call this at natural yield points.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    /// Create a no-op progress callback (for testing).
    pub fn noop() -> Self {
        Self {
            update_fn: Arc::new(|_| {}),
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }
}

/// Handle for the host to signal cancellation.
pub struct CancellationHandle {
    cancelled: Arc<AtomicBool>,
}

impl CancellationHandle {
    /// Signal the plugin to cancel.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    /// Check if cancellation was requested.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    /// Reset the cancellation flag (for reuse).
    pub fn reset(&self) {
        self.cancelled.store(false, Ordering::Relaxed);
    }
}

/// Progress state reported by plugins.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressState {
    /// Current phase of the conversion pipeline.
    pub phase: ConversionPhase,
    /// Overall progress percentage (0.0–100.0).
    pub percent: f32,
    /// Bytes processed so far.
    pub bytes_processed: u64,
    /// Total bytes expected (if known).
    pub bytes_total: Option<u64>,
    /// Time elapsed since conversion started.
    pub elapsed: Duration,
    /// Estimated time remaining (if known).
    pub eta: Option<Duration>,
    /// Human-readable status message.
    pub message: Option<String>,
}

impl ProgressState {
    pub fn new(phase: ConversionPhase) -> Self {
        Self {
            phase,
            percent: 0.0,
            bytes_processed: 0,
            bytes_total: None,
            elapsed: Duration::ZERO,
            eta: None,
            message: None,
        }
    }

    pub fn with_percent(mut self, percent: f32) -> Self {
        self.percent = percent.clamp(0.0, 100.0);
        self
    }

    pub fn with_bytes(mut self, processed: u64, total: Option<u64>) -> Self {
        self.bytes_processed = processed;
        self.bytes_total = total;
        if let Some(t) = total {
            if t > 0 {
                self.percent = (processed as f32 / t as f32) * 100.0;
            }
        }
        self
    }

    pub fn with_message(mut self, msg: impl Into<String>) -> Self {
        self.message = Some(msg.into());
        self
    }
}

/// Phase of the conversion pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConversionPhase {
    Probing,
    Decoding,
    Transforming,
    Encoding,
    Verifying,
}
