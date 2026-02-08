# Changelog

All notable changes to Terra Store will be documented in this file.

## [1.0.0] - 2026-02-09

### The New Beginning

This marks the **first standalone release** of TerraStore—a fresh start after being separated from TerraFlow-Dotfiles.

While the internal version history reached 2.9 during development inside dotfiles, this release represents a new identity: TerraStore is now an independent, first-class project.

### What's New in 1.0.0

- **Standalone Release**: TerraStore is now an independent project with its own repository
- **Optional TerraFlow Integration**: The `terraflow` feature (enabled by default) allows syncing with dotfiles package lists
- **Configurable Paths**: Added `TERRA_PACKAGES_DIR` environment variable for specifying package list location
- **XDG Compliance**: Default path is now `~/.config/terra-store/packages`
- **Proper Documentation**: Comprehensive README, LICENSE, and CHANGELOG

### Core Features (Inherited)

- Zero-Stress fuzzy search across all packages
- Multi-source support: Official repos, AUR (via paru), and Flatpak
- Beautiful Gruvbox-themed TUI built with ratatui
- Installation history tracking
- Binary cache for faster startup
- TerraFlow audit mode for package list synchronization

---

## Pre-1.0 History (Inside Dotfiles)

The journey before independence:

| Version | Stack | Notes |
|---------|-------|-------|
| 0.x | Shell (sh) | First implementation, rough but functional |
| 1.x | Lua | Over-engineered, abandoned |
| 2.0-2.5 | Shell | Refined, stable, long-running |
| 2.7-2.9 | Rust | Learning phase, multiple rewrites |
| 3.0 | Rust | "Metamorphosis" - finally faster than shell |

Version 3.0 inside dotfiles became 1.0.0 as standalone—a symbolic fresh start.
