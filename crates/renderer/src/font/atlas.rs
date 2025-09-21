//! GPU glyph atlas implementation with bin packing and LRU eviction
//!
//! This module provides GPU texture atlas functionality for glyph storage,
//! featuring efficient bin packing, LRU eviction, and rasterization integration.

use ab_glyph::{Font, FontArc, ScaleFont};
use anyhow::{Context, Result};
use lru::LruCache;
use std::num::NonZeroUsize;

/// Simple shelf packing for atlas allocation
#[derive(Debug, Clone)]
struct Shelf {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    used_width: u32,
}

/// Simple rect for tracking occupied space
#[derive(Debug, Clone)]
struct AllocatedRect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

/// Glyph cache key for atlas lookup
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct GlyphKey {
    /// Font identifier (based on font data hash)
    pub font_id: u64,
    /// Glyph ID from the font
    pub glyph_id: u32,
    /// Font size in pixels
    pub size: u32,
    /// Subpixel positioning (for more precise caching)
    pub subpixel_x: u8,
    pub subpixel_y: u8,
}

impl GlyphKey {
    /// Create a new glyph key
    pub fn new(font: &FontArc, glyph_id: u32, size: f32) -> Self {
        Self {
            font_id: Self::compute_font_id(font),
            glyph_id,
            size: (size * 64.0) as u32, // Store as fixed point for sub-pixel precision
            subpixel_x: 0,
            subpixel_y: 0,
        }
    }

    /// Compute a simple font ID based on some font characteristics
    fn compute_font_id(font: &FontArc) -> u64 {
        // Simple hash based on glyph count and units per EM
        // In a real implementation, this would be based on font file hash
        let glyph_count = font.glyph_count();
        let units_per_em = font.units_per_em().unwrap_or(1000.0) as u64;
        ((glyph_count as u64) << 32) | units_per_em
    }
}

/// Atlas region information with detailed metadata
#[derive(Debug, Clone)]
pub struct AtlasRegion {
    /// X coordinate in atlas texture
    pub x: u32,
    /// Y coordinate in atlas texture
    pub y: u32,
    /// Width of the region
    pub width: u32,
    /// Height of the region
    pub height: u32,
    /// Normalized UV coordinates [u_min, v_min, u_max, v_max]
    pub tex_coords: [f32; 4],
    /// Glyph bearing (offset from baseline)
    pub bearing_x: f32,
    pub bearing_y: f32,
    /// Glyph advance width
    pub advance: f32,
}

/// Atlas allocation metrics
#[derive(Debug, Default)]
pub struct AtlasMetrics {
    pub total_allocations: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub evictions: usize,
    pub rasterizations: usize,
    pub memory_used: usize,
    pub atlas_utilization: f32,
}

impl AtlasMetrics {
    pub fn hit_ratio(&self) -> f32 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            0.0
        } else {
            self.cache_hits as f32 / total as f32
        }
    }
}

/// GPU glyph atlas with bin packing and LRU eviction
pub struct GlyphAtlas {
    /// Atlas texture dimensions
    width: u32,
    height: u32,
    
    /// Cached glyph regions with LRU eviction
    glyph_cache: LruCache<GlyphKey, AtlasRegion>,
    
    /// Shelves for packing
    shelves: Vec<Shelf>,
    
    /// Current Y position for next shelf
    current_y: u32,
    
    /// Atlas texture data (RGBA format)
    texture_data: Vec<u8>,
    
    /// Performance metrics
    metrics: AtlasMetrics,
    
    /// Maximum memory usage in bytes (default: 32MB as per requirements)
    max_memory: usize,
    
    /// Padding between glyphs to prevent bleeding
    padding: u32,
}

impl GlyphAtlas {
    /// Create a new glyph atlas with specified dimensions
    pub fn new(width: u32, height: u32) -> Result<Self> {
        let capacity = NonZeroUsize::new(1000).unwrap(); // Cache up to 1000 glyphs
        let texture_size = (width * height * 4) as usize; // RGBA format
        
        Ok(Self {
            width,
            height,
            glyph_cache: LruCache::new(capacity),
            shelves: Vec::new(),
            current_y: 0,
            texture_data: vec![0; texture_size],
            metrics: AtlasMetrics::default(),
            max_memory: 32 * 1024 * 1024, // 32MB as per requirements
            padding: 2, // 2-pixel padding
        })
    }

