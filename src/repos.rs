//! Terra Store v3.0 - Repository Abstraction Layer
//!
//! This module defines the `Repository` trait and implementations for
//! Pacman (Official repos) and Paru (AUR).

use std::io;
use std::process::{Command, Stdio};

use thiserror::Error;

use crate::package::{Package, PackageInfo, PackageSource};

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum RepoError {
    #[error("Failed to execute command: {0}")]
    CommandFailed(#[from] io::Error),

    #[error("Package not found: {0}")]
    PackageNotFound(String),

    #[error("Repository unavailable: {0}")]
    Unavailable(String),

    #[error("Failed to parse package data")]
    ParseError,

    #[error("Installation failed with exit code: {0}")]
    InstallFailed(i32),

    #[error("AUR helper not installed. Please install paru or yay.")]
    AurHelperNotFound,
}

/// Trait defining the interface for package repositories
#[allow(dead_code)]
pub trait Repository {
    /// Get the display name of this repository
    fn name(&self) -> &str;

    /// Get the package source type
    fn source(&self) -> PackageSource;

    /// Check if this repository is available (e.g., AUR helper installed)
    fn is_available(&self) -> bool;

    /// List all available packages (names only for fuzzy search)
    fn list_packages(&self) -> Result<Vec<String>, RepoError>;

    /// Get detailed information about a specific package
    fn get_info(&self, name: &str) -> Result<PackageInfo, RepoError>;

    /// Install a package (with inherited stdout for progress display)
    fn install(&self, name: &str) -> Result<(), RepoError>;

    /// Search packages by name (returns matching packages with basic info)
    fn search(&self, query: &str) -> Result<Vec<Package>, RepoError>;
}

// ============================================================================
// Pacman Implementation (Official Repositories)
// ============================================================================

/// Official Arch Linux repository handler
pub struct Pacman;

impl Pacman {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Pacman {
    fn default() -> Self {
        Self::new()
    }
}

impl Repository for Pacman {
    fn name(&self) -> &str {
        "Official Repositories"
    }

    fn source(&self) -> PackageSource {
        PackageSource::Official
    }

    fn is_available(&self) -> bool {
        Command::new("pacman")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    fn list_packages(&self) -> Result<Vec<String>, RepoError> {
        let output = Command::new("pacman").args(["-Slq"]).output()?;

        if !output.status.success() {
            return Err(RepoError::Unavailable(
                "Failed to query pacman database".to_string(),
            ));
        }

        let packages = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|s| s.to_string())
            .collect();

        Ok(packages)
    }

    fn get_info(&self, name: &str) -> Result<PackageInfo, RepoError> {
        let output = Command::new("pacman").args(["-Si", name]).output()?;

        if !output.status.success() {
            return Err(RepoError::PackageNotFound(name.to_string()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        PackageInfo::from_pacman_output(&stdout, PackageSource::Official)
            .ok_or(RepoError::ParseError)
    }

    fn install(&self, name: &str) -> Result<(), RepoError> {
        let status = Command::new("sudo")
            .args(["pacman", "-S", "--noconfirm", name])
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()?;

        if status.success() {
            Ok(())
        } else {
            Err(RepoError::InstallFailed(status.code().unwrap_or(-1)))
        }
    }

    fn search(&self, query: &str) -> Result<Vec<Package>, RepoError> {
        let output = Command::new("pacman").args(["-Ss", query]).output()?;

        if !output.status.success() {
            return Ok(Vec::new()); // No results is not an error
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let packages = parse_pacman_search_output(&stdout, PackageSource::Official);

        Ok(packages)
    }
}

// ============================================================================
// Paru Implementation (AUR)
// ============================================================================

/// AUR repository handler using paru
pub struct Paru;

impl Paru {
    pub fn new() -> Self {
        Self
    }

    /// Check if paru is installed
    fn paru_available() -> bool {
        Command::new("paru")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Check if yay is installed as fallback
    fn yay_available() -> bool {
        Command::new("yay")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Get the available AUR helper command
    fn get_helper() -> Option<&'static str> {
        if Self::paru_available() {
            Some("paru")
        } else if Self::yay_available() {
            Some("yay")
        } else {
            None
        }
    }
}

impl Default for Paru {
    fn default() -> Self {
        Self::new()
    }
}

impl Repository for Paru {
    fn name(&self) -> &str {
        "Arch User Repository (AUR)"
    }

    fn source(&self) -> PackageSource {
        PackageSource::Aur
    }

    fn is_available(&self) -> bool {
        Self::get_helper().is_some()
    }

    fn list_packages(&self) -> Result<Vec<String>, RepoError> {
        let helper = Self::get_helper().ok_or(RepoError::AurHelperNotFound)?;

        let output = Command::new(helper).args(["-Slq"]).output()?;

        if !output.status.success() {
            return Err(RepoError::Unavailable(
                "Failed to query AUR database".to_string(),
            ));
        }

        let packages = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|s| s.to_string())
            .collect();

        Ok(packages)
    }

    fn get_info(&self, name: &str) -> Result<PackageInfo, RepoError> {
        let helper = Self::get_helper().ok_or(RepoError::AurHelperNotFound)?;

        let output = Command::new(helper).args(["-Si", name]).output()?;

        if !output.status.success() {
            return Err(RepoError::PackageNotFound(name.to_string()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        PackageInfo::from_pacman_output(&stdout, PackageSource::Aur).ok_or(RepoError::ParseError)
    }

    fn install(&self, name: &str) -> Result<(), RepoError> {
        let helper = Self::get_helper().ok_or(RepoError::AurHelperNotFound)?;

        let status = Command::new(helper)
            .args(["-S", "--noconfirm", name])
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()?;

        if status.success() {
            Ok(())
        } else {
            Err(RepoError::InstallFailed(status.code().unwrap_or(-1)))
        }
    }

    fn search(&self, query: &str) -> Result<Vec<Package>, RepoError> {
        let helper = Self::get_helper().ok_or(RepoError::AurHelperNotFound)?;

        let output = Command::new(helper).args(["-Ss", query]).output()?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let packages = parse_pacman_search_output(&stdout, PackageSource::Aur);

        Ok(packages)
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Parse the output of `pacman -Ss` or `paru -Ss`
#[allow(dead_code)]
fn parse_pacman_search_output(output: &str, source: PackageSource) -> Vec<Package> {
    let mut packages = Vec::new();
    let mut lines = output.lines().peekable();

    while let Some(line) = lines.next() {
        // Package lines start with repo/name version
        if line.starts_with(char::is_whitespace) {
            continue; // Skip description lines for now
        }

        // Parse: "extra/package-name 1.2.3-1 [installed]"
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }

        let name_part = parts[0];
        let version = parts[1].to_string();

        // Extract package name from "repo/name"
        let name = name_part
            .split('/')
            .nth(1)
            .unwrap_or(name_part)
            .to_string();

        // Get description from next line
        let description = lines
            .peek()
            .filter(|l| l.starts_with(char::is_whitespace))
            .map(|l| l.trim().to_string())
            .unwrap_or_default();

        if lines.peek().is_some_and(|l| l.starts_with(char::is_whitespace)) {
            lines.next(); // Consume description line
        }

        packages.push(Package::with_details(name, version, description, source));
    }

    packages
}

/// Unified repository manager that can query both sources
pub struct RepoManager {
    pub pacman: Pacman,
    pub aur: Paru,
}

impl RepoManager {
    pub fn new() -> Self {
        Self {
            pacman: Pacman::new(),
            aur: Paru::new(),
        }
    }

    /// Get a list of all available packages from both sources
    #[allow(dead_code)]
    pub fn list_all(&self) -> Result<Vec<String>, RepoError> {
        let mut all = self.pacman.list_packages()?;

        if self.aur.is_available() {
            if let Ok(aur_packages) = self.aur.list_packages() {
                all.extend(aur_packages);
            }
        }

        Ok(all)
    }

    /// Smart search: Try official first, fall back to AUR
    #[allow(dead_code)]
    pub fn smart_search(&self, query: &str) -> Result<Vec<Package>, RepoError> {
        let mut results = self.pacman.search(query)?;

        if self.aur.is_available() {
            if let Ok(aur_results) = self.aur.search(query) {
                results.extend(aur_results);
            }
        }

        Ok(results)
    }
}

impl Default for RepoManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pacman_available() {
        let pacman = Pacman::new();
        // This will be true on Arch systems
        let _ = pacman.is_available();
    }

    #[test]
    fn test_parse_search_output() {
        let output = "extra/neofetch 7.1.0-2
    A CLI system information tool
core/coreutils 9.4-3
    The basic file, shell and text manipulation utilities";

        let packages = parse_pacman_search_output(output, PackageSource::Official);
        assert_eq!(packages.len(), 2);
        assert_eq!(packages[0].name, "neofetch");
        assert_eq!(packages[1].name, "coreutils");
    }
}
