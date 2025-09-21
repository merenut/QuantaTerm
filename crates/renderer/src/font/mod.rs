//! Font system for QuantaTerm renderer
//!
//! This module provides font loading, caching, and management functionality
//! for the terminal renderer. It supports cross-platform font discovery
//! and fallback chains for missing glyphs.

use anyhow::Result;
use std::collections::HashMap;
use ab_glyph::Font; // Add this import for glyph_id method

pub mod atlas;
pub mod loader;
pub mod shaper;

pub use loader::{FontLoader, SystemFontLoader};
pub use shaper::{GlyphInfo, GlyphShaper};
// pub use atlas::{GlyphAtlas, AtlasRegion}; // Will be implemented in Task 1.3

/// Font information metadata
#[derive(Debug, Clone)]
pub struct FontInfo {
    /// Font family name
    pub family: String,
    /// Font style
    pub style: FontStyle,
    /// Font weight
    pub weight: FontWeight,
    /// Optional path to font file
    pub path: Option<std::path::PathBuf>,
}

/// Font style variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontStyle {
    /// Normal/regular style
    Normal,
    /// Italic style
    Italic,
    /// Oblique style
    Oblique,
}

/// Font weight variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontWeight {
    /// Normal weight
    Normal,
    /// Bold weight
    Bold,
    /// Light weight
    Light,
    /// Extra bold weight
    ExtraBold,
}

/// Font cache key for efficient lookups
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct FontKey {
    /// Font family name
    family: String,
    /// Size in points * 64 for sub-pixel precision
    size: u32,
    /// Font style
    style: FontStyle,
    /// Font weight
    weight: FontWeight,
}

/// Main font system that manages font loading and caching
pub struct FontSystem {
    /// Font loader implementation
    loader: Box<dyn FontLoader>,
    /// Cache of loaded fonts
    font_cache: HashMap<FontKey, ab_glyph::FontArc>,
    /// Fallback font chain
    fallback_chain: Vec<FontInfo>,
}

impl FontSystem {
    /// Create a new font system with platform-specific loader
    pub fn new() -> Result<Self> {
        let loader = Box::new(SystemFontLoader::new()?);
        let fallback_chain = Self::build_fallback_chain();

        Ok(Self {
            loader,
            font_cache: HashMap::new(),
            fallback_chain,
        })
    }

    /// Load a font by family name and size
    pub fn load_font(&mut self, family: &str, size: f32) -> Result<ab_glyph::FontArc> {
        let key = FontKey {
            family: family.to_string(),
            size: (size * 64.0) as u32,
            style: FontStyle::Normal,
            weight: FontWeight::Normal,
        };

        // Check cache first
        if let Some(font) = self.font_cache.get(&key) {
            return Ok(font.clone());
        }

        // Try to load primary font
        if let Ok(font) = self.loader.load_font(family, size) {
            self.font_cache.insert(key.clone(), font.clone());
            return Ok(font);
        }

        // Try fallback fonts
        for fallback in &self.fallback_chain {
            if let Ok(font) = self.loader.load_font(&fallback.family, size) {
                self.font_cache.insert(key.clone(), font.clone());
                return Ok(font);
            }
        }

        anyhow::bail!("No suitable font found for family: {}", family)
    }

    /// Find a font that can render the given codepoint
    pub fn find_font_for_codepoint(&mut self, codepoint: u32, size: f32) -> Result<ab_glyph::FontArc> {
        let ch = char::from_u32(codepoint).unwrap_or('?');
        
        // Check all fonts in fallback chain to see which can render this codepoint
        for fallback in &self.fallback_chain {
            if let Ok(font) = self.loader.load_font(&fallback.family, size) {
                // Check if this font has a glyph for the character
                let glyph_id = font.glyph_id(ch);
                if glyph_id.0 != 0 { // glyph ID 0 usually means missing glyph
                    // Cache this font
                    let key = FontKey {
                        family: fallback.family.clone(),
                        size: (size * 64.0) as u32,
                        style: fallback.style,
                        weight: fallback.weight,
                    };
                    self.font_cache.insert(key, font.clone());
                    return Ok(font);
                }
            }
        }
        
        // If no font in fallback chain can render it, return the first available font
        self.load_font("monospace", size)
    }

