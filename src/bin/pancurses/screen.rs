//! Terminal I/O ... screen and keyboard input
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::Path,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use log::{debug, error, info, trace, warn};
use pancurses::{Input, Window};
use regex::Regex;
use zm::{
    config::Config,
    error::{ErrorCode, RuntimeError},
    recoverable_error,
    sound::Manager,
    zmachine::{InputEvent, Interrupt},
};

use crate::files;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Z-Machine colour constants
pub enum Colour {
    Black = 2,
    Red = 3,
    Green = 4,
    Yellow = 5,
    Blue = 6,
    Magenta = 7,
    Cyan = 8,
    White = 9,
}

/// Z-Machine text styles
pub enum Style {
    Roman = 0,
    Reverse = 1,
    Bold = 2,
    Italic = 4,
    Fixed = 8,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
/// Current text styling
pub struct CellStyle {
    mask: u8,
}

impl Default for CellStyle {
    fn default() -> Self {
        CellStyle::new()
    }
}

impl CellStyle {
    /// Constructor
    ///
    /// Defaults to unstyled (plain text)
    pub fn new() -> CellStyle {
        CellStyle { mask: 0 }
    }

    /// Additively sets a style.
    ///
    /// [Style::Roman] clears the style.
    ///
    /// # Arguments
    /// * `style` - Style mask bit to add
    pub fn set(&mut self, style: u8) {
        match style {
            0 => self.mask = 0,
            _ => self.mask |= style & 0xf,
        }
    }

    /// Removes a style
    ///
    /// # Arguments
    /// * `style` - Style mask bit to remove
    pub fn clear(&mut self, style: u8) {
        let mask = !(style & 0xF);
        self.mask &= mask;
    }

    /// Tests if the style has a style attribute
    ///
    /// # Arguments
    /// * `style` - Style to test
    ///
    /// # Returns
    /// `true` if the style includes the attribute, `false` if not
    pub fn is_style(&self, style: Style) -> bool {
        let s = style as u8;
        self.mask & s == s
    }
}

/// Map a colour number to a [Colour] enum value
///
/// # Arguments
/// * `colour` - Colour value
///
/// # Returns
/// [Result] containing the [Colour] enum or a [RuntimeError] if the `colour`` is invalid.
fn map_colour(colour: u8) -> Result<Colour, RuntimeError> {
    match colour {
        2 => Ok(Colour::Black),
        3 => Ok(Colour::Red),
        4 => Ok(Colour::Green),
        5 => Ok(Colour::Yellow),
        6 => Ok(Colour::Blue),
        7 => Ok(Colour::Magenta),
        8 => Ok(Colour::Cyan),
        9 => Ok(Colour::White),
        _ => recoverable_error!(ErrorCode::InvalidColor, "Invalid color {}", colour),
    }
}

/// Map a foreground and background colour to a Colour enum tuple
///
/// # Arguments
/// * `foreground` - foreground colour value
/// * `background` - background colour value
///
/// # Returns
/// [Result] containing a (foreground, background) tuple or a [RuntimeError] if either colour is invalid.
fn map_colours(foreground: u8, background: u8) -> Result<(Colour, Colour), RuntimeError> {
    Ok((map_colour(foreground)?, map_colour(background)?))
}

#[derive(Debug)]
/// Screen state
pub struct Screen {
    /// ZCode version
    version: u8,
    /// Window height
    rows: u32,
    /// Window width
    columns: u32,
    /// Top row - if there is a status line, this will be the _second_ line in the window
    top: u32,
    /// Top row of window 1 when split, will be equal to `top`
    window_1_top: Option<u32>,
    /// Bottom row of window 1 when split
    window_1_bottom: Option<u32>,
    /// Top of window 0, will be equal to `top` or `window_1_bottom` + 1
    window_0_top: u32,
    /// The currently selected window
    selected_window: u8,
    /// Text buffering and word wrapping
    buffer_mode: u16,
    /// Default (foreground, background) colours
    default_colors: (Colour, Colour),
    /// Current (foreground, background) colours
    current_colors: (Colour, Colour),
    /// Current text styling
    current_style: CellStyle,
    /// Current font
    font: u8,
    /// Window 0 cursor position (row, column) relative to the upper left corner of the window (1,1)
    cursor_0: (u32, u32),
    /// Window 0 cursor position (row, column) when split
    cursor_1: Option<(u32, u32)>,
    /// Pancurses Window
    window: Window,
    /// Lines printed since the last keyboard input, used to pause output
    lines_since_input: u32,
}

impl Screen {
    /// Constructor
    ///
    /// # Arguments
    /// * `version` - ZCode version
    /// * `config` - Reference to configuration
    pub fn new(version: u8, config: &Config) -> Result<Screen, RuntimeError> {
        info!(target: "app::screen", "Initialize pancurses terminal");
        let window = pancurses::initscr();
        pancurses::curs_set(0);
        pancurses::noecho();
        pancurses::cbreak();
        pancurses::start_color();
        pancurses::mousemask(pancurses::ALL_MOUSE_EVENTS, None);
        pancurses::set_title("mxyzptlk - a rusty z-machine interpreter");

        window.keypad(true);
        window.clear();
        window.refresh();

        // Initialize the curses fg/bg colour pairs
        for fg in 0..8 {
            for bg in 0..8 {
                pancurses::init_pair(cp(fg, bg), fg, bg);
            }
        }

        // Get window (height, width)
        let (y, x) = window.get_max_yx();
        let (rows, columns) = (y as u32, x as u32);

        // Map the default (foreground, background) colours
        let colors = map_colours(config.foreground(), config.background())?;

        // V3 has a status line, so drop the top row to the second line
        let top = if version == 3 { 2 } else { 1 };
        // Window 0 cursor position is (bottom, left) for V3, V4 and (top, left) for V5+
        let cursor_0 = if version > 4 { (1, 1) } else { (rows, 1) };
        Ok(Screen {
            version,
            rows,
            columns,
            top,
            window_0_top: top,
            window_1_top: None,
            window_1_bottom: None,
            selected_window: 0,
            buffer_mode: 1,
            default_colors: colors,
            current_colors: colors,
            current_style: CellStyle::new(),
            font: 1,
            cursor_0,
            cursor_1: None,
            window,
            lines_since_input: 0,
        })
    }

    /// Get the window height
    ///
    /// # Returns
    /// Window height in rows
    pub fn rows(&self) -> u32 {
        self.rows
    }

