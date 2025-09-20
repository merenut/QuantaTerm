//! Glyph shaping implementation using basic font processing
//!
//! This module provides text shaping functionality. Initially implements basic ASCII shaping
//! with caching, and provides the foundation for future Harfbuzz integration.

use anyhow::Result;
use std::collections::HashMap;
use ab_glyph::{FontArc, Font, ScaleFont};

/// Glyph shaping information
#[derive(Debug, Clone)]
pub struct GlyphInfo {
    /// Glyph identifier
    pub glyph_id: u32,
    /// Horizontal advance in pixels
    pub x_advance: f32,
    /// Vertical advance in pixels  
    pub y_advance: f32,
    /// Horizontal offset in pixels
    pub x_offset: f32,
    /// Vertical offset in pixels
    pub y_offset: f32,
}

/// Glyph shaper for text layout and positioning
pub struct GlyphShaper {
    /// The font being used for shaping
    font: FontArc,
    /// Font size in pixels
    font_size: f32,
    /// Cache for shaping results to improve performance
    feature_cache: HashMap<String, Vec<GlyphInfo>>,
    /// Cache hit counter for performance tracking
    cache_hits: usize,
    /// Cache miss counter for performance tracking
    cache_misses: usize,
}

impl GlyphShaper {
    /// Create a new glyph shaper with the given font
    pub fn new(font: FontArc, font_size: f32) -> Result<Self> {
        Ok(Self {
            font,
            font_size,
            feature_cache: HashMap::new(),
            cache_hits: 0,
            cache_misses: 0,
        })
    }
    
    /// Shape text into a sequence of positioned glyphs
    pub fn shape(&mut self, text: &str) -> Vec<GlyphInfo> {
        // Check cache first
        if let Some(cached_result) = self.feature_cache.get(text) {
            self.cache_hits += 1;
            return cached_result.clone();
        }
        
        self.cache_misses += 1;
        
        // Perform basic ASCII shaping
        let mut glyphs = Vec::new();
        let scaled_font = self.font.as_scaled(self.font_size);
        
        for ch in text.chars() {
            let glyph_id = self.font.glyph_id(ch);
            
            // Get advance width
            let advance_width = scaled_font.h_advance(glyph_id);
            
            glyphs.push(GlyphInfo {
                glyph_id: glyph_id.0 as u32,
                x_advance: advance_width,
                y_advance: 0.0, // Horizontal text
                x_offset: 0.0,
                y_offset: 0.0,
            });
        }
        
        // Cache the result
        self.feature_cache.insert(text.to_string(), glyphs.clone());
        
        glyphs
    }
    
    /// Shape text with specific font features (basic implementation)
    pub fn shape_with_features(&mut self, text: &str, features: &[&str]) -> Vec<GlyphInfo> {
        // Create cache key that includes features
        let cache_key = format!("{}|{}", text, features.join(","));
        
        // Check cache first
        if let Some(cached_result) = self.feature_cache.get(&cache_key) {
            self.cache_hits += 1;
            return cached_result.clone();
        }
        
        self.cache_misses += 1;
        
        // For now, basic implementation doesn't support advanced features
        // This is a foundation for future Harfbuzz integration
        let mut glyphs = Vec::new();
        let scaled_font = self.font.as_scaled(self.font_size);
        
        // Handle basic ligatures for programming fonts
        let processed_text = self.process_ligatures(text, features);
        
        for ch in processed_text.chars() {
            let glyph_id = self.font.glyph_id(ch);
            let advance_width = scaled_font.h_advance(glyph_id);
            
            glyphs.push(GlyphInfo {
                glyph_id: glyph_id.0 as u32,
                x_advance: advance_width,
                y_advance: 0.0,
                x_offset: 0.0,
                y_offset: 0.0,
            });
        }
        
        // Cache the result
        self.feature_cache.insert(cache_key, glyphs.clone());
        
        glyphs
    }
    
    /// Basic ligature processing (placeholder for full implementation)
    fn process_ligatures(&self, text: &str, features: &[&str]) -> String {
        if !features.contains(&"liga") && !features.contains(&"calt") {
            return text.to_string();
        }
        
        // Simple ligature replacements for common programming symbols
        let mut result = text.to_string();
        
        // Replace common programming ligatures with Unicode equivalents
        result = result.replace("->", "→");
        result = result.replace("=>", "⇒");
        result = result.replace("<=", "≤");
        result = result.replace(">=", "≥");
        result = result.replace("!=", "≠");
        result = result.replace("==", "≡");
        
        result
    }
    
    /// Clear the shaping cache
    pub fn clear_cache(&mut self) {
        self.feature_cache.clear();
        self.cache_hits = 0;
        self.cache_misses = 0;
    }
    
    /// Get cache statistics (entry count, capacity)
    pub fn cache_stats(&self) -> (usize, usize) {
        (self.feature_cache.len(), self.feature_cache.capacity())
    }
    
