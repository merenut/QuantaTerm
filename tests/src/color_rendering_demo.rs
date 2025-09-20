//! Color and attribute rendering demonstration
//!
//! This module provides tests that demonstrate the enhanced renderer's ability
//! to handle color and text attributes for 16/256/truecolor modes.

use quantaterm_blocks::{CellAttrs, Color, TerminalGrid};
use quantaterm_pty::{CsiAction, ParseAction, TerminalParser};

/// Integration test demonstrating full color rendering pipeline
#[test]
fn test_color_rendering_integration() {
    let mut parser = TerminalParser::new();
    let mut grid = TerminalGrid::new(20, 5);

    // Test various SGR sequences that should be rendered with colors
    let test_sequences: Vec<(&[u8], &str)> = vec![
        // Basic colors
        (b"\x1b[31mRed\x1b[0m", "Red text in standard red"),
        (b"\x1b[92mBright Green\x1b[0m", "Green text in bright green"),
        
        // 256-color mode
        (b"\x1b[38;5;196mBright Red 256\x1b[0m", "256-color bright red"),
        (b"\x1b[38;5;21mDeep Blue 256\x1b[0m", "256-color deep blue"),
        
        // Truecolor (RGB)
        (b"\x1b[38;2;255;165;0mOrange RGB\x1b[0m", "RGB orange"),
        (b"\x1b[38;2;75;0;130mIndigo RGB\x1b[0m", "RGB indigo"),
        
        // Attributes
        (b"\x1b[1mBold\x1b[0m", "Bold text"),
        (b"\x1b[3mItalic\x1b[0m", "Italic text"),
        (b"\x1b[4mUnderline\x1b[0m", "Underlined text"),
        
        // Combined color and attributes
        (b"\x1b[1;31mBold Red\x1b[0m", "Bold red text"),
        (b"\x1b[3;34mItalic Blue\x1b[0m", "Italic blue text"),
    ];

    for (sequence, description) in test_sequences.iter() {
        println!("Testing: {}", description);
        
        // Parse the sequence
        let actions = parser.parse(sequence);
        
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
        
        // Move to next line for next test
        grid.newline();
    }

    // Verify the grid contains formatted content
    let viewport = grid.get_viewport();
    assert!(!viewport.is_empty());
    
    // Check that we have actual content with various colors
    let mut has_colored_cells = false;
    let mut has_bold_cells = false;
    let mut has_italic_cells = false;
    
    for row in viewport.iter() {
        for cell in row.iter() {
            // Check for non-default colors
            if cell.fg_color != Color::DEFAULT_FG {
                has_colored_cells = true;
            }
            
            // Check for text attributes
            if cell.attrs.contains(CellAttrs::BOLD) {
                has_bold_cells = true;
            }
            if cell.attrs.contains(CellAttrs::ITALIC) {
                has_italic_cells = true;
            }
        }
    }
    
    assert!(has_colored_cells, "Should have cells with custom colors");
    assert!(has_bold_cells, "Should have cells with bold attribute");
    assert!(has_italic_cells, "Should have cells with italic attribute");
    
    println!("✓ Color rendering integration test passed!");
}

/// Test that demonstrates renderer can store and retrieve color information
#[test]
fn test_renderer_color_storage() {
    // Test basic color and attribute combinations
    let mut grid = TerminalGrid::new(10, 5);
    
    // Apply red bold formatting and add a character
    grid.apply_sgr(Color::rgb(255, 0, 0), Color::DEFAULT_BG, CellAttrs::BOLD);
    grid.print_char('R');
    
    // Apply green italic formatting and add a character  
    grid.apply_sgr(Color::rgb(0, 255, 0), Color::DEFAULT_BG, CellAttrs::ITALIC);
    grid.print_char('G');
    
    // Apply blue underlined formatting with custom background
    grid.apply_sgr(Color::rgb(0, 0, 255), Color::rgb(128, 128, 128), CellAttrs::UNDERLINE);
    grid.print_char('B');
    
    // Verify the cells have the correct attributes
    let cell_r = grid.get_cell(0, 0).unwrap();
    let cell_g = grid.get_cell(1, 0).unwrap();
    let cell_b = grid.get_cell(2, 0).unwrap();
    
    assert_eq!(cell_r.fg_color, Color::rgb(255, 0, 0));
    assert_eq!(cell_r.attrs, CellAttrs::BOLD);
    
    assert_eq!(cell_g.fg_color, Color::rgb(0, 255, 0));
    assert_eq!(cell_g.attrs, CellAttrs::ITALIC);
    
    assert_eq!(cell_b.fg_color, Color::rgb(0, 0, 255));
    assert_eq!(cell_b.bg_color, Color::rgb(128, 128, 128));
    assert_eq!(cell_b.attrs, CellAttrs::UNDERLINE);
    
    println!("✓ Renderer color storage test passed!");
}

