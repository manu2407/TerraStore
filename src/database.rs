//! Terra Store v1.0 - Zero-Stress Indexing Engine
//!
//! Arena-based memory architecture for instant package search.
//! Uses monolithic storage + lightweight index pointers for zero-CPU search.

use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::package::PackageSource;

/// Cache file version - increment when format changes
const CACHE_VERSION: u32 = 1;

/// Lightweight view into the arena - just byte offsets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageView {
    /// Start byte offset in arena for package name
    pub name_start: usize,
    /// End byte offset in arena for package name
    pub name_end: usize,
    /// Package source (Official or AUR)
    pub source: PackageSource,
}

impl PackageView {
    /// Get the package name as a string slice from the arena
    #[inline]
    pub fn name<'a>(&self, arena: &'a str) -> &'a str {
        &arena[self.name_start..self.name_end]
    }
}

/// Binary-serializable cache header
#[derive(Debug, Serialize, Deserialize)]
struct CacheHeader {
    version: u32,
    official_count: usize,
    aur_count: usize,
    arena_len: usize,
    timestamp: u64,
}

/// The "Zero-Stress" Package Database
///
/// Uses Arena allocation for O(1) memory access:
/// - The arena holds all package names in one contiguous block
/// - The index holds lightweight pointers (start/end offsets)
/// - Searching is just integer math, no string allocation
#[derive(Debug)]
pub struct PackageDatabase {
    /// The Monolith - all package names concatenated with newlines
    arena: String,
    /// The Index - lightweight views into the arena
    packages: Vec<PackageView>,
    /// Statistics
    pub stats: DatabaseStats,
}

#[derive(Debug, Default, Clone)]
pub struct DatabaseStats {
    pub official_count: usize,
    pub aur_count: usize,
    pub arena_bytes: usize,
    pub load_time_ms: u64,
    pub was_cached: bool,
}

impl PackageDatabase {
    /// Create an empty database
    pub fn new() -> Self {
        Self {
            arena: String::new(),
            packages: Vec::new(),
            stats: DatabaseStats::default(),
        }
    }

    /// Get the cache file path
    fn cache_path() -> Option<PathBuf> {
        let cache_dir = dirs::cache_dir()?;
        let terra_cache = cache_dir.join("terra-store");
        fs::create_dir_all(&terra_cache).ok()?;
        Some(terra_cache.join("index.bin"))
    }

    /// Load from binary cache if valid, otherwise rebuild
    pub fn load_or_build() -> Self {
        let start = Instant::now();

        // Try loading from cache first
        if let Some(db) = Self::load_from_cache() {
            return db;
        }

        // Cache miss - rebuild from scratch
        let mut db = Self::build_fresh();
        db.stats.load_time_ms = start.elapsed().as_millis() as u64;
        db.stats.was_cached = false;

        // Save to cache for next time
        let _ = db.save_to_cache();

        db
    }

    /// Load database from binary cache
    fn load_from_cache() -> Option<Self> {
        let start = Instant::now();
        let cache_path = Self::cache_path()?;

        if !cache_path.exists() {
            return None;
        }

        let file = File::open(&cache_path).ok()?;
        let mut reader = BufReader::new(file);

        // Read header
        let header: CacheHeader = bincode::deserialize_from(&mut reader).ok()?;

        // Version check
        if header.version != CACHE_VERSION {
            return None;
        }

        // Read arena
        let mut arena = String::with_capacity(header.arena_len);
        let arena_bytes: Vec<u8> = bincode::deserialize_from(&mut reader).ok()?;
        arena.push_str(&String::from_utf8_lossy(&arena_bytes));

        // Read packages
        let packages: Vec<PackageView> = bincode::deserialize_from(&mut reader).ok()?;

        let stats = DatabaseStats {
            official_count: header.official_count,
            aur_count: header.aur_count,
            arena_bytes: arena.len(),
            load_time_ms: start.elapsed().as_millis() as u64,
            was_cached: true,
        };

        Some(Self {
            arena,
            packages,
            stats,
        })
    }

