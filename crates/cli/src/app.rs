//! QuantaTerm application
//!
//! Main application logic for handling window creation, events, and rendering.

use anyhow::{Context, Result};
use std::sync::Arc;
use tracing::{debug, info, warn};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId, WindowAttributes},
};

use quantaterm_renderer::Renderer;

/// Main QuantaTerm application
pub struct QuantaTermApp {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
}

impl QuantaTermApp {
    /// Create a new QuantaTerm application
    pub fn new() -> Self {
        Self {
            window: None,
            renderer: None,
        }
    }

    /// Handle keyboard input events
    fn handle_keyboard_input(&mut self, event: KeyEvent, event_loop: &ActiveEventLoop) {
        debug!("Keyboard input: {:?}", event);

        if event.state == ElementState::Pressed {
            match event.physical_key {
                PhysicalKey::Code(KeyCode::Escape) => {
                    info!("Escape key pressed, exiting application");
                    event_loop.exit();
                }
                PhysicalKey::Code(keycode) => {
                    debug!("Key pressed: {:?}", keycode);
                }
                _ => {}
            }
        }
    }

    /// Create the main window
    fn create_window(&mut self, event_loop: &ActiveEventLoop) -> Result<()> {
        let window_attributes = WindowAttributes::default()
            .with_title("QuantaTerm")
            .with_inner_size(winit::dpi::LogicalSize::new(800, 600));

        let window = event_loop
            .create_window(window_attributes)
            .context("Failed to create window")?;

        let window = Arc::new(window);
        
        // Initialize renderer with the window
        let renderer = pollster::block_on(Renderer::new(window.clone()))
            .context("Failed to initialize renderer")?;

        self.window = Some(window);
        self.renderer = Some(renderer);

        info!("Window and renderer initialized successfully");
        Ok(())
    }
}

impl Default for QuantaTermApp {
    fn default() -> Self {
        Self::new()
    }
}

impl ApplicationHandler for QuantaTermApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            if let Err(e) = self.create_window(event_loop) {
                warn!("Failed to create window: {}", e);
                event_loop.exit();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                info!("Window close requested");
                event_loop.exit();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                self.handle_keyboard_input(event, event_loop);
            }
            WindowEvent::RedrawRequested => {
                if let Some(renderer) = &mut self.renderer {
                    if let Err(e) = renderer.render() {
                        warn!("Render error: {}", e);
                    }
                }
            }
            WindowEvent::Resized(physical_size) => {
                debug!("Window resized to {:?}", physical_size);
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(physical_size);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_creation() {
        let app = QuantaTermApp::new();
        assert!(app.window.is_none());
        assert!(app.renderer.is_none());
    }

    #[test]
    fn test_app_default() {
        let app = QuantaTermApp::default();
        assert!(app.window.is_none());
        assert!(app.renderer.is_none());
    }
}