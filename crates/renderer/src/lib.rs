//! QuantaTerm Renderer
//!
//! GPU-accelerated rendering for the terminal emulator.

#![warn(missing_docs)]
#![deny(unsafe_code)]

pub mod font;

use anyhow::{Context, Result};
use bitflags::bitflags;
use std::sync::Arc;
use tracing::{debug, info, instrument, trace, warn};
use wgpu::{Device, Queue, Surface, SurfaceConfiguration};
use winit::{dpi::PhysicalSize, window::Window};

/// A color representation for terminal cells (renderer-specific copy)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RendererColor {
    /// Red component (0-255)
    pub r: u8,
    /// Green component (0-255)
    pub g: u8,
    /// Blue component (0-255)
    pub b: u8,
    /// Alpha component (0-255)
    pub a: u8,
}

impl RendererColor {
    /// Create a new color
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Create a new RGB color with full alpha
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::new(r, g, b, 255)
    }
}

bitflags! {
    /// Cell attribute flags for styling (renderer-specific copy)
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct RendererCellAttrs: u32 {
        /// Bold text
        const BOLD = 1 << 0;
        /// Italic text
        const ITALIC = 1 << 1;
        /// Underlined text
        const UNDERLINE = 1 << 2;
        /// Strikethrough text
        const STRIKETHROUGH = 1 << 3;
        /// Blinking text
        const BLINK = 1 << 4;
        /// Reversed colors (fg/bg swapped)
        const REVERSE = 1 << 5;
        /// Hidden/invisible text
        const HIDDEN = 1 << 6;
    }
}

/// A terminal cell for rendering (renderer-specific copy)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RendererCell {
    /// Unicode glyph identifier
    pub glyph_id: u32,
    /// Foreground color
    pub fg_color: RendererColor,
    /// Background color
    pub bg_color: RendererColor,
    /// Formatting attributes
    pub attrs: RendererCellAttrs,
}

impl RendererCell {
    /// Create a new cell with the given glyph
    pub fn new(glyph_id: u32) -> Self {
        Self {
            glyph_id,
            fg_color: RendererColor::rgb(255, 255, 255), // White
            bg_color: RendererColor::rgb(0, 0, 0),       // Black
            attrs: RendererCellAttrs::empty(),
        }
    }

    /// Create a cell with custom colors and attributes
    pub fn with_style(
        glyph_id: u32,
        fg_color: RendererColor,
        bg_color: RendererColor,
        attrs: RendererCellAttrs,
    ) -> Self {
        Self {
            glyph_id,
            fg_color,
            bg_color,
            attrs,
        }
    }
}

/// A row of terminal cells for rendering
pub type RendererCellRow = Vec<RendererCell>;

/// GPU-accelerated renderer for QuantaTerm
pub struct Renderer {
    _instance: wgpu::Instance,
    surface: Surface<'static>,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    size: PhysicalSize<u32>,
    /// Terminal viewport with full color and attribute data
    viewport: Vec<RendererCellRow>,
    /// Current background color (changes when we receive shell output)
    background_color: wgpu::Color,
}

impl Renderer {
    /// Create a new renderer instance
    #[instrument(name = "renderer_new", skip(window))]
    pub async fn new(window: Arc<Window>) -> Result<Self> {
        let size = window.inner_size();
        info!(
            subsystem = "renderer",
            width = size.width,
            height = size.height,
            "Initializing wgpu renderer"
        );

        // Create wgpu instance
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        debug!(subsystem = "renderer", "Created wgpu instance");

        // Create surface
        let surface = instance
            .create_surface(window.clone())
            .context("Failed to create surface")?;

        debug!(subsystem = "renderer", "Created surface");

        // Request adapter
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .context("Failed to find an appropriate adapter")?;

        debug!(
            subsystem = "renderer",
            adapter_name = ?adapter.get_info().name,
            adapter_backend = ?adapter.get_info().backend,
            "Using GPU adapter"
        );

        // Request device and queue
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .context("Failed to create device")?;

        debug!(subsystem = "renderer", "Created device and queue");

        // Configure surface
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        trace!(
            subsystem = "renderer",
            surface_format = ?surface_format,
            "Configured surface"
        );

        info!(
            subsystem = "renderer",
            surface_format = ?surface_format,
            "Renderer initialization complete"
        );

        Ok(Self {
            _instance: instance,
            surface,
            device,
            queue,
            config,
            size,
            viewport: Vec::new(),
            background_color: wgpu::Color {
                r: 0.1,
                g: 0.2,
                b: 0.3,
                a: 1.0,
            },
        })
    }

