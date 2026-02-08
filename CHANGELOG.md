# Changelog

All notable changes to Terra Store will be documented in this file.

## [3.1.0] - 2026-02-09

### Changed
- **Standalone Release**: TerraStore is now an independent project, separated from TerraFlow-Dotfiles
- TerraFlow integration is now an optional feature (enabled by default)
- Added `TERRA_PACKAGES_DIR` environment variable for configuring package list location
- XDG-compliant default path: `~/.config/terra-store/packages`

### Added
- Proper documentation for standalone usage
- Feature flag `terraflow` for optional dotfiles integration
- Environment variable configuration support

## [3.0.0] - 2026-01-28

### Added
- Zero-Stress indexing for instant package search
- Binary cache for faster startup
- Flatpak AppStream support
- Installation history tracking
- TerraFlow config sync (audit mode)

### Changed
- Complete TUI rewrite using ratatui
- Gruvbox/CuteCat theme integration

## [2.0.0] - Previous

- Initial TUI implementation
- Basic pacman/AUR support
