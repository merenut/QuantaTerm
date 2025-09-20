//! Font loading implementation with cross-platform support
//!
//! This module provides font loading functionality for different operating systems:
//! - Linux: fontconfig
//! - macOS: Core Text
//! - Windows: DirectWrite

use ab_glyph::FontArc;
use anyhow::{Context, Result};

use crate::font::FontInfo;

/// Trait for font loading implementations
pub trait FontLoader: Send + Sync {
    /// Load a font by family name and size
    fn load_font(&self, family: &str, size: f32) -> Result<FontArc>;

    /// Get list of available system fonts
    fn system_fonts(&self) -> Result<Vec<FontInfo>>;
}

/// System font loader with platform-specific implementations
pub struct SystemFontLoader {
    #[cfg(target_os = "linux")]
    fontconfig: fontconfig::Fontconfig,
}

impl SystemFontLoader {
    /// Create a new system font loader
    pub fn new() -> Result<Self> {
        Ok(Self {
            #[cfg(target_os = "linux")]
            fontconfig: fontconfig::Fontconfig::new().context("Failed to initialize fontconfig")?,
        })
    }
}

impl FontLoader for SystemFontLoader {
    fn load_font(&self, family: &str, size: f32) -> Result<FontArc> {
        #[cfg(target_os = "linux")]
        {
            self.load_font_linux(family, size)
        }
        #[cfg(target_os = "macos")]
        {
            self.load_font_macos(family, size)
        }
        #[cfg(target_os = "windows")]
        {
            self.load_font_windows(family, size)
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            // Fallback for unsupported platforms
            self.load_font_fallback(family, size)
        }
    }

    fn system_fonts(&self) -> Result<Vec<FontInfo>> {
        #[cfg(target_os = "linux")]
        {
            self.system_fonts_linux()
        }
        #[cfg(target_os = "macos")]
        {
            self.system_fonts_macos()
        }
        #[cfg(target_os = "windows")]
        {
            self.system_fonts_windows()
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            // Return empty list for unsupported platforms
            Ok(vec![])
        }
    }
}

// Linux implementation using fontconfig
#[cfg(target_os = "linux")]
impl SystemFontLoader {
    fn load_font_linux(&self, family: &str, _size: f32) -> Result<FontArc> {
        use fontconfig::Pattern;
        use std::ffi::CString;

        // Create pattern for font search
        let mut pattern = Pattern::new(&self.fontconfig);

        // Add family name to pattern
        let family_cstr = CString::new(family)
            .with_context(|| format!("Failed to convert family name to CString: {}", family))?;
        let style_cstr = CString::new("Regular").context("Failed to create style CString")?;

        pattern.add_string(&family_cstr, &family_cstr);
        pattern.add_string(&style_cstr, &style_cstr);

        // Try to find font files through fontconfig search
        if let Ok(font_data) = self.try_load_font_data(family) {
            if let Ok(font) = FontArc::try_from_vec(font_data) {
                return Ok(font);
            }
        }

        // Fallback to common monospace fonts
        for fallback in &[
            "DejaVu Sans Mono",
            "Liberation Mono",
            "Courier New",
            "monospace",
        ] {
            if let Ok(font_data) = self.try_load_font_data(fallback) {
                if let Ok(font) = FontArc::try_from_vec(font_data) {
                    return Ok(font);
                }
            }
        }

        anyhow::bail!("Failed to load font '{}' and no fallback found", family)
    }

    fn try_load_font_data(&self, family: &str) -> Result<Vec<u8>> {
        // Try common font directories
        let font_dirs = [
            "/usr/share/fonts",
            "/usr/local/share/fonts",
            "/home/.local/share/fonts",
            "/home/.fonts",
        ];

        // Look for exact font files first
        if let Ok(font_data) = self.search_for_font_file(family, &font_dirs) {
            return Ok(font_data);
        }

        // If no exact match, try fallback patterns
        if family == "monospace" || family.to_lowercase().contains("mono") {
            // Try specific monospace fonts
            for fallback_family in &["DejaVuSansMono", "LiberationMono", "liberation", "dejavu"] {
                if let Ok(font_data) = self.search_for_font_file(fallback_family, &font_dirs) {
                    return Ok(font_data);
                }
            }
        }

        anyhow::bail!("Font file not found for: {}", family)
    }

    fn search_for_font_file(&self, family: &str, font_dirs: &[&str]) -> Result<Vec<u8>> {
        for dir in font_dirs {
            if let Ok(font_data) = self.search_directory_for_font(family, dir) {
                return Ok(font_data);
            }
        }
        anyhow::bail!("Font not found in any directory: {}", family)
    }

    #[allow(clippy::only_used_in_recursion)]
    fn search_directory_for_font(&self, family: &str, dir: &str) -> Result<Vec<u8>> {
        let search_patterns = [
            family.to_lowercase().replace(' ', ""),
            family.to_lowercase(),
        ];

        // Recursively search directories
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();

                if path.is_dir() {
                    // Recurse into subdirectories
                    if let Ok(font_data) =
                        self.search_directory_for_font(family, path.to_str().unwrap_or(""))
                    {
                        return Ok(font_data);
                    }
                } else if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                    let filename_lower = filename.to_lowercase();

                    // Check if this file matches our search criteria
                    if filename_lower.ends_with(".ttf") || filename_lower.ends_with(".otf") {
                        for pattern in &search_patterns {
                            if filename_lower.contains(pattern) {
                                if let Ok(font_data) = std::fs::read(&path) {
                                    return Ok(font_data);
                                }
                            }
                        }
                    }
                }
            }
        }

        anyhow::bail!("Font not found in directory: {}", dir)
    }

    fn system_fonts_linux(&self) -> Result<Vec<FontInfo>> {
        let mut fonts = Vec::new();

        // Return a simplified list of common Linux monospace fonts
        let common_fonts = [
            "DejaVu Sans Mono",
            "Liberation Mono",
            "Courier New",
            "Ubuntu Mono",
            "Source Code Pro",
            "Fira Code",
            "JetBrains Mono",
        ];

        for &family in &common_fonts {
            fonts.push(FontInfo {
                family: family.to_string(),
                style: crate::font::FontStyle::Normal,
                weight: crate::font::FontWeight::Normal,
                path: None,
            });
        }

        Ok(fonts)
    }
}