/// Test 256-color palette rendering capability
#[test]
fn test_256_color_rendering() {
    let mut parser = TerminalParser::new();
    let mut grid = TerminalGrid::new(30, 10);

    // Test a range of 256-color palette entries
    let test_colors = [
        16,  // First color in 6x6x6 cube
        21,  // Blue
        46,  // Green
        196, // Red
        226, // Yellow
        231, // White-ish
        232, // First grayscale
        255, // Brightest grayscale
    ];

    for (i, color_index) in test_colors.iter().enumerate() {
        let sequence = format!("\x1b[38;5;{}m{}\x1b[0m", color_index, color_index);
        
        let actions = parser.parse(sequence.as_bytes());
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
        
        if i < test_colors.len() - 1 {
            grid.print_char(' ');
        }
    }

    // Verify that different colors were applied
    let viewport = grid.get_viewport();
    let first_row = &viewport[0];
    
    let mut unique_colors = std::collections::HashSet::new();
    for cell in first_row.iter() {
        if cell.glyph_id != b' ' as u32 && cell.glyph_id != 0 {
            unique_colors.insert((cell.fg_color.r, cell.fg_color.g, cell.fg_color.b));
        }
    }
    
    // Should have multiple unique colors
    assert!(unique_colors.len() > 1, "Should have multiple unique colors from 256-color palette");
    
    println!("✓ 256-color rendering test passed with {} unique colors!", unique_colors.len());
}

/// Test truecolor (24-bit RGB) rendering capability  
#[test]
fn test_truecolor_rendering() {
    let mut parser = TerminalParser::new();
    let mut grid = TerminalGrid::new(20, 5);

    // Test specific RGB colors
    let rgb_colors = [
        (255, 165, 0),   // Orange
        (75, 0, 130),    // Indigo
        (238, 130, 238), // Violet
        (255, 20, 147),  // Deep pink
        (0, 191, 255),   // Deep sky blue
    ];

    for (r, g, b) in rgb_colors.iter() {
        let sequence = format!("\x1b[38;2;{};{};{}m█\x1b[0m", r, g, b);
        
        let actions = parser.parse(sequence.as_bytes());
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

    // Verify that the exact RGB colors were preserved
    let viewport = grid.get_viewport();
    let first_row = &viewport[0];
    
    let mut color_matches = 0;
    let mut cell_index = 0;
    
    for cell in first_row.iter() {
        if cell.glyph_id == '█' as u32 && cell_index < rgb_colors.len() {
            let expected = rgb_colors[cell_index];
            assert_eq!(cell.fg_color.r, expected.0);
            assert_eq!(cell.fg_color.g, expected.1);
            assert_eq!(cell.fg_color.b, expected.2);
            color_matches += 1;
            cell_index += 1;
        }
    }
    
    assert_eq!(color_matches, rgb_colors.len(), "All RGB colors should be preserved exactly");
    
    println!("✓ Truecolor rendering test passed with {} exact RGB matches!", color_matches);
}

/// Comprehensive rendering demonstration
#[test] 
fn test_comprehensive_rendering_demo() {
    let mut parser = TerminalParser::new();
    let mut grid = TerminalGrid::new(80, 24);

    println!("\n=== Comprehensive Color and Attribute Rendering Demo ===");
    
    // Test all basic attributes
    let attr_tests: Vec<(&[u8], &str)> = vec![
        (b"\x1b[1mBold\x1b[0m", "Bold text"),
        (b"\x1b[3mItalic\x1b[0m", "Italic text"), 
        (b"\x1b[4mUnderline\x1b[0m", "Underlined text"),
        (b"\x1b[9mStrikethrough\x1b[0m", "Strikethrough text"),
        (b"\x1b[7mReverse\x1b[0m", "Reverse video text"),
    ];
    
    for (sequence, description) in attr_tests.iter() {
        println!("Testing: {}", description);
        
        let actions = parser.parse(sequence);
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
        grid.newline();
    }

    // Test color combinations
    let color_tests: Vec<(&[u8], &str)> = vec![
        (b"\x1b[31;42mRed on Green\x1b[0m", "Foreground/background combo"),
        (b"\x1b[1;33;44mBold Yellow on Blue\x1b[0m", "Bold with colors"),
        (b"\x1b[3;35;46mItalic Magenta on Cyan\x1b[0m", "Italic with colors"),
    ];
    
    for (sequence, description) in color_tests.iter() {
        println!("Testing: {}", description);
        
        let actions = parser.parse(sequence);
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
        grid.newline();
    }

    // Verify comprehensive attribute support
    let viewport = grid.get_viewport();
    let mut attribute_counts = std::collections::HashMap::new();
    let mut color_counts = std::collections::HashMap::new();
    
    for row in viewport.iter() {
        for cell in row.iter() {
            // Count attributes
            if cell.attrs.contains(CellAttrs::BOLD) {
                *attribute_counts.entry("bold").or_insert(0) += 1;
            }
            if cell.attrs.contains(CellAttrs::ITALIC) {
                *attribute_counts.entry("italic").or_insert(0) += 1;
            }
            if cell.attrs.contains(CellAttrs::UNDERLINE) {
                *attribute_counts.entry("underline").or_insert(0) += 1;
            }
            
            // Count unique colors
            let color_key = (cell.fg_color.r, cell.fg_color.g, cell.fg_color.b);
            *color_counts.entry(color_key).or_insert(0) += 1;
        }
    }
    
    println!("Attribute usage: {:?}", attribute_counts);
    println!("Unique colors found: {}", color_counts.len());
    
    // Verify we have good coverage
    assert!(attribute_counts.contains_key("bold"), "Should have bold attributes");
    assert!(attribute_counts.contains_key("italic"), "Should have italic attributes");
    assert!(attribute_counts.contains_key("underline"), "Should have underline attributes");
    assert!(color_counts.len() > 5, "Should have multiple unique colors");
    
    println!("✓ Comprehensive rendering demo passed!");
}