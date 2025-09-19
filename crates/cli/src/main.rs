//! QuantaTerm main binary
//!
//! Main application entry point for QuantaTerm terminal emulator.

use anyhow::{Context, Result};
use quantaterm_core::logging::{self, dev_config};
use tracing::info;
use winit::event_loop::{ControlFlow, EventLoop};

mod app;

use app::QuantaTermApp;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize structured logging with development configuration
    let logging_config = dev_config();
    logging::init_logging(&logging_config).context("Failed to initialize logging")?;

    info!(
        version = quantaterm_core::VERSION,
        config = ?logging_config,
        "Starting QuantaTerm"
    );

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
