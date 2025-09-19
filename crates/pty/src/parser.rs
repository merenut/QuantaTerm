//! Terminal escape sequence parser
//!
//! This module provides VTE-based parsing for terminal escape sequences,
//! with a focus on SGR (Select Graphic Rendition) codes.

use quantaterm_blocks::{Color, CellAttrs};
use tracing::{debug, trace};
use vte::{Params, Perform};

/// Actions that can be taken as a result of parsing escape sequences
#[derive(Debug, Clone)]
pub enum ParseAction {
    /// Print a character to the terminal
    Print(char),
    /// Execute a control character
    Execute(u8),
    /// Perform a CSI (Control Sequence Introducer) action
    CsiDispatch(CsiAction),
    /// Perform an escape sequence action
    EscDispatch(EscAction),
    /// OSC (Operating System Command) action
    OscDispatch(Vec<Vec<u8>>),
}

/// CSI sequence actions
#[derive(Debug, Clone)]
pub enum CsiAction {
    /// SGR (Select Graphic Rendition) - formatting attributes
    Sgr(Vec<u16>),
    /// Cursor movement and other CSI commands
    Other {
        /// The final byte of the CSI sequence
        command: char,
        /// Parameters for the command
        params: Vec<u16>,
    },
}

/// ESC sequence actions
#[derive(Debug, Clone)]
pub enum EscAction {
    /// Reset terminal state
    Reset,
    /// Other escape sequences
    Other(char),
}

/// Parser state for tracking current formatting attributes
#[derive(Debug, Clone)]
pub struct ParserState {
    /// Current foreground color
    pub fg_color: Color,
    /// Current background color
    pub bg_color: Color,
    /// Current cell attributes
    pub attrs: CellAttrs,
}

impl Default for ParserState {
    fn default() -> Self {
        Self {
            fg_color: Color::DEFAULT_FG,
            bg_color: Color::DEFAULT_BG,
            attrs: CellAttrs::empty(),
        }
    }
}

impl ParserState {
    /// Reset all formatting to defaults
    pub fn reset(&mut self) {
        self.fg_color = Color::DEFAULT_FG;
        self.bg_color = Color::DEFAULT_BG;
        self.attrs = CellAttrs::empty();
    }

    /// Apply SGR parameters to update the current state
    pub fn apply_sgr(&mut self, params: &[u16]) {
        let mut i = 0;
        while i < params.len() {
            match params[i] {
                // Reset/default
                0 => self.reset(),
                
                // Bold
                1 => self.attrs |= CellAttrs::BOLD,
                
                // Dim (interpreted as bold off)
                2 => self.attrs &= !CellAttrs::BOLD,
                
                // Italic
                3 => self.attrs |= CellAttrs::ITALIC,
                
                // Underline
                4 => self.attrs |= CellAttrs::UNDERLINE,
                
                // Blink
                5 => self.attrs |= CellAttrs::BLINK,
                
                // Reverse
                7 => self.attrs |= CellAttrs::REVERSE,
                
                // Strikethrough
                9 => self.attrs |= CellAttrs::STRIKETHROUGH,
                
                // Bold off
                22 => self.attrs &= !CellAttrs::BOLD,
                
                // Italic off
                23 => self.attrs &= !CellAttrs::ITALIC,
                
                // Underline off
                24 => self.attrs &= !CellAttrs::UNDERLINE,
                
                // Blink off
                25 => self.attrs &= !CellAttrs::BLINK,
                
                // Reverse off
                27 => self.attrs &= !CellAttrs::REVERSE,
                
                // Strikethrough off
                29 => self.attrs &= !CellAttrs::STRIKETHROUGH,
                
                // Standard foreground colors (30-37)
                30..=37 => {
                    self.fg_color = standard_color(params[i] - 30);
                }
                
                // Extended foreground color
                38 => {
                    if let Some(color) = parse_extended_color(&params[i..]) {
                        self.fg_color = color.0;
                        i += color.1; // Skip consumed parameters
                    }
                }
                
                // Default foreground
                39 => self.fg_color = Color::DEFAULT_FG,
                
                // Standard background colors (40-47)
                40..=47 => {
                    self.bg_color = standard_color(params[i] - 40);
                }
                
                // Extended background color
                48 => {
                    if let Some(color) = parse_extended_color(&params[i..]) {
                        self.bg_color = color.0;
                        i += color.1; // Skip consumed parameters
                    }
                }
                
                // Default background
                49 => self.bg_color = Color::DEFAULT_BG,
                
                // Bright foreground colors (90-97)
                90..=97 => {
                    self.fg_color = bright_color(params[i] - 90);
                }
                
                // Bright background colors (100-107)
                100..=107 => {
                    self.bg_color = bright_color(params[i] - 100);
                }
                
                // Unknown parameter
                n => {
                    debug!("Unknown SGR parameter: {}", n);
                }
            }
            i += 1;
        }
    }
}

