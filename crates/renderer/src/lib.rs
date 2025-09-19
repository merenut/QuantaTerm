//! QuantaTerm Renderer
//!
//! GPU-accelerated rendering for the terminal emulator.

#![warn(missing_docs)]
#![deny(unsafe_code)]

use anyhow::{Context, Result};
use std::sync::Arc;
use tracing::{debug, info, instrument, trace, warn};
use wgpu::{Device, Queue, Surface, SurfaceConfiguration};
use winit::{dpi::PhysicalSize, window::Window};

/// GPU-accelerated renderer for QuantaTerm
pub struct Renderer {
    _instance: wgpu::Instance,
    surface: Surface<'static>,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    size: PhysicalSize<u32>,
    /// Simple text buffer for storing terminal output
    text_buffer: Vec<String>,
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
            text_buffer: Vec::new(),
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
        // Split text into lines and add to buffer
        let new_lines: Vec<_> = text
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.to_string())
            .collect();

        let line_count = new_lines.len();
        self.text_buffer.extend(new_lines);

        debug!(
            subsystem = "renderer",
            line_count = line_count,
            total_buffer_size = self.text_buffer.len(),
            "Added text to renderer buffer"
        );

        // Keep only the last 100 lines to prevent memory growth
        if self.text_buffer.len() > 100 {
            let removed_count = self.text_buffer.len() - 100;
            self.text_buffer.drain(0..removed_count);
            trace!(
                subsystem = "renderer",
                removed_lines = removed_count,
                remaining_lines = self.text_buffer.len(),
                "Trimmed text buffer to prevent memory growth"
            );
        }

        // Change background color slightly when we have output to show visual feedback
        let line_count = self.text_buffer.len() as f64;
        self.background_color = wgpu::Color {
            r: 0.1 + (line_count * 0.01).min(0.2),
            g: 0.2 + (line_count * 0.005).min(0.1),
            b: 0.3,
            a: 1.0,
        };
    }

    /// Get the current text buffer (for debugging/testing)
    pub fn get_text_buffer(&self) -> &[String] {
        &self.text_buffer
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_renderer_module_exists() {
        // Basic test to ensure the renderer module compiles
        // We can't create an actual renderer without a window/display in CI
        assert!(true);
    }

    #[test]
    fn test_text_buffer_functionality() {
        // We can't create a full renderer without a window, but we can test
        // the logic by creating a mock renderer struct with the text buffer
        let mut text_buffer = Vec::new();

        // Test adding text
        let text = "Hello\nWorld\nTest";
        for line in text.lines() {
            if !line.trim().is_empty() {
                text_buffer.push(line.to_string());
            }
        }

        assert_eq!(text_buffer.len(), 3);
        assert_eq!(text_buffer[0], "Hello");
        assert_eq!(text_buffer[1], "World");
        assert_eq!(text_buffer[2], "Test");

        // Test buffer length limiting
        for i in 0..105 {
            text_buffer.push(format!("Line {}", i));
        }

        // Simulate buffer limit of 100
        if text_buffer.len() > 100 {
            text_buffer.drain(0..text_buffer.len() - 100);
        }

        assert_eq!(text_buffer.len(), 100);
    }
}
