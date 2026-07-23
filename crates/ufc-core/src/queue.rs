use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::PathBuf;
use uuid::Uuid;

/// Configuration for the conversion queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueConfig {
    pub max_concurrent: usize,
    pub max_memory_per_conversion: u64,
    pub max_total_memory: u64,
    pub auto_retry: bool,
    pub max_retries: u32,
    pub verify_output: bool,
    pub overwrite_existing: bool,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            max_concurrent: num_cpus::get(),
            max_memory_per_conversion: 512 * 1024 * 1024,
            max_total_memory: 4 * 1024 * 1024 * 1024,
            auto_retry: false,
            max_retries: 2,
            verify_output: true,
            overwrite_existing: false,
        }
    }
}

/// A single item in the conversion queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItem {
    pub id: Uuid,
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub source_format: Option<String>,
    pub target_format: String,
    pub status: QueueItemStatus,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub priority: u8,
    pub retry_count: u32,
    pub error: Option<String>,
    pub bytes_written: Option<u64>,
    pub output_checksum: Option<String>,
}

/// Status of a queue item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueueItemStatus {
    Pending,
    Detecting,
    Converting,
    Paused,
    Completed,
    Failed,
    Cancelled,
    Skipped,
}

impl QueueItem {
    pub fn new(input_path: PathBuf, output_path: PathBuf, target_format: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            input_path,
            output_path,
            source_format: None,
            target_format,
            status: QueueItemStatus::Pending,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            priority: 0,
            retry_count: 0,
            error: None,
            bytes_written: None,
            output_checksum: None,
        }
    }

    pub fn start(&mut self) {
        self.status = QueueItemStatus::Converting;
        self.started_at = Some(Utc::now());
    }

    pub fn complete(&mut self, bytes_written: u64, checksum: String) {
        self.status = QueueItemStatus::Completed;
        self.completed_at = Some(Utc::now());
        self.bytes_written = Some(bytes_written);
        self.output_checksum = Some(checksum);
    }

    pub fn fail(&mut self, error: String) {
        self.status = QueueItemStatus::Failed;
        self.completed_at = Some(Utc::now());
        self.error = Some(error);
    }

    pub fn cancel(&mut self) {
        self.status = QueueItemStatus::Cancelled;
        self.completed_at = Some(Utc::now());
    }

    pub fn pause(&mut self) {
        self.status = QueueItemStatus::Paused;
    }

    pub fn resume(&mut self) {
        self.status = QueueItemStatus::Pending;
    }

    pub fn duration(&self) -> Option<chrono::Duration> {
        match (self.started_at, self.completed_at) {
            (Some(start), Some(end)) => Some(end - start),
            (Some(start), None) => Some(Utc::now() - start),
            _ => None,
        }
    }
}

/// The conversion queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionQueue {
    pub items: Vec<QueueItem>,
    pub config: QueueConfig,
}

impl ConversionQueue {
    pub fn new(config: QueueConfig) -> Self {
        Self {
            items: Vec::new(),
            config,
        }
    }

    /// Add an item to the queue.
    pub fn push(&mut self, item: QueueItem) -> Uuid {
        let id = item.id;
        self.items.push(item);
        self.sort_by_priority();
        id
    }

    /// Get the next pending item.
    pub fn next_pending(&mut self) -> Option<&mut QueueItem> {
        self.items.iter_mut().find(|i| i.status == QueueItemStatus::Pending)
    }

    /// Get count of items by status.
    pub fn count_by_status(&self, status: QueueItemStatus) -> usize {
        self.items.iter().filter(|i| i.status == status).count()
    }

    /// Get count of currently converting items.
    pub fn active_count(&self) -> usize {
        self.items.iter().filter(|i| i.status == QueueItemStatus::Converting).count()
    }

    /// Check if we can start more conversions.
    pub fn can_start_more(&self) -> bool {
        self.active_count() < self.config.max_concurrent
    }

    /// Get all pending items.
    pub fn pending_items(&self) -> Vec<&QueueItem> {
        self.items.iter().filter(|i| i.status == QueueItemStatus::Pending).collect()
    }

    /// Get all items.
    pub fn all_items(&self) -> &[QueueItem] {
        &self.items
    }

    /// Get an item by ID.
    pub fn get(&self, id: Uuid) -> Option<&QueueItem> {
        self.items.iter().find(|i| i.id == id)
    }

    /// Get a mutable reference to an item by ID.
    pub fn get_mut(&mut self, id: Uuid) -> Option<&mut QueueItem> {
        self.items.iter_mut().find(|i| i.id == id)
    }

    /// Cancel all pending items.
    pub fn cancel_all(&mut self) {
        for item in &mut self.items {
            if item.status == QueueItemStatus::Pending || item.status == QueueItemStatus::Converting {
                item.cancel();
            }
        }
    }

    /// Pause all converting items.
    pub fn pause_all(&mut self) {
        for item in &mut self.items {
            if item.status == QueueItemStatus::Converting {
                item.pause();
            }
        }
    }

    /// Resume all paused items.
    pub fn resume_all(&mut self) {
        for item in &mut self.items {
            if item.status == QueueItemStatus::Paused {
                item.resume();
            }
        }
    }

    /// Clear completed, failed, and cancelled items.
    pub fn clear_finished(&mut self) {
        self.items.retain(|i| {
            i.status == QueueItemStatus::Pending
                || i.status == QueueItemStatus::Converting
                || i.status == QueueItemStatus::Paused
                || i.status == QueueItemStatus::Detecting
        });
    }

    /// Remove an item by ID.
    pub fn remove(&mut self, id: Uuid) -> bool {
        let len = self.items.len();
        self.items.retain(|i| i.id != id);
        self.items.len() < len
    }

    /// Sort items by priority (higher first), then by creation time.
    fn sort_by_priority(&mut self) {
        self.items.sort_by(|a, b| {
            b.priority.cmp(&a.priority)
                .then(a.created_at.cmp(&b.created_at))
        });
    }

    /// Summary statistics.
    pub fn stats(&self) -> QueueStats {
        QueueStats {
            total: self.items.len(),
            pending: self.count_by_status(QueueItemStatus::Pending),
            converting: self.count_by_status(QueueItemStatus::Converting),
            completed: self.count_by_status(QueueItemStatus::Completed),
            failed: self.count_by_status(QueueItemStatus::Failed),
            cancelled: self.count_by_status(QueueItemStatus::Cancelled),
            paused: self.count_by_status(QueueItemStatus::Paused),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStats {
    pub total: usize,
    pub pending: usize,
    pub converting: usize,
    pub completed: usize,
    pub failed: usize,
    pub cancelled: usize,
    pub paused: usize,
}

impl Default for ConversionQueue {
    fn default() -> Self {
        Self::new(QueueConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_operations() {
        let mut queue = ConversionQueue::default();

        let item1 = QueueItem::new(
            PathBuf::from("a.png"),
            PathBuf::from("a.jpg"),
            "image/jpeg".to_string(),
        );
        let item2 = QueueItem::new(
            PathBuf::from("b.png"),
            PathBuf::from("b.jpg"),
            "image/jpeg".to_string(),
        );

        let id1 = queue.push(item1);
        let _id2 = queue.push(item2);

        assert_eq!(queue.stats().total, 2);
        assert_eq!(queue.stats().pending, 2);

        let next = queue.next_pending().unwrap();
        assert_eq!(next.id, id1);

        let item = queue.get_mut(id1).unwrap();
        item.start();
        assert_eq!(queue.stats().converting, 1);

        item.complete(1024, "abc123".to_string());
        assert_eq!(queue.stats().completed, 1);
    }
}