    /// Get the window width
    ///
    /// # Returns
    /// Window width in columns
    pub fn columns(&self) -> u32 {
        self.columns
    }

    /// Get the cursor position for the selected windows.
    ///
    /// The window origin is at the top left of the screen (1,1).
    ///
    /// # Returns
    /// Tuple (row, column) with the currently selected window's cursor position.
    pub fn cursor(&self) -> (u32, u32) {
        if self.selected_window == 0 {
            self.cursor_0
        } else {
            // unwrap() should be safe here because when selected_window
            // is 1, cursor_1 is Some
            self.cursor_1.unwrap()
        }
    }

    /// Move the cursor in the currently selected window.
    ///
    /// The cursor position is constrained to the window.  Moving the cursor out of bounds places
    /// it at the edge(s) of the screen.
    ///
    /// # Arguments
    /// * `row` - Row
    /// * `column` - Column
    pub fn move_cursor(&mut self, row: u32, column: u32) {
        // Constrain the column between 1 and the width of the screen
        let c = u32::max(1, u32::min(self.columns, column));
        if self.selected_window == 0 {
            // Constrain row between top of window 0 and the bottom of the screen
            let r = u32::max(self.window_0_top, u32::min(self.rows, row));
            self.cursor_0 = (r, c);
            self.window.mv(r as i32 - 1, c as i32 - 1);
        } else {
            // Constrain row between top of window 1 and bottom of window 1
            // unwrap() should be safe here because if window 1 is selected, then top/bottom are Some
            let r = u32::max(
                self.window_1_top.unwrap(),
                u32::min(self.window_1_bottom.unwrap(), row),
            );
            self.cursor_1 = Some((r, c));
            self.window.mv(r as i32 - 1, c as i32 - 1);
        }
    }

    /// Maps a Z-Machine colour to a Colour enum value.
    ///
    /// `colour` 0 is the current colour, 1 is the default colour.
    ///
    /// # Arguments
    /// * `colour` - colour value
    /// * `current` - the current colour
    /// * `dafault` - the default colour
    ///
    /// # Returns
    /// [Result] containing the Colour enum value, or a [RuntimeError] if the `colour` is invalid
    fn map_color(
        &self,
        color: u8,
        current: Colour,
        default: Colour,
    ) -> Result<Colour, RuntimeError> {
        match color {
            0 => Ok(current),
            1 => Ok(default),
            2 => Ok(Colour::Black),
            3 => Ok(Colour::Red),
            4 => Ok(Colour::Green),
            5 => Ok(Colour::Yellow),
            6 => Ok(Colour::Blue),
            7 => Ok(Colour::Magenta),
            8 => Ok(Colour::Cyan),
            9 => Ok(Colour::White),
            _ => recoverable_error!(ErrorCode::InvalidColor, "Invalid color {}", color),
        }
    }

    /// Maps a foreground and background Z-Machine colour values.
    ///
    /// # Arguments
    /// * `foreground` - foreground colour value
    /// * `background` - background colour value
    ///
    /// # Returns
    /// [Result] containing a tuple (foreground, background) with the Colour enum values,
    /// or a [RuntimeError] if the either colour value is invalid
    fn map_colors(&self, foreground: u8, background: u8) -> Result<(Colour, Colour), RuntimeError> {
        Ok((
            self.map_color(foreground, self.current_colors.0, self.default_colors.0)?,
            self.map_color(background, self.current_colors.1, self.default_colors.1)?,
        ))
    }

    /// Set the current text colours
    ///
    /// # Arguments
    /// * `foreground` - Foreground colour
    /// * `background` - Background colour
    ///
    /// # Returns
    /// Empty [Result] or a [RuntimeError] if either colour value is invalid
    pub fn set_colors(&mut self, foreground: u16, background: u16) -> Result<(), RuntimeError> {
        self.current_colors = self.map_colors(foreground as u8, background as u8)?;
        let cp = cp(
            self.as_color(self.current_colors.0),
            self.as_color(self.current_colors.1),
        );
        self.window.color_set(cp);
        Ok(())
    }

    /// Split the window.
    ///
    /// Window 1 will occupy the screen from `top` down for the number of `lines` specified.
    ///
    /// # Arguments
    /// * `lines` - number of lines for window 1
    pub fn split_window(&mut self, lines: u32) {
        let bottom = self.top + lines - 1;
        self.window_1_top = Some(self.top);
        self.window_1_bottom = Some(bottom);
        self.cursor_1 = Some((1, 1));
        self.window_0_top = bottom + 1;
        if self.cursor_0.0 < self.window_0_top {
            self.cursor_0 = (self.window_0_top, self.cursor_0.1)
        }
    }

    /// Unsplit the window
    ///
    /// Removes window 1, but does not change the screen contents
    pub fn unsplit_window(&mut self) {
        self.window_0_top = self.top;
        self.window_1_top = None;
        self.window_1_bottom = None;
        self.cursor_1 = None;
        self.selected_window = 0;
    }

    /// Select a window
    ///
    /// # Arguments
    /// * `window` - Window to select - 0 or 1.
    ///
    /// # Returns
    /// Empty [Result] or a [RuntimeError] if an invalid window is selected, or if window
    /// 1 is selected but hasn't been split.
    pub fn select_window(&mut self, window: u8) -> Result<(), RuntimeError> {
        self.lines_since_input = 0;
        if window == 0 {
            self.selected_window = 0;
            Ok(())
        } else if self.cursor_1.is_some() {
            self.selected_window = 1;
            self.cursor_1 = Some((self.top, 1));
            Ok(())
        } else {
            recoverable_error!(ErrorCode::InvalidWindow, "Invalid window {}", window)
        }
    }