    /// Get the fallback chain for missing glyphs
    pub fn fallback_chain(&self) -> &[FontInfo] {
        &self.fallback_chain
    }

    /// Add a font to the fallback chain
    pub fn add_fallback_font(&mut self, font_info: FontInfo) {
        self.fallback_chain.push(font_info);
    }

    /// Get the list of available system fonts
    pub fn system_fonts(&self) -> Result<Vec<FontInfo>> {
        self.loader.system_fonts()
    }

    /// Build default fallback font chain with comprehensive Unicode coverage
    fn build_fallback_chain() -> Vec<FontInfo> {
        vec![
            // Primary monospace fonts with good Unicode support
            FontInfo {
                family: "JetBrains Mono".to_string(),
                style: FontStyle::Normal,
                weight: FontWeight::Normal,
                path: None,
            },
            FontInfo {
                family: "Fira Code".to_string(),
                style: FontStyle::Normal,
                weight: FontWeight::Normal,
                path: None,
            },
            FontInfo {
                family: "Source Code Pro".to_string(),
                style: FontStyle::Normal,
                weight: FontWeight::Normal,
                path: None,
            },
            // System monospace fonts
            FontInfo {
                family: "Consolas".to_string(),
                style: FontStyle::Normal,
                weight: FontWeight::Normal,
                path: None,
            },
            FontInfo {
                family: "Monaco".to_string(),
                style: FontStyle::Normal,
                weight: FontWeight::Normal,
                path: None,
            },
            FontInfo {
                family: "Menlo".to_string(),
                style: FontStyle::Normal,
                weight: FontWeight::Normal,
                path: None,
            },
            // Unicode coverage fonts
            FontInfo {
                family: "DejaVu Sans Mono".to_string(),
                style: FontStyle::Normal,
                weight: FontWeight::Normal,
                path: None,
            },
            FontInfo {
                family: "Liberation Mono".to_string(),
                style: FontStyle::Normal,
                weight: FontWeight::Normal,
                path: None,
            },
            // Noto fonts for comprehensive script coverage
            FontInfo {
                family: "Noto Sans Mono".to_string(),
                style: FontStyle::Normal,
                weight: FontWeight::Normal,
                path: None,
            },
            FontInfo {
                family: "Noto Sans Mono CJK SC".to_string(), // Chinese Simplified
                style: FontStyle::Normal,
                weight: FontWeight::Normal,
                path: None,
            },
            FontInfo {
                family: "Noto Sans Mono CJK JP".to_string(), // Japanese
                style: FontStyle::Normal,
                weight: FontWeight::Normal,
                path: None,
            },
            FontInfo {
                family: "Noto Sans Mono CJK KR".to_string(), // Korean
                style: FontStyle::Normal,
                weight: FontWeight::Normal,
                path: None,
            },
            // Arabic script support
            FontInfo {
                family: "Noto Sans Arabic".to_string(),
                style: FontStyle::Normal,
                weight: FontWeight::Normal,
                path: None,
            },
            // Hebrew script support
            FontInfo {
                family: "Noto Sans Hebrew".to_string(),
                style: FontStyle::Normal,
                weight: FontWeight::Normal,
                path: None,
            },
            // Devanagari script support
            FontInfo {
                family: "Noto Sans Devanagari".to_string(),
                style: FontStyle::Normal,
                weight: FontWeight::Normal,
                path: None,
            },
            // Emoji support
            FontInfo {
                family: "Noto Color Emoji".to_string(),
                style: FontStyle::Normal,
                weight: FontWeight::Normal,
                path: None,
            },
            FontInfo {
                family: "Apple Color Emoji".to_string(),
                style: FontStyle::Normal,
                weight: FontWeight::Normal,
                path: None,
            },
            FontInfo {
                family: "Segoe UI Emoji".to_string(),
                style: FontStyle::Normal,
                weight: FontWeight::Normal,
                path: None,
            },
            // Final fallback
            FontInfo {
                family: "Courier New".to_string(),
                style: FontStyle::Normal,
                weight: FontWeight::Normal,
                path: None,
            },
        ]
    }

