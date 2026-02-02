//! Terra Store v3.0 - Flatpak Universal Module
//!
//! Lazy-loaded Flatpak support via AppStream XML parsing.
//! Only loads when user explicitly requests Universal mode.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

use flate2::read::GzDecoder;
use quick_xml::events::Event;
use quick_xml::Reader;

/// A Flatpak application entry
#[derive(Debug, Clone)]
pub struct FlatpakApp {
    /// Application ID (e.g., org.mozilla.firefox)
    pub id: String,
    /// Display name
    pub name: String,
    /// Short description
    pub summary: String,
}

/// Flatpak database statistics
#[derive(Debug, Default, Clone)]
pub struct FlatpakStats {
    pub app_count: usize,
    pub load_time_ms: u64,
    pub source: String,
}

/// Lazy-loaded Flatpak database
#[derive(Debug, Default)]
pub struct FlatpakDatabase {
    /// Applications (None = not loaded yet)
    apps: Option<Vec<FlatpakApp>>,
    /// Load statistics
    pub stats: FlatpakStats,
}

impl FlatpakDatabase {
    /// Create an empty (unloaded) database
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if Flatpak is installed
    pub fn is_available() -> bool {
        Command::new("flatpak")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Check if database is loaded
    pub fn is_loaded(&self) -> bool {
        self.apps.is_some()
    }

    /// Lazy load: ingest Flatpak apps on demand
    pub fn load(&mut self) -> Result<(), String> {
        if self.is_loaded() {
            return Ok(());
        }

        let start = Instant::now();

        // Try AppStream XML first (fastest)
        if let Some(apps) = Self::parse_appstream() {
            self.stats = FlatpakStats {
                app_count: apps.len(),
                load_time_ms: start.elapsed().as_millis() as u64,
                source: "AppStream".to_string(),
            };
            self.apps = Some(apps);
            return Ok(());
        }

        // Fallback to flatpak CLI
        if let Some(apps) = Self::parse_flatpak_cli() {
            self.stats = FlatpakStats {
                app_count: apps.len(),
                load_time_ms: start.elapsed().as_millis() as u64,
                source: "CLI".to_string(),
            };
            self.apps = Some(apps);
            return Ok(());
        }

        Err("Failed to load Flatpak database".to_string())
    }

    /// Parse AppStream XML from Flathub
    fn parse_appstream() -> Option<Vec<FlatpakApp>> {
        // Common AppStream locations
        let paths = [
            PathBuf::from("/var/lib/flatpak/appstream/flathub/x86_64/active/appstream.xml.gz"),
            PathBuf::from("/var/lib/flatpak/appstream/flathub/x86_64/active/appstream.xml"),
        ];

        for path in &paths {
            if !path.exists() {
                continue;
            }

            let apps = if path.extension().map(|e| e == "gz").unwrap_or(false) {
                Self::parse_gzipped_xml(path)
            } else {
                Self::parse_plain_xml(path)
            };

            if let Some(apps) = apps {
                if !apps.is_empty() {
                    return Some(apps);
                }
            }
        }

        None
    }

    /// Parse gzipped AppStream XML
    fn parse_gzipped_xml(path: &PathBuf) -> Option<Vec<FlatpakApp>> {
        let file = File::open(path).ok()?;
        let decoder = GzDecoder::new(file);
        let reader = BufReader::new(decoder);
        Self::parse_xml_reader(reader)
    }

    /// Parse plain AppStream XML
    fn parse_plain_xml(path: &PathBuf) -> Option<Vec<FlatpakApp>> {
        let file = File::open(path).ok()?;
        let reader = BufReader::new(file);
        Self::parse_xml_reader(reader)
    }

    /// Stream-parse XML to extract app info (memory efficient)
    fn parse_xml_reader<R: BufRead>(reader: R) -> Option<Vec<FlatpakApp>> {
        let mut xml = Reader::from_reader(reader);
        xml.config_mut().trim_text(true);

        let mut apps = Vec::with_capacity(3000);
        let mut buf = Vec::with_capacity(1024);

        let mut in_component = false;
        let mut current_id = String::new();
        let mut current_name = String::new();
        let mut current_summary = String::new();
        let mut current_tag = String::new();

        loop {
            match xml.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let name = e.name();
                    let tag = String::from_utf8_lossy(name.as_ref()).to_string();

                    if tag == "component" {
                        in_component = true;
                        current_id.clear();
                        current_name.clear();
                        current_summary.clear();
                    }

                    if in_component {
                        current_tag = tag;
                    }
                }
                Ok(Event::Text(e)) => {
                    if in_component {
                        let text = e.unescape().unwrap_or_default().to_string();
                        match current_tag.as_str() {
                            "id" => current_id = text,
                            "name" => {
                                if current_name.is_empty() {
                                    current_name = text;
                                }
                            }
                            "summary" => {
                                if current_summary.is_empty() {
                                    current_summary = text;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Ok(Event::End(e)) => {
                    let name = e.name();
                    if name.as_ref() == b"component" && in_component {
                        if !current_id.is_empty() && !current_name.is_empty() {
                            apps.push(FlatpakApp {
                                id: current_id.clone(),
                                name: current_name.clone(),
                                summary: current_summary.clone(),
                            });
                        }
                        in_component = false;
                    }
                    current_tag.clear();
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
            buf.clear();
        }

        apps.shrink_to_fit();
        Some(apps)
    }

    /// Fallback: Parse from flatpak CLI
    fn parse_flatpak_cli() -> Option<Vec<FlatpakApp>> {
        let output = Command::new("flatpak")
            .args(["remote-ls", "--app", "--columns=application,name,description"])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let text = String::from_utf8_lossy(&output.stdout);
        let mut apps = Vec::new();

        for line in text.lines() {
            let parts: Vec<&str> = line.splitn(3, '\t').collect();
            if parts.len() >= 2 {
                apps.push(FlatpakApp {
                    id: parts[0].to_string(),
                    name: parts[1].to_string(),
                    summary: parts.get(2).unwrap_or(&"").to_string(),
                });
            }
        }

        Some(apps)
    }

    /// Get app count (0 if not loaded)
    pub fn len(&self) -> usize {
        self.apps.as_ref().map(|a| a.len()).unwrap_or(0)
    }

    /// Search Flatpaks (only if loaded)
    pub fn search(&self, query: &str, limit: usize) -> Vec<&FlatpakApp> {
        let Some(apps) = &self.apps else {
            return Vec::new();
        };

        let query_lower = query.to_lowercase();

        apps.iter()
            .filter(|app| {
                app.id.to_lowercase().contains(&query_lower)
                    || app.name.to_lowercase().contains(&query_lower)
            })
            .take(limit)
            .collect()
    }

    /// Install a Flatpak
    #[allow(dead_code)]
    pub fn install(&self, app_id: &str) -> Result<(), String> {
        let status = Command::new("flatpak")
            .args(["install", "-y", "flathub", app_id])
            .status()
            .map_err(|e| e.to_string())?;

        if status.success() {
            Ok(())
        } else {
            Err(format!("Flatpak install failed with code: {:?}", status.code()))
        }
    }

    /// Unload to free memory
    #[allow(dead_code)]
    pub fn unload(&mut self) {
        self.apps = None;
        self.stats = FlatpakStats::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flatpak_available() {
        // Just check it doesn't panic
        let _ = FlatpakDatabase::is_available();
    }

    #[test]
    fn test_empty_database() {
        let db = FlatpakDatabase::new();
        assert!(!db.is_loaded());
        assert_eq!(db.len(), 0);
    }
}