    /// Erase a window
    ///
    /// Valid `window` values:
    /// * 0 - Window 0
    /// * 1 - Window 1 (if split)
    /// * -1 - Unsplits the screen and then erases it
    /// * -2 - Erases the screen without unsplitting it
    ///
    /// # Arguments
    /// * `window` - Window to erase
    ///
    /// # Returns
    /// Empty [Result] or a [RuntimeError] if the `window` value is invalid.
    pub fn erase_window(&mut self, window: i8) -> Result<(), RuntimeError> {
        match window {
            0 => {
                for i in self.window_0_top..=self.rows {
                    for j in 1..=self.columns {
                        self.print_char_at(0x20, i, j, self.current_colors, &CellStyle::new(), 1);
                    }
                }
                self.cursor_0 = if self.version == 4 {
                    (self.rows, 1)
                } else {
                    (self.window_0_top, 1)
                };

                self.lines_since_input = 0;
                Ok(())
            }
            1 => {
                if let Some(start) = self.window_1_top {
                    if let Some(end) = self.window_1_bottom {
                        for i in start..=end {
                            for j in 1..=self.columns {
                                self.print_char_at(
                                    0x20,
                                    i,
                                    j,
                                    self.current_colors,
                                    &CellStyle::new(),
                                    1,
                                );
                            }
                        }
                        self.cursor_1 = Some((start, 1))
                    }
                }
                Ok(())
            }
            -1 => {
                self.window_1_top = None;
                self.window_1_bottom = None;
                self.cursor_1 = None;
                self.window_0_top = self.top;
                for i in self.window_0_top..=self.rows {
                    for j in 1..=self.columns {
                        self.print_char_at(0x20, i, j, self.current_colors, &CellStyle::new(), 1);
                    }
                }
                self.cursor_0 = if self.version == 4 {
                    (self.rows, 1)
                } else {
                    (self.window_0_top, 1)
                };
                self.selected_window = 0;
                self.lines_since_input = 0;
                Ok(())
            }
            -2 => {
                for i in 1..=self.rows {
                    for j in 1..=self.columns {
                        self.print_char_at(0x20, i, j, self.current_colors, &CellStyle::new(), 1);
                    }
                    if self.cursor_1.is_some() {
                        self.cursor_1 = Some((1, 1))
                    }
                    self.cursor_0 = if self.version == 4 {
                        (self.rows, 1)
                    } else {
                        (self.window_0_top, 1)
                    };
                }
                self.lines_since_input = 0;
                Ok(())
            }
            _ => recoverable_error!(
                ErrorCode::InvalidWindow,
                "ERASE_WINDOW invalid window {}",
                window
            ),
        }
    }

    /// Erase from the current cursor position to the end of the line
    pub fn erase_line(&mut self) {
        let (row, col) = if self.selected_window == 0 {
            self.cursor_0
        } else {
            // unwrap() should be safe here because when selected_window
            // is 1, cursor_1 is Some
            self.cursor_1.unwrap()
        };
        for i in col..self.columns {
            self.print_char_at(0x20, row, i, self.current_colors, &CellStyle::new(), 1);
        }
    }

    /// Advance the cursor to the next line.
    ///
    /// The function will scroll the screen when advancing past the bottom of the screen.  If a full screen
    /// has been printed since the last user input, a prompt is provided to allow the player to finish reading
    /// the screen before continuing.
    fn next_line(&mut self) {
        self.lines_since_input += 1;
        if self.cursor_0.0 == self.rows {
            self.window.mv(self.window_0_top as i32 - 1, 0);
            self.window.insdelln(-1);
            for i in 0..=self.columns {
                self.print_at(&[' ' as u16], (self.rows, i), &CellStyle::new());
            }
            self.window.refresh();
            self.cursor_0 = (self.rows, 1);
        } else {
            self.cursor_0 = (self.cursor_0.0 + 1, 1);
        }

        let l = self.rows - self.window_0_top;
        if self.lines_since_input >= l {
            let reverse = self.current_style.is_style(Style::Reverse);
            self.current_style.set(Style::Reverse as u8);
            self.print(&"[MORE]".chars().map(|c| c as u16).collect());
            if let Some(c) = self.key(true).zchar() {
                if c == 0xd {
                    self.lines_since_input = l - 1;
                } else {
                    self.lines_since_input = 0;
                }
            }
            self.cursor_0 = (self.rows, 1);
            self.current_style.clear(Style::Reverse as u8);
            self.print(&vec![0x20; 6]);
            if reverse {
                self.current_style.set(Style::Reverse as u8)
            }
            self.cursor_0 = (self.rows, 1);
        }
    }

    /// Advance the cursor to the next position.
    ///
    /// If the cursor is advance past the right edge of the current window, it is:
    /// * Moved to the next line, scrolling if necessary when window 0 is selected
    /// * Moved to the next line unless already at the bottom when window 1 is selected,
    /// in which case the cursor is left at the bottom right corner of the window
    fn advance_cursor(&mut self) {
        if self.selected_window == 0 {
            if self.cursor_0.1 == self.columns {
                // At the end of the row
                self.next_line();
            } else {
                // Just move the cursor to the right
                self.cursor_0 = (self.cursor_0.0, self.cursor_0.1 + 1)
            }
        } else {
            // unwrap() should be safe here because when selected_window
            // is 1, cursor_1 is Some
            if self.cursor_1.unwrap().1 == self.columns {
                // At the end of the row
                if self.cursor_1.unwrap().0 < self.window_1_bottom.unwrap() {
                    // Not at the bottom of the window yet, so move to the start of the next line
                    self.cursor_1 = Some((self.cursor_1.unwrap().0 + 1, 1))
                }
                // If at the bottom right of the window, leave the cursor in place
            } else {
                // Just move the cursor to the right
                self.cursor_1 = Some((self.cursor_1.unwrap().0, self.cursor_1.unwrap().1 + 1))
            }
        }
    }

    /// Print a string starting at the cursor position of the currently selected window.
    ///
    /// # Arguments
    /// * `str` - String to print.
    pub fn print_str(&mut self, str: &str) {
        let v: Vec<u16> = str.chars().map(|x| (x as u8) as u16).collect();
        self.print(&v);
    }

    /// Print a vector of characters starting at the cursor position of the currently selected window.
    ///
    /// If window 0 is selected and buffering is enabled, text will be wrapped to the next line if a word
    /// would not fit in the space remaining.
    ///
    /// # Arguments
    /// * `text` - Vector of characters to print
    pub fn print(&mut self, text: &Vec<u16>) {
        if self.selected_window == 0 && self.buffer_mode == 1 {
            let words = text.split_inclusive(|c| *c == 0x20);
            for word in words {
                if self.columns() - self.cursor().1 < word.len() as u32 {
                    self.new_line();
                }

                let w = word.to_vec();
                for c in w {
                    self.print_char(c);
                }
            }
        } else {
            for c in text {
                self.print_char(*c);
            }
        }

        self.window.refresh();
    }

