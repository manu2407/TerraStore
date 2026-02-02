//! Terra Store v3.0 - Pywal Theme Integration
//!
//! Loads color schemes from ~/.cache/wal/colors.json for dynamic theming.

use std::fs;
use std::path::PathBuf;

use ratatui::style::Color;
use serde::Deserialize;

/// Pywal color scheme
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct PywalColors {
    pub wallpaper: Option<String>,
    pub special: SpecialColors,
    pub colors: ColorPalette,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct SpecialColors {
    pub background: String,
    pub foreground: String,
    pub cursor: String,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct ColorPalette {
    pub color0: String,
    pub color1: String,
    pub color2: String,
    pub color3: String,
    pub color4: String,
    pub color5: String,
    pub color6: String,
    pub color7: String,
    pub color8: String,
    pub color9: String,
    pub color10: String,
    pub color11: String,
    pub color12: String,
    pub color13: String,
    pub color14: String,
    pub color15: String,
}

/// Application theme derived from Pywal or defaults
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Theme {
    pub bg: Color,
    pub fg: Color,
    pub accent: Color,
    pub secondary: Color,
    pub success: Color,
    pub error: Color,
    pub warning: Color,
    pub muted: Color,
    pub highlight_bg: Color,
    pub border: Color,
}

impl Default for Theme {
    fn default() -> Self {
        // Terra Store default dark theme
        Self {
            bg: Color::Rgb(31, 36, 40),           // #1f2428
            fg: Color::Rgb(225, 228, 232),        // #e1e4e8
            accent: Color::Rgb(152, 195, 121),    // #98c379 (green)
            secondary: Color::Rgb(209, 154, 102), // #d19a66 (orange)
            success: Color::Rgb(152, 195, 121),   // #98c379
            error: Color::Rgb(224, 108, 117),     // #e06c75
            warning: Color::Rgb(229, 192, 123),   // #e5c07b
            muted: Color::Rgb(92, 99, 112),       // #5c6370
            highlight_bg: Color::Rgb(40, 44, 52), // #282c34
            border: Color::Rgb(62, 68, 81),       // #3e4451
        }
    }
}

impl Theme {
    /// Load theme from Pywal colors.json
    pub fn from_pywal() -> Option<Self> {
        let path = pywal_colors_path()?;
        let contents = fs::read_to_string(path).ok()?;
        let pywal: PywalColors = serde_json::from_str(&contents).ok()?;

        Some(Self {
            bg: parse_hex_color(&pywal.special.background)?,
            fg: parse_hex_color(&pywal.special.foreground)?,
            accent: parse_hex_color(&pywal.colors.color2)?,     // Usually green
            secondary: parse_hex_color(&pywal.colors.color3)?,  // Usually yellow/orange
            success: parse_hex_color(&pywal.colors.color2)?,    // Green
            error: parse_hex_color(&pywal.colors.color1)?,      // Red
            warning: parse_hex_color(&pywal.colors.color3)?,    // Yellow
            muted: parse_hex_color(&pywal.colors.color8)?,      // Bright black
            highlight_bg: parse_hex_color(&pywal.colors.color0)?, // Black variant
            border: parse_hex_color(&pywal.colors.color8)?,     // Bright black
        })
    }

    /// Load Pywal theme or fall back to defaults
    pub fn load() -> Self {
        Self::from_pywal().unwrap_or_default()
    }
}

/// Get the path to Pywal's colors.json
fn pywal_colors_path() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    let path = home.join(".cache/wal/colors.json");
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

/// Parse a hex color string like "#1f2428" to ratatui Color
fn parse_hex_color(hex: &str) -> Option<Color> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }

    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;

    Some(Color::Rgb(r, g, b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex_color() {
        assert_eq!(parse_hex_color("#ff0000"), Some(Color::Rgb(255, 0, 0)));
        assert_eq!(parse_hex_color("#00ff00"), Some(Color::Rgb(0, 255, 0)));
        assert_eq!(parse_hex_color("1f2428"), Some(Color::Rgb(31, 36, 40)));
    }

    #[test]
    fn test_default_theme() {
        let theme = Theme::default();
        assert_eq!(theme.bg, Color::Rgb(31, 36, 40));
    }
}