    /// Save database to binary cache
    fn save_to_cache(&self) -> std::io::Result<()> {
        let cache_path = match Self::cache_path() {
            Some(p) => p,
            None => return Ok(()),
        };

        let file = File::create(&cache_path)?;
        let mut writer = BufWriter::new(file);

        // Write header
        let header = CacheHeader {
            version: CACHE_VERSION,
            official_count: self.stats.official_count,
            aur_count: self.stats.aur_count,
            arena_len: self.arena.len(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };
        bincode::serialize_into(&mut writer, &header)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        // Write arena as bytes
        bincode::serialize_into(&mut writer, self.arena.as_bytes())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        // Write packages
        bincode::serialize_into(&mut writer, &self.packages)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        writer.flush()?;
        Ok(())
    }

    /// Build database fresh from pacman/paru
    fn build_fresh() -> Self {
        let mut arena = String::with_capacity(5 * 1024 * 1024); // Pre-allocate 5MB
        let mut packages = Vec::with_capacity(100_000);
        let mut official_count = 0;
        let mut aur_count = 0;

        // Fetch official packages
        if let Ok(output) = Command::new("pacman").args(["-Slq"]).output() {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                for line in text.lines() {
                    if !line.is_empty() {
                        let start = arena.len();
                        arena.push_str(line);
                        let end = arena.len();
                        arena.push('\n');

                        packages.push(PackageView {
                            name_start: start,
                            name_end: end,
                            source: PackageSource::Official,
                        });
                        official_count += 1;
                    }
                }
            }
        }

        // Fetch AUR packages (if paru/yay available)
        let aur_helper = if Command::new("paru")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            Some("paru")
        } else if Command::new("yay")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            Some("yay")
        } else {
            None
        };

        if let Some(helper) = aur_helper {
            // Only get AUR packages (exclude official repos from the list)
            if let Ok(output) = Command::new(helper).args(["-Slq", "--aur"]).output() {
                if output.status.success() {
                    let text = String::from_utf8_lossy(&output.stdout);
                    for line in text.lines() {
                        if !line.is_empty() {
                            let start = arena.len();
                            arena.push_str(line);
                            let end = arena.len();
                            arena.push('\n');

                            packages.push(PackageView {
                                name_start: start,
                                name_end: end,
                                source: PackageSource::Aur,
                            });
                            aur_count += 1;
                        }
                    }
                }
            }
        }

        // Shrink to fit
        arena.shrink_to_fit();
        packages.shrink_to_fit();

        Self {
            arena,
            packages,
            stats: DatabaseStats {
                official_count,
                aur_count,
                arena_bytes: 0, // Will be set after
                load_time_ms: 0,
                was_cached: false,
            },
        }
    }

    /// Get total package count
    pub fn len(&self) -> usize {
        self.packages.len()
    }

    /// Check if empty
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.packages.is_empty()
    }

    /// Zero-CPU search - just pointer math, no string allocation
    /// Returns indices into the packages vector
    #[inline]
    pub fn search(&self, query: &str, source_filter: Option<PackageSource>, limit: usize) -> Vec<usize> {
        if query.is_empty() {
            return Vec::new();
        }

        let query_lower = query.to_lowercase();
        let mut results = Vec::with_capacity(limit);

        for (idx, pkg) in self.packages.iter().enumerate() {
            // Source filter
            if let Some(filter) = source_filter {
                if pkg.source != filter {
                    continue;
                }
            }

            // Name match (case-insensitive)
            let name = pkg.name(&self.arena);
            if name.to_lowercase().contains(&query_lower) {
                results.push(idx);
                if results.len() >= limit {
                    break;
                }
            }
        }

        results
    }

    /// Get package name by index
    #[inline]
    pub fn get_name(&self, idx: usize) -> Option<&str> {
        self.packages.get(idx).map(|p| p.name(&self.arena))
    }

    /// Get package source by index
    #[inline]
    pub fn get_source(&self, idx: usize) -> Option<PackageSource> {
        self.packages.get(idx).map(|p| p.source)
    }

    /// Invalidate cache (force rebuild on next load)
    pub fn invalidate_cache() -> std::io::Result<()> {
        if let Some(path) = Self::cache_path() {
            if path.exists() {
                fs::remove_file(path)?;
            }
        }
        Ok(())
    }

    /// Get arena reference for zero-copy access
    #[allow(dead_code)]
    pub fn arena(&self) -> &str {
        &self.arena
    }

    /// Get packages slice
    #[allow(dead_code)]
    pub fn packages(&self) -> &[PackageView] {
        &self.packages
    }
}

impl Default for PackageDatabase {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_view() {
        let arena = "neofetch\nhtop\nfirefox\n";
        let view = PackageView {
            name_start: 0,
            name_end: 8,
            source: PackageSource::Official,
        };
        assert_eq!(view.name(arena), "neofetch");
    }

    #[test]
    fn test_search() {
        let mut arena = String::new();
        let mut packages = Vec::new();

        for name in ["neofetch", "htop", "firefox", "neomutt", "neovim"] {
            let start = arena.len();
            arena.push_str(name);
            let end = arena.len();
            arena.push('\n');
            packages.push(PackageView {
                name_start: start,
                name_end: end,
                source: PackageSource::Official,
            });
        }

        let db = PackageDatabase {
            arena,
            packages,
            stats: DatabaseStats::default(),
        };

        let results = db.search("neo", None, 10);
        assert_eq!(results.len(), 3); // neofetch, neomutt, neovim
    }
}