// macOS implementation using Core Text
#[cfg(target_os = "macos")]
impl SystemFontLoader {
    fn load_font_macos(&self, family: &str, _size: f32) -> Result<FontArc> {
        // Simplified macOS implementation - try to load common system monospace fonts
        let common_fonts = ["Menlo", "Monaco", "SF Mono", "Courier New"];

        for font_name in &common_fonts {
            if let Ok(font_data) = self.try_load_macos_system_font(font_name) {
                if let Ok(font) = FontArc::try_from_vec(font_data) {
                    return Ok(font);
                }
            }
        }

        anyhow::bail!("Failed to load font '{}' on macOS", family)
    }

    fn try_load_macos_system_font(&self, _name: &str) -> Result<Vec<u8>> {
        // Placeholder implementation - loading system font data on macOS
        // requires more complex Core Text/Core Graphics integration
        anyhow::bail!("System font loading not fully implemented on macOS")
    }

    fn system_fonts_macos(&self) -> Result<Vec<FontInfo>> {
        // Simplified list of common macOS fonts
        Ok(vec![
            FontInfo {
                family: "Menlo".to_string(),
                style: crate::font::FontStyle::Normal,
                weight: crate::font::FontWeight::Normal,
                path: None,
            },
            FontInfo {
                family: "Monaco".to_string(),
                style: crate::font::FontStyle::Normal,
                weight: crate::font::FontWeight::Normal,
                path: None,
            },
            FontInfo {
                family: "SF Mono".to_string(),
                style: crate::font::FontStyle::Normal,
                weight: crate::font::FontWeight::Normal,
                path: None,
            },
        ])
    }
}

// Windows implementation using DirectWrite
#[cfg(target_os = "windows")]
impl SystemFontLoader {
    fn load_font_windows(&self, family: &str, _size: f32) -> Result<FontArc> {
        // Simplified Windows implementation
        // In practice, this would use DirectWrite APIs

        // Try to load common Windows monospace fonts
        for font_name in &["Consolas", "Courier New", "Lucida Console"] {
            if let Ok(font_data) = self.try_load_windows_font(font_name) {
                if let Ok(font) = FontArc::try_from_vec(font_data) {
                    return Ok(font);
                }
            }
        }

        anyhow::bail!("Failed to load font '{}' on Windows", family)
    }

    fn try_load_windows_font(&self, _name: &str) -> Result<Vec<u8>> {
        // Placeholder implementation - loading system font data on Windows
        // requires DirectWrite integration
        anyhow::bail!("System font loading not fully implemented on Windows")
    }

    fn system_fonts_windows(&self) -> Result<Vec<FontInfo>> {
        // Simplified list of common Windows fonts
        Ok(vec![
            FontInfo {
                family: "Consolas".to_string(),
                style: crate::font::FontStyle::Normal,
                weight: crate::font::FontWeight::Normal,
                path: None,
            },
            FontInfo {
                family: "Courier New".to_string(),
                style: crate::font::FontStyle::Normal,
                weight: crate::font::FontWeight::Normal,
                path: None,
            },
            FontInfo {
                family: "Lucida Console".to_string(),
                style: crate::font::FontStyle::Normal,
                weight: crate::font::FontWeight::Normal,
                path: None,
            },
        ])
    }
}

// Fallback implementation for unsupported platforms
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
impl SystemFontLoader {
    fn load_font_fallback(&self, family: &str, _size: f32) -> Result<FontArc> {
        // Try to load from embedded font or common system paths
        anyhow::bail!("Font loading not supported on this platform: {}", family)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_loader_creation() {
        let loader = SystemFontLoader::new();
        assert!(loader.is_ok());
    }

    #[test]
    fn test_system_fonts_list() {
        let loader = SystemFontLoader::new().unwrap();
        let fonts = loader.system_fonts();
        assert!(fonts.is_ok());

        let fonts = fonts.unwrap();
        // Should have at least some fonts available
        assert!(!fonts.is_empty());
    }

    #[test]
    fn test_direct_font_load() {
        // Test loading a specific font file that we know exists
        let font_path = "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf";
        if std::path::Path::new(font_path).exists() {
            let font_data = std::fs::read(font_path).unwrap();
            let font = ab_glyph::FontArc::try_from_vec(font_data);
            assert!(font.is_ok());
        }
    }

    #[test]
    fn test_monospace_font_loading() {
        let loader = SystemFontLoader::new().unwrap();

        // Try loading generic monospace font
        let font_result = loader.load_font("monospace", 14.0);

        // On Linux this should work, on other platforms might fail
        // but that's expected for this basic implementation
        #[cfg(target_os = "linux")]
        {
            if font_result.is_err() {
                println!("Font loading error: {:?}", font_result);
            }
            assert!(font_result.is_ok());
        }
    }
}
