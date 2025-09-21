//! Enhanced glyph shaping implementation with Unicode support
//!
//! This module provides text shaping functionality with improved Unicode handling,
//! cluster mapping, and the foundation for future HarfBuzz integration.

use ab_glyph::{Font, FontArc, ScaleFont};
use anyhow::Result;
use std::collections::HashMap;
use unicode_normalization::UnicodeNormalization;
use unicode_script::{Script, UnicodeScript};

/// Glyph shaping information with cluster mapping
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
    /// Cluster index for mapping back to original text
    pub cluster: u32,
}

/// Text direction for shaping
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    /// Left-to-right text direction
    LeftToRight,
    /// Right-to-left text direction
    RightToLeft,
    /// Top-to-bottom text direction
    TopToBottom,
    /// Bottom-to-top text direction
    BottomToTop,
}

impl Direction {
    fn to_string(&self) -> String {
        match self {
            Direction::LeftToRight => "ltr".to_string(),
            Direction::RightToLeft => "rtl".to_string(),
            Direction::TopToBottom => "ttb".to_string(),
            Direction::BottomToTop => "btt".to_string(),
        }
    }
}

/// Text segment for script-based processing
#[derive(Debug, Clone)]
struct TextSegment {
    text: String,
    script: Script,
    start_index: usize,
}

/// Cache key for shaping results
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct ShapingCacheKey {
    text: String,
    features: Vec<String>,
    script: String, // Convert Script to string for hashing
    direction: String, // Convert Direction to string for hashing
}

/// Enhanced glyph shaper with Unicode support
pub struct GlyphShaper {
    /// The font being used for shaping
    font: FontArc,
    /// Font size in pixels
    font_size: f32,
    /// Cache for shaping results to improve performance
    shaping_cache: HashMap<ShapingCacheKey, Vec<GlyphInfo>>,
    /// Cache hit counter for performance tracking
    cache_hits: usize,
    /// Cache miss counter for performance tracking
    cache_misses: usize,
    /// Fallback font system for missing glyphs
    font_system: Option<Box<dyn Fn(&str) -> Option<FontArc> + Send + Sync>>,
}

impl GlyphShaper {
    /// Create a new glyph shaper with the given font
    pub fn new(font: FontArc, font_size: f32) -> Result<Self> {
        Ok(Self {
            font,
            font_size,
            shaping_cache: HashMap::new(),
            cache_hits: 0,
            cache_misses: 0,
            font_system: None,
        })
    }

    /// Set fallback font system for missing glyphs
    pub fn set_fallback_system<F>(&mut self, fallback_fn: F)
    where
        F: Fn(&str) -> Option<FontArc> + Send + Sync + 'static,
    {
        self.font_system = Some(Box::new(fallback_fn));
    }

    /// Shape text into a sequence of positioned glyphs with Unicode support
    pub fn shape(&mut self, text: &str) -> Vec<GlyphInfo> {
        self.shape_with_features(text, &[])
    }

    /// Shape text with specific font features and proper Unicode handling
    pub fn shape_with_features(&mut self, text: &str, features: &[&str]) -> Vec<GlyphInfo> {
        // Normalize Unicode text
        let normalized_text: String = text.nfc().collect();
        
        // Detect script and direction
        let script = self.detect_script(&normalized_text);
        let direction = self.detect_direction(&normalized_text);
        
        // Create cache key
        let cache_key = ShapingCacheKey {
            text: normalized_text.clone(),
            features: features.iter().map(|f| f.to_string()).collect(),
            script: format!("{:?}", script), // Convert Script to string
            direction: direction.to_string(),
        };

        // Check cache first
        if let Some(cached_result) = self.shaping_cache.get(&cache_key) {
            self.cache_hits += 1;
            return cached_result.clone();
        }

        self.cache_misses += 1;

        // Perform enhanced shaping
        let glyphs = self.shape_enhanced(&normalized_text, features, script, direction);

        // Cache the result
        self.shaping_cache.insert(cache_key, glyphs.clone());

        glyphs
    }

    /// Enhanced shaping with proper cluster handling and Unicode support
    fn shape_enhanced(
        &mut self,
        text: &str,
        features: &[&str],
        script: Script,
        direction: Direction,
    ) -> Vec<GlyphInfo> {
        let scaled_font = self.font.as_scaled(self.font_size);
        let mut glyphs = Vec::new();
        
        // Handle text segmentation for complex scripts
        let segments = self.segment_text(text, script);
        
        for segment in segments {
            let segment_glyphs = self.shape_segment(&scaled_font, &segment, features, direction);
            glyphs.extend(segment_glyphs);
        }

        // Handle fallback for missing glyphs if needed
        self.handle_fallback_glyphs(text, &mut glyphs);

        glyphs
    }