    /// Print a character to the cursor position of the currentl selected window, then advance the cursor.
    ///
    /// # Arguments
    /// `zchar` - Character to print
    fn print_char(&mut self, zchar: u16) {
        if zchar == 0xd {
            self.new_line();
        } else if zchar != 0 {
            let (r, c) = if self.selected_window == 0 {
                self.cursor_0
            } else {
                self.cursor_1.unwrap()
            };

            let style = self.current_style;

            self.print_char_at(zchar, r, c, self.current_colors, &style, self.font);
            self.advance_cursor();
        }
    }

    /// Print a vector of characters to a specific position on the screen
    ///
    /// # Arguments
    /// * `text` - Array of characters to print
    /// * `at` - (row, column) cursor position to start printing at
    /// * `style` - Text style
    pub fn print_at(&mut self, text: &[u16], at: (u32, u32), style: &CellStyle) {
        for (i, c) in text.iter().enumerate() {
            self.print_char_at(
                *c,
                u32::min(self.rows, at.0),
                u32::min(self.columns, at.1 + i as u32),
                self.current_colors,
                style,
                self.font,
            );
        }
        self.window.refresh();
    }

    /// Print a character to a specific position on the screen.  The underlying pancurses cursor position is advanced.
    ///
    /// # Arguments
    /// * `zchar` - Character to print
    /// * `row` - Row offset of the position to print to
    /// * `column` - Column offset of the position to print to
    /// * `colors` - (foreground, background) colour tuple
    /// * `style` - Text style
    /// * `font` - Text font
    fn print_char_at(
        &mut self,
        zchar: u16,
        row: u32,
        column: u32,
        colors: (Colour, Colour),
        style: &CellStyle,
        font: u8,
    ) {
        let c = map_output(zchar, font);
        let cp = cp(self.as_color(colors.0), self.as_color(colors.1));
        let mut attributes = 0;
        if style.is_style(Style::Bold) {
            attributes |= pancurses::A_BOLD;
        }
        if style.is_style(Style::Italic) {
            if cfg!(target_os = "macos") {
                attributes |= pancurses::A_UNDERLINE;
            } else {
                attributes |= pancurses::A_ITALIC;
            }
        }
        if style.is_style(Style::Reverse) {
            attributes |= pancurses::A_REVERSE;
        }
        self.window.mv(row as i32 - 1, column as i32 - 1);
        self.window.addstr(format!("{}", c));
        self.window.mv(row as i32 - 1, column as i32 - 1);
        self.window.chgat(1, attributes, cp);
    }

    /// Print a new line, advance the cursor to the start of the next line.
    ///
    /// In window 0, this will scroll the screen if the cursor is on the bottom line of the screen.
    pub fn new_line(&mut self) {
        if self.selected_window == 0 {
            self.next_line();
        } else {
            // unwrap() should be safe here because when selected_window
            // is 1, cursor_1 is Some
            if self.cursor_1.unwrap().0 < self.window_1_bottom.unwrap() {
                self.cursor_1 = Some((self.cursor_1.unwrap().0 + 1, 1));
            }
        }
    }

    /// Poll for a keypress
    ///
    /// # Arguments
    /// * `wait` - When `true`, blocks until a key is returned
    ///
    /// # Returns
    /// The input event, which may be "no event" if `wait` is false.
    pub fn key(&mut self, wait: bool) -> InputEvent {
        self.lines_since_input = 0;
        if self.selected_window == 0 {
            self.window
                .mv(self.cursor_0.0 as i32 - 1, self.cursor_0.1 as i32 - 1);
        } else {
            // unwrap() should be safe here because when selected_window
            // is 1, cursor_1 is Some
            self.window.mv(
                self.cursor_1.unwrap().0 as i32 - 1,
                self.cursor_1.unwrap().1 as i32 - 1,
            );
        }

        if wait {
            self.window.nodelay(false);
        } else {
            self.window.nodelay(true);
        }
        pancurses::curs_set(1);
        pancurses::raw();

        if let Some(i) = self.window.getch() {
            pancurses::curs_set(0);
            self.input_to_u16(i)
        } else {
            InputEvent::no_input()
        }
    }

    /// Gets the current timestamp, offset by an optional timeout period
    ///
    /// # Arguments
    /// * `timeout` - Optional timeout in 10ths of a second.
    ///
    /// # Returns
    /// Current timestamp, offset by the timeout if provided.
    fn now(&self, timeout: Option<u16>) -> u128 {
        match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(t) => {
                if let Some(d) = timeout {
                    t.as_millis() + (d * 100) as u128
                } else {
                    t.as_millis()
                }
            }
            Err(e) => {
                error!(target: "app::state", "Error getting current system time: {}", e);
                0
            }
        }
    }

    /// Read a keypress
    ///
    /// # Arguments
    /// * `timeout` - Timeout in 10ths of a second, 0 for none
    ///
    /// # Returns
    /// [Result] containing the keypress event or timeout or a [RuntimeError]
    pub fn read_key(&mut self, timeout: u16) -> Result<InputEvent, RuntimeError> {
        let end = if timeout > 0 {
            self.now(Some(timeout))
        } else {
            0
        };

        loop {
            let now = self.now(None);
            if end > 0 && now > end {
                debug!(target: "app::screen", "Read interrupted: timed out");
                return Ok(InputEvent::from_interrupt(Interrupt::ReadTimeout));
            }

            let key = self.key(end == 0);
            if key.zchar().is_some() {
                return Ok(key);
            }

            thread::sleep(Duration::from_millis(10));
        }
    }

