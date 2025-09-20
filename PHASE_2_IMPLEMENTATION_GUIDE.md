# Phase 2 Implementation Guide for AI Coding Agents

## Quick Start Guide

This document provides AI coding agents with specific implementation examples, code templates, and testing patterns for Phase 2 tasks.

## Code Templates and Examples

### Task 1: Font System Implementation

#### 1.1 Font Loading - Complete Implementation Example

```rust
// File: crates/renderer/src/font/mod.rs
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;

pub mod loader;
pub mod shaper;
pub mod atlas;

pub use loader::{FontLoader, SystemFontLoader};
pub use shaper::{GlyphShaper, GlyphInfo};
pub use atlas::{GlyphAtlas, AtlasRegion};

#[derive(Debug, Clone)]
pub struct FontInfo {
    pub family: String,
    pub style: FontStyle,
    pub weight: FontWeight,
    pub path: Option<std::path::PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FontStyle {
    Normal,
    Italic,
    Oblique,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FontWeight {
    Normal,
    Bold,
    Light,
    ExtraBold,
}

pub struct FontSystem {
    loader: Box<dyn FontLoader>,
    font_cache: HashMap<FontKey, ab_glyph::FontArc>,
    fallback_chain: Vec<FontInfo>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct FontKey {
    family: String,
    size: u32, // Size in points * 64 for sub-pixel precision
    style: FontStyle,
    weight: FontWeight,
}

impl FontSystem {
    pub fn new() -> Result<Self> {
        let loader = Box::new(SystemFontLoader::new()?);
        let fallback_chain = Self::build_fallback_chain();
        
        Ok(Self {
            loader,
            font_cache: HashMap::new(),
            fallback_chain,
        })
    }
    
    pub fn load_font(&mut self, family: &str, size: f32) -> Result<ab_glyph::FontArc> {
        let key = FontKey {
            family: family.to_string(),
            size: (size * 64.0) as u32,
            style: FontStyle::Normal,
            weight: FontWeight::Normal,
        };
        
        if let Some(font) = self.font_cache.get(&key) {
            return Ok(font.clone());
        }
        
        // Try primary font
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
    
    fn build_fallback_chain() -> Vec<FontInfo> {
        vec![
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
}

// Test template
#[cfg(test)]
mod tests {
    use super::*;
    
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
}
```

#### 1.2 Platform-Specific Font Loader

```rust
// File: crates/renderer/src/font/loader.rs
use anyhow::{Context, Result};
use ab_glyph::{FontArc, Font};

pub trait FontLoader: Send + Sync {
    fn load_font(&self, family: &str, size: f32) -> Result<FontArc>;
    fn system_fonts(&self) -> Result<Vec<crate::font::FontInfo>>;
}

pub struct SystemFontLoader {
    #[cfg(target_os = "linux")]
    fontconfig: fontconfig::Fontconfig,
}

impl SystemFontLoader {
    pub fn new() -> Result<Self> {
        Ok(Self {
            #[cfg(target_os = "linux")]
            fontconfig: fontconfig::Fontconfig::new()
                .context("Failed to initialize fontconfig")?,
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
    }
    
    fn system_fonts(&self) -> Result<Vec<crate::font::FontInfo>> {
        // Implementation depends on platform
        Ok(vec![])
    }
}

#[cfg(target_os = "linux")]
impl SystemFontLoader {
    fn load_font_linux(&self, family: &str, _size: f32) -> Result<FontArc> {
        use fontconfig::Pattern;
        
        let mut pattern = Pattern::new(&self.fontconfig);
        pattern.add_string("family", family);
        pattern.add_string("style", "Regular");
        pattern.set_double("size", _size as f64);
        
        let font_match = pattern.font_match();
        let path = font_match.get_string("file")
            .context("Font file path not found")?;
            
        let font_data = std::fs::read(&path)
            .with_context(|| format!("Failed to read font file: {}", path))?;
            
        FontArc::try_from_vec(font_data)
            .context("Failed to parse font data")
    }
}

// Add similar implementations for macOS and Windows
```

