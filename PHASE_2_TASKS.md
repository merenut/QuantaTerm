# Phase 2 Task Breakdown: GPU & UX Essentials (Weeks 9-14)

## Overview
This document provides a detailed breakdown of Phase 2 tasks from the PROJECT_PLAN.md, specifically designed for AI coding agents. Each task includes clear acceptance criteria, implementation guidance, and testing requirements.

**Phase 2 Goals**: Implement GPU rendering fundamentals and core UX features
**Duration**: 6 weeks (Weeks 9-14)
**Dependencies**: Phase 0-1 foundations (PTY, basic parsing, grid model)

---

## Task 1: Glyph Shaping & Font Atlas System

### **1.1 Font Loading Infrastructure**
**Priority**: Critical
**Estimated Time**: 3-4 days
**Dependencies**: None

#### Requirements
- Implement font loading using system fonts and custom font files
- Support TrueType (.ttf) and OpenType (.otf) fonts
- Handle font fallback chains for missing glyphs
- Cross-platform font discovery (Linux: fontconfig, macOS: Core Text, Windows: DirectWrite)

#### Implementation Guidelines
```rust
// File: crates/renderer/src/font/mod.rs
pub struct FontSystem {
    primary_font: Font,
    fallback_fonts: Vec<Font>,
    font_cache: HashMap<FontKey, Font>,
}

// File: crates/renderer/src/font/loader.rs
pub trait FontLoader {
    fn load_font(&self, family: &str, size: f32) -> Result<Font>;
    fn system_fonts(&self) -> Vec<FontInfo>;
}
```

#### Acceptance Criteria
- [ ] Load system monospace font on all platforms
- [ ] Load custom font from file path
- [ ] Fallback to default font when requested font missing
- [ ] Support font sizes from 8pt to 72pt
- [ ] Memory usage < 50MB for basic font set

#### Test Requirements
```rust
#[test]
fn test_font_loading() {
    let font_system = FontSystem::new();
    let font = font_system.load_font("monospace", 14.0).unwrap();
    assert!(font.glyph_count() > 0);
}

#[test] 
fn test_fallback_fonts() {
    // Test missing font falls back to default
}
```

---

### **1.2 Glyph Shaping with Harfbuzz**
**Priority**: Critical  
**Estimated Time**: 4-5 days
**Dependencies**: Task 1.1

#### Requirements
- Integrate harfbuzz-rs for text shaping
- Handle complex scripts (Arabic, Hindi, etc.)
- Support font features (ligatures, kerning)
- Cache shaping results for performance

#### Implementation Guidelines
```rust
// File: crates/renderer/src/font/shaper.rs
pub struct GlyphShaper {
    hb_font: harfbuzz::Font,
    feature_cache: HashMap<String, Vec<GlyphInfo>>,
}

pub struct GlyphInfo {
    pub glyph_id: u32,
    pub x_advance: f32,
    pub y_advance: f32,
    pub x_offset: f32,
    pub y_offset: f32,
}
```

#### Acceptance Criteria
- [x] Shape basic ASCII text correctly
- [x] Handle Unicode combining characters  
- [x] Support programming ligatures (if font supports)
- [x] Cache shaping results (hit ratio > 85%)
- [x] Shaping latency < 1ms for typical line (80 chars)

#### Test Requirements
```rust
#[test]
fn test_ascii_shaping() {
    let shaper = GlyphShaper::new(font);
    let glyphs = shaper.shape("Hello World");
    assert_eq!(glyphs.len(), 11);
}

#[test]
fn test_ligature_shaping() {
    // Test programming ligatures like "=>" "->"
}
```

---

### **1.3 GPU Glyph Atlas**
**Priority**: Critical
**Estimated Time**: 5-6 days  
**Dependencies**: Task 1.2

#### Requirements
- Create GPU texture atlas for glyph storage
- Dynamic atlas growth and management
- Efficient packing algorithm (bin packing)
- LRU eviction for atlas overflow
- Multi-channel SDF (Signed Distance Field) for crisp rendering

#### Implementation Guidelines
```rust
// File: crates/renderer/src/font/atlas.rs
pub struct GlyphAtlas {
    texture: wgpu::Texture,
    texture_view: wgpu::TextureView,
    packer: BinPacker,
    glyph_cache: HashMap<GlyphKey, AtlasRegion>,
    usage_tracker: LruCache<GlyphKey>,
}

pub struct AtlasRegion {
    pub x: u32, pub y: u32,
    pub width: u32, pub height: u32,
    pub tex_coords: [f32; 4], // normalized UV coordinates
}
```

