//! Integration tests for VTE parser with terminal grid

use quantaterm_pty::{TerminalParser, ParseAction, CsiAction};
use quantaterm_blocks::{TerminalGrid, Color, CellAttrs};

/// Test basic SGR integration between parser and terminal grid
#[test]
fn test_sgr_integration() {
    let mut parser = TerminalParser::new();
    let mut grid = TerminalGrid::new(20, 5);
    
    // Parse a sequence with SGR formatting step by step
    let sequence_parts = [
        b"\x1b[1;31m".as_slice(),  // Bold red
        b"Hello",                   // Text 
        b"\x1b[0m".as_slice(),     // Reset
        b" World",                  // More text
    ];
    
    for part in sequence_parts.iter() {
        let actions = parser.parse(part);
        
        for action in actions {
            match action {
                ParseAction::Print(c) => {
                    // Apply current parser formatting to grid before printing
                    let state = parser.state();
                    grid.apply_sgr(state.fg_color, state.bg_color, state.attrs);
                    grid.print_char(c);
                }
                ParseAction::CsiDispatch(CsiAction::Sgr(_)) => {
                    // SGR state is already updated in parser
                    let state = parser.state();
                    grid.apply_sgr(state.fg_color, state.bg_color, state.attrs);
                }
                _ => {
                    // Handle other actions as needed
                }
            }
        }
    }
    
    // Verify formatting was applied correctly
    // "Hello" should be bold red
    let h_cell = grid.get_cell(0, 0).unwrap();
    assert_eq!(h_cell.glyph_id, b'H' as u32);
    assert!(h_cell.attrs.contains(CellAttrs::BOLD));
    
    let e_cell = grid.get_cell(1, 0).unwrap();
    assert_eq!(e_cell.glyph_id, b'e' as u32);
    assert!(e_cell.attrs.contains(CellAttrs::BOLD));
    
    // " World" should be reset to defaults - check the space character
    let space_cell = grid.get_cell(5, 0).unwrap();
    assert_eq!(space_cell.glyph_id, b' ' as u32);
    assert_eq!(space_cell.fg_color, Color::DEFAULT_FG);
    assert!(!space_cell.attrs.contains(CellAttrs::BOLD));
}

/// Test multiple SGR attributes
#[test]
fn test_multiple_sgr_attributes() {
    let mut parser = TerminalParser::new();
    let mut grid = TerminalGrid::new(20, 5);
    
    // Test bold + italic + underline - parse step by step
    let sequence_parts = [
        b"\x1b[1;3;4m".as_slice(), // Bold + italic + underline
        b"Formatted",              // Text to format
        b"\x1b[0m".as_slice(),     // Reset
    ];
    
    for part in sequence_parts.iter() {
        let actions = parser.parse(part);
        
        for action in actions {
            match action {
                ParseAction::Print(c) => {
                    let state = parser.state();
                    grid.apply_sgr(state.fg_color, state.bg_color, state.attrs);
                    grid.print_char(c);
                }
                ParseAction::CsiDispatch(CsiAction::Sgr(_)) => {
                    let state = parser.state();
                    grid.apply_sgr(state.fg_color, state.bg_color, state.attrs);
                }
                _ => {}
            }
        }
    }
    
    // Check that multiple attributes were applied
    let f_cell = grid.get_cell(0, 0).unwrap();
    assert!(f_cell.attrs.contains(CellAttrs::BOLD));
    assert!(f_cell.attrs.contains(CellAttrs::ITALIC));
    assert!(f_cell.attrs.contains(CellAttrs::UNDERLINE));
}

/// Test 256-color support
#[test]
fn test_256_color_support() {
    let mut parser = TerminalParser::new();
    let mut grid = TerminalGrid::new(20, 5);
    
    // Test 256-color foreground (color 196 = bright red)
    let sequence_parts = [
        b"\x1b[38;5;196m".as_slice(), // 256-color red
        b"Red",                       // Text
        b"\x1b[0m".as_slice(),        // Reset
    ];
    
    for part in sequence_parts.iter() {
        let actions = parser.parse(part);
        
        for action in actions {
            match action {
                ParseAction::Print(c) => {
                    let state = parser.state();
                    grid.apply_sgr(state.fg_color, state.bg_color, state.attrs);
                    grid.print_char(c);
                }
                ParseAction::CsiDispatch(CsiAction::Sgr(_)) => {
                    let state = parser.state();
                    grid.apply_sgr(state.fg_color, state.bg_color, state.attrs);
                }
                _ => {}
            }
        }
    }
    
    // Verify color was applied
    let r_cell = grid.get_cell(0, 0).unwrap();
    assert_eq!(r_cell.glyph_id, b'R' as u32);
    // 256-color 196 should be a bright red color
    assert!(r_cell.fg_color.r > 200); // Should be quite red
}

