//! QuantaTerm PTY management and shell interaction
//!
//! PTY management and shell interaction.

#![warn(missing_docs)]
#![deny(unsafe_code)]

use anyhow::{Context, Result};
use portable_pty::{CommandBuilder, PtySize};
use std::io::{BufRead, BufReader, Write};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn, instrument, trace};

pub mod parser;

pub use parser::{TerminalParser, ParseAction, CsiAction, EscAction, ParserState};

/// Events from the PTY that need to be handled by the application
#[derive(Debug, Clone)]
pub enum PtyEvent {
    /// Raw data received from the shell (stdout/stderr)
    Data(Vec<u8>),
    /// Parsed actions from terminal escape sequences
    ParsedActions(Vec<ParseAction>),
    /// Shell process has exited
    ProcessExit(i32),
    /// Error occurred in PTY operations
    Error(String),
}

/// Commands that can be sent to the PTY
#[derive(Debug, Clone)]
pub enum PtyCommand {
    /// Write data to the shell's stdin
    WriteData(Vec<u8>),
    /// Resize the PTY
    Resize { 
        /// New width in columns
        width: u16, 
        /// New height in rows
        height: u16 
    },
    /// Shutdown the PTY
    Shutdown,
}

/// PTY management and shell interaction
pub struct Pty {
    /// Channel for sending commands to the PTY
    command_tx: Option<mpsc::UnboundedSender<PtyCommand>>,
    /// Channel for receiving events from the PTY
    event_rx: Option<mpsc::UnboundedReceiver<PtyEvent>>,
}

impl Pty {
    /// Create a new PTY instance
    pub fn new() -> Self {
        Self {
            command_tx: None,
            event_rx: None,
        }
    }

    /// Start the shell and PTY communication
    #[instrument(name = "pty_start_shell", skip(self))]
    pub async fn start_shell(&mut self, width: u16, height: u16) -> Result<()> {
        info!(
            subsystem = "pty",
            width = width,
            height = height,
            "Starting shell session"
        );

        // Create channels for communication
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        self.command_tx = Some(command_tx);
        self.event_rx = Some(event_rx);

        // Spawn the PTY task
        tokio::spawn(async move {
            if let Err(e) = Self::pty_task(command_rx, event_tx, width, height).await {
                error!(
                    subsystem = "pty",
                    error = %e,
                    "PTY task failed"
                );
            }
        });

        info!(subsystem = "pty", "Shell session started successfully");
        Ok(())
    }

    /// Send a command to the PTY
    pub fn send_command(&self, command: PtyCommand) -> Result<()> {
        if let Some(ref tx) = self.command_tx {
            tx.send(command)
                .context("Failed to send command to PTY")?;
            Ok(())
        } else {
            anyhow::bail!("PTY not started")
        }
    }

    /// Try to receive a PTY event (non-blocking)
    pub fn try_recv_event(&mut self) -> Option<PtyEvent> {
        if let Some(ref mut rx) = self.event_rx {
            rx.try_recv().ok()
        } else {
            None
        }
    }

    /// Receive a PTY event (blocking)
    pub async fn recv_event(&mut self) -> Option<PtyEvent> {
        if let Some(ref mut rx) = self.event_rx {
            rx.recv().await
        } else {
            None
        }
    }

    /// Write data to the shell's stdin
    #[instrument(name = "pty_write_data", skip(self, data))]
    pub fn write_data(&self, data: &[u8]) -> Result<()> {
        debug!(
            subsystem = "pty",
            byte_count = data.len(),
            "Writing data to shell"
        );
        self.send_command(PtyCommand::WriteData(data.to_vec()))
    }

    /// Resize the PTY
    #[instrument(name = "pty_resize", skip(self))]
    pub fn resize(&self, width: u16, height: u16) -> Result<()> {
        debug!(
            subsystem = "pty",
            width = width,
            height = height,
            "Resizing PTY"
        );
        self.send_command(PtyCommand::Resize { width, height })
    }

    /// Shutdown the PTY
    #[instrument(name = "pty_shutdown", skip(self))]
    pub fn shutdown(&self) -> Result<()> {
        info!(subsystem = "pty", "Shutting down PTY");
        self.send_command(PtyCommand::Shutdown)
    }

    /// Get the default shell for the current platform
    fn get_default_shell() -> CommandBuilder {
        #[cfg(windows)]
        {
            let mut cmd = CommandBuilder::new("cmd.exe");
            cmd.arg("/k"); // Keep cmd.exe open after running command
            cmd
        }

        #[cfg(not(windows))]
        {
            // Try to get shell from environment or use default
            let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
            CommandBuilder::new(shell)
        }
    }

