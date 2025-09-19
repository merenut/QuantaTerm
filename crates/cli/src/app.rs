//! QuantaTerm application
//!
//! Main application logic for handling window creation, events, and rendering.

use anyhow::{Context, Result};
use quantaterm_pty::{Pty, PtyEvent};
use quantaterm_renderer::Renderer;
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes, WindowId},
};

/// Main QuantaTerm application
pub struct QuantaTermApp {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    pty: Option<Pty>,
}

impl QuantaTermApp {
    /// Create a new QuantaTerm application
    pub async fn new() -> Result<Self> {
        info!("Initializing QuantaTerm application");
        
        // Initialize PTY 
        let pty = Pty::new();
        
        Ok(Self {
            window: None,
            renderer: None,
            pty: Some(pty),
        })
    }

    /// Handle keyboard input events
    fn handle_keyboard_input(&mut self, event: KeyEvent, event_loop: &ActiveEventLoop) {
        debug!("Keyboard input: {:?}", event);

        if event.state == ElementState::Pressed {
            match event.physical_key {
                PhysicalKey::Code(KeyCode::Escape) => {
                    info!("Escape key pressed, exiting application");
                    if let Some(ref pty) = self.pty {
                        if let Err(e) = pty.shutdown() {
                            warn!("Failed to shutdown PTY: {}", e);
                        }
                    }
                    event_loop.exit();
                }
                PhysicalKey::Code(keycode) => {
                    debug!("Key pressed: {:?}", keycode);
                    
                    // Convert keycode to bytes and send to PTY
                    if let Some(ref pty) = self.pty {
                        if let Some(data) = self.keycode_to_bytes(keycode) {
                            if let Err(e) = pty.write_data(&data) {
                                warn!("Failed to write to PTY: {}", e);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Convert a keycode to bytes to send to the shell
    fn keycode_to_bytes(&self, keycode: KeyCode) -> Option<Vec<u8>> {
        match keycode {
            KeyCode::Enter | KeyCode::NumpadEnter => Some(b"\r".to_vec()),
            KeyCode::Backspace => Some(b"\x08".to_vec()),
            KeyCode::Tab => Some(b"\t".to_vec()),
            KeyCode::Space => Some(b" ".to_vec()),
            KeyCode::ArrowUp => Some(b"\x1b[A".to_vec()),
            KeyCode::ArrowDown => Some(b"\x1b[B".to_vec()),
            KeyCode::ArrowRight => Some(b"\x1b[C".to_vec()),
            KeyCode::ArrowLeft => Some(b"\x1b[D".to_vec()),
            // Letter keys (simplified - would need more complete mapping)
            KeyCode::KeyA => Some(b"a".to_vec()),
            KeyCode::KeyB => Some(b"b".to_vec()),
            KeyCode::KeyC => Some(b"c".to_vec()),
            KeyCode::KeyD => Some(b"d".to_vec()),
            KeyCode::KeyE => Some(b"e".to_vec()),
            KeyCode::KeyF => Some(b"f".to_vec()),
            KeyCode::KeyG => Some(b"g".to_vec()),
            KeyCode::KeyH => Some(b"h".to_vec()),
            KeyCode::KeyI => Some(b"i".to_vec()),
            KeyCode::KeyJ => Some(b"j".to_vec()),
            KeyCode::KeyK => Some(b"k".to_vec()),
            KeyCode::KeyL => Some(b"l".to_vec()),
            KeyCode::KeyM => Some(b"m".to_vec()),
            KeyCode::KeyN => Some(b"n".to_vec()),
            KeyCode::KeyO => Some(b"o".to_vec()),
            KeyCode::KeyP => Some(b"p".to_vec()),
            KeyCode::KeyQ => Some(b"q".to_vec()),
            KeyCode::KeyR => Some(b"r".to_vec()),
            KeyCode::KeyS => Some(b"s".to_vec()),
            KeyCode::KeyT => Some(b"t".to_vec()),
            KeyCode::KeyU => Some(b"u".to_vec()),
            KeyCode::KeyV => Some(b"v".to_vec()),
            KeyCode::KeyW => Some(b"w".to_vec()),
            KeyCode::KeyX => Some(b"x".to_vec()),
            KeyCode::KeyY => Some(b"y".to_vec()),
            KeyCode::KeyZ => Some(b"z".to_vec()),
            // Digits
            KeyCode::Digit1 => Some(b"1".to_vec()),
            KeyCode::Digit2 => Some(b"2".to_vec()),
            KeyCode::Digit3 => Some(b"3".to_vec()),
            KeyCode::Digit4 => Some(b"4".to_vec()),
            KeyCode::Digit5 => Some(b"5".to_vec()),
            KeyCode::Digit6 => Some(b"6".to_vec()),
            KeyCode::Digit7 => Some(b"7".to_vec()),
            KeyCode::Digit8 => Some(b"8".to_vec()),
            KeyCode::Digit9 => Some(b"9".to_vec()),
            KeyCode::Digit0 => Some(b"0".to_vec()),
            _ => {
                debug!("Unhandled keycode: {:?}", keycode);
                None
            }
        }
    }

    /// Create the main window
    async fn create_window(&mut self, event_loop: &ActiveEventLoop) -> Result<()> {
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

        // Start PTY with initial size (rough terminal size calculation)
        if let Some(ref mut pty) = self.pty {
            let cols = 80; // Default terminal width
            let rows = 24; // Default terminal height
            
            pty.start_shell(cols, rows).await
                .context("Failed to start shell")?;
                
            // Add welcome message to renderer
            if let Some(ref mut renderer) = self.renderer {
                renderer.add_text("QuantaTerm v0.1.0 - Shell Started");
                renderer.add_text("Type commands and see output appear!");
                renderer.add_text("Press Escape to exit.");
            }
        }

        info!("Window and renderer initialized successfully");
        Ok(())
    }

    /// Process PTY events
    fn process_pty_events(&mut self) {
        if let Some(ref mut pty) = self.pty {
            while let Some(event) = pty.try_recv_event() {
                match event {
                    PtyEvent::Data(data) => {
                        // Convert bytes to string for display
                        if let Ok(text) = String::from_utf8(data.clone()) {
                            debug!("Shell output: {}", text.trim());
                            
                            // Send text to renderer for display
                            if let Some(ref mut renderer) = self.renderer {
                                renderer.add_text(&text);
                            }
                        } else {
                            debug!("Shell output (binary): {} bytes", data.len());
                        }
                    }
                    PtyEvent::ParsedActions(_actions) => {
                        // TODO: Apply parsed actions to terminal grid
                        // For now, we just use the raw data above
                        debug!("Received parsed terminal actions");
                    }
                    PtyEvent::ProcessExit(code) => {
                        info!("Shell process exited with code: {}", code);
                        
                        // Add exit message to display
                        if let Some(ref mut renderer) = self.renderer {
                            renderer.add_text(&format!("Shell exited with code: {}", code));
                        }
                    }
                    PtyEvent::Error(error) => {
                        error!("PTY error: {}", error);
                        
                        // Add error message to display
                        if let Some(ref mut renderer) = self.renderer {
                            renderer.add_text(&format!("PTY Error: {}", error));
                        }
                    }
                }
            }
        }
    }
}

impl ApplicationHandler for QuantaTermApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            // Use block_on since we can't make resumed async
            if let Err(e) = pollster::block_on(self.create_window(event_loop)) {
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
                if let Some(ref pty) = self.pty {
                    if let Err(e) = pty.shutdown() {
                        warn!("Failed to shutdown PTY: {}", e);
                    }
                }
                event_loop.exit();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                self.handle_keyboard_input(event, event_loop);
            }
            WindowEvent::RedrawRequested => {
                // Process PTY events before rendering
                self.process_pty_events();

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
                
                // Resize PTY to match window 
                if let Some(ref pty) = self.pty {
                    // Rough calculation: divide by character size (estimate)
                    let cols = (physical_size.width / 8).max(1) as u16; // ~8px per char width
                    let rows = (physical_size.height / 16).max(1) as u16; // ~16px per char height
                    
                    if let Err(e) = pty.resize(cols, rows) {
                        warn!("Failed to resize PTY: {}", e);
                    }
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_app_creation() {
        let app = QuantaTermApp::new().await.unwrap();
        assert!(app.window.is_none());
        assert!(app.renderer.is_none());
        assert!(app.pty.is_some());
    }

    #[test]
    fn test_keycode_conversion() {
        let app = pollster::block_on(QuantaTermApp::new()).unwrap();
        
        assert_eq!(app.keycode_to_bytes(KeyCode::Enter), Some(b"\r".to_vec()));
        assert_eq!(app.keycode_to_bytes(KeyCode::KeyA), Some(b"a".to_vec()));
        assert_eq!(app.keycode_to_bytes(KeyCode::Space), Some(b" ".to_vec()));
        assert_eq!(app.keycode_to_bytes(KeyCode::ArrowUp), Some(b"\x1b[A".to_vec()));
    }

    #[test]
    fn test_pty_event_handling() {
        use quantaterm_pty::PtyEvent;
        
        // Test that our event handling logic works
        let event_data = PtyEvent::Data(b"Hello World\n".to_vec());
        let event_exit = PtyEvent::ProcessExit(0);
        let event_error = PtyEvent::Error("Test error".to_string());
        
        // Verify events can be created and matched
        match event_data {
            PtyEvent::Data(data) => {
                let text = String::from_utf8(data).unwrap();
                assert!(text.contains("Hello World"));
            }
            _ => panic!("Expected Data event"),
        }
        
        match event_exit {
            PtyEvent::ProcessExit(code) => assert_eq!(code, 0),
            _ => panic!("Expected ProcessExit event"),
        }
        
        match event_error {
            PtyEvent::Error(err) => assert_eq!(err, "Test error"),
            _ => panic!("Expected Error event"),
        }
    }
}