/// VTE-based terminal parser
pub struct TerminalParser {
    /// VTE parser instance
    parser: vte::Parser,
    /// Current parser state
    state: ParserState,
}

/// Parser performer that collects actions
struct ParsePerformer {
    /// Current parser state
    state: ParserState,
    /// Buffer for collected actions
    actions: Vec<ParseAction>,
}

impl ParsePerformer {
    fn new(state: ParserState) -> Self {
        Self {
            state,
            actions: Vec::new(),
        }
    }
}

impl TerminalParser {
    /// Create a new terminal parser
    pub fn new() -> Self {
        Self {
            parser: vte::Parser::new(),
            state: ParserState::default(),
        }
    }

    /// Parse a chunk of data and return the resulting actions
    pub fn parse(&mut self, data: &[u8]) -> Vec<ParseAction> {
        let mut performer = ParsePerformer::new(self.state.clone());
        
        self.parser.advance(&mut performer, data);
        
        // Update our state from the performer
        self.state = performer.state;
        
        performer.actions
    }

    /// Get the current parser state
    pub fn state(&self) -> &ParserState {
        &self.state
    }

    /// Reset the parser state
    pub fn reset(&mut self) {
        self.state.reset();
    }
}

impl Default for TerminalParser {
    fn default() -> Self {
        Self::new()
    }
}

impl Perform for ParsePerformer {
    fn print(&mut self, c: char) {
        trace!("Parser: print '{}'", c);
        self.actions.push(ParseAction::Print(c));
    }

    fn execute(&mut self, byte: u8) {
        trace!("Parser: execute {:#x}", byte);
        self.actions.push(ParseAction::Execute(byte));
    }

    fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _c: char) {
        // DCS sequences - not implemented yet
    }

    fn put(&mut self, _byte: u8) {
        // DCS data - not implemented yet
    }

    fn unhook(&mut self) {
        // End of DCS - not implemented yet
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        trace!("Parser: OSC dispatch with {} params", params.len());
        let params_vec: Vec<Vec<u8>> = params.iter().map(|p| p.to_vec()).collect();
        self.actions.push(ParseAction::OscDispatch(params_vec));
    }

    fn csi_dispatch(&mut self, params: &Params, _intermediates: &[u8], _ignore: bool, c: char) {
        trace!("Parser: CSI dispatch '{}'", c);
        
        let params_vec: Vec<u16> = params.iter().map(|p| p[0]).collect();
        
        match c {
            'm' => {
                // SGR - Select Graphic Rendition
                self.state.apply_sgr(&params_vec);
                self.actions.push(ParseAction::CsiDispatch(CsiAction::Sgr(params_vec)));
            }
            _ => {
                // Other CSI commands
                self.actions.push(ParseAction::CsiDispatch(CsiAction::Other {
                    command: c,
                    params: params_vec,
                }));
            }
        }
    }

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, byte: u8) {
        trace!("Parser: ESC dispatch {:#x}", byte);
        
        match byte {
            b'c' => {
                // RIS - Reset to Initial State
                self.state.reset();
                self.actions.push(ParseAction::EscDispatch(EscAction::Reset));
            }
            _ => {
                self.actions.push(ParseAction::EscDispatch(EscAction::Other(byte as char)));
            }
        }
    }
}

/// Convert a standard color index (0-7) to a Color
fn standard_color(index: u16) -> Color {
    match index {
        0 => Color::rgb(0, 0, 0),       // Black
        1 => Color::rgb(128, 0, 0),     // Red
        2 => Color::rgb(0, 128, 0),     // Green
        3 => Color::rgb(128, 128, 0),   // Yellow
        4 => Color::rgb(0, 0, 128),     // Blue
        5 => Color::rgb(128, 0, 128),   // Magenta
        6 => Color::rgb(0, 128, 128),   // Cyan
        7 => Color::rgb(192, 192, 192), // White
        _ => Color::DEFAULT_FG,
    }
}

/// Convert a bright color index (0-7) to a Color
fn bright_color(index: u16) -> Color {
    match index {
        0 => Color::rgb(128, 128, 128), // Bright Black (Gray)
        1 => Color::rgb(255, 0, 0),     // Bright Red
        2 => Color::rgb(0, 255, 0),     // Bright Green
        3 => Color::rgb(255, 255, 0),   // Bright Yellow
        4 => Color::rgb(0, 0, 255),     // Bright Blue
        5 => Color::rgb(255, 0, 255),   // Bright Magenta
        6 => Color::rgb(0, 255, 255),   // Bright Cyan
        7 => Color::rgb(255, 255, 255), // Bright White
        _ => Color::DEFAULT_FG,
    }
}