    /// Read a line of input.
    ///
    /// Normally, input is read until a terminator character is typed.  If a `timeout`
    /// is provided, or an end-of-sound routine is triggered, input may be interrupted.
    ///
    /// # Arguments
    /// * `text` - Existing input that can be added to or deleted
    /// * `len` - Maximum number of characters, including the terminator
    /// * `terminators` - Keys that terminate input
    /// * `timeout` - Timeout in 10ths of a second or 0 for none
    /// * `sound` - Sound manager.
    ///
    /// # Returns
    /// * [Result] containing a tuple (input buffer, terminator) or a [RuntimeError]
    pub fn read_line(
        &mut self,
        text: &[u16],
        len: usize,
        terminators: &[u16],
        timeout: u16,
        mut sound: Option<&mut Manager>,
    ) -> Result<(Vec<u16>, InputEvent), RuntimeError> {
        let mut input_buffer = text.to_vec();

        let end = if timeout > 0 {
            self.now(Some(timeout))
        } else {
            0
        };

        debug!(target: "app::screen", "Read until {}", end);
        let terminator;

        loop {
            let now = self.now(None);
            if end > 0 && now > end {
                debug!(target: "app::screen", "Read interrupted: timed out");
                return Ok((input_buffer, InputEvent::no_input()));
            }

            let (routine, playing) = if let Some(ref mut s) = sound.as_deref_mut() {
                (s.routine() > 0, s.is_playing())
            } else {
                (false, false)
            };

            if routine && !playing {
                debug!(target: "app::screen", "Read interrupted: sound finished playing");
                return Ok((input_buffer, InputEvent::no_input()));
            }

            let timeout = if end > 0 { end - now } else { 0 };
            trace!(target: "app::screen", "Now: {}, End: {}, Timeout: {}", now, end, timeout);

            let e = self.key(end == 0 && !routine);
            match e.zchar() {
                Some(key) => {
                    if terminators.contains(&key)
                        // Terminator 255 means "any function key"
                        || (terminators.contains(&255) && ((129..155).contains(&key) || key > 251))
                    {
                        terminator = e;
                        input_buffer.push(key);
                        // Only print the terminator if it was the return key
                        if key == 0x0d {
                            self.print_char(key);
                        }
                        break;
                    } else if key == 0x08 {
                        if !input_buffer.is_empty() {
                            input_buffer.pop();
                            self.backspace()?;
                        }
                    } else if input_buffer.len() < len && (0x20..0x7f).contains(&key) {
                        input_buffer.push(key);
                        self.print_char(key);
                    }
                }
                None => thread::sleep(Duration::from_millis(10)),
            }
        }

        Ok((input_buffer, terminator))
    }

    /// Prints the status line in a V3 game.  The status line occupies the topmost line on the screen.
    ///
    /// # Arguments
    /// * `left` - Left side of the status line, typically the room occupied by the player
    /// * `right` - right side of the status line, either the current score/turns or time of day
    ///
    /// # Returns
    /// Empty [Result] or a [RuntimeError]
    pub fn status_line(&mut self, left: &Vec<u16>, right: &Vec<u16>) -> Result<(), RuntimeError> {
        let width = self.columns() as usize;
        let available_for_left = width - right.len() - 1;
        let mut l = left.clone();
        if left.len() > available_for_left {
            l.truncate(available_for_left - 4);
            l.push('.' as u16);
            l.push('.' as u16);
            l.push('.' as u16);
        }

        let mut spaces = vec![b' ' as u16; width - l.len() - right.len() - 2];
        let mut status_line = vec![b' ' as u16];
        status_line.append(&mut l);
        status_line.append(&mut spaces);
        status_line.append(&mut right.clone());
        status_line.push(b' ' as u16);
        let mut style = CellStyle::new();
        style.set(Style::Reverse as u8);

        self.print_at(&status_line, (1, 1), &style);
        self.reset_cursor();
        Ok(())
    }

    /// Backspace
    ///
    /// If the cursor is not at the left edge of the screen, move it to the left and clear
    /// any text at that location.
    ///
    /// # Returns
    /// Empty [Result] or a [RuntimeError]
    pub fn backspace(&mut self) -> Result<(), RuntimeError> {
        if self.selected_window == 0 && self.cursor_0.1 > 1 {
            let attributes = self
                .window
                .mvinch(self.cursor_0.0 as i32 - 1, self.cursor_0.1 as i32 - 1);
            let ch = (attributes & 0xFFFFFF00) | 0x20;
            self.window
                .mvaddch(self.cursor_0.0 as i32 - 1, self.cursor_0.1 as i32 - 1, ch);
            self.window
                .mv(self.cursor_0.0 as i32 - 1, self.cursor_0.1 as i32 - 1);
            self.cursor_0 = (self.cursor_0.0, self.cursor_0.1 - 1);
        } else if self.selected_window == 1 && self.cursor_1.unwrap().1 > 1 {
            let attributes = self.window.mvinch(
                self.cursor_1.unwrap().0 as i32 - 1,
                self.cursor_1.unwrap().1 as i32 - 1,
            );
            let ch = (attributes & 0xFFFFFF00) | 0x20;
            self.window.mvaddch(
                self.cursor_1.unwrap().0 as i32 - 1,
                self.cursor_1.unwrap().1 as i32 - 1,
                ch,
            );
            self.window.mv(
                self.cursor_1.unwrap().0 as i32 - 1,
                self.cursor_1.unwrap().1 as i32 - 1,
            );
            self.cursor_1 = Some((self.cursor_1.unwrap().0, self.cursor_1.unwrap().1 - 1));
        }
        Ok(())
    }

    /// Set the current text style
    ///
    /// # Arguments
    /// * `style` - text style bitmap
    pub fn set_style(&mut self, style: u8) -> Result<(), RuntimeError> {
        self.current_style.set(style);
        Ok(())
    }

    /// Set the text buffering mode
    ///
    /// # Arguments
    /// * `mode` - 0 to disabled buffering, 1 to enable
    pub fn buffer_mode(&mut self, mode: u16) {
        self.buffer_mode = mode;
    }

    /// Play a simple beep
    pub fn beep(&mut self) {
        pancurses::beep();
    }

    /// Reset the current screen cursor to the selected window cursor position
    pub fn reset_cursor(&mut self) {
        if self.selected_window == 0 {
            self.window
                .mv(self.cursor_0.0 as i32 - 1, self.cursor_0.1 as i32 - 1);
        } else {
            // unwrap() should be safe here because when selected_window
            // is 1, cursor_1 is Some
            self.window.mv(
                self.cursor_1.unwrap().0 as i32 - 1,
                self.cursor_1.unwrap().1 as i32 - 1,
            );
        }
    }

    /// Set the text font.
    ///
    /// # Arguments
    /// * `font` - Font to set
    ///
    /// # Returns
    /// Previous font value
    pub fn set_font(&mut self, font: u8) -> u8 {
        match font {
            0 => self.font,
            1 | 3 | 4 => {
                let result = self.font;
                self.font = font;
                result
            }
            _ => 0,
        }
    }

    /// Reset the screen.
    pub fn reset(&mut self) {
        self.window.clear();
    }

    /// Clean up curses
    pub fn quit(&mut self) {
        info!(target: "app::screen", "Closing pancurses terminal");
        self.window.keypad(false);
        pancurses::curs_set(2);
        pancurses::mousemask(0, None);
        pancurses::endwin();
        pancurses::doupdate();
        pancurses::reset_prog_mode();
    }

