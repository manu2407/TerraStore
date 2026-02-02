//! Terra Store v3.0 - Package Data Structures
//!
//! This module defines the core data types for representing packages
//! and their metadata across different repository sources.

use serde::{Deserialize, Serialize};

/// Represents the source repository of a package
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PackageSource {
    /// Official Arch Linux repositories (core, extra, multilib)
    #[default]
    Official,
    /// Arch User Repository (AUR)
    Aur,
}

impl std::fmt::Display for PackageSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PackageSource::Official => write!(f, "Official"),
            PackageSource::Aur => write!(f, "AUR"),
        }
    }
}

/// A minimal package representation for list views
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub description: String,
    pub source: PackageSource,
}

impl Package {
    #[allow(dead_code)]
    pub fn new(name: impl Into<String>, source: PackageSource) -> Self {
        Self {
            name: name.into(),
            version: String::new(),
            description: String::new(),
            source,
        }
    }

    /// Create a package with full metadata
    #[allow(dead_code)]
    pub fn with_details(
        name: impl Into<String>,
        version: impl Into<String>,
        description: impl Into<String>,
        source: PackageSource,
    ) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            description: description.into(),
            source,
        }
    }
}

/// Extended package information for the detail view
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub url: String,
    pub licenses: Vec<String>,
    pub groups: Vec<String>,
    pub provides: Vec<String>,
    pub depends: Vec<String>,
    pub optional_deps: Vec<String>,
    pub conflicts: Vec<String>,
    pub replaces: Vec<String>,
    pub download_size: u64,
    pub installed_size: u64,
    pub packager: String,
    pub build_date: String,
    pub install_reason: Option<String>,
    pub source: PackageSource,
}

impl PackageInfo {
    /// Parse package info from `pacman -Si` or `paru -Si` output
    #[allow(dead_code)]
    pub fn from_pacman_output(output: &str, source: PackageSource) -> Option<Self> {
        let mut info = PackageInfo {
            source,
            ..Default::default()
        };

        for line in output.lines() {
            let line = line.trim();
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.trim();

                match key {
                    "Name" => info.name = value.to_string(),
                    "Version" => info.version = value.to_string(),
                    "Description" => info.description = value.to_string(),
                    "URL" => info.url = value.to_string(),
                    "Licenses" => {
                        info.licenses = value.split_whitespace().map(String::from).collect()
                    }
                    "Groups" => info.groups = value.split_whitespace().map(String::from).collect(),
                    "Provides" => {
                        info.provides = value.split_whitespace().map(String::from).collect()
                    }
                    "Depends On" => {
                        info.depends = value.split_whitespace().map(String::from).collect()
                    }
                    "Optional Deps" => {
                        info.optional_deps = value.split_whitespace().map(String::from).collect()
                    }
                    "Conflicts With" => {
                        info.conflicts = value.split_whitespace().map(String::from).collect()
                    }
                    "Replaces" => {
                        info.replaces = value.split_whitespace().map(String::from).collect()
                    }
                    "Download Size" => {
                        info.download_size = parse_size(value);
                    }
                    "Installed Size" => {
                        info.installed_size = parse_size(value);
                    }
                    "Packager" => info.packager = value.to_string(),
                    "Build Date" => info.build_date = value.to_string(),
                    _ => {}
                }
            }
        }

        if info.name.is_empty() {
            None
        } else {
            Some(info)
        }
    }

    /// Format the info for display in the preview pane
    #[allow(dead_code)]
    pub fn to_display_string(&self) -> String {
        let mut output = String::new();

        output.push_str(&format!("📦 {} {}\n", self.name, self.version));
        output.push_str(&format!("   {}\n\n", self.description));

        if !self.url.is_empty() {
            output.push_str(&format!("🔗 {}\n", self.url));
        }

        if !self.licenses.is_empty() {
            output.push_str(&format!("📜 License: {}\n", self.licenses.join(", ")));
        }

        output.push_str(&format!(
            "💾 Download: {} | Installed: {}\n",
            format_size(self.download_size),
            format_size(self.installed_size)
        ));

        if !self.depends.is_empty() {
            output.push_str(&format!("\n📋 Dependencies ({}):\n", self.depends.len()));
            for dep in &self.depends {
                output.push_str(&format!("   • {}\n", dep));
            }
        }

        if !self.optional_deps.is_empty() {
            output.push_str(&format!(
                "\n📋 Optional Dependencies ({}):\n",
                self.optional_deps.len()
            ));
            for dep in &self.optional_deps {
                output.push_str(&format!("   • {}\n", dep));
            }
        }

        output
    }
}

/// Parse size string like "1.5 MiB" to bytes
#[allow(dead_code)]
fn parse_size(s: &str) -> u64 {
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() != 2 {
        return 0;
    }

    let num: f64 = parts[0].parse().unwrap_or(0.0);
    let unit = parts[1].to_uppercase();

    let multiplier = match unit.as_str() {
        "B" => 1,
        "KIB" | "KB" => 1024,
        "MIB" | "MB" => 1024 * 1024,
        "GIB" | "GB" => 1024 * 1024 * 1024,
        _ => 1,
    };

    (num * multiplier as f64) as u64
}

/// Format bytes to human-readable size
#[allow(dead_code)]
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GiB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MiB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KiB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_size() {
        assert_eq!(parse_size("1.5 MiB"), 1572864);
        assert_eq!(parse_size("100 KiB"), 102400);
        assert_eq!(parse_size("1 GiB"), 1073741824);
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(1572864), "1.50 MiB");
        assert_eq!(format_size(102400), "100.00 KiB");
    }
}