### Task 2: Dirty Rendering Implementation

#### 2.1 Dirty Region Tracking

```rust
// File: crates/renderer/src/dirty.rs
use bit_vec::BitVec;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct DirtyRegion {
    pub start_row: usize,
    pub end_row: usize,
    pub start_col: usize,
    pub end_col: usize,
}

impl DirtyRegion {
    pub fn new(row: usize, col: usize) -> Self {
        Self {
            start_row: row,
            end_row: row,
            start_col: col,
            end_col: col,
        }
    }
    
    pub fn expand_to_include(&mut self, row: usize, col: usize) {
        self.start_row = self.start_row.min(row);
        self.end_row = self.end_row.max(row);
        self.start_col = self.start_col.min(col);
        self.end_col = self.end_col.max(col);
    }
    
    pub fn area(&self) -> usize {
        (self.end_row - self.start_row + 1) * (self.end_col - self.start_col + 1)
    }
    
    pub fn overlaps(&self, other: &DirtyRegion) -> bool {
        !(self.end_row < other.start_row || 
          other.end_row < self.start_row ||
          self.end_col < other.start_col ||
          other.end_col < self.start_col)
    }
    
    pub fn merge(&mut self, other: &DirtyRegion) {
        self.start_row = self.start_row.min(other.start_row);
        self.end_row = self.end_row.max(other.end_row);
        self.start_col = self.start_col.min(other.start_col);
        self.end_col = self.end_col.max(other.end_col);
    }
}

pub struct DirtyTracker {
    dirty_cells: BitVec,
    grid_width: usize,
    grid_height: usize,
    regions: Vec<DirtyRegion>,
    region_merge_threshold: usize,
}

impl DirtyTracker {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            dirty_cells: BitVec::from_elem(width * height, false),
            grid_width: width,
            grid_height: height,
            regions: Vec::new(),
            region_merge_threshold: 50, // Merge regions if combined area < threshold
        }
    }
    
    pub fn mark_cell_dirty(&mut self, row: usize, col: usize) {
        if row >= self.grid_height || col >= self.grid_width {
            return;
        }
        
        let index = row * self.grid_width + col;
        self.dirty_cells.set(index, true);
    }
    
    pub fn mark_line_dirty(&mut self, row: usize) {
        for col in 0..self.grid_width {
            self.mark_cell_dirty(row, col);
        }
    }
    
    pub fn build_regions(&mut self) -> Vec<DirtyRegion> {
        self.regions.clear();
        
        for row in 0..self.grid_height {
            let mut current_region: Option<DirtyRegion> = None;
            
            for col in 0..self.grid_width {
                let index = row * self.grid_width + col;
                
                if self.dirty_cells[index] {
                    match &mut current_region {
                        Some(region) => {
                            region.end_col = col;
                        },
                        None => {
                            current_region = Some(DirtyRegion::new(row, col));
                        }
                    }
                }
            }
            
            if let Some(region) = current_region {
                self.add_region(region);
            }
        }
        
        self.merge_overlapping_regions();
        self.regions.clone()
    }
    
    fn add_region(&mut self, new_region: DirtyRegion) {
        // Try to merge with existing regions first
        for existing in &mut self.regions {
            if existing.overlaps(&new_region) && 
               existing.area() + new_region.area() < self.region_merge_threshold {
                existing.merge(&new_region);
                return;
            }
        }
        
        self.regions.push(new_region);
    }
    
    fn merge_overlapping_regions(&mut self) {
        let mut i = 0;
        while i < self.regions.len() {
            let mut j = i + 1;
            while j < self.regions.len() {
                if self.regions[i].overlaps(&self.regions[j]) {
                    let other = self.regions.remove(j);
                    self.regions[i].merge(&other);
                } else {
                    j += 1;
                }
            }
            i += 1;
        }
    }
    
    pub fn clear(&mut self) {
        self.dirty_cells.clear();
        self.regions.clear();
    }
    
    pub fn stats(&self) -> DirtyStats {
        let total_cells = self.grid_width * self.grid_height;
        let dirty_count = self.dirty_cells.iter().filter(|&b| b).count();
        
        DirtyStats {
            total_cells,
            dirty_cells: dirty_count,
            dirty_percentage: (dirty_count as f32 / total_cells as f32) * 100.0,
            region_count: self.regions.len(),
        }
    }
}

#[derive(Debug)]
pub struct DirtyStats {
    pub total_cells: usize,
    pub dirty_cells: usize,
    pub dirty_percentage: f32,
    pub region_count: usize,
}

// Test framework
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_single_cell_dirty() {
        let mut tracker = DirtyTracker::new(80, 24);
        tracker.mark_cell_dirty(5, 10);
        
        let regions = tracker.build_regions();
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].start_row, 5);
        assert_eq!(regions[0].end_row, 5);
        assert_eq!(regions[0].start_col, 10);
        assert_eq!(regions[0].end_col, 10);
    }
    
    #[test]
    fn test_region_merging() {
        let mut tracker = DirtyTracker::new(80, 24);
        
        // Mark adjacent cells dirty
        tracker.mark_cell_dirty(5, 10);
        tracker.mark_cell_dirty(5, 11);
        tracker.mark_cell_dirty(5, 12);
        
        let regions = tracker.build_regions();
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].start_col, 10);
        assert_eq!(regions[0].end_col, 12);
    }
    
    #[test]
    fn test_performance_typical_usage() {
        let mut tracker = DirtyTracker::new(120, 40);
        
        // Simulate typical terminal usage - few scattered changes
        for i in 0..10 {
            tracker.mark_cell_dirty(i * 2, i * 3);
        }
        
        let start = std::time::Instant::now();
        let regions = tracker.build_regions();
        let duration = start.elapsed();
        
        // Should complete in under 1ms for typical usage
        assert!(duration.as_micros() < 1000);
        assert!(regions.len() <= 10);
    }
}
```