    /// Display an error message
    ///
    /// If the message is recoverable and the error handling strategy allows, permit the user to opt
    /// to continue execution or exit.
    ///
    /// # Arguments
    /// * `instruction` - address of the instruction where the error occurred
    /// * `message` - error message
    /// * `recoverable` - true if the error is recoverable
    ///
    /// # Returns
    /// `true` if execution should continue, false if not.
    pub fn error(&mut self, instruction: &str, message: &str, recoverable: bool) -> bool {
        let (rows, cols) = self.window.get_max_yx();
        let height = 7;
        let prompt_str = "Press 'c' to continue or any other key to exit";
        let width = usize::max(
            prompt_str.len(),
            usize::max(instruction.len(), message.len()),
        ) as i32
            + 8;
        let err_row = (rows - height) / 2;
        let err_col = (cols - width) / 2;

        let errwin = pancurses::newwin(height, width, err_row, err_col);
        errwin.draw_box(0, 0);
        errwin.mv(1, 2);
        errwin.addstr(message);
        errwin.mv(3, 2);
        errwin.addstr(instruction);
        errwin.mv(5, 2);
        errwin.addstr(prompt_str);
        errwin.refresh();
        errwin.nodelay(false);
        pancurses::flushinp();
        loop {
            if let Some(ch) = errwin.getch() {
                errwin.delwin();
                self.window.touch();
                self.window.refresh();

                if recoverable && (ch == Input::Character('c') || ch == Input::Character('C')) {
                    return true;
                }

                return false;
            }
        }
    }

    /// Prompt the player for a filename
    ///
    /// Provides a suggested filename, which may be an existing filename (as when restoring) or a new filename (as when saving)
    ///
    /// # Arguments
    /// * `prompt` - Prompt message
    /// * `name` - Base filename suggestion
    /// * `suffix` - File extension suggestion
    /// * `overwrite` - True if an existing filename is allowed
    /// * `first` - When true, the default is the first available filename, otherwise the last existing filename
    ///
    /// # Returns
    /// [Result] containing the file name or a [RuntimeError]
    pub fn prompt_filename(
        &mut self,
        prompt: &str,
        name: &str,
        suffix: &str,
        overwrite: bool,
        first: bool,
    ) -> Result<String, RuntimeError> {
        self.print_str(prompt);
        let n = if first {
            files::first_available(name, suffix)?
        } else {
            files::last_existing(name, suffix)?
        };

        self.print(&n);

        let f = self.read_line(&n, 32, &['\r' as u16], 0, None)?.0;
        let filename = match String::from_utf16(&f) {
            Ok(s) => s.trim().to_string(),
            Err(e) => {
                return recoverable_error!(
                    ErrorCode::InvalidInput,
                    "Error parsing user input: {}",
                    e
                )
            }
        };

        if !overwrite {
            match Path::new(&filename).try_exists() {
                Ok(b) => match b {
                    true => {
                        return recoverable_error!(
                            ErrorCode::FileExists,
                            "'{}' already exists.",
                            filename
                        )
                    }
                    false => {}
                },
                Err(e) => {
                    return recoverable_error!(
                        ErrorCode::Interpreter,
                        "Error checking if '{}' exists: {}",
                        filename,
                        e
                    )
                }
            }
        }

        match Regex::new(r"^((.*\.z\d)|(.*\.blb)|(.*\.blorb))$") {
            Ok(r) => {
                if r.is_match(&filename) {
                    recoverable_error!(
                        ErrorCode::InvalidFilename,
                        "Filenames ending in '.z#', '.blb', or '.blorb' are not allowed"
                    )
                } else {
                    Ok(filename)
                }
            }
            Err(e) => recoverable_error!(
                ErrorCode::Interpreter,
                "Interal error with regex checking filename: {}",
                e
            ),
        }
    }

    /// Prompt for a filename, then create and open it.
    ///
    /// # Arguments
    /// * `prompt` - Prompt message
    /// * `name` - Base filename
    /// * `suffix` - File extension
    /// * `overwrite` - if false, an existing filename will be rejected
    ///
    /// # Returns
    /// [Result] containing a file opened for writing or a [RuntimeError]
    pub fn prompt_and_create(
        &mut self,
        prompt: &str,
        name: &str,
        suffix: &str,
        overwrite: bool,
    ) -> Result<File, RuntimeError> {
        match self.prompt_filename(prompt, name, suffix, overwrite, true) {
            Ok(filename) => match fs::OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(filename.trim())
            {
                Ok(f) => Ok(f),
                Err(e) => recoverable_error!(ErrorCode::FileError, "{}", e),
            },
            Err(e) => {
                self.print_str(&format!("Error creating file: {}\r", e));
                Err(e)
            }
        }
    }

    /// Prompt for a filename, create, open it, write to it, then flush and close it.
    ///
    /// # Arguments
    /// * `prompt` - Prompt message
    /// * `name` - Base filename
    /// * `suffix` - File extension
    /// * `data` - Vector of bytes to write to the file
    /// * `overwrite` - if false, an existing filename will be rejected
    ///
    /// # Returns
    /// Empty [Result] or a [RuntimeError]
    pub fn prompt_and_write(
        &mut self,
        prompt: &str,
        name: &str,
        suffix: &str,
        data: &[u8],
        overwrite: bool,
    ) -> Result<(), RuntimeError> {
        let mut file = self.prompt_and_create(prompt, name, suffix, overwrite)?;

        match file.write_all(data) {
            Ok(_) => (),
            Err(e) => return recoverable_error!(ErrorCode::FileError, "{}", e),
        };
        match file.flush() {
            Ok(_) => Ok(()),
            Err(e) => recoverable_error!(ErrorCode::FileError, "{}", e),
        }
    }

    /// Prompt for an existing filename, open it, read it, then close it.
    ///
    /// # Arguments
    /// * `prompt` - Prompt message
    /// * `name` - Base filename
    /// * `suffix` - File extension
    ///
    /// # Returns
    /// [Result] containing the file data or a [RuntimeError]    
    pub fn prompt_and_read(
        &mut self,
        prompt: &str,
        name: &str,
        suffix: &str,
    ) -> Result<Vec<u8>, RuntimeError> {
        let filename = self.prompt_filename(prompt, name, suffix, true, false)?;
        let mut data = Vec::new();
        match File::open(filename.trim()) {
            Ok(mut file) => match file.read_to_end(&mut data) {
                Ok(_) => Ok(data),
                Err(e) => recoverable_error!(ErrorCode::FileError, "{}", e),
            },
            Err(e) => recoverable_error!(ErrorCode::FileError, "{}: {}", filename, e),
        }
    }

