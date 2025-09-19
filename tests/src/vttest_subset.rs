//! VTTest subset validation for SGR handling
//!
//! This module provides tests for a subset of VTTest functionality,
//! specifically focusing on SGR (Select Graphic Rendition) codes.

use quantaterm_pty::{TerminalParser, ParseAction, CsiAction};
use quantaterm_blocks::{TerminalGrid, Color, CellAttrs};

/// VTTest subset - basic SGR functionality
/// This covers the most common SGR codes that a terminal emulator should support
#[test]
fn vttest_sgr_basic_attributes() {
    let mut parser = TerminalParser::new();
    let mut grid = TerminalGrid::new(80, 24);
    
    let test_cases: &[(&[u8], CellAttrs)] = &[
        // Reset
        (b"\x1b[0m", CellAttrs::empty()),
        
        // Basic attributes
        (b"\x1b[1m", CellAttrs::BOLD),
        (b"\x1b[3m", CellAttrs::ITALIC),
        (b"\x1b[4m", CellAttrs::UNDERLINE),
        (b"\x1b[7m", CellAttrs::REVERSE),
        (b"\x1b[9m", CellAttrs::STRIKETHROUGH),
        
        // Attribute combinations
        (b"\x1b[1;4m", CellAttrs::BOLD | CellAttrs::UNDERLINE),
        (b"\x1b[1;3;4m", CellAttrs::BOLD | CellAttrs::ITALIC | CellAttrs::UNDERLINE),
    ];
    
    let mut passed = 0;
    let total = test_cases.len();
    
    for (i, (sequence, expected_attrs)) in test_cases.iter().enumerate() {
        // Reset parser state
        parser.reset();
        grid.reset_formatting();
        grid.set_cursor_position(0, i as u16);
        
        // Parse and apply the sequence
        let actions = parser.parse(sequence);
        for action in actions {
            if let ParseAction::CsiDispatch(CsiAction::Sgr(_)) = action {
                let state = parser.state();
                grid.apply_sgr(state.fg_color, state.bg_color, state.attrs);
            }
        }
        
        // Parse and print a test character
        let char_actions = parser.parse(b"T");
        for action in char_actions {
            if let ParseAction::Print(c) = action {
                grid.print_char(c);
                break; // Only test the first character
            }
        }
        
        // Check the result
        let cell = grid.get_cell(0, i as u16).unwrap();
        if cell.attrs == *expected_attrs {
            passed += 1;
            println!("✓ Test {}: SGR attributes correct", i + 1);
        } else {
            println!("✗ Test {}: Expected {:?}, got {:?}", i + 1, expected_attrs, cell.attrs);
        }
    }
    
    let percentage = (passed * 100) / total;
    println!("VTTest SGR Basic: {}/{} tests passed ({}%)", passed, total, percentage);
    
    // Require at least 90% pass rate
    assert!(percentage >= 90, "VTTest SGR Basic failed: {}% < 90%", percentage);
}

/// VTTest subset - color support
#[test]
fn vttest_sgr_colors() {
    let mut parser = TerminalParser::new();
    let mut grid = TerminalGrid::new(80, 24);
    
    let test_cases: &[(&[u8], Color)] = &[
        // Standard colors (foreground)
        (b"\x1b[30m", Color::rgb(0, 0, 0)),
        (b"\x1b[31m", Color::rgb(128, 0, 0)),
        (b"\x1b[32m", Color::rgb(0, 128, 0)),
        (b"\x1b[33m", Color::rgb(128, 128, 0)),
        (b"\x1b[34m", Color::rgb(0, 0, 128)),
        (b"\x1b[35m", Color::rgb(128, 0, 128)),
        (b"\x1b[36m", Color::rgb(0, 128, 128)),
        (b"\x1b[37m", Color::rgb(192, 192, 192)),
        
        // Bright colors (foreground)
        (b"\x1b[90m", Color::rgb(128, 128, 128)),
        (b"\x1b[91m", Color::rgb(255, 0, 0)),
        (b"\x1b[92m", Color::rgb(0, 255, 0)),
        (b"\x1b[93m", Color::rgb(255, 255, 0)),
        (b"\x1b[94m", Color::rgb(0, 0, 255)),
        (b"\x1b[95m", Color::rgb(255, 0, 255)),
        (b"\x1b[96m", Color::rgb(0, 255, 255)),
        (b"\x1b[97m", Color::rgb(255, 255, 255)),
    ];
    
    let mut passed = 0;
    let total = test_cases.len();
    
    for (i, (sequence, expected_color)) in test_cases.iter().enumerate() {
        // Reset parser state
        parser.reset();
        grid.reset_formatting();
        grid.set_cursor_position(0, i as u16);
        
        // Parse and apply the sequence
        let actions = parser.parse(sequence);
        for action in actions {
            if let ParseAction::CsiDispatch(CsiAction::Sgr(_)) = action {
                let state = parser.state();
                grid.apply_sgr(state.fg_color, state.bg_color, state.attrs);
            }
        }
        
        // Parse and print a test character
        let char_actions = parser.parse(b"T");
        for action in char_actions {
            if let ParseAction::Print(c) = action {
                grid.print_char(c);
                break; // Only test the first character
            }
        }
        
        // Check the result
        let cell = grid.get_cell(0, i as u16).unwrap();
        if cell.fg_color == *expected_color {
            passed += 1;
            println!("✓ Color test {}: Correct", i + 1);
        } else {
            println!("✗ Color test {}: Expected {:?}, got {:?}", i + 1, expected_color, cell.fg_color);
        }
    }
    
    let percentage = (passed * 100) / total;
    println!("VTTest SGR Colors: {}/{} tests passed ({}%)", passed, total, percentage);
    
    // Require at least 90% pass rate
    assert!(percentage >= 90, "VTTest SGR Colors failed: {}% < 90%", percentage);
}

/// Comprehensive VTTest subset runner
#[test]
fn vttest_comprehensive() {
    println!("Running VTTest subset for SGR handling...");
    
    // This will run the basic VTTest functions
    // Each one validates >= 90% pass rate individually
    
    vttest_sgr_basic_attributes();
    vttest_sgr_colors();
    
    println!("✓ All VTTest SGR subset tests passed with >= 90% success rate");
}