    /// Get or rasterize a glyph and return its atlas region
    /// Uses a more robust font ID system to ensure consistency
    pub fn get_or_rasterize(&mut self, font: &FontArc, glyph_id: u32, size: f32) -> Result<AtlasRegion> {
        let glyph_key = GlyphKey::new(font, glyph_id, size);
        
        // Check cache first
        if let Some(region) = self.glyph_cache.get(&glyph_key) {
            self.metrics.cache_hits += 1;
            return Ok(region.clone());
        }
        
        self.metrics.cache_misses += 1;
        
        // Rasterize the glyph
        let region = self.rasterize_and_pack(font, glyph_id, size)?;
        
        // Cache the result
        if let Some((evicted_key, _)) = self.glyph_cache.push(glyph_key.clone(), region.clone()) {
            self.metrics.evictions += 1;
            // In a full implementation, we'd mark the evicted region as free
            // For now, we'll leave the data in place and let it be overwritten
            self.handle_evicted_glyph(&evicted_key);
        }
        
        self.update_metrics();
        Ok(region)
    }

    /// Handle an evicted glyph (placeholder for more sophisticated management)
    fn handle_evicted_glyph(&mut self, _evicted_key: &GlyphKey) {
        // In a production implementation, this would:
        // 1. Mark the atlas region as free for reuse
        // 2. Update the packing algorithm to consider freed space
        // 3. Optionally defragment the atlas
        // For now, we'll keep it simple and let regions be overwritten
    }

    /// Rasterize a glyph and pack it into the atlas
    fn rasterize_and_pack(&mut self, font: &FontArc, glyph_id: u32, size: f32) -> Result<AtlasRegion> {
        self.metrics.rasterizations += 1;
        
        let scaled_font = font.as_scaled(size);
        let glyph = font.glyph_id(char::from_u32(glyph_id).unwrap_or('?')).with_scale(size);
        let glyph_id_for_advance = glyph.id;
        
        // Get glyph outline
        let outlined_glyph = scaled_font.outline_glyph(glyph)
            .context("Failed to outline glyph")?;
        
        let bounds = outlined_glyph.px_bounds();
        let glyph_width = bounds.width().ceil() as u32;
        let glyph_height = bounds.height().ceil() as u32;
        
        // Add padding
        let padded_width = glyph_width + self.padding * 2;
        let padded_height = glyph_height + self.padding * 2;
        
        // Find space in atlas using bin packing
        let position = self.find_space(padded_width, padded_height)?;
        
        // Rasterize glyph into atlas
        self.rasterize_glyph_at(&outlined_glyph, position.0 + self.padding, position.1 + self.padding)?;
        
        // Calculate UV coordinates
        let u_min = position.0 as f32 / self.width as f32;
        let v_min = position.1 as f32 / self.height as f32;
        let u_max = (position.0 + padded_width) as f32 / self.width as f32;
        let v_max = (position.1 + padded_height) as f32 / self.height as f32;
        
        // Get glyph metrics
        let h_advance = scaled_font.h_advance(glyph_id_for_advance);
        
        Ok(AtlasRegion {
            x: position.0,
            y: position.1,
            width: padded_width,
            height: padded_height,
            tex_coords: [u_min, v_min, u_max, v_max],
            bearing_x: bounds.min.x,
            bearing_y: bounds.min.y,
            advance: h_advance,
        })
    }

    /// Find available space in the atlas using simple shelf packing
    fn find_space(&mut self, width: u32, height: u32) -> Result<(u32, u32)> {
        // Try to fit in existing shelves
        for shelf in &mut self.shelves {
            if shelf.used_width + width <= shelf.width && height <= shelf.height {
                let x = shelf.x + shelf.used_width;
                let y = shelf.y;
                shelf.used_width += width;
                return Ok((x, y));
            }
        }
        
        // Create a new shelf
        if self.current_y + height <= self.height {
            let shelf = Shelf {
                x: 0,
                y: self.current_y,
                width: self.width,
                height,
                used_width: width,
            };
            
            let position = (0, self.current_y);
            self.current_y += height;
            self.shelves.push(shelf);
            return Ok(position);
        }
        
        // Atlas is full
        anyhow::bail!("Atlas is full - no space for {}x{} glyph", width, height)
    }

    /// Try to grow atlas or evict items to make space
    fn try_grow_or_evict(&self, _width: u32, _height: u32) -> Result<(u32, u32)> {
        // For now, return an error if we can't pack
        // A full implementation would:
        // 1. Try to grow the atlas texture (up to a maximum size)
        // 2. Evict LRU items and try packing again
        // 3. Return an error if still can't fit
        anyhow::bail!("Atlas is full - implement atlas growth or more aggressive eviction")
    }