#### Acceptance Criteria
- [x] Atlas size: 2048x2048 initial, expandable
- [x] Cache common ASCII set (95 glyphs) on startup  
- [x] Cache hit ratio ≥ 90% after warm-up (10 seconds typical use)
- [x] Atlas upload latency < 2ms per glyph
- [x] Memory usage < 32MB for typical glyph set (500 unique glyphs)

#### Test Requirements
```rust
#[test]
fn test_atlas_packing() {
    let mut atlas = GlyphAtlas::new(512, 512);
    let region = atlas.allocate_glyph(glyph_id, 16, 20).unwrap();
    assert!(region.x + region.width <= 512);
}

#[test]
fn test_cache_behavior() {
    // Test LRU eviction and hit ratios
}
```

---

## Task 2: Dirty Region Rendering System

### **2.1 Cell Change Tracking**
**Priority**: High
**Estimated Time**: 3-4 days
**Dependencies**: Task 1 (for rendering backend)

#### Requirements
- Track which terminal grid cells have changed since last frame
- Implement efficient diff algorithm for cell comparison
- Mark dirty regions (rectangular areas) for partial updates
- Handle scrolling and selection efficiently

#### Implementation Guidelines
```rust
// File: crates/renderer/src/dirty.rs
pub struct DirtyTracker {
    dirty_cells: BitVec,
    dirty_regions: Vec<DirtyRegion>,
    previous_grid: Option<TerminalGrid>,
    scroll_delta: i32,
}

pub struct DirtyRegion {
    pub start_row: usize,
    pub end_row: usize,
    pub start_col: usize, 
    pub end_col: usize,
}
```

#### Acceptance Criteria
- [ ] Detect single cell changes accurately
- [ ] Combine adjacent dirty cells into regions
- [ ] Handle scrolling without marking entire screen dirty  
- [ ] Dirty tracking overhead < 0.5ms per frame
- [ ] ≥ 30% frame reduction vs full redraw under typical usage

#### Test Requirements
```rust
#[test]
fn test_single_cell_change() {
    let mut tracker = DirtyTracker::new();
    // Change one cell, verify only that cell marked dirty
}

#[test]
fn test_region_coalescing() {
    // Test adjacent dirty cells combine into single region
}
```

---

### **2.2 Partial Render Pipeline**
**Priority**: High
**Estimated Time**: 4-5 days
**Dependencies**: Task 2.1

#### Requirements
- Modify GPU renderer to update only dirty regions
- Efficient GPU buffer updates (avoid full buffer upload)
- Maintain render quality with partial updates
- Handle overlapping content (cursor, selection)

#### Implementation Guidelines
```rust
// File: crates/renderer/src/render.rs
impl Renderer {
    pub fn render_partial(&mut self, dirty_regions: &[DirtyRegion]) -> Result<()> {
        for region in dirty_regions {
            self.update_region_buffers(region)?;
            self.render_region(region)?;
        }
    }
    
    fn update_region_buffers(&mut self, region: &DirtyRegion) -> Result<()> {
        // Upload only changed vertices/indices to GPU
    }
}
```

#### Acceptance Criteria
- [ ] Render only dirty regions, skip unchanged areas
- [ ] No visual artifacts from partial rendering
- [ ] GPU buffer updates use mapping/staging buffers efficiently
- [ ] Frame time reduction ≥ 30% for partial updates
- [ ] Full screen updates fall back gracefully

#### Test Requirements
```rust
#[test]
fn test_partial_render_quality() {
    // Compare partial render output to full render - should be identical
}

#[test]
fn test_performance_improvement() {
    // Measure frame time improvement with dirty rendering
}
```

---

## Task 3: Configuration System v1 with Live Reload

### **3.1 Enhanced Config Structure**
**Priority**: Medium
**Estimated Time**: 2-3 days
**Dependencies**: Existing config system

#### Requirements
- Extend existing TOML config with renderer-specific options
- Support font configuration (family, size, features)
- Color scheme configuration
- Key binding configuration

#### Implementation Guidelines
```rust
// File: crates/config/src/renderer.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RendererConfig {
    pub font: FontConfig,
    pub colors: ColorScheme,
    pub performance: PerformanceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontConfig {
    pub family: String,
    pub size: f32,
    pub ligatures: bool,
    pub fallback_families: Vec<String>,
}
```

