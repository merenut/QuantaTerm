//! Font system for QuantaTerm renderer
//!
//! This module provides font loading, caching, and management functionality
//! for the terminal renderer. It supports cross-platform font discovery
//! and fallback chains for missing glyphs.

use anyhow::Result;
use std::collections::HashMap;

pub mod loader;
pub mod shaper;
pub mod atlas;

pub use loader::{FontLoader, SystemFontLoader};
// pub use shaper::{GlyphShaper, GlyphInfo}; // Will be implemented in Task 1.2
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

    /// Get the list of available system fonts
    pub fn system_fonts(&self) -> Result<Vec<FontInfo>> {
        self.loader.system_fonts()
    }

    /// Build default fallback font chain
    fn build_fallback_chain() -> Vec<FontInfo> {
        vec![
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
}