    /// Main PTY task that handles shell communication
    #[instrument(name = "pty_task", skip(command_rx, event_tx))]
    async fn pty_task(
        mut command_rx: mpsc::UnboundedReceiver<PtyCommand>,
        event_tx: mpsc::UnboundedSender<PtyEvent>,
        width: u16,
        height: u16,
    ) -> Result<()> {
        let pty_system = portable_pty::native_pty_system();

        // Create PTY with initial size
        let pty_size = PtySize {
            rows: height,
            cols: width,
            pixel_width: 0,
            pixel_height: 0,
        };

        let pty_pair = pty_system
            .openpty(pty_size)
            .context("Failed to create PTY")?;

        // Create command for default shell
        let cmd = Self::get_default_shell();
        info!(
            subsystem = "pty",
            command = ?cmd,
            "Spawning shell process"
        );

        // Spawn the shell process
        let mut child = pty_pair
            .slave
            .spawn_command(cmd)
            .context("Failed to spawn shell")?;

        // Get handles for reading and writing
        let reader = pty_pair.master.try_clone_reader().context("Failed to clone reader")?;
        let mut writer = pty_pair.master.take_writer().context("Failed to take writer")?;

        // Spawn task to read shell output
        let read_event_tx = event_tx.clone();
        let read_task = tokio::spawn(async move {
            let mut buf_reader = BufReader::new(reader);
            let mut buffer = Vec::new();
            let mut parser = TerminalParser::new();

            loop {
                buffer.clear();
                match buf_reader.read_until(b'\n', &mut buffer) {
                    Ok(0) => {
                        // EOF - shell has closed
                        debug!(subsystem = "pty", "Shell output stream closed");
                        break;
                    }
                    Ok(bytes_read) => {
                        trace!(
                            subsystem = "pty",
                            bytes_read = bytes_read,
                            "Read data from shell"
                        );
                        
                        // Send raw data event
                        if let Err(e) = read_event_tx.send(PtyEvent::Data(buffer.clone())) {
                            warn!(
                                subsystem = "pty",
                                error = %e,
                                "Failed to send data event"
                            );
                            break;
                        }
                        
                        // Parse the data and send parsed actions
                        let actions = parser.parse(&buffer);
                        if !actions.is_empty() {
                            if let Err(e) = read_event_tx.send(PtyEvent::ParsedActions(actions)) {
                                warn!(
                                    subsystem = "pty",
                                    error = %e,
                                    "Failed to send parsed actions event"
                                );
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        error!(
                            subsystem = "pty",
                            error = %e,
                            "Failed to read from shell"
                        );
                        let _ = read_event_tx.send(PtyEvent::Error(e.to_string()));
                        break;
                    }
                }
            }
        });

        // Main command processing loop
        loop {
            tokio::select! {
                // Handle incoming commands
                cmd = command_rx.recv() => {
                    match cmd {
                        Some(PtyCommand::WriteData(data)) => {
                            trace!(
                                subsystem = "pty",
                                byte_count = data.len(),
                                "Processing write command"
                            );
                            if let Err(e) = writer.write_all(&data) {
                                error!(
                                    subsystem = "pty",
                                    error = %e,
                                    "Failed to write to shell"
                                );
                                let _ = event_tx.send(PtyEvent::Error(e.to_string()));
                            } else if let Err(e) = writer.flush() {
                                error!(
                                    subsystem = "pty",
                                    error = %e,
                                    "Failed to flush shell writer"
                                );
                                let _ = event_tx.send(PtyEvent::Error(e.to_string()));
                            }
                        }
                        Some(PtyCommand::Resize { width, height }) => {
                            debug!(
                                subsystem = "pty",
                                width = width,
                                height = height,
                                "Processing resize command"
                            );
                            let new_size = PtySize {
                                rows: height,
                                cols: width,
                                pixel_width: 0,
                                pixel_height: 0,
                            };
                            if let Err(e) = pty_pair.master.resize(new_size) {
                                error!(
                                    subsystem = "pty",
                                    error = %e,
                                    "Failed to resize PTY"
                                );
                                let _ = event_tx.send(PtyEvent::Error(e.to_string()));
                            } else {
                                debug!(
                                    subsystem = "pty",
                                    width = width,
                                    height = height,
                                    "PTY resized successfully"
                                );
                            }
                        }
                        Some(PtyCommand::Shutdown) => {
                            info!(subsystem = "pty", "PTY shutdown requested");
                            break;
                        }
                        None => {
                            debug!(subsystem = "pty", "Command channel closed");
                            break;
                        }
                    }
                }

                // Check if shell process has exited
                _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                    if let Ok(Some(exit_status)) = child.try_wait() {
                        let exit_code = exit_status.exit_code() as i32;
                        info!(
                            subsystem = "pty",
                            exit_code = exit_code,
                            "Shell process exited"
                        );
                        let _ = event_tx.send(PtyEvent::ProcessExit(exit_code));
                        break;
                    }
                }
            }
        }

        // Cleanup
        read_task.abort();
        let _ = child.kill();
        info!(subsystem = "pty", "PTY session ended");

        Ok(())
    }
}

impl Default for Pty {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pty_creation() {
        let pty = Pty::new();
        assert!(pty.command_tx.is_none());
        assert!(pty.event_rx.is_none());
    }

    #[test]
    fn test_pty_default() {
        let pty = Pty::default();
        assert!(pty.command_tx.is_none());
        assert!(pty.event_rx.is_none());
    }

    #[cfg(not(windows))]
    #[test]
    fn test_default_shell_unix() {
        let _cmd = Pty::get_default_shell();
        // Shell command should be created successfully
        // (We can't easily test the exact command without exposing internals)
    }

    #[cfg(windows)]
    #[test]
    fn test_default_shell_windows() {
        let _cmd = Pty::get_default_shell();
        // Windows command should be created successfully
        // (We can't easily test the exact command without exposing internals)
    }
}