#### Acceptance Criteria
- [ ] Load font configuration from TOML
- [ ] Validate configuration values (font size > 0, etc.)
- [ ] Generate JSON schema for editor support
- [ ] Backward compatibility with existing configs
- [ ] Config validation errors provide helpful messages

#### Test Requirements
```rust
#[test]
fn test_font_config_loading() {
    let config_toml = r#"
        [renderer.font]
        family = "JetBrains Mono"
        size = 14.0
        ligatures = true
    "#;
    let config: Config = toml::from_str(config_toml).unwrap();
    assert_eq!(config.renderer.font.family, "JetBrains Mono");
}
```

---

### **3.2 File Watcher and Live Reload**
**Priority**: Medium
**Estimated Time**: 3-4 days
**Dependencies**: Task 3.1

#### Requirements
- Watch config file for changes using filesystem events
- Reload configuration without restarting application
- Apply changes to running renderer (font size, colors)
- Handle reload errors gracefully

#### Implementation Guidelines
```rust
// File: crates/config/src/watcher.rs
pub struct ConfigWatcher {
    watcher: notify::RecommendedWatcher,
    config_path: PathBuf,
    reload_tx: mpsc::Sender<ConfigChange>,
}

pub enum ConfigChange {
    FontSize(f32),
    FontFamily(String),
    ColorScheme(ColorScheme),
    Full(Config),
}
```

#### Acceptance Criteria
- [ ] Detect config file changes within 100ms
- [ ] Font size changes apply immediately without restart
- [ ] Invalid config changes don't crash application
- [ ] Rollback to previous config on parse errors
- [ ] Live reload works on all platforms

#### Test Requirements
```rust
#[test]
fn test_config_file_watching() {
    // Test file change detection and reload triggering
}

#[test]
fn test_live_font_size_change() {
    // Test font size change applies to renderer immediately
}
```

---

## Task 4: Command Palette Basic

### **4.1 Action System Foundation**
**Priority**: Medium
**Estimated Time**: 3-4 days
**Dependencies**: None

#### Requirements
- Define action trait and registry system
- Implement core terminal actions (copy, paste, search, config)
- Support action descriptions and key bindings
- Fuzzy search for action names

#### Implementation Guidelines
```rust
// File: crates/core/src/actions.rs
pub trait Action {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn execute(&self, context: &mut ActionContext) -> Result<()>;
}

pub struct ActionRegistry {
    actions: HashMap<String, Box<dyn Action>>,
    search_index: FuzzySearchIndex,
}
```

#### Acceptance Criteria
- [ ] Register 10+ core actions (copy, paste, find, config reload, etc.)
- [ ] Fuzzy search finds actions with typos (≥90% accuracy)
- [ ] Action execution time < 50ms for core actions
- [ ] Thread-safe action registry
- [ ] Actions can be disabled/enabled dynamically

#### Test Requirements
```rust
#[test]
fn test_action_registration() {
    let mut registry = ActionRegistry::new();
    registry.register(CopyAction::new());
    assert!(registry.find_action("copy").is_some());
}
```

---

### **4.2 Command Palette UI**
**Priority**: Medium
**Estimated Time**: 4-5 days
**Dependencies**: Task 4.1

#### Requirements
- Overlay UI for command palette (above terminal content)
- Text input with live search results
- Keyboard navigation (up/down arrows, enter to execute)
- Open with Ctrl+Shift+P, close with Escape

#### Implementation Guidelines
```rust
// File: crates/renderer/src/palette.rs
pub struct CommandPalette {
    visible: bool,
    search_query: String,
    filtered_actions: Vec<ActionMatch>,
    selected_index: usize,
    input_buffer: TextInput,
}

pub struct ActionMatch {
    pub action_id: String,
    pub score: f32,
    pub highlighted_name: String,
}
```

#### Acceptance Criteria
- [ ] Palette opens in < 50ms from keypress
- [ ] Search updates in real-time as user types
- [ ] Results ranked by relevance and usage frequency
- [ ] Keyboard navigation feels responsive (< 16ms)
- [ ] Visual design matches terminal aesthetic

#### Test Requirements
```rust
#[test]
fn test_palette_search() {
    let mut palette = CommandPalette::new();
    palette.set_query("cop");
    let results = palette.get_filtered_actions();
    assert!(results.iter().any(|r| r.action_id == "copy"));
}
```