    /// Segment text into runs of the same script/direction
    fn segment_text(&self, text: &str, _script: Script) -> Vec<TextSegment> {
        // For now, treat the entire text as one segment
        // A full implementation would segment by script changes
        vec![TextSegment {
            text: text.to_string(),
            script: _script,
            start_index: 0,
        }]
    }

    /// Shape a single text segment
    fn shape_segment(
        &self,
        scaled_font: &ab_glyph::PxScaleFont<&FontArc>,
        segment: &TextSegment,
        features: &[&str],
        direction: Direction,
    ) -> Vec<GlyphInfo> {
        let mut glyphs = Vec::new();
        let mut cluster_index = segment.start_index as u32;
        
        // Handle programming ligatures if requested
        let processed_text = if features.contains(&"liga") || features.contains(&"calt") {
            self.process_ligatures(&segment.text, features)
        } else {
            segment.text.clone()
        };
        
        // Shape each character/grapheme cluster
        let chars: Vec<char> = processed_text.chars().collect();
        
        for (i, &ch) in chars.iter().enumerate() {
            // Handle combining characters
            if self.is_combining_mark(ch) && i > 0 {
                // Combining mark - attach to previous glyph
                if let Some(_last_glyph) = glyphs.last_mut() {
                    // For combining marks, we might need to adjust positioning
                    // For now, we'll just mark it with the same cluster
                    continue;
                }
            }
            
            let glyph_id = self.font.glyph_id(ch);
            let advance_width = scaled_font.h_advance(glyph_id);
            
            // Calculate positioning based on direction
            let (x_advance, y_advance) = match direction {
                Direction::LeftToRight => (advance_width, 0.0),
                Direction::RightToLeft => (-advance_width, 0.0),
                Direction::TopToBottom => (0.0, self.font_size),
                Direction::BottomToTop => (0.0, -self.font_size),
            };

            glyphs.push(GlyphInfo {
                glyph_id: glyph_id.0 as u32,
                x_advance,
                y_advance,
                x_offset: 0.0,
                y_offset: 0.0,
                cluster: cluster_index,
            });
            
            // Advance cluster for most characters, but not for combining marks
            if !self.is_combining_mark(ch) {
                cluster_index += ch.len_utf8() as u32;
            }
        }
        
        // Handle RTL reversal if needed
        if direction == Direction::RightToLeft {
            glyphs.reverse();
        }
        
        glyphs
    }

    /// Check if a character is a combining mark
    fn is_combining_mark(&self, ch: char) -> bool {
        matches!(
            ch.script(),
            Script::Inherited | Script::Common
        ) && !ch.is_control() && (ch as u32) >= 0x0300 && (ch as u32) <= 0x036F
    }

    /// Process programming ligatures
    fn process_ligatures(&self, text: &str, features: &[&str]) -> String {
        if !features.contains(&"liga") && !features.contains(&"calt") {
            return text.to_string();
        }

        let mut result = text.to_string();

        // Common programming ligatures - replace with Unicode equivalents where appropriate
        let ligatures = &[
            ("->", "→"),
            ("=>", "⇒"),
            ("<=", "≤"),
            (">=", "≥"),
            ("!=", "≠"),
            ("==", "≡"),
            ("===", "≡"),
            ("!==", "≢"),
            ("&&", "∧"),
            ("||", "∨"),
            ("..", "‥"),
            ("...", "…"),
        ];

        for (from, to) in ligatures {
            result = result.replace(from, to);
        }

        result
    }

    /// Clear the shaping cache
    pub fn clear_cache(&mut self) {
        self.shaping_cache.clear();
        self.cache_hits = 0;
        self.cache_misses = 0;
    }

