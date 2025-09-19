//! QuantaTerm main binary
//!
//! Main application entry point for QuantaTerm terminal emulator.

use anyhow::{Context, Result};
use tracing::info;
use winit::event_loop::{ControlFlow, EventLoop};

mod app;

use app::QuantaTermApp;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting QuantaTerm v{}", quantaterm_core::VERSION);

    // Create event loop
    let event_loop = EventLoop::new().context("Failed to create event loop")?;
    event_loop.set_control_flow(ControlFlow::Wait);

    // Create and run application
    let mut app = QuantaTermApp::new().await?;
    event_loop
        .run_app(&mut app)
        .context("Failed to run application")?;

    info!("QuantaTerm shutting down");
    Ok(())
}