### Task 3: Configuration System with Live Reload

#### 3.1 Enhanced Configuration Structure

```rust
// File: crates/config/src/font.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontConfig {
    pub family: String,
    pub size: f32,
    pub ligatures: bool,
    pub fallback_families: Vec<String>,
    pub features: Vec<FontFeature>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontFeature {
    pub tag: String,
    pub value: u32,
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            family: "monospace".to_string(),
            size: 14.0,
            ligatures: true,
            fallback_families: vec![
                "Consolas".to_string(),
                "Monaco".to_string(),
                "Courier New".to_string(),
            ],
            features: vec![],
        }
    }
}

impl FontConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.size <= 0.0 {
            return Err("Font size must be positive".to_string());
        }
        
        if self.size < 6.0 || self.size > 72.0 {
            return Err("Font size must be between 6 and 72 points".to_string());
        }
        
        if self.family.is_empty() {
            return Err("Font family cannot be empty".to_string());
        }
        
        Ok(())
    }
}

// Example TOML config structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub renderer: RendererConfig,
    pub terminal: TerminalConfig,
    pub keybindings: KeyBindings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RendererConfig {
    pub font: FontConfig,
    pub colors: ColorScheme,
    pub performance: PerformanceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorScheme {
    pub foreground: String,
    pub background: String,
    pub cursor: String,
    pub selection: String,
    pub ansi_colors: [String; 16],
}

impl Default for ColorScheme {
    fn default() -> Self {
        Self {
            foreground: "#ffffff".to_string(),
            background: "#000000".to_string(),
            cursor: "#ffffff".to_string(),
            selection: "#444444".to_string(),
            ansi_colors: [
                "#000000".to_string(), "#cd0000".to_string(),
                "#00cd00".to_string(), "#cdcd00".to_string(),
                "#0000ee".to_string(), "#cd00cd".to_string(),
                "#00cdcd".to_string(), "#e5e5e5".to_string(),
                "#7f7f7f".to_string(), "#ff0000".to_string(),
                "#00ff00".to_string(), "#ffff00".to_string(),
                "#5c5cff".to_string(), "#ff00ff".to_string(),
                "#00ffff".to_string(), "#ffffff".to_string(),
            ],
        }
    }
}
```

