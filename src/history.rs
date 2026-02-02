//! Terra Store v3.0 - Installation History
//!
//! Tracks package installations for rollback and audit purposes.

use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::package::PackageSource;

/// Maximum history entries to keep
const MAX_HISTORY_ENTRIES: usize = 500;

/// A single installation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallRecord {
    /// Package name
    pub name: String,
    /// Package source
    pub source: PackageSource,
    /// Unix timestamp of installation
    pub timestamp: u64,
    /// Whether installation succeeded
    pub success: bool,
    /// Optional error message if failed
    pub error: Option<String>,
}

impl InstallRecord {
    /// Create a successful installation record
    pub fn success(name: impl Into<String>, source: PackageSource) -> Self {
        Self {
            name: name.into(),
            source,
            timestamp: current_timestamp(),
            success: true,
            error: None,
        }
    }

    /// Create a failed installation record
    pub fn failure(name: impl Into<String>, source: PackageSource, error: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            source,
            timestamp: current_timestamp(),
            success: false,
            error: Some(error.into()),
        }
    }

    /// Format timestamp for display
    pub fn formatted_time(&self) -> String {
        // Simple formatting - just show relative time
        let now = current_timestamp();
        let diff = now.saturating_sub(self.timestamp);

        if diff < 60 {
            "just now".to_string()
        } else if diff < 3600 {
            format!("{} min ago", diff / 60)
        } else if diff < 86400 {
            format!("{} hours ago", diff / 3600)
        } else {
            format!("{} days ago", diff / 86400)
        }
    }
}

/// Installation history manager
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct History {
    /// List of installation records (newest first)
    pub records: Vec<InstallRecord>,
}

impl History {
    /// Get the history file path
    fn path() -> Option<PathBuf> {
        let data_dir = dirs::data_dir()?;
        let terra_dir = data_dir.join("terra-store");
        fs::create_dir_all(&terra_dir).ok()?;
        Some(terra_dir.join("history.json"))
    }

    /// Load history from disk
    pub fn load() -> Self {
        let path = match Self::path() {
            Some(p) => p,
            None => return Self::default(),
        };

        if !path.exists() {
            return Self::default();
        }

        let file = match File::open(&path) {
            Ok(f) => f,
            Err(_) => return Self::default(),
        };

        let reader = BufReader::new(file);
        serde_json::from_reader(reader).unwrap_or_default()
    }

    /// Save history to disk
    pub fn save(&self) -> std::io::Result<()> {
        let path = match Self::path() {
            Some(p) => p,
            None => return Ok(()),
        };

        let file = File::create(&path)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, self)?;
        Ok(())
    }

    /// Add a new installation record
    pub fn add(&mut self, record: InstallRecord) {
        self.records.insert(0, record);

        // Trim to max size
        if self.records.len() > MAX_HISTORY_ENTRIES {
            self.records.truncate(MAX_HISTORY_ENTRIES);
        }
    }

    /// Record a successful installation
    pub fn record_success(&mut self, name: &str, source: PackageSource) {
        self.add(InstallRecord::success(name, source));
        let _ = self.save();
    }

    /// Record a failed installation
    pub fn record_failure(&mut self, name: &str, source: PackageSource, error: &str) {
        self.add(InstallRecord::failure(name, source, error));
        let _ = self.save();
    }

    /// Get recent installations (last N)
    pub fn recent(&self, count: usize) -> &[InstallRecord] {
        let end = count.min(self.records.len());
        &self.records[..end]
    }

    /// Get count of successful installations
    pub fn success_count(&self) -> usize {
        self.records.iter().filter(|r| r.success).count()
    }

    /// Get count of failed installations
    pub fn failure_count(&self) -> usize {
        self.records.iter().filter(|r| !r.success).count()
    }

    /// Get last installation
    #[allow(dead_code)]
    pub fn last(&self) -> Option<&InstallRecord> {
        self.records.first()
    }

    /// Clear all history
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.records.clear();
        let _ = self.save();
    }
}

/// Get current unix timestamp
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_install_record() {
        let record = InstallRecord::success("neofetch", PackageSource::Official);
        assert!(record.success);
        assert!(record.error.is_none());
    }

    #[test]
    fn test_history_add() {
        let mut history = History::default();
        history.add(InstallRecord::success("neofetch", PackageSource::Official));
        history.add(InstallRecord::success("htop", PackageSource::Official));
        assert_eq!(history.records.len(), 2);
        assert_eq!(history.records[0].name, "htop"); // Newest first
    }
}