    /// Resize the renderer to match window size
    #[instrument(name = "renderer_resize", skip(self))]
    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            debug!(
                subsystem = "renderer",
                width = new_size.width,
                height = new_size.height,
                "Renderer resized"
            );
        } else {
            warn!(
                subsystem = "renderer",
                width = new_size.width,
                height = new_size.height,
                "Ignoring invalid resize request"
            );
        }
    }

    /// Render a frame
    pub fn render(&mut self) -> Result<()> {
        let output = self
            .surface
            .get_current_texture()
            .context("Failed to acquire next swap chain texture")?;

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // Clear screen with background color (changes slightly when we have output)
        {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.background_color),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
        }

        // Submit commands and present
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    /// Add text to the terminal buffer and update display
    #[instrument(name = "renderer_add_text", skip(self))]
    pub fn add_text(&mut self, text: &str) {
        // Convert plain text to cell data for backward compatibility
        let new_lines: Vec<_> = text
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                line.chars()
                    .map(|c| RendererCell::new(c as u32))
                    .collect::<Vec<RendererCell>>()
            })
            .collect();

        let line_count = new_lines.len();
        self.viewport.extend(new_lines);

        debug!(
            subsystem = "renderer",
            line_count = line_count,
            total_viewport_lines = self.viewport.len(),
            "Added text to renderer viewport"
        );

        // Keep only the last 100 lines to prevent memory growth
        if self.viewport.len() > 100 {
            let removed_count = self.viewport.len() - 100;
            self.viewport.drain(0..removed_count);
            trace!(
                subsystem = "renderer",
                removed_lines = removed_count,
                remaining_lines = self.viewport.len(),
                "Trimmed viewport to prevent memory growth"
            );
        }

        // Change background color slightly when we have output to show visual feedback
        let line_count = self.viewport.len() as f64;
        self.background_color = wgpu::Color {
            r: 0.1 + (line_count * 0.01).min(0.2),
            g: 0.2 + (line_count * 0.005).min(0.1),
            b: 0.3,
            a: 1.0,
        };
    }

    /// Update the renderer with formatted viewport data from terminal grid
    /// This is the primary method for rendering color and attribute information
    #[instrument(name = "renderer_update_viewport", skip(self, viewport))]
    pub fn update_viewport(&mut self, viewport: Vec<RendererCellRow>) {
        debug!(
            subsystem = "renderer",
            rows = viewport.len(),
            cols = viewport.first().map(|row| row.len()).unwrap_or(0),
            "Updating renderer viewport with formatted cell data"
        );

        self.viewport = viewport;

        // Update background color based on content with formatted cells
        let total_cells: usize = self.viewport.iter().map(|row| row.len()).sum();
        let non_empty_cells = self
            .viewport
            .iter()
            .flat_map(|row| row.iter())
            .filter(|cell| cell.glyph_id != b' ' as u32 && cell.glyph_id != 0)
            .count();

        let content_ratio = if total_cells > 0 {
            non_empty_cells as f64 / total_cells as f64
        } else {
            0.0
        };

        self.background_color = wgpu::Color {
            r: 0.1 + (content_ratio * 0.15).min(0.15),
            g: 0.2 + (content_ratio * 0.1).min(0.1),
            b: 0.3,
            a: 1.0,
        };

        trace!(
            subsystem = "renderer",
            total_cells = total_cells,
            non_empty_cells = non_empty_cells,
            content_ratio = content_ratio,
            "Updated background color based on content density"
        );
    }

    /// Get a reference to the current viewport data
    pub fn get_viewport(&self) -> &[RendererCellRow] {
        &self.viewport
    }

    /// Get color information from a specific cell for rendering
    pub fn get_cell_colors(
        &self,
        row: usize,
        col: usize,
    ) -> Option<(RendererColor, RendererColor)> {
        self.viewport
            .get(row)
            .and_then(|row| row.get(col))
            .map(|cell| (cell.fg_color, cell.bg_color))
    }

    /// Get attribute information from a specific cell for rendering
    pub fn get_cell_attributes(&self, row: usize, col: usize) -> Option<RendererCellAttrs> {
        self.viewport
            .get(row)
            .and_then(|row| row.get(col))
            .map(|cell| cell.attrs)
    }

    /// Extract text content from viewport for compatibility
    pub fn get_viewport_text(&self) -> Vec<String> {
        self.viewport
            .iter()
            .map(|row| {
                row.iter()
                    .map(|cell| {
                        if cell.glyph_id == 0 {
                            ' '
                        } else {
                            (cell.glyph_id as u8) as char
                        }
                    })
                    .collect()
            })
            .collect()
    }

    /// Get the current text buffer (for debugging/testing)
    pub fn get_text_buffer(&self) -> Vec<String> {
        // Convert viewport to text for backward compatibility
        self.get_viewport_text()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_renderer_module_exists() {
        // Basic test to ensure the renderer module compiles
        // We can't create an actual renderer without a window/display in CI
        // Test passes if we reach this point
    }

    #[test]
    fn test_text_buffer_functionality() {
        // Test legacy text buffer functionality for backward compatibility
        let mut viewport = Vec::new();

        // Test adding text converted to cells
        let text = "Hello\nWorld\nTest";
        for line in text.lines() {
            if !line.trim().is_empty() {
                let cell_row: Vec<RendererCell> =
                    line.chars().map(|c| RendererCell::new(c as u32)).collect();
                viewport.push(cell_row);
            }
        }

        assert_eq!(viewport.len(), 3);
        assert_eq!(viewport[0].len(), 5); // "Hello"
        assert_eq!(viewport[1].len(), 5); // "World"
        assert_eq!(viewport[2].len(), 4); // "Test"

        // Verify cell content
        assert_eq!(viewport[0][0].glyph_id, b'H' as u32);
        assert_eq!(viewport[0][4].glyph_id, b'o' as u32);
        assert_eq!(viewport[1][0].glyph_id, b'W' as u32);

        // Test buffer length limiting simulation
        for i in 0..105 {
            let line: Vec<RendererCell> = format!("Line {}", i)
                .chars()
                .map(|c| RendererCell::new(c as u32))
                .collect();
            viewport.push(line);
        }

        // Simulate buffer limit of 100
        if viewport.len() > 100 {
            viewport.drain(0..viewport.len() - 100);
        }

        assert_eq!(viewport.len(), 100);
    }

    #[test]
    fn test_color_and_attribute_handling() {
        // Test that renderer can handle colored and styled cells
        let mut viewport = Vec::new();

        // Create a row with various colors and attributes
        let row = vec![
            // Red bold 'R'
            RendererCell::with_style(
                b'R' as u32,
                RendererColor::rgb(255, 0, 0),
                RendererColor::rgb(0, 0, 0),
                RendererCellAttrs::BOLD,
            ),
            // Green italic 'G'
            RendererCell::with_style(
                b'G' as u32,
                RendererColor::rgb(0, 255, 0),
                RendererColor::rgb(0, 0, 0),
                RendererCellAttrs::ITALIC,
            ),
            // Blue underlined 'B' with custom background
            RendererCell::with_style(
                b'B' as u32,
                RendererColor::rgb(0, 0, 255),
                RendererColor::rgb(128, 128, 128),
                RendererCellAttrs::UNDERLINE,
            ),
        ];

        viewport.push(row);

        // Verify color information can be extracted
        assert_eq!(viewport[0][0].fg_color, RendererColor::rgb(255, 0, 0));
        assert_eq!(viewport[0][0].attrs, RendererCellAttrs::BOLD);

        assert_eq!(viewport[0][1].fg_color, RendererColor::rgb(0, 255, 0));
        assert_eq!(viewport[0][1].attrs, RendererCellAttrs::ITALIC);

        assert_eq!(viewport[0][2].fg_color, RendererColor::rgb(0, 0, 255));
        assert_eq!(viewport[0][2].bg_color, RendererColor::rgb(128, 128, 128));
        assert_eq!(viewport[0][2].attrs, RendererCellAttrs::UNDERLINE);
    }

    #[test]
    fn test_viewport_text_conversion() {
        // Test conversion from viewport cells back to text
        let mut viewport = Vec::new();

        let text_line = "Hello World";
        let cell_row: Vec<RendererCell> = text_line
            .chars()
            .map(|c| RendererCell::new(c as u32))
            .collect();
        viewport.push(cell_row);

        // Convert back to text
        let converted_text: String = viewport[0]
            .iter()
            .map(|cell| (cell.glyph_id as u8) as char)
            .collect();

        assert_eq!(converted_text, text_line);
    }

    #[test]
    fn test_truecolor_support() {
        // Test that renderer can handle truecolor (24-bit) colors
        let cell = RendererCell::with_style(
            b'T' as u32,
            RendererColor::rgb(123, 45, 67),  // Custom RGB color
            RendererColor::rgb(234, 156, 78), // Custom RGB background
            RendererCellAttrs::BOLD | RendererCellAttrs::ITALIC,
        );

        assert_eq!(cell.fg_color.r, 123);
        assert_eq!(cell.fg_color.g, 45);
        assert_eq!(cell.fg_color.b, 67);

        assert_eq!(cell.bg_color.r, 234);
        assert_eq!(cell.bg_color.g, 156);
        assert_eq!(cell.bg_color.b, 78);

        assert!(cell.attrs.contains(RendererCellAttrs::BOLD));
        assert!(cell.attrs.contains(RendererCellAttrs::ITALIC));
    }

    #[test]
    fn test_256_color_cell_creation() {
        // Test that renderer can handle 256-color palette colors
        // (This would typically come from the parser, but we can test the cell structure)

        // Standard red (index 1 in 256-color palette)
        let red_cell = RendererCell::with_style(
            b'R' as u32,
            RendererColor::rgb(128, 0, 0),
            RendererColor::rgb(0, 0, 0),
            RendererCellAttrs::empty(),
        );

        // Bright green (index 10 in 256-color palette)
        let green_cell = RendererCell::with_style(
            b'G' as u32,
            RendererColor::rgb(0, 255, 0),
            RendererColor::rgb(0, 0, 0),
            RendererCellAttrs::empty(),
        );

        assert_eq!(red_cell.fg_color, RendererColor::rgb(128, 0, 0));
        assert_eq!(green_cell.fg_color, RendererColor::rgb(0, 255, 0));
    }
}