#### 3.2 Live Reload Implementation

```rust
// File: crates/config/src/watcher.rs
use notify::{Watcher, RecursiveMode, Result as NotifyResult};
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;
use tokio::sync::mpsc as tokio_mpsc;

#[derive(Debug, Clone)]
pub enum ConfigChange {
    FontSize(f32),
    FontFamily(String),
    ColorScheme(super::ColorScheme),
    FullReload(super::Config),
    Error(String),
}

pub struct ConfigWatcher {
    _watcher: notify::RecommendedWatcher,
    change_tx: tokio_mpsc::UnboundedSender<ConfigChange>,
}

impl ConfigWatcher {
    pub fn new(
        config_path: impl AsRef<Path>,
    ) -> Result<(Self, tokio_mpsc::UnboundedReceiver<ConfigChange>), Box<dyn std::error::Error>> {
        let (change_tx, change_rx) = tokio_mpsc::unbounded_channel();
        let (file_tx, file_rx) = mpsc::channel();
        
        let config_path = config_path.as_ref().to_path_buf();
        let change_tx_clone = change_tx.clone();
        
        // Spawn file watcher thread
        let _watcher = notify::recommended_watcher(move |res: NotifyResult<notify::Event>| {
            let _ = file_tx.send(res);
        })?;
        
        let mut watcher = _watcher;
        watcher.watch(&config_path, RecursiveMode::NonRecursive)?;
        
        // Spawn config processing task
        tokio::spawn(async move {
            Self::process_file_events(file_rx, config_path, change_tx_clone).await;
        });
        
        Ok((Self { _watcher: watcher, change_tx }, change_rx))
    }
    
    async fn process_file_events(
        file_rx: mpsc::Receiver<NotifyResult<notify::Event>>,
        config_path: std::path::PathBuf,
        change_tx: tokio_mpsc::UnboundedSender<ConfigChange>,
    ) {
        let mut last_reload = std::time::Instant::now();
        
        while let Ok(event) = file_rx.recv() {
            match event {
                Ok(event) => {
                    if event.kind.is_modify() {
                        // Debounce rapid file changes
                        let now = std::time::Instant::now();
                        if now.duration_since(last_reload) < Duration::from_millis(100) {
                            continue;
                        }
                        last_reload = now;
                        
                        // Wait a bit for file to be fully written
                        tokio::time::sleep(Duration::from_millis(50)).await;
                        
                        match Self::reload_config(&config_path) {
                            Ok(config) => {
                                let _ = change_tx.send(ConfigChange::FullReload(config));
                            }
                            Err(e) => {
                                let _ = change_tx.send(ConfigChange::Error(e.to_string()));
                            }
                        }
                    }
                }
                Err(e) => {
                    let _ = change_tx.send(ConfigChange::Error(e.to_string()));
                }
            }
        }
    }
    
    fn reload_config(config_path: &Path) -> anyhow::Result<super::Config> {
        let content = std::fs::read_to_string(config_path)?;
        let config: super::Config = toml::from_str(&content)?;
        
        // Validate the new config
        config.renderer.font.validate()
            .map_err(|e| anyhow::anyhow!("Font config validation failed: {}", e))?;
            
        Ok(config)
    }
}

// Test framework for config watching
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_config_file_watching() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");
        
        // Create initial config
        let initial_config = r#"
            [renderer.font]
            family = "monospace"
            size = 14.0
            ligatures = true
        "#;
        fs::write(&config_path, initial_config).unwrap();
        
        let (_watcher, mut change_rx) = ConfigWatcher::new(&config_path).unwrap();
        
        // Modify config file
        let updated_config = r#"
            [renderer.font]
            family = "monospace"
            size = 16.0
            ligatures = false
        "#;
        fs::write(&config_path, updated_config).unwrap();
        
        // Should receive a change notification
        let change = tokio::time::timeout(
            Duration::from_secs(2),
            change_rx.recv()
        ).await.unwrap().unwrap();
        
        match change {
            ConfigChange::FullReload(config) => {
                assert_eq!(config.renderer.font.size, 16.0);
                assert_eq!(config.renderer.font.ligatures, false);
            }
            _ => panic!("Expected FullReload change"),
        }
    }
}
```