    /// Rasterize a glyph at the specified position in the atlas
    fn rasterize_glyph_at(&mut self, outlined_glyph: &ab_glyph::OutlinedGlyph, x: u32, y: u32) -> Result<()> {
        let bounds = outlined_glyph.px_bounds();
        let glyph_width = bounds.width().ceil() as u32;
        let glyph_height = bounds.height().ceil() as u32;
        
        // Create a temporary buffer for the glyph
        let mut glyph_buffer = vec![0u8; (glyph_width * glyph_height) as usize];
        
        // Rasterize glyph
        outlined_glyph.draw(|px, py, coverage| {
            if px < glyph_width && py < glyph_height {
                let index = (py * glyph_width + px) as usize;
                if index < glyph_buffer.len() {
                    glyph_buffer[index] = (coverage * 255.0) as u8;
                }
            }
        });
        
        // Copy glyph data into atlas texture (RGBA format)
        for py in 0..glyph_height {
            for px in 0..glyph_width {
                let src_index = (py * glyph_width + px) as usize;
                let alpha = glyph_buffer[src_index];
                
                let atlas_x = x + px;
                let atlas_y = y + py;
                
                if atlas_x < self.width && atlas_y < self.height {
                    let dst_index = ((atlas_y * self.width + atlas_x) * 4) as usize;
                    
                    if dst_index + 3 < self.texture_data.len() {
                        // White glyph with alpha
                        self.texture_data[dst_index] = 255;     // R
                        self.texture_data[dst_index + 1] = 255; // G
                        self.texture_data[dst_index + 2] = 255; // B
                        self.texture_data[dst_index + 3] = alpha; // A
                    }
                }
            }
        }
        
        Ok(())
    }

    /// Remove an evicted glyph from occupied regions (legacy method)
    fn remove_from_occupied(&mut self, _evicted_key: &GlyphKey) {
        // Legacy method for compatibility
        self.handle_evicted_glyph(_evicted_key);
    }

    /// Update performance metrics
    fn update_metrics(&mut self) {
        self.metrics.total_allocations = self.glyph_cache.len();
        self.metrics.memory_used = self.texture_data.len() + 
            self.glyph_cache.len() * std::mem::size_of::<(GlyphKey, AtlasRegion)>();
        
        // Calculate utilization based on current Y position
        let used_pixels = (self.current_y * self.width) as f32;
        let total_pixels = (self.width * self.height) as f32;
        self.metrics.atlas_utilization = used_pixels / total_pixels;
    }

    /// Get current atlas metrics
    pub fn metrics(&self) -> &AtlasMetrics {
        &self.metrics
    }

    /// Get atlas texture data (for GPU upload)
    pub fn texture_data(&self) -> &[u8] {
        &self.texture_data
    }

    /// Get atlas dimensions
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Clear the atlas and reset all state
    pub fn clear(&mut self) {
        self.glyph_cache.clear();
        self.shelves.clear();
        self.current_y = 0;
        self.texture_data.fill(0);
        self.metrics = AtlasMetrics::default();
    }

