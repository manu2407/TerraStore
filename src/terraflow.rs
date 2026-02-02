//! Terra Store v3.0 - TerraFlow Config Sync
//!
//! Audits system packages against dotfiles package lists.
//! Provides "what's missing" and "what's extra" reports.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::package::PackageSource;

/// Result of auditing packages against config
#[derive(Debug, Default)]
pub struct AuditResult {
    /// Packages in config but not installed
    pub missing: Vec<PackageEntry>,
    /// Packages installed but not in config (optional tracking)
    pub extra: Vec<String>,
    /// Total packages in config files
    pub config_count: usize,
    /// Total packages installed on system
    pub installed_count: usize,
}

/// A package entry from config files
#[derive(Debug, Clone)]
pub struct PackageEntry {
    pub name: String,
    pub source: PackageSource,
    pub file: String,
}

/// TerraFlow configuration manager
pub struct TerraFlow {
    /// Path to the dotfiles packages directory
    packages_dir: PathBuf,
}

impl TerraFlow {
    /// Create a new TerraFlow instance
    pub fn new(packages_dir: impl Into<PathBuf>) -> Self {
        Self {
            packages_dir: packages_dir.into(),
        }
    }

    /// Auto-detect packages directory in common locations
    pub fn auto_detect() -> Option<Self> {
        // Check common dotfiles locations
        let home = dirs::home_dir()?;
        
        let candidates = [
            home.join("TerraFlow-Dotfiles/packages"),
            home.join(".dotfiles/packages"),
            home.join("dotfiles/packages"),
            home.join(".config/terraflow/packages"),
        ];

        for path in candidates {
            if path.is_dir() {
                return Some(Self::new(path));
            }
        }

        None
    }

    /// Load all package entries from config files
    pub fn load_config_packages(&self) -> Vec<PackageEntry> {
        let mut packages = Vec::new();

        if !self.packages_dir.is_dir() {
            return packages;
        }

        // Read all .txt files in the packages directory
        if let Ok(entries) = fs::read_dir(&self.packages_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.extension().map(|e| e == "txt").unwrap_or(false) {
                    let source = Self::detect_source(&path);
                    let file_name = path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string();

                    if let Ok(contents) = fs::read_to_string(&path) {
                        for line in contents.lines() {
                            let name = line.trim();
                            if !name.is_empty() && !name.starts_with('#') && name != "." {
                                packages.push(PackageEntry {
                                    name: name.to_string(),
                                    source,
                                    file: file_name.clone(),
                                });
                            }
                        }
                    }
                }
            }
        }

        packages
    }

    /// Detect package source from filename
    fn detect_source(path: &Path) -> PackageSource {
        let name = path.file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        if name.contains("aur") {
            PackageSource::Aur
        } else {
            PackageSource::Official
        }
    }

    /// Get list of installed packages on the system
    pub fn get_installed_packages() -> HashSet<String> {
        let mut installed = HashSet::new();

        // Get explicitly installed packages
        if let Ok(output) = Command::new("pacman").args(["-Qeq"]).output() {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                for line in text.lines() {
                    if !line.is_empty() {
                        installed.insert(line.to_string());
                    }
                }
            }
        }

        installed
    }

    /// Audit: compare config packages against installed packages
    pub fn audit(&self) -> AuditResult {
        let config_packages = self.load_config_packages();
        let installed = Self::get_installed_packages();

        let config_names: HashSet<String> = config_packages.iter()
            .map(|p| p.name.clone())
            .collect();

        let mut missing = Vec::new();
        for pkg in &config_packages {
            if !installed.contains(&pkg.name) {
                missing.push(pkg.clone());
            }
        }

        // Packages installed but not in config (informational)
        let extra: Vec<String> = installed.iter()
            .filter(|p| !config_names.contains(*p))
            .cloned()
            .collect();

        AuditResult {
            missing,
            extra,
            config_count: config_packages.len(),
            installed_count: installed.len(),
        }
    }

    /// Export currently installed packages to a file
    #[allow(dead_code)]
    pub fn export_installed(&self, output_path: &Path) -> std::io::Result<usize> {
        let installed = Self::get_installed_packages();
        let mut sorted: Vec<_> = installed.into_iter().collect();
        sorted.sort();

        let content = sorted.join("\n");
        fs::write(output_path, content)?;

        Ok(sorted.len())
    }

    /// Get the packages directory path
    #[allow(dead_code)]
    pub fn packages_dir(&self) -> &Path {
        &self.packages_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_source() {
        assert_eq!(
            TerraFlow::detect_source(Path::new("aur.txt")),
            PackageSource::Aur
        );
        assert_eq!(
            TerraFlow::detect_source(Path::new("pacman_system.txt")),
            PackageSource::Official
        );
    }
}