    /// Map a Colour enum value to a curses color value
    ///
    /// # Arguments
    /// * `colour` - enum value
    ///
    /// # Returns
    /// Pancurses colour constant
    fn as_color(&self, colour: Colour) -> i16 {
        match colour {
            Colour::Black => pancurses::COLOR_BLACK,
            Colour::Red => pancurses::COLOR_RED,
            Colour::Green => pancurses::COLOR_GREEN,
            Colour::Yellow => pancurses::COLOR_YELLOW,
            Colour::Blue => pancurses::COLOR_BLUE,
            Colour::Magenta => pancurses::COLOR_MAGENTA,
            Colour::Cyan => pancurses::COLOR_CYAN,
            Colour::White => pancurses::COLOR_WHITE,
        }
    }

    /// Map curses input to an Z-Machine input event.
    ///
    /// # Arguments
    /// * `input` - Curses input
    ///
    /// # Returns
    /// Z-Machine input event
    fn input_to_u16(&self, input: Input) -> InputEvent {
        match input {
            Input::Character(c) => char_to_u16(c),
            Input::KeyUp => InputEvent::from_char(129),
            Input::KeyDown => InputEvent::from_char(130),
            Input::KeyLeft => InputEvent::from_char(131),
            Input::KeyRight => InputEvent::from_char(132),
            Input::KeyF1 => InputEvent::from_char(133),
            Input::KeyF2 => InputEvent::from_char(134),
            Input::KeyF3 => InputEvent::from_char(135),
            Input::KeyF4 => InputEvent::from_char(136),
            Input::KeyF5 => InputEvent::from_char(137),
            Input::KeyF6 => InputEvent::from_char(138),
            Input::KeyF7 => InputEvent::from_char(139),
            Input::KeyF8 => InputEvent::from_char(140),
            Input::KeyF9 => InputEvent::from_char(141),
            Input::KeyF10 => InputEvent::from_char(142),
            Input::KeyF11 => InputEvent::from_char(143),
            Input::KeyF12 => InputEvent::from_char(144),
            Input::KeyBackspace => InputEvent::from_char(8),
            Input::KeyMouse => match pancurses::getmouse() {
                Ok(event) => {
                    if event.bstate & pancurses::BUTTON1_CLICKED == pancurses::BUTTON1_CLICKED {
                        InputEvent::from_mouse(254, event.y as u16 + 1, event.x as u16 + 1)
                    } else if event.bstate & pancurses::BUTTON1_DOUBLE_CLICKED
                        == pancurses::BUTTON1_DOUBLE_CLICKED
                    {
                        InputEvent::from_mouse(253, event.y as u16 + 1, event.x as u16 + 1)
                    } else {
                        InputEvent::no_input()
                    }
                }
                Err(e) => {
                    warn!(target: "app::screen", "Error reading mouse event: {}", e);
                    InputEvent::no_input()
                }
            },
            _ => {
                info!(target: "app::screen", "Unprocssed input: {:?}", input);
                InputEvent::no_input()
            }
        }
    }
}

/// Builds a curses color pair number for a foreground and background pair.
///
/// The index is masked from the foreground and background: 0b00fffbbb
///
/// # Arguments
/// * `fg` - foreground colour
/// * `bg` - background colour
///
/// # Returns
/// Color pair index
fn cp(fg: i16, bg: i16) -> i16 {
    // color range 0-7, so 3 bits each
    // color pair index is 6 bits, 00ff fbbb + 1
    // pairs 1 - 64 are used by the basic colors, leaving 191 for "true" colors
    ((fg << 3) & 0x38) + (bg & 0x07) + 1
}