/// Parse extended color sequences (256-color or RGB)
/// Returns (Color, consumed_params_count) or None if invalid
fn parse_extended_color(params: &[u16]) -> Option<(Color, usize)> {
    if params.len() < 2 {
        return None;
    }
    
    match params[1] {
        // 256-color mode
        5 => {
            if params.len() < 3 {
                return None;
            }
            let color_index = params[2];
            Some((color_256(color_index), 2))
        }
        
        // RGB mode
        2 => {
            if params.len() < 5 {
                return None;
            }
            let r = (params[2] as u8).min(255);
            let g = (params[3] as u8).min(255);
            let b = (params[4] as u8).min(255);
            Some((Color::rgb(r, g, b), 4))
        }
        
        _ => None,
    }
}

/// Convert a 256-color palette index to a Color
fn color_256(index: u16) -> Color {
    match index {
        // Standard colors (0-15)
        0..=7 => standard_color(index),
        8..=15 => bright_color(index - 8),
        
        // 216 color cube (16-231)
        16..=231 => {
            let index = index - 16;
            let r = (index / 36) * 51;
            let g = ((index % 36) / 6) * 51;
            let b = (index % 6) * 51;
            Color::rgb(r as u8, g as u8, b as u8)
        }
        
        // Grayscale ramp (232-255)
        232..=255 => {
            let gray = 8 + (index - 232) * 10;
            Color::rgb(gray as u8, gray as u8, gray as u8)
        }
        
        _ => Color::DEFAULT_FG,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_creation() {
        let parser = TerminalParser::new();
        assert_eq!(parser.state.fg_color, Color::DEFAULT_FG);
        assert_eq!(parser.state.bg_color, Color::DEFAULT_BG);
        assert!(parser.state.attrs.is_empty());
    }

    #[test]
    fn test_simple_text_parsing() {
        let mut parser = TerminalParser::new();
        let actions = parser.parse(b"Hello");
        
        assert_eq!(actions.len(), 5);
        for (i, action) in actions.iter().enumerate() {
            match action {
                ParseAction::Print(c) => {
                    assert_eq!(*c, "Hello".chars().nth(i).unwrap());
                }
                _ => panic!("Expected Print action"),
            }
        }
    }

    #[test]
    fn test_sgr_bold() {
        let mut parser = TerminalParser::new();
        let actions = parser.parse(b"\x1b[1mBold");
        
        // Should have SGR action followed by Print actions
        let sgr_found = actions.iter().any(|action| {
            matches!(action, ParseAction::CsiDispatch(CsiAction::Sgr(params)) if params == &[1])
        });
        assert!(sgr_found);
        assert!(parser.state.attrs.contains(CellAttrs::BOLD));
    }

    #[test]
    fn test_sgr_color() {
        let mut parser = TerminalParser::new();
        let actions = parser.parse(b"\x1b[31m"); // Red foreground
        
        let sgr_found = actions.iter().any(|action| {
            matches!(action, ParseAction::CsiDispatch(CsiAction::Sgr(params)) if params == &[31])
        });
        assert!(sgr_found);
        assert_eq!(parser.state.fg_color, standard_color(1)); // Red
    }

    #[test]
    fn test_sgr_reset() {
        let mut parser = TerminalParser::new();
        // Set bold and color
        parser.parse(b"\x1b[1;31m");
        assert!(parser.state.attrs.contains(CellAttrs::BOLD));
        
        // Reset
        parser.parse(b"\x1b[0m");
        assert!(!parser.state.attrs.contains(CellAttrs::BOLD));
        assert_eq!(parser.state.fg_color, Color::DEFAULT_FG);
    }

    #[test]
    fn test_color_256() {
        // Test standard colors
        assert_eq!(color_256(1), standard_color(1));
        assert_eq!(color_256(9), bright_color(1));
        
        // Test color cube
        let cube_color = color_256(16); // First color cube entry
        assert_eq!(cube_color, Color::rgb(0, 0, 0));
        
        // Test grayscale
        let gray_color = color_256(232); // First grayscale entry
        assert_eq!(gray_color, Color::rgb(8, 8, 8));
    }

    #[test]
    fn test_extended_color_parsing() {
        // Test 256-color mode
        let result = parse_extended_color(&[38, 5, 196]); // Bright red
        assert!(result.is_some());
        let (_color, consumed) = result.unwrap();
        assert_eq!(consumed, 2);
        
        // Test RGB mode
        let result = parse_extended_color(&[38, 2, 255, 128, 64]);
        assert!(result.is_some());
        let (color, consumed) = result.unwrap();
        assert_eq!(color, Color::rgb(255, 128, 64));
        assert_eq!(consumed, 4);
    }
}