---

## Task 5: Shell Integration for Command Boundaries

### **5.1 Shell Hook System**
**Priority**: High
**Estimated Time**: 4-5 days
**Dependencies**: Existing PTY system

#### Requirements
- Inject shell hooks for bash, zsh, fish
- Detect command start/end boundaries
- Capture command text and execution time
- Handle nested commands and complex pipelines

#### Implementation Guidelines
```rust
// File: crates/pty/src/hooks.rs
pub struct ShellHooks {
    shell_type: ShellType,
    hook_sequences: HashMap<ShellType, HookConfig>,
    boundary_detector: BoundaryDetector,
}

#[derive(Debug)]
pub enum ShellType {
    Bash,
    Zsh, 
    Fish,
    Unknown,
}

pub struct CommandBoundary {
    pub start_offset: usize,
    pub end_offset: usize,
    pub command_text: String,
    pub start_time: Instant,
    pub end_time: Option<Instant>,
}
```

#### Acceptance Criteria
- [ ] Detect ≥95% of command boundaries in test corpus
- [ ] Support bash PS1/PS2 and PROMPT_COMMAND
- [ ] Support zsh precmd/preexec hooks
- [ ] Support fish shell event system
- [ ] Handle complex prompts (multi-line, ANSI escapes)

#### Test Requirements
```rust
#[test]
fn test_bash_boundary_detection() {
    let hooks = ShellHooks::new(ShellType::Bash);
    let output = "user@host:~$ ls -la\ntotal 8\nuser@host:~$ ";
    let boundaries = hooks.detect_boundaries(output);
    assert_eq!(boundaries.len(), 1);
    assert_eq!(boundaries[0].command_text, "ls -la");
}
```

---

### **5.2 Shell Auto-Detection**
**Priority**: Medium
**Estimated Time**: 2-3 days
**Dependencies**: Task 5.1

#### Requirements
- Automatically detect shell type from PTY output
- Parse shell identification strings and prompts
- Fallback heuristics for unknown shells
- Dynamic hook injection based on detected shell

#### Implementation Guidelines
```rust
// File: crates/pty/src/detection.rs
pub struct ShellDetector {
    detection_patterns: HashMap<ShellType, Vec<Regex>>,
    confidence_scores: HashMap<ShellType, f32>,
}

impl ShellDetector {
    pub fn analyze_output(&mut self, data: &[u8]) -> Option<ShellType> {
        // Analyze PTY output to detect shell type
    }
}
```

#### Acceptance Criteria
- [ ] Detect shell type within first 10 lines of output
- [ ] ≥95% accuracy on common shells (bash, zsh, fish)
- [ ] Graceful fallback for unknown shells
- [ ] Detection latency < 100ms
- [ ] Works with custom prompts and themes

#### Test Requirements
```rust
#[test]
fn test_shell_detection() {
    let mut detector = ShellDetector::new();
    let bash_output = b"GNU bash, version 5.1.16";
    assert_eq!(detector.analyze_output(bash_output), Some(ShellType::Bash));
}
```

---

## Task 6: Command Blocks v1

### **6.1 Block Data Model**
**Priority**: High
**Estimated Time**: 3-4 days
**Dependencies**: Task 5 (shell integration)

#### Requirements
- Define command block structure with metadata
- Link blocks to terminal grid line ranges
- Support block annotations and tags
- Efficient storage and retrieval

#### Implementation Guidelines
```rust
// File: crates/blocks/src/model.rs
#[derive(Debug, Clone)]
pub struct CommandBlock {
    pub id: Uuid,
    pub command: String,
    pub working_dir: PathBuf,
    pub start_line: usize,
    pub end_line: Option<usize>,
    pub start_time: Instant,
    pub end_time: Option<Instant>,
    pub exit_code: Option<i32>,
    pub tags: Vec<String>,
    pub collapsed: bool,
}

pub struct BlockManager {
    blocks: Vec<CommandBlock>,
    line_to_block: HashMap<usize, Uuid>,
    current_block: Option<Uuid>,
}
```

#### Acceptance Criteria
- [ ] Create new block for each detected command
- [ ] Track block metadata accurately
- [ ] Handle overlapping/nested commands
- [ ] Memory usage < 100KB for 1000 blocks
- [ ] Block lookup by line number in O(log n)