## Testing Framework Templates

### Performance Testing Template

```rust
// File: tests/performance/phase2_benchmarks.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use quantaterm_renderer::font::FontSystem;
use quantaterm_renderer::dirty::DirtyTracker;

fn bench_font_loading(c: &mut Criterion) {
    c.bench_function("font_loading_cold", |b| {
        b.iter(|| {
            let mut font_system = FontSystem::new().unwrap();
            let font = font_system.load_font("monospace", 14.0).unwrap();
            black_box(font);
        });
    });
    
    let mut font_system = FontSystem::new().unwrap();
    c.bench_function("font_loading_cached", |b| {
        b.iter(|| {
            let font = font_system.load_font("monospace", 14.0).unwrap();
            black_box(font);
        });
    });
}

fn bench_dirty_tracking(c: &mut Criterion) {
    let mut tracker = DirtyTracker::new(120, 40);
    
    c.bench_function("dirty_single_cell", |b| {
        b.iter(|| {
            tracker.mark_cell_dirty(black_box(10), black_box(20));
            let regions = tracker.build_regions();
            black_box(regions);
            tracker.clear();
        });
    });
    
    c.bench_function("dirty_full_line", |b| {
        b.iter(|| {
            tracker.mark_line_dirty(black_box(15));
            let regions = tracker.build_regions();
            black_box(regions);
            tracker.clear();
        });
    });
}

criterion_group!(benches, bench_font_loading, bench_dirty_tracking);
criterion_main!(benches);
```

### Integration Testing Template

```rust
// File: tests/integration/font_pipeline_test.rs
use quantaterm_renderer::font::{FontSystem, GlyphShaper};
use quantaterm_renderer::atlas::GlyphAtlas;

#[tokio::test]
async fn test_complete_font_pipeline() {
    // Test the complete font loading -> shaping -> atlas pipeline
    let mut font_system = FontSystem::new().unwrap();
    let font = font_system.load_font("monospace", 14.0).unwrap();
    
    let mut shaper = GlyphShaper::new(font.clone());
    let glyphs = shaper.shape("Hello World!");
    assert!(!glyphs.is_empty());
    
    let mut atlas = GlyphAtlas::new(512, 512).unwrap();
    for glyph in &glyphs {
        let region = atlas.allocate_glyph(glyph.glyph_id, 16, 20);
        assert!(region.is_ok());
    }
    
    // Verify atlas utilization
    let stats = atlas.stats();
    assert!(stats.utilization > 0.0);
    assert!(stats.utilization < 1.0);
}
```

## Common Pitfalls and Solutions

### 1. Font Loading Issues
**Problem**: Fonts not found on different platforms
**Solution**: Always implement fallback chain and platform-specific discovery

### 2. Performance Issues
**Problem**: Dirty tracking takes too long
**Solution**: Use bit vectors and limit region merging operations

### 3. Configuration Validation
**Problem**: Invalid configs crash the application  
**Solution**: Always validate configs and provide meaningful error messages

### 4. Cross-Platform Compatibility
**Problem**: Code works on Linux but fails on Windows/macOS
**Solution**: Use conditional compilation and test on all platforms

This implementation guide provides AI coding agents with concrete starting points and patterns to follow for successful Phase 2 implementation.