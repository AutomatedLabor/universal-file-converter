//! # UFC Core Engine
//!
//! The core conversion engine that orchestrates the entire file conversion pipeline:
//! format detection → routing → decode → IR → encode → verification.

pub mod config;
pub mod detector;
pub mod error;
pub mod integrity;
pub mod orchestrator;
pub mod queue;
pub mod router;
pub mod state;
pub mod temp_manager;

pub use config::AppConfig;
pub use detector::FormatDetector;
pub use error::CoreError;
pub use integrity::IntegrityChecker;
pub use orchestrator::Orchestrator;
pub use queue::{QueueConfig, QueueItem, QueueItemStatus, ConversionQueue};
pub use router::ConversionRouter;
pub use state::StateManager;
pub use temp_manager::TempManager;