#### Test Requirements
```rust
#[test]
fn test_block_creation() {
    let mut manager = BlockManager::new();
    let block_id = manager.start_block("ls -la", 10);
    manager.end_block(block_id, 15, Some(0));
    
    let block = manager.get_block(block_id).unwrap();
    assert_eq!(block.command, "ls -la");
    assert_eq!(block.start_line, 10);
}
```

---

### **6.2 Block UI and Interaction**
**Priority**: High
**Estimated Time**: 4-5 days
**Dependencies**: Task 6.1

#### Requirements
- Visual indicators for command blocks in terminal
- Click to collapse/expand blocks
- Hover tooltips with block metadata
- Block selection and operations (copy, export)

#### Implementation Guidelines
```rust
// File: crates/renderer/src/blocks_ui.rs
pub struct BlockRenderer {
    block_decorations: HashMap<Uuid, BlockDecoration>,
    hover_state: Option<BlockHoverState>,
    selection_state: Option<BlockSelection>,
}

pub struct BlockDecoration {
    pub fold_indicator: FoldIndicator,
    pub border_style: BorderStyle,
    pub background_tint: Option<Color>,
}
```

#### Acceptance Criteria
- [ ] Visual block boundaries clearly visible
- [ ] Collapse/expand animation smooth (60fps)
- [ ] Block operations accessible via right-click menu
- [ ] Hover tooltips show command info < 100ms
- [ ] Block UI doesn't interfere with text selection

#### Test Requirements
```rust
#[test]
fn test_block_collapse() {
    let mut renderer = BlockRenderer::new();
    renderer.collapse_block(block_id);
    assert!(renderer.is_block_collapsed(block_id));
}
```

---

## Integration and Testing Requirements

### **End-to-End Testing Framework**
Each task must include integration tests that verify the complete pipeline:

```rust
// File: tests/integration/phase2_tests.rs
#[test]
fn test_font_to_screen_pipeline() {
    // Test: Load font -> Shape text -> Create atlas -> Render to screen
}

#[test]  
fn test_config_to_visual_change() {
    // Test: Change config -> Live reload -> Visual update
}

#[test]
fn test_command_to_block_pipeline() {
    // Test: Shell command -> Boundary detection -> Block creation -> UI update
}
```

### **Performance Benchmarks**
Each task must meet specific performance targets:

```rust
// File: benchmarks/phase2_benchmarks.rs
#[bench]
fn bench_glyph_shaping(b: &mut Bencher) {
    // Target: < 1ms for 80 character line
}

#[bench]
fn bench_dirty_rendering(b: &mut Bencher) {
    // Target: 30% reduction in frame time
}

#[bench]
fn bench_command_palette_search(b: &mut Bencher) {
    // Target: < 50ms to open and populate
}
```

### **Cross-Platform Validation**
All tasks must work correctly on:
- Linux (X11 and Wayland)
- macOS (Intel and Apple Silicon)  
- Windows 10/11

### **Acceptance Gate for Phase 2**
Phase 2 is complete when ALL of the following criteria are met:

#### **Technical Criteria**
- [ ] All 6 task groups implemented and tested
- [ ] Performance targets met for all components
- [ ] Cross-platform compatibility verified
- [ ] Memory usage within specified limits
- [ ] No regressions in existing functionality

#### **Quality Criteria**
- [ ] Unit test coverage ≥ 80% for new code
- [ ] Integration tests pass on CI matrix
- [ ] Benchmark performance within 10% of targets
- [ ] No clippy warnings or unsafe code
- [ ] Documentation updated for new features

#### **User Experience Criteria**
- [ ] Font rendering crisp at all sizes (8pt-72pt)
- [ ] Smooth scrolling with dirty rendering
- [ ] Command palette feels responsive (< 50ms)
- [ ] Config changes apply without restart
- [ ] Command blocks clearly visible and functional

---

## Implementation Priority Order

For AI coding agents, implement tasks in this order to minimize dependencies:

1. **Task 1.1-1.3**: Font system (critical foundation)
2. **Task 3.1-3.2**: Config system (enables font configuration)
3. **Task 2.1-2.2**: Dirty rendering (performance improvement)
4. **Task 5.1-5.2**: Shell integration (enables block detection)
5. **Task 6.1-6.2**: Command blocks (depends on shell integration)
6. **Task 4.1-4.2**: Command palette (final UX feature)

Each task should be fully completed and tested before moving to the next task.