/// Maps a character to a Z-Machine input event
///
/// # Arguments
/// * `c` - Characteer
///
/// Returns
/// Input event
fn char_to_u16(c: char) -> InputEvent {
    match c {
        // Mac | Windows - slight differences in character values for backspace and return
        '\u{7f}' | '\u{08}' => InputEvent::from_char(0x08),
        '\u{0a}' | '\u{0d}' => InputEvent::from_char(0x0d),
        ' '..='~' => InputEvent::from_char(c as u16),
        '\u{e4}' => InputEvent::from_char(0x9b),
        '\u{f6}' => InputEvent::from_char(0x9c),
        '\u{fc}' => InputEvent::from_char(0x9d),
        '\u{c4}' => InputEvent::from_char(0x9e),
        '\u{d6}' => InputEvent::from_char(0x9f),
        '\u{dc}' => InputEvent::from_char(0xa0),
        '\u{df}' => InputEvent::from_char(0xa1),
        '\u{bb}' => InputEvent::from_char(0xa2),
        '\u{ab}' => InputEvent::from_char(0xa3),
        '\u{eb}' => InputEvent::from_char(0xa4),
        '\u{ef}' => InputEvent::from_char(0xa5),
        '\u{ff}' => InputEvent::from_char(0xa6),
        '\u{cb}' => InputEvent::from_char(0xa7),
        '\u{cf}' => InputEvent::from_char(0xa8),
        '\u{e1}' => InputEvent::from_char(0xa9),
        '\u{e9}' => InputEvent::from_char(0xaa),
        '\u{ed}' => InputEvent::from_char(0xab),
        '\u{f3}' => InputEvent::from_char(0xac),
        '\u{fa}' => InputEvent::from_char(0xad),
        '\u{fd}' => InputEvent::from_char(0xae),
        '\u{c1}' => InputEvent::from_char(0xaf),
        '\u{c9}' => InputEvent::from_char(0xb0),
        '\u{cd}' => InputEvent::from_char(0xb1),
        '\u{d3}' => InputEvent::from_char(0xb2),
        '\u{da}' => InputEvent::from_char(0xb3),
        '\u{dd}' => InputEvent::from_char(0xb4),
        '\u{e0}' => InputEvent::from_char(0xb5),
        '\u{e8}' => InputEvent::from_char(0xb6),
        '\u{ec}' => InputEvent::from_char(0xb7),
        '\u{f2}' => InputEvent::from_char(0xb8),
        '\u{f9}' => InputEvent::from_char(0xb9),
        '\u{c0}' => InputEvent::from_char(0xba),
        '\u{c8}' => InputEvent::from_char(0xbb),
        '\u{cc}' => InputEvent::from_char(0xbc),
        '\u{d2}' => InputEvent::from_char(0xbd),
        '\u{d9}' => InputEvent::from_char(0xbe),
        '\u{e2}' => InputEvent::from_char(0xbf),
        '\u{ea}' => InputEvent::from_char(0xc0),
        '\u{ee}' => InputEvent::from_char(0xc1),
        '\u{f4}' => InputEvent::from_char(0xc2),
        '\u{fb}' => InputEvent::from_char(0xc3),
        '\u{c2}' => InputEvent::from_char(0xc4),
        '\u{ca}' => InputEvent::from_char(0xc5),
        '\u{ce}' => InputEvent::from_char(0xc6),
        '\u{d4}' => InputEvent::from_char(0xc7),
        '\u{db}' => InputEvent::from_char(0xc8),
        '\u{e5}' => InputEvent::from_char(0xc9),
        '\u{c5}' => InputEvent::from_char(0xca),
        '\u{f8}' => InputEvent::from_char(0xcb),
        '\u{d8}' => InputEvent::from_char(0xcc),
        '\u{e3}' => InputEvent::from_char(0xcd),
        '\u{f1}' => InputEvent::from_char(0xce),
        '\u{f5}' => InputEvent::from_char(0xcf),
        '\u{c3}' => InputEvent::from_char(0xd0),
        '\u{d1}' => InputEvent::from_char(0xd1),
        '\u{d5}' => InputEvent::from_char(0xd2),
        '\u{e6}' => InputEvent::from_char(0xd3),
        '\u{c6}' => InputEvent::from_char(0xd4),
        '\u{e7}' => InputEvent::from_char(0xd5),
        '\u{c7}' => InputEvent::from_char(0xd6),
        '\u{fe}' => InputEvent::from_char(0xd7),
        '\u{f0}' => InputEvent::from_char(0xd8),
        '\u{de}' => InputEvent::from_char(0xd9),
        '\u{d0}' => InputEvent::from_char(0xda),
        '\u{a3}' => InputEvent::from_char(0xdb),
        '\u{153}' => InputEvent::from_char(0xdc),
        '\u{152}' => InputEvent::from_char(0xdd),
        '\u{a1}' => InputEvent::from_char(0xde),
        '\u{bf}' => InputEvent::from_char(0xdf),
        _ => {
            error!(target: "app::screen", "Unmapped input {:02x}", c as u8);
            InputEvent::no_input()
        }
    }
}

/// Maps a Z-Machine character to a char value
///
/// # Arguments
/// * `zchar` - Z-Machine character value
/// * `font` - font
///
/// # Returns
/// Char value
fn map_output(zchar: u16, font: u8) -> char {
    match font {
        1 | 4 => match zchar {
            0x18 => '\u{2191}',
            0x19 => '\u{2193}',
            0x1a => '\u{2192}',
            0x1b => '\u{2190}',
            0x20..=0x7E => (zchar as u8) as char,
            0x9b => '\u{e4}',
            0x9c => '\u{f6}',
            0x9d => '\u{fc}',
            0x9e => '\u{c4}',
            0x9f => '\u{d6}',
            0xa0 => '\u{dc}',
            0xa1 => '\u{df}',
            0xa2 => '\u{bb}',
            0xa3 => '\u{ab}',
            0xa4 => '\u{eb}',
            0xa5 => '\u{ef}',
            0xa6 => '\u{ff}',
            0xa7 => '\u{cb}',
            0xa8 => '\u{cf}',
            0xa9 => '\u{e1}',
            0xaa => '\u{e9}',
            0xab => '\u{ed}',
            0xac => '\u{f3}',
            0xad => '\u{fa}',
            0xae => '\u{fd}',
            0xaf => '\u{c1}',
            0xb0 => '\u{c9}',
            0xb1 => '\u{cd}',
            0xb2 => '\u{d3}',
            0xb3 => '\u{da}',
            0xb4 => '\u{dd}',
            0xb5 => '\u{e0}',
            0xb6 => '\u{e8}',
            0xb7 => '\u{ec}',
            0xb8 => '\u{f2}',
            0xb9 => '\u{f9}',
            0xba => '\u{c0}',
            0xbb => '\u{c8}',
            0xbc => '\u{cc}',
            0xbd => '\u{d2}',
            0xbe => '\u{d9}',
            0xbf => '\u{e2}',
            0xc0 => '\u{ea}',
            0xc1 => '\u{ee}',
            0xc2 => '\u{f4}',
            0xc3 => '\u{fb}',
            0xc4 => '\u{c2}',
            0xc5 => '\u{ca}',
            0xc6 => '\u{ce}',
            0xc7 => '\u{d4}',
            0xc8 => '\u{db}',
            0xc9 => '\u{e5}',
            0xca => '\u{c5}',
            0xcb => '\u{f8}',
            0xcc => '\u{d8}',
            0xcd => '\u{e3}',
            0xce => '\u{f1}',
            0xcf => '\u{f5}',
            0xd0 => '\u{c3}',
            0xd1 => '\u{d1}',
            0xd2 => '\u{d5}',
            0xd3 => '\u{e6}',
            0xd4 => '\u{c6}',
            0xd5 => '\u{e7}',
            0xd6 => '\u{c7}',
            0xd7 => '\u{fe}',
            0xd8 => '\u{f0}',
            0xd9 => '\u{de}',
            0xda => '\u{d0}',
            0xdb => '\u{a3}',
            0xdc => '\u{153}',
            0xdd => '\u{152}',
            0xde => '\u{a1}',
            0xdf => '\u{bf}',
            _ => {
                error!(target: "app::screen", "Unmapped font {} character {:04x}", font, zchar);
                zchar as u8 as char
            }
        },
        3 => match zchar {
            0x20..=0x7e => (zchar as u8) as char,
            0xb3 => '\u{2502}',
            0xbf => '\u{2510}',
            0xc0 => '\u{2514}',
            0xc4 => '\u{2500}',
            0xd9 => '\u{2518}',
            0xda => '\u{250c}',
            _ => {
                warn!(target: "app::screen", "Unmapped font 3 character {:04x}", zchar);
                zchar as u8 as char
            }
        },
        _ => '@',
    }
}
