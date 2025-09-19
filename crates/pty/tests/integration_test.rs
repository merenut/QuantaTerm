use quantaterm_pty::{Pty, PtyEvent};
use std::time::Duration;
use tokio::time;

/// Integration test for PTY functionality
/// This test verifies that the PTY can spawn a shell and execute commands
#[tokio::test]
#[ignore = "requires terminal environment"]
async fn test_pty_shell_integration() -> Result<(), Box<dyn std::error::Error>> {
    // Create and start PTY
    let mut pty = Pty::new();
    pty.start_shell(80, 24).await?;
    
    // Send a simple command that should produce output
    pty.write_data(b"echo test\n")?;
    
    // Wait for output
    let mut output_received = false;
    let start_time = std::time::Instant::now();
    
    while start_time.elapsed() < Duration::from_secs(3) && !output_received {
        if let Some(event) = pty.try_recv_event() {
            match event {
                PtyEvent::Data(data) => {
                    let text = String::from_utf8_lossy(&data);
                    if text.contains("test") {
                        output_received = true;
                    }
                }
                PtyEvent::ProcessExit(_) | PtyEvent::Error(_) => {
                    break;
                }
            }
        }
        time::sleep(Duration::from_millis(50)).await;
    }
    
    // Shutdown
    pty.shutdown()?;
    
    assert!(output_received, "Should receive echo output from shell");
    Ok(())
}

/// Test that PTY can be created and basic operations work
#[tokio::test]
async fn test_pty_basic_operations() -> Result<(), Box<dyn std::error::Error>> {
    let mut pty = Pty::new();
    
    // Should be able to start shell
    pty.start_shell(80, 24).await?;
    
    // Should be able to send commands
    assert!(pty.write_data(b"test\n").is_ok());
    assert!(pty.resize(120, 30).is_ok());
    assert!(pty.shutdown().is_ok());
    
    Ok(())
}