    /// Get cache statistics (entry count, capacity)
    pub fn cache_stats(&self) -> (usize, usize) {
        (self.shaping_cache.len(), self.shaping_cache.capacity())
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

    /// Handle fallback fonts for missing glyphs
    fn handle_fallback_glyphs(&mut self, _text: &str, _glyphs: &mut [GlyphInfo]) {
        // TODO: Implement fallback font system
        // For now, leave missing glyphs as-is (glyph_id 0)
        // This would iterate through fallback fonts and re-shape problematic runs
    }

    /// Get glyph metrics for a specific character using enhanced processing
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
            cluster: 0,
        })
    }

    /// Auto-detect script for text using unicode-script
    fn detect_script(&self, text: &str) -> Script {
        // Find the first non-common, non-inherited script
        for ch in text.chars() {
            let script = ch.script();
            match script {
                Script::Common | Script::Inherited => continue,
                _ => return script,
            }
        }
        Script::Latin // Default fallback
    }

    /// Auto-detect text direction based on script
    fn detect_direction(&self, text: &str) -> Direction {
        // Simple RTL detection based on script
        for ch in text.chars() {
            let script = ch.script();
            match script {
                Script::Arabic | Script::Hebrew => {
                    return Direction::RightToLeft;
                }
                _ => continue,
            }
        }
        Direction::LeftToRight
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

        // Test common programming ligatures with HarfBuzz
        let ligature_features = &["liga", "calt"];

        // Test arrow ligature - HarfBuzz should handle this if font supports it
        let glyphs = shaper.shape_with_features("->", ligature_features);
        assert!(!glyphs.is_empty());

        // Test equals arrow
        let glyphs = shaper.shape_with_features("=>", ligature_features);
        assert!(!glyphs.is_empty());

        // Test without ligatures
        let glyphs = shaper.shape_with_features("->", &[]);
        assert!(!glyphs.is_empty());
    }

    #[test]
    fn test_unicode_characters() {
        let mut shaper = create_test_shaper();

        // Test some Unicode characters
        let glyphs = shaper.shape("café");
        assert_eq!(glyphs.len(), 4);

        // All glyphs should have valid cluster mappings
        for glyph in &glyphs {
            assert!(glyph.cluster <= 4);
        }
    }

    #[test]
    fn test_combining_characters() {
        let mut shaper = create_test_shaper();

        // Test combining diacritics - should not increase glyph count
        let text = "e\u{0301}"; // e + combining acute accent
        let glyphs = shaper.shape(text);
        
        // Should be 1 glyph for the combined character
        assert_eq!(glyphs.len(), 1);
        assert!(glyphs[0].glyph_id > 0);
    }

    #[test]
    fn test_rtl_text() {
        let mut shaper = create_test_shaper();

        // Test simple Arabic text (if font supports it)
        let arabic_text = "مرحبا"; // "Hello" in Arabic
        let glyphs = shaper.shape(arabic_text);
        
        assert!(!glyphs.is_empty());
        // Glyphs should have cluster mappings
        for glyph in &glyphs {
            assert!(glyph.cluster < arabic_text.len() as u32);
        }
    }

    #[test]
    fn test_cjk_characters() {
        let mut shaper = create_test_shaper();

        // Test CJK characters - font may not support them, so we just check for graceful handling
        let cjk_text = "漢字";
        let glyphs = shaper.shape(cjk_text);
        
        assert_eq!(glyphs.len(), 2);
        for glyph in &glyphs {
            // Glyph ID 0 is acceptable for unsupported characters
            assert!(glyph.cluster < cjk_text.len() as u32 * 4); // UTF-8 max bytes per char
        }
    }

    #[test]
    fn test_emoji_sequences() {
        let mut shaper = create_test_shaper();

        // Test simple emoji
        let emoji = "😀";
        let glyphs = shaper.shape(emoji);
        
        assert!(!glyphs.is_empty());
        
        // Test ZWJ sequence (may not work with all fonts)
        let zwj_emoji = "👩‍💻"; // Woman technologist
        let glyphs = shaper.shape(zwj_emoji);
        assert!(!glyphs.is_empty());
    }

    #[test]
    fn test_performance_requirements() {
        let mut shaper = create_test_shaper();

        // Test typical terminal line (80 characters)
        let test_line =
            "The quick brown fox jumps over the lazy dog. 1234567890!@#$%^&*()_+-=[]{}|;'";
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

    #[test]
    fn test_cluster_mapping() {
        let mut shaper = create_test_shaper();

        // Test that cluster mapping works correctly
        let text = "Hello World";
        let glyphs = shaper.shape(text);
        
        // Each glyph should have a valid cluster index
        for (_i, glyph) in glyphs.iter().enumerate() {
            assert!(glyph.cluster <= text.len() as u32);
        }
    }

    #[test]
    fn test_harfbuzz_features() {
        let mut shaper = create_test_shaper();

        // Test that different feature sets create different cache entries
        let text = "Test";
        
        let glyphs1 = shaper.shape_with_features(text, &[]);
        let glyphs2 = shaper.shape_with_features(text, &["liga"]);
        let glyphs3 = shaper.shape_with_features(text, &["kern"]);
        
        // All should succeed
        assert!(!glyphs1.is_empty());
        assert!(!glyphs2.is_empty());
        assert!(!glyphs3.is_empty());
        
        // Cache should have separate entries for different feature sets
        let (cache_entries, _) = shaper.cache_stats();
        assert!(cache_entries >= 3);
    }

    #[test]
    fn test_fallback_handling() {
        let mut shaper = create_test_shaper();

        // Test text with potentially missing glyphs
        let mixed_text = "Hello 🌍 World";
        let glyphs = shaper.shape(mixed_text);
        
        assert!(!glyphs.is_empty());
        // Should handle mixed content gracefully
    }
}
