//! Simple color rendering test to debug the issue

use quantaterm_blocks::{Color, TerminalGrid};
use quantaterm_pty::{CsiAction, ParseAction, TerminalParser};

#[test]
fn test_simple_color_parsing() {
    let mut parser = TerminalParser::new();
    let mut grid = TerminalGrid::new(10, 5);

    println!("Testing simple red color parsing...");

    // Parse the SGR sequence separately
    let sgr_sequence = b"\x1b[31m";
    let actions = parser.parse(sgr_sequence);

    println!("SGR Actions parsed: {:?}", actions);

    for action in actions {
        if let ParseAction::CsiDispatch(CsiAction::Sgr(_)) = action {
            let state = parser.state();
            println!(
                "Parser state after SGR: fg_color: {:?}, bg_color: {:?}, attrs: {:?}",
                state.fg_color, state.bg_color, state.attrs
            );
            grid.apply_sgr(state.fg_color, state.bg_color, state.attrs);
        }
    }

    // Now parse and print the character separately
    let char_actions = parser.parse(b"R");
    for action in char_actions {
        if let ParseAction::Print(c) = action {
            println!("Printing character: {}", c);
            grid.print_char(c);
            break;
        }
    }

    // Check the resulting cell
    let cell = grid.get_cell(0, 0).unwrap();
    println!(
        "Resulting cell: glyph_id: {}, fg_color: {:?}, bg_color: {:?}, attrs: {:?}",
        cell.glyph_id, cell.fg_color, cell.bg_color, cell.attrs
    );

    // This should be red
    assert_ne!(
        cell.fg_color,
        Color::DEFAULT_FG,
        "Cell should have non-default foreground color"
    );
    assert_eq!(cell.fg_color, Color::rgb(128, 0, 0), "Cell should be red");
}