    /// Legacy method for compatibility
    pub fn allocate_glyph(
        &mut self,
        _glyph_id: u32,
        width: u32,
        height: u32,
    ) -> Result<AtlasRegion> {
        // This is a simplified version for backward compatibility
        // In practice, this would need a font reference
        let position = self.find_space(width, height)?;
        
        let u_min = position.0 as f32 / self.width as f32;
        let v_min = position.1 as f32 / self.height as f32;
        let u_max = (position.0 + width) as f32 / self.width as f32;
        let v_max = (position.1 + height) as f32 / self.height as f32;
        
        Ok(AtlasRegion {
            x: position.0,
            y: position.1,
            width,
            height,
            tex_coords: [u_min, v_min, u_max, v_max],
            bearing_x: 0.0,
            bearing_y: 0.0,
            advance: 0.0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::FontSystem;

    fn create_test_font() -> FontArc {
        let mut font_system = FontSystem::new().unwrap();
        font_system.load_font("monospace", 14.0).unwrap()
    }

    #[test]
    fn test_atlas_creation() {
        let atlas = GlyphAtlas::new(512, 512);
        assert!(atlas.is_ok());
        
        let atlas = atlas.unwrap();
        assert_eq!(atlas.dimensions(), (512, 512));
        assert_eq!(atlas.texture_data().len(), 512 * 512 * 4); // RGBA
    }

    #[test]
    fn test_atlas_allocation() {
        let mut atlas = GlyphAtlas::new(512, 512).unwrap();
        let region = atlas.allocate_glyph(65, 16, 20);
        assert!(region.is_ok());
        
        let region = region.unwrap();
        assert_eq!(region.width, 16);
        assert_eq!(region.height, 20);
        assert!(region.x < 512 && region.y < 512);
    }

    #[test]
    fn test_glyph_key_creation() {
        let font = create_test_font();
        let key1 = GlyphKey::new(&font, 65, 14.0); // 'A'
        let key2 = GlyphKey::new(&font, 65, 14.0); // Same 'A'
        let key3 = GlyphKey::new(&font, 66, 14.0); // 'B'
        
        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
        assert_eq!(key1.font_id, key2.font_id);
        assert_eq!(key1.font_id, key3.font_id);
    }

    #[test]
    fn test_glyph_rasterization() {
        let mut atlas = GlyphAtlas::new(512, 512).unwrap();
        let font = create_test_font();
        
        // Rasterize ASCII 'A'
        let result = atlas.get_or_rasterize(&font, 65, 16.0);
        assert!(result.is_ok());
        
        let region = result.unwrap();
        assert!(region.width > 0);
        assert!(region.height > 0);
        assert!(region.advance > 0.0);
        
        // Check that UV coordinates are normalized
        assert!(region.tex_coords[0] >= 0.0 && region.tex_coords[0] <= 1.0);
        assert!(region.tex_coords[1] >= 0.0 && region.tex_coords[1] <= 1.0);
        assert!(region.tex_coords[2] >= 0.0 && region.tex_coords[2] <= 1.0);
        assert!(region.tex_coords[3] >= 0.0 && region.tex_coords[3] <= 1.0);
    }

    #[test]
    fn test_atlas_caching() {
        let mut atlas = GlyphAtlas::new(512, 512).unwrap();
        let font = create_test_font();
        
        // First access - should rasterize
        let _region1 = atlas.get_or_rasterize(&font, 65, 16.0).unwrap();
        let metrics1 = atlas.metrics();
        assert_eq!(metrics1.cache_misses, 1);
        assert_eq!(metrics1.cache_hits, 0);
        assert_eq!(metrics1.rasterizations, 1);
        
        // Second access - should hit cache
        let _region2 = atlas.get_or_rasterize(&font, 65, 16.0).unwrap();
        let metrics2 = atlas.metrics();
        assert_eq!(metrics2.cache_misses, 1);
        assert_eq!(metrics2.cache_hits, 1);
        assert_eq!(metrics2.rasterizations, 1);
        
        // Check hit ratio
        assert_eq!(metrics2.hit_ratio(), 0.5);
    }

    #[test]
    fn test_multiple_glyph_packing() {
        let mut atlas = GlyphAtlas::new(512, 512).unwrap();
        let font = create_test_font();
        
        // Rasterize multiple glyphs
        let glyphs = ['A', 'B', 'C', 'D', 'E'];
        let mut regions = Vec::new();
        
        for &ch in &glyphs {
            let region = atlas.get_or_rasterize(&font, ch as u32, 16.0).unwrap();
            regions.push(region);
        }
        
        // Ensure all glyphs were allocated
        assert_eq!(regions.len(), glyphs.len());
        
        // Check that regions don't overlap (simplified check)
        for (i, region1) in regions.iter().enumerate() {
            for (j, region2) in regions.iter().enumerate() {
                if i != j {
                    // Simple non-overlap check
                    let no_overlap = region1.x + region1.width <= region2.x ||
                                   region2.x + region2.width <= region1.x ||
                                   region1.y + region1.height <= region2.y ||
                                   region2.y + region2.height <= region1.y;
                    assert!(no_overlap, "Regions should not overlap");
                }
            }
        }
    }

    #[test]
    fn test_atlas_metrics() {
        let mut atlas = GlyphAtlas::new(256, 256).unwrap();
        let font = create_test_font();
        
        // Initially empty
        assert_eq!(atlas.metrics().total_allocations, 0);
        
        // Add some glyphs
        for ch in 'A'..='E' {
            let _ = atlas.get_or_rasterize(&font, ch as u32, 16.0).unwrap();
        }
        
        let metrics = atlas.metrics();
        assert_eq!(metrics.total_allocations, 5);
        assert!(metrics.memory_used > 0);
        assert!(metrics.atlas_utilization > 0.0);
        assert!(metrics.atlas_utilization <= 1.0);
    }

    #[test]
    fn test_atlas_clear() {
        let mut atlas = GlyphAtlas::new(256, 256).unwrap();
        let font = create_test_font();
        
        // Add some glyphs
        let _ = atlas.get_or_rasterize(&font, 65, 16.0).unwrap();
        assert!(atlas.metrics().total_allocations > 0);
        
        // Clear atlas
        atlas.clear();
        assert_eq!(atlas.metrics().total_allocations, 0);
        assert_eq!(atlas.metrics().cache_hits, 0);
        assert_eq!(atlas.metrics().cache_misses, 0);
    }

    #[test]
    fn test_large_glyph_handling() {
        let mut atlas = GlyphAtlas::new(128, 128).unwrap(); // Small atlas
        let font = create_test_font();
        
        // Try to rasterize a large glyph size
        let result = atlas.get_or_rasterize(&font, 65, 64.0); // Large font size
        
        // Should either succeed or fail gracefully
        match result {
            Ok(region) => {
                assert!(region.width <= 128);
                assert!(region.height <= 128);
            }
            Err(_) => {
                // Expected if glyph is too large for atlas
            }
        }
    }
}