    /// Get cache hit ratio
    pub fn cache_hit_ratio(&self) -> f32 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            0.0
        } else {
            self.cache_hits as f32 / total as f32
        }
    }
    
    /// Get the font size
    pub fn font_size(&self) -> f32 {
        self.font_size
    }
    
    /// Get glyph metrics for a specific character
    pub fn get_glyph_metrics(&self, ch: char) -> Option<GlyphInfo> {
        let glyph_id = self.font.glyph_id(ch);
        let scaled_font = self.font.as_scaled(self.font_size);
        let advance_width = scaled_font.h_advance(glyph_id);
        
        Some(GlyphInfo {
            glyph_id: glyph_id.0 as u32,
            x_advance: advance_width,
            y_advance: 0.0,
            x_offset: 0.0,
            y_offset: 0.0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::FontSystem;
    
    fn create_test_shaper() -> GlyphShaper {
        let mut font_system = FontSystem::new().unwrap();
        let font = font_system.load_font("monospace", 14.0).unwrap();
        GlyphShaper::new(font, 14.0).unwrap()
    }
    
    #[test]
    fn test_shaper_creation() {
        let shaper = create_test_shaper();
        assert_eq!(shaper.font_size, 14.0);
    }
    
    #[test]
    fn test_ascii_shaping() {
        let mut shaper = create_test_shaper();
        let glyphs = shaper.shape("Hello World");
        
        // Should have 11 glyphs (including space)
        assert_eq!(glyphs.len(), 11);
        
        // All glyphs should have valid glyph IDs
        for glyph in &glyphs {
            assert!(glyph.glyph_id > 0);
        }
        
        // Should have reasonable advance values
        for glyph in &glyphs {
            assert!(glyph.x_advance > 0.0);
        }
    }
    
    #[test] 
    fn test_empty_string_shaping() {
        let mut shaper = create_test_shaper();
        let glyphs = shaper.shape("");
        assert_eq!(glyphs.len(), 0);
    }
    
    #[test]
    fn test_single_char_shaping() {
        let mut shaper = create_test_shaper();
        let glyphs = shaper.shape("A");
        assert_eq!(glyphs.len(), 1);
        assert!(glyphs[0].glyph_id > 0);
        assert!(glyphs[0].x_advance > 0.0);
    }
    
    #[test]
    fn test_cache_behavior() {
        let mut shaper = create_test_shaper();
        
        let initial_stats = shaper.cache_stats();
        assert_eq!(initial_stats.0, 0); // Empty cache
        
        // Shape some text
        let _glyphs1 = shaper.shape("Hello");
        let stats_after_first = shaper.cache_stats();
        assert_eq!(stats_after_first.0, 1); // One entry cached
        
        // Shape same text - should use cache
        let _glyphs2 = shaper.shape("Hello");
        let stats_after_second = shaper.cache_stats();
        assert_eq!(stats_after_second.0, 1); // Still one entry
        
        // Check cache hit ratio
        let hit_ratio = shaper.cache_hit_ratio();
        assert!(hit_ratio > 0.0);
        
        // Shape different text
        let _glyphs3 = shaper.shape("World");
        let stats_after_third = shaper.cache_stats();
        assert_eq!(stats_after_third.0, 2); // Two entries cached
        
        // Clear cache
        shaper.clear_cache();
        let final_stats = shaper.cache_stats();
        assert_eq!(final_stats.0, 0); // Cache cleared
    }
    
    #[test]
    fn test_programming_ligatures() {
        let mut shaper = create_test_shaper();
        
        // Test common programming ligatures
        let ligature_features = &["liga", "calt"];
        
        // Test arrow ligature - should be replaced with Unicode arrow
        let glyphs = shaper.shape_with_features("->", ligature_features);
        assert_eq!(glyphs.len(), 1); // Should be converted to single arrow character
        
        // Test equals arrow
        let glyphs = shaper.shape_with_features("=>", ligature_features);
        assert_eq!(glyphs.len(), 1); // Should be converted to single arrow character
        
        // Test without ligatures
        let glyphs = shaper.shape_with_features("->", &[]);
        assert_eq!(glyphs.len(), 2); // Should remain as two characters
    }
    
    #[test]
    fn test_unicode_characters() {
        let mut shaper = create_test_shaper();
        
        // Test some Unicode characters
        let glyphs = shaper.shape("café");
        assert_eq!(glyphs.len(), 4);
        
        // All glyphs should have valid IDs
        for glyph in &glyphs {
            assert!(glyph.glyph_id > 0);
        }
    }
    
    #[test]
    fn test_performance_requirements() {
        let mut shaper = create_test_shaper();
        
        // Test typical terminal line (80 characters)
        let test_line = "The quick brown fox jumps over the lazy dog. 1234567890!@#$%^&*()_+-=[]{}|;'";
        assert_eq!(test_line.len(), 76); // Close to 80 chars
        
        let start = std::time::Instant::now();
        let _glyphs = shaper.shape(test_line);
        let duration = start.elapsed();
        
        // Should complete in under 1ms as per requirements
        assert!(duration.as_millis() < 1);
        
        // Test cache hit ratio after multiple shapes of the same text
        let _glyphs2 = shaper.shape(test_line); // Should hit cache
        let _glyphs3 = shaper.shape(test_line); // Should hit cache again
        let hit_ratio = shaper.cache_hit_ratio();
        
        // Should have good hit ratio with repeated identical text
        assert!(hit_ratio >= 0.66); // 2 hits out of 3 total (66%)
    }
    
    #[test]
    fn test_cache_hit_ratio_requirement() {
        let mut shaper = create_test_shaper();
        
        // Shape various text multiple times to build up cache hits
        let test_texts = ["Hello", "World", "Test"];
        
        // First round - all cache misses
        for text in &test_texts {
            let _ = shaper.shape(text);
        }
        
        // Second round - should be cache hits
        for text in &test_texts {
            let _ = shaper.shape(text);
        }
        
        // Third round - more cache hits  
        for text in &test_texts {
            let _ = shaper.shape(text);
        }
        
        let hit_ratio = shaper.cache_hit_ratio();
        
        // Should achieve good hit ratio: 6 hits out of 9 total (66%)
        assert!(hit_ratio >= 0.65);
    }
}