    /// Clear the font cache
    pub fn clear_cache(&mut self) {
        self.font_cache.clear();
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> (usize, usize) {
        (self.font_cache.len(), self.font_cache.capacity())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ab_glyph::Font;

    #[test]
    fn test_font_system_creation() {
        let font_system = FontSystem::new();
        assert!(font_system.is_ok());
    }

    #[test]
    fn test_font_loading() {
        let mut font_system = FontSystem::new().unwrap();
        let font = font_system.load_font("monospace", 14.0);
        assert!(font.is_ok());

        let font = font.unwrap();
        assert!(font.glyph_count() > 0);
    }

    #[test]
    fn test_fallback_behavior() {
        let mut font_system = FontSystem::new().unwrap();
        // Try loading a font that definitely doesn't exist
        let font = font_system.load_font("NonExistentFont123", 14.0);
        assert!(font.is_ok()); // Should succeed via fallback
    }

    #[test]
    fn test_font_caching() {
        let mut font_system = FontSystem::new().unwrap();

        // Load same font twice - should use cache
        let _font1 = font_system.load_font("monospace", 14.0).unwrap();
        let _font2 = font_system.load_font("monospace", 14.0).unwrap();

        // Cache should contain only one entry for the same font parameters
        let stats = font_system.cache_stats();
        assert_eq!(stats.0, 1); // Only one font cached despite two loads
    }

    #[test]
    fn test_cache_management() {
        let mut font_system = FontSystem::new().unwrap();

        let initial_stats = font_system.cache_stats();
        assert_eq!(initial_stats.0, 0); // Empty cache initially

        // Load a font
        let _font = font_system.load_font("monospace", 14.0).unwrap();
        let stats = font_system.cache_stats();
        assert_eq!(stats.0, 1); // One font cached

        // Clear cache
        font_system.clear_cache();
        let final_stats = font_system.cache_stats();
        assert_eq!(final_stats.0, 0); // Cache cleared
    }

    #[test]
    fn test_codepoint_fallback() {
        let mut font_system = FontSystem::new().unwrap();
        
        // Test ASCII character (should work with most fonts)
        let ascii_font = font_system.find_font_for_codepoint(b'A' as u32, 14.0);
        assert!(ascii_font.is_ok());
        
        // Test Unicode character - may or may not be supported
        let unicode_font = font_system.find_font_for_codepoint(0x1F600, 14.0); // 😀 emoji
        // Should at least return a fallback font, even if it can't render the character
        assert!(unicode_font.is_ok());
    }

    #[test]
    fn test_fallback_chain_access() {
        let font_system = FontSystem::new().unwrap();
        let fallback_chain = font_system.fallback_chain();
        
        // Should have a reasonable number of fallback fonts
        assert!(fallback_chain.len() > 5);
        
        // Should include common monospace fonts
        let families: Vec<&str> = fallback_chain.iter().map(|f| f.family.as_str()).collect();
        assert!(families.contains(&"Courier New"));
    }

    #[test]
    fn test_add_fallback_font() {
        let mut font_system = FontSystem::new().unwrap();
        let initial_count = font_system.fallback_chain().len();
        
        font_system.add_fallback_font(FontInfo {
            family: "Test Font".to_string(),
            style: FontStyle::Normal,
            weight: FontWeight::Normal,
            path: None,
        });
        
        assert_eq!(font_system.fallback_chain().len(), initial_count + 1);
        assert!(font_system.fallback_chain().iter().any(|f| f.family == "Test Font"));
    }
}