/// Test RGB color support
#[test]
fn test_rgb_color_support() {
    let mut parser = TerminalParser::new();
    let mut grid = TerminalGrid::new(20, 5);
    
    // Test RGB color (bright orange)
    let sequence_parts = [
        b"\x1b[38;2;255;165;0m".as_slice(), // RGB orange
        b"Orange",                          // Text
        b"\x1b[0m".as_slice(),              // Reset
    ];
    
    for part in sequence_parts.iter() {
        let actions = parser.parse(part);
        
        for action in actions {
            match action {
                ParseAction::Print(c) => {
                    let state = parser.state();
                    grid.apply_sgr(state.fg_color, state.bg_color, state.attrs);
                    grid.print_char(c);
                }
                ParseAction::CsiDispatch(CsiAction::Sgr(_)) => {
                    let state = parser.state();
                    grid.apply_sgr(state.fg_color, state.bg_color, state.attrs);
                }
                _ => {}
            }
        }
    }
    
    // Verify RGB color was applied correctly
    let o_cell = grid.get_cell(0, 0).unwrap();
    assert_eq!(o_cell.glyph_id, b'O' as u32);
    assert_eq!(o_cell.fg_color, Color::rgb(255, 165, 0));
}

/// Test control character handling
#[test]
fn test_control_character_integration() {
    let mut parser = TerminalParser::new();
    let mut grid = TerminalGrid::new(10, 5);
    
    // Test simple case first - just text with newline
    let test_data = b"ABC\nDEF";
    let actions = parser.parse(test_data);
    
    println!("Actions: {:?}", actions);
    
    for action in actions {
        match action {
            ParseAction::Print(c) => {
                println!("Printing '{}' at {:?}", c, grid.cursor_position());
                grid.print_char(c);
            }
            ParseAction::Execute(byte) => {
                println!("Executing control byte: {:#x} at {:?}", byte, grid.cursor_position());
                grid.execute_control(byte);
                println!("Cursor after control: {:?}", grid.cursor_position());
            }
            _ => {}
        }
    }
    
    // Debug: print the entire grid content
    for row in 0..3 {
        print!("Row {}: ", row);
        for col in 0..10 {
            let cell = grid.get_cell(col, row).unwrap();
            if cell.glyph_id == 0 {
                print!("_");
            } else {
                print!("{}", (cell.glyph_id as u8) as char);
            }
        }
        println!();
    }
    
    // Verify simple case first
    let a_cell = grid.get_cell(0, 0).unwrap();
    assert_eq!(a_cell.glyph_id, b'A' as u32, "Expected 'A' at (0,0), got: {}", a_cell.glyph_id);
    
    let d_cell = grid.get_cell(0, 1).unwrap();
    assert_eq!(d_cell.glyph_id, b'D' as u32, "Expected 'D' at (0,1), got: {}", d_cell.glyph_id);
}

/// Test a comprehensive terminal session simulation
#[test]
fn test_terminal_session_simulation() {
    let mut parser = TerminalParser::new();
    let mut grid = TerminalGrid::new(40, 10);
    
    // Simulate a colorful terminal session step by step
    let session_data: &[&[u8]] = &[
        b"\x1b[1;32m".as_slice(),              // Bold green
        b"$ ".as_slice(),                      // Prompt
        b"\x1b[0m".as_slice(),                 // Reset
        b"\x1b[33m".as_slice(),                // Yellow
        b"ls -la".as_slice(),                  // Command
        b"\x1b[0m".as_slice(),                 // Reset
        b"\r\n".as_slice(),                    // Newline
        b"\x1b[34m".as_slice(),                // Blue
        b"total 8".as_slice(),                 // Output
        b"\x1b[0m".as_slice(),                 // Reset
        b"\r\n".as_slice(),                    // Newline
        b"\x1b[1;31m".as_slice(),              // Bold red
        b"-rw-r--r--".as_slice(),              // Permissions
        b"\x1b[0m".as_slice(),                 // Reset
        b" file.txt\r\n".as_slice(),           // Filename and newline
        b"\x1b[1;32m".as_slice(),              // Bold green
        b"$ ".as_slice(),                      // Another prompt
        b"\x1b[0m".as_slice(),                 // Reset
    ];
    
    for part in session_data.iter() {
        let actions = parser.parse(part);
        
        for action in actions {
            match action {
                ParseAction::Print(c) => {
                    let state = parser.state();
                    grid.apply_sgr(state.fg_color, state.bg_color, state.attrs);
                    grid.print_char(c);
                }
                ParseAction::Execute(byte) => {
                    grid.execute_control(byte);
                }
                ParseAction::CsiDispatch(CsiAction::Sgr(_)) => {
                    let state = parser.state();
                    grid.apply_sgr(state.fg_color, state.bg_color, state.attrs);
                }
                _ => {}
            }
        }
    }
    
    // Verify the session was processed correctly
    // Should have at least 4 lines of content
    let viewport = grid.get_viewport();
    assert!(!viewport[0].iter().all(|cell| cell.is_empty()));
    assert!(!viewport[1].iter().all(|cell| cell.is_empty()));
    assert!(!viewport[2].iter().all(|cell| cell.is_empty()));
    assert!(!viewport[3].iter().all(|cell| cell.is_empty()));
    
    // Check that some formatting was applied - look for the first dollar sign
    let dollar_cell = grid.get_cell(0, 0).unwrap();
    assert_eq!(dollar_cell.glyph_id, b'$' as u32);
    // Dollar sign should be bold green from the prompt
    assert!(dollar_cell.attrs.contains(CellAttrs::BOLD), 
            "First $ should be bold, got attrs: {:?}", dollar_cell.attrs);
}