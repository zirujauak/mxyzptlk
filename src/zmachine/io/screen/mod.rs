mod curses;

use crate::config::Config;
use crate::error::*;

#[cfg(not(test))]
use curses::pancurses::new_terminal;

#[cfg(test)]
use curses::test_terminal::new_terminal;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Color {
    Black = 2,
    Red = 3,
    Green = 4,
    Yellow = 5,
    Blue = 6,
    Magenta = 7,
    Cyan = 8,
    White = 9,
}

pub enum Style {
    Roman = 0,
    Reverse = 1,
    Bold = 2,
    Italic = 4,
    Fixed = 8,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct CellStyle {
    mask: u8,
}

impl Default for CellStyle {
    fn default() -> Self {
        CellStyle::new()
    }
}

impl CellStyle {
    pub fn new() -> CellStyle {
        CellStyle { mask: 0 }
    }

    pub fn set(&mut self, style: u8) {
        match style {
            0 => self.mask = 0,
            _ => self.mask |= style & 0xf,
        }
    }

    pub fn clear(&mut self, style: u8) {
        let mask = !(style & 0xF);
        self.mask &= mask;
    }

    pub fn is_style(&self, style: Style) -> bool {
        let s = style as u8;
        self.mask & s == s
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum Interrupt {
    ReadTimeout,
    Sound,
}

#[derive(Debug, Eq, PartialEq)]
pub struct InputEvent {
    zchar: Option<u16>,
    row: Option<u16>,
    column: Option<u16>,
    interrupt: Option<Interrupt>,
}

impl InputEvent {
    pub fn no_input() -> InputEvent {
        InputEvent {
            zchar: None,
            row: None,
            column: None,
            interrupt: None,
        }
    }
    pub fn from_char(zchar: u16) -> InputEvent {
        InputEvent {
            zchar: Some(zchar),
            row: None,
            column: None,
            interrupt: None,
        }
    }
    pub fn from_mouse(zchar: u16, row: u16, column: u16) -> InputEvent {
        InputEvent {
            zchar: Some(zchar),
            row: Some(row),
            column: Some(column),
            interrupt: None,
        }
    }
    pub fn from_interrupt(interrupt: Interrupt) -> InputEvent {
        InputEvent {
            zchar: None,
            row: None,
            column: None,
            interrupt: Some(interrupt),
        }
    }
    pub fn zchar(&self) -> Option<u16> {
        self.zchar
    }

    pub fn row(&self) -> Option<u16> {
        self.row
    }

    pub fn column(&self) -> Option<u16> {
        self.column
    }

    pub fn interrupt(&self) -> Option<&Interrupt> {
        self.interrupt.as_ref()
    }
}

fn map_color(color: u8) -> Result<Color, RuntimeError> {
    match color {
        2 => Ok(Color::Black),
        3 => Ok(Color::Red),
        4 => Ok(Color::Green),
        5 => Ok(Color::Yellow),
        6 => Ok(Color::Blue),
        7 => Ok(Color::Magenta),
        8 => Ok(Color::Cyan),
        9 => Ok(Color::White),
        _ => Err(RuntimeError::new(
            ErrorCode::InvalidColor,
            format!("Invalid color {}", color),
        )),
    }
}

fn map_colors(foreground: u8, background: u8) -> Result<(Color, Color), RuntimeError> {
    Ok((map_color(foreground)?, map_color(background)?))
}
pub struct Screen {
    version: u8,
    rows: u32,
    columns: u32,
    top: u32,
    window_1_top: Option<u32>,
    window_1_bottom: Option<u32>,
    window_0_top: u32,
    selected_window: u8,
    // foreground, background
    default_colors: (Color, Color),
    current_colors: (Color, Color),
    current_style: CellStyle,
    font: u8,
    // row, column with 1,1 as origin
    cursor_0: (u32, u32),
    cursor_1: Option<(u32, u32)>,
    terminal: Box<dyn Terminal>,
    lines_since_input: u32,
}

impl Screen {
    pub fn new_v3(config: Config) -> Result<Screen, RuntimeError> {
        let terminal = new_terminal();

        let (rows, columns) = terminal.as_ref().size();
        let colors = map_colors(config.foreground(), config.background())?;

        Ok(Screen {
            version: 3,
            rows,
            columns,
            top: 2,
            window_0_top: 2,
            window_1_top: None,
            window_1_bottom: None,
            selected_window: 0,
            default_colors: colors,
            current_colors: colors,
            current_style: CellStyle::new(),
            font: 1,
            cursor_0: (rows, 1),
            cursor_1: None,
            terminal,
            lines_since_input: 0,
        })
    }

    pub fn new_v4(config: Config) -> Result<Screen, RuntimeError> {
        let terminal = new_terminal();

        let (rows, columns) = terminal.as_ref().size();
        let colors = map_colors(config.foreground(), config.background())?;

        Ok(Screen {
            version: 4,
            rows,
            columns,
            top: 1,
            window_0_top: 1,
            window_1_top: None,
            window_1_bottom: None,
            selected_window: 0,
            default_colors: colors,
            current_colors: colors,
            current_style: CellStyle::new(),
            font: 1,
            cursor_0: (rows, 1),
            cursor_1: None,
            terminal,
            lines_since_input: 0,
        })
    }

    pub fn new_v5(config: Config) -> Result<Screen, RuntimeError> {
        let terminal = new_terminal();

        let (rows, columns) = terminal.as_ref().size();
        let colors = map_colors(config.foreground(), config.background())?;

        Ok(Screen {
            version: 5,
            rows,
            columns,
            top: 1,
            window_1_top: None,
            window_1_bottom: None,
            window_0_top: 1,
            selected_window: 0,
            default_colors: colors,
            current_colors: colors,
            current_style: CellStyle::new(),
            font: 1,
            cursor_0: (1, 1),
            cursor_1: None,
            terminal,
            lines_since_input: 0,
        })
    }

    pub fn rows(&self) -> u32 {
        self.rows
    }

    pub fn columns(&self) -> u32 {
        self.columns
    }

    pub fn cursor(&self) -> (u32, u32) {
        if self.selected_window == 0 {
            self.cursor_0
        } else {
            // unwrap() should be safe here because when selected_window
            // is 1, cursor_1 is Some
            self.cursor_1.unwrap()
        }
    }

    pub fn default_colors(&self) -> (Color, Color) {
        self.default_colors
    }

    pub fn selected_window(&self) -> u8 {
        self.selected_window
    }

    pub fn move_cursor(&mut self, row: u32, column: u32) {
        // Constrain the column between 1 and the width of the screen
        let c = u32::max(1, u32::min(self.columns, column));
        if self.selected_window == 0 {
            // Constrain row between top of window 0 and the bottom of the screen
            let r = u32::max(self.window_0_top, u32::min(self.rows, row));
            self.cursor_0 = (r, c);
            self.terminal.move_cursor((r, c));
        } else {
            // Constrain row between top of window 1 and bottom of window 1
            // unwrap() should be safe here because if window 1 is selected, then top/bottom are Some
            let r = u32::max(
                self.window_1_top.unwrap(),
                u32::min(self.window_1_bottom.unwrap(), row),
            );
            self.cursor_1 = Some((r, c));
            self.terminal.move_cursor((r, c));
        }
    }

    fn map_color(&self, color: u8, current: Color, default: Color) -> Result<Color, RuntimeError> {
        match color {
            0 => Ok(current),
            1 => Ok(default),
            2 => Ok(Color::Black),
            3 => Ok(Color::Red),
            4 => Ok(Color::Green),
            5 => Ok(Color::Yellow),
            6 => Ok(Color::Blue),
            7 => Ok(Color::Magenta),
            8 => Ok(Color::Cyan),
            9 => Ok(Color::White),
            _ => Err(RuntimeError::new(
                ErrorCode::InvalidColor,
                format!("Invalid color {}", color),
            )),
        }
    }

    fn map_colors(&self, foreground: u8, background: u8) -> Result<(Color, Color), RuntimeError> {
        Ok((
            self.map_color(foreground, self.current_colors.0, self.default_colors.0)?,
            self.map_color(background, self.current_colors.1, self.default_colors.1)?,
        ))
    }

    pub fn set_colors(&mut self, foreground: u16, background: u16) -> Result<(), RuntimeError> {
        self.current_colors = self.map_colors(foreground as u8, background as u8)?;
        self.terminal.set_colors(self.current_colors);
        Ok(())
    }

    pub fn split_window(&mut self, lines: u32) {
        let bottom = self.top + lines - 1;
        self.window_1_top = Some(self.top);
        self.window_1_bottom = Some(bottom);
        self.cursor_1 = Some((1, 1));
        self.window_0_top = bottom + 1;
        if self.cursor_0.0 < self.window_0_top {
            self.cursor_0 = (self.window_0_top, self.cursor_0.1)
        }
        self.terminal.split_window(lines);
    }

    pub fn unsplit_window(&mut self) {
        self.window_0_top = self.top;
        self.window_1_top = None;
        self.window_1_bottom = None;
        self.cursor_1 = None;
        self.selected_window = 0;
        self.terminal.split_window(0);
    }

    pub fn select_window(&mut self, window: u8) -> Result<(), RuntimeError> {
        self.lines_since_input = 0;
        self.terminal.set_window(window);
        if window == 0 {
            self.selected_window = 0;
            Ok(())
        } else if self.cursor_1.is_some() {
            self.selected_window = 1;
            self.cursor_1 = Some((self.top, 1));
            Ok(())
        } else {
            Err(RuntimeError::new(
                ErrorCode::InvalidWindow,
                format!("Invalid window {}", window),
            ))
        }
    }

    pub fn erase_window(&mut self, window: i8) -> Result<(), RuntimeError> {
        self.terminal.erase_window(window);
        match window {
            0 => {
                for i in self.window_0_top..=self.rows {
                    for j in 1..=self.columns {
                        self.terminal.as_mut().print_at(
                            0x20,
                            i,
                            j,
                            self.current_colors,
                            &CellStyle::new(),
                            1,
                        );
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
                                self.terminal.as_mut().print_at(
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
                self.window_0_top = 1;
                for i in self.window_0_top..=self.rows {
                    for j in 1..=self.columns {
                        self.terminal.as_mut().print_at(
                            0x20,
                            i,
                            j,
                            self.current_colors,
                            &CellStyle::new(),
                            1,
                        );
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
                        self.terminal.as_mut().print_at(
                            0x20,
                            i,
                            j,
                            self.current_colors,
                            &CellStyle::new(),
                            1,
                        );
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
            _ => Err(RuntimeError::new(
                ErrorCode::InvalidWindow,
                format!("ERASE_WINDOW invalid window {}", window),
            )), // This is an error
        }
    }

    pub fn erase_line(&mut self) {
        self.terminal.erase_line();
        let (row, col) = if self.selected_window == 0 {
            self.cursor_0
        } else {
            // unwrap() should be safe here because when selected_window
            // is 1, cursor_1 is Some
            self.cursor_1.unwrap()
        };
        for i in col..self.columns {
            self.terminal.as_mut().print_at(
                0x20,
                row,
                i,
                self.current_colors,
                &CellStyle::new(),
                1,
            );
        }
    }

    fn next_line(&mut self) {
        self.lines_since_input += 1;
        if self.cursor_0.0 == self.rows {
            self.terminal.scroll(self.window_0_top);
            self.cursor_0 = (self.rows, 1);
        } else {
            self.cursor_0 = (self.cursor_0.0 + 1, 1);
        }

        let l = self.rows - self.window_0_top;
        if self.lines_since_input >= l {
            let reverse = self.current_style.is_style(Style::Reverse);
            self.current_style.set(Style::Reverse as u8);
            self.print(&"[MORE]".chars().map(|c| c as u16).collect());
            if let Some(c) = self.read_key(true).zchar() {
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

    pub fn print(&mut self, text: &Vec<u16>) {
        for c in text {
            self.print_char(*c);
        }
        self.terminal.flush();
    }

    fn print_char(&mut self, zchar: u16) {
        if zchar == 0xd {
            self.new_line();
        } else if zchar != 0 {
            let (r, c) = if self.selected_window == 0 {
                self.cursor_0
            } else {
                self.cursor_1.unwrap()
            };

            self.terminal.print_at(
                zchar,
                r,
                c,
                self.current_colors,
                &self.current_style,
                self.font,
            );
            self.advance_cursor();
        }
    }

    pub fn print_at(&mut self, text: &[u16], at: (u32, u32), style: &CellStyle) {
        for (i, c) in text.iter().enumerate() {
            self.terminal.print_at(
                *c,
                u32::min(self.rows, at.0),
                u32::min(self.columns, at.1 + i as u32),
                self.current_colors,
                style,
                self.font,
            );
        }
        self.terminal.flush()
    }

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

    pub fn flush_buffer(&mut self) -> Result<(), RuntimeError> {
        self.terminal.flush();
        Ok(())
    }

    pub fn read_key(&mut self, wait: bool) -> InputEvent {
        self.lines_since_input = 0;
        if self.selected_window == 0 {
            self.terminal.move_cursor(self.cursor_0);
        } else {
            // unwrap() should be safe here because when selected_window
            // is 1, cursor_1 is Some
            self.terminal.move_cursor(self.cursor_1.unwrap());
        }

        self.terminal.read_key(wait)
    }

    pub fn backspace(&mut self) -> Result<(), RuntimeError> {
        if self.selected_window == 0 && self.cursor_0.1 > 1 {
            self.terminal
                .backspace((self.cursor_0.0, self.cursor_0.1 - 1));
            self.cursor_0 = (self.cursor_0.0, self.cursor_0.1 - 1);
        } else if self.selected_window == 1 && self.cursor_1.unwrap().1 > 1 {
            self.terminal
                .backspace((self.cursor_1.unwrap().0, self.cursor_1.unwrap().1 - 1));
            self.cursor_1 = Some((self.cursor_1.unwrap().0, self.cursor_1.unwrap().1 - 1));
        }
        Ok(())
    }

    pub fn set_style(&mut self, style: u8) -> Result<(), RuntimeError> {
        self.current_style.set(style);
        self.terminal.set_style(self.current_style.mask);
        Ok(())
    }

    pub fn buffer_mode(&mut self, mode: u16) {
        self.terminal.buffer_mode(mode);
    }

    pub fn beep(&mut self) {
        self.terminal.beep()
    }

    pub fn reset_cursor(&mut self) {
        if self.selected_window == 0 {
            self.terminal.move_cursor(self.cursor_0);
        } else {
            // unwrap() should be safe here because when selected_window
            // is 1, cursor_1 is Some
            self.terminal.move_cursor(self.cursor_1.unwrap());
        }
    }

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

    pub fn output_stream(&mut self, mask: u8, table: Option<usize>) {
        self.terminal.output_stream(mask, table);
    }

    pub fn reset(&mut self) {
        self.terminal.reset();
    }

    pub fn quit(&mut self) {
        self.terminal.quit();
    }
}

pub trait Terminal {
    fn size(&self) -> (u32, u32);
    fn print_at(
        &mut self,
        zchar: u16,
        row: u32,
        column: u32,
        colors: (Color, Color),
        style: &CellStyle,
        font: u8,
    );
    fn flush(&mut self);
    fn read_key(&mut self, wait: bool) -> InputEvent;
    fn scroll(&mut self, row: u32);
    fn backspace(&mut self, at: (u32, u32));
    fn beep(&mut self);
    fn move_cursor(&mut self, at: (u32, u32));
    fn reset(&mut self);
    fn quit(&mut self);
    fn set_colors(&mut self, colors: (Color, Color));
    // Below are hooks used by TestTerminal as part of unit testing
    fn split_window(&mut self, _lines: u32) {}
    fn set_window(&mut self, _window: u8) {}
    fn erase_window(&mut self, _window: i8) {}
    fn erase_line(&mut self) {}
    fn set_style(&mut self, _style: u8) {}
    fn buffer_mode(&mut self, _mode: u16) {}
    fn output_stream(&mut self, _stream: u8, _table: Option<usize>) {}
}

#[cfg(test)]
mod tests {
    use crate::{
        assert_ok, assert_ok_eq, assert_print, assert_some_eq,
        test_util::{
            backspace, beep, buffer_mode, colors, cursor, input, output_stream, quit, reset,
            scroll, split, style,
        },
    };

    use super::*;

    #[test]
    fn test_cellstyle_new() {
        let cs = CellStyle::new();
        assert_eq!(cs.mask, 0);
        assert!(!cs.is_style(Style::Bold));
        assert!(!cs.is_style(Style::Italic));
        assert!(!cs.is_style(Style::Reverse));
        assert!(!cs.is_style(Style::Fixed));
    }

    #[test]
    fn test_cellstyle_set() {
        let mut cs = CellStyle::new();
        cs.set(Style::Italic as u8);
        assert!(cs.is_style(Style::Italic));
        cs.set(Style::Bold as u8);
        assert!(cs.is_style(Style::Italic));
        assert!(cs.is_style(Style::Bold));
        cs.set(Style::Roman as u8);
        assert!(!cs.is_style(Style::Bold));
        assert!(!cs.is_style(Style::Italic));
        assert!(!cs.is_style(Style::Reverse));
        assert!(!cs.is_style(Style::Fixed));
    }

    #[test]
    fn test_cellstyle_clear() {
        let mut cs = CellStyle::new();
        cs.set(Style::Italic as u8);
        cs.set(Style::Bold as u8);
        assert!(cs.is_style(Style::Italic));
        assert!(cs.is_style(Style::Bold));
        cs.clear(Style::Bold as u8);
        assert!(cs.is_style(Style::Italic));
        assert!(!cs.is_style(Style::Bold));
    }

    #[test]
    fn test_inputevent_no_input() {
        let ie = InputEvent::no_input();
        assert!(ie.zchar().is_none());
        assert!(ie.row().is_none());
        assert!(ie.column().is_none());
        assert!(ie.interrupt().is_none());
    }

    #[test]
    fn test_inputevent_from_char() {
        let ie = InputEvent::from_char(0xFFFF);
        assert_some_eq!(ie.zchar(), 0xFFFF);
        assert!(ie.row().is_none());
        assert!(ie.column().is_none());
        assert!(ie.interrupt().is_none());
    }

    #[test]
    fn test_inputevent_from_mouse() {
        let ie = InputEvent::from_mouse(0xFFFF, 1, 2);
        assert_some_eq!(ie.zchar(), 0xFFFF);
        assert_some_eq!(ie.row(), 1);
        assert_some_eq!(ie.column(), 2);
        assert!(ie.interrupt().is_none());
    }

    #[test]
    fn test_inputevent_from_interrupt() {
        let ie = InputEvent::from_interrupt(Interrupt::ReadTimeout);
        assert!(ie.zchar().is_none());
        assert!(ie.row().is_none());
        assert!(ie.column().is_none());
        assert_some_eq!(ie.interrupt(), &Interrupt::ReadTimeout);
    }

    #[test]
    fn test_map_color() {
        assert_ok_eq!(map_color(2), Color::Black);
        assert_ok_eq!(map_color(3), Color::Red);
        assert_ok_eq!(map_color(4), Color::Green);
        assert_ok_eq!(map_color(5), Color::Yellow);
        assert_ok_eq!(map_color(6), Color::Blue);
        assert_ok_eq!(map_color(7), Color::Magenta);
        assert_ok_eq!(map_color(8), Color::Cyan);
        assert_ok_eq!(map_color(9), Color::White);
        assert!(map_color(0).is_err());
    }

    #[test]
    fn test_screen_new_v3() {
        let screen = assert_ok!(Screen::new_v3(Config::default()));
        assert_eq!(screen.version, 3);
        assert_eq!(screen.rows(), 24);
        assert_eq!(screen.columns(), 80);
        assert_eq!(screen.top, 2);
        assert_eq!(screen.window_0_top, 2);
        assert!(screen.window_1_top.is_none());
        assert!(screen.window_1_bottom.is_none());
        assert_eq!(screen.selected_window(), 0);
        assert_eq!(screen.default_colors(), (Color::White, Color::Black));
        assert_eq!(screen.current_colors, (Color::White, Color::Black));
        assert_eq!(screen.current_style, CellStyle::new());
        assert_eq!(screen.font, 1);
        assert_eq!(screen.cursor_0, (24, 1));
        assert!(screen.cursor_1.is_none());
        assert_eq!(screen.lines_since_input, 0);
    }

    #[test]
    fn test_screen_new_v4() {
        let screen = assert_ok!(Screen::new_v4(Config::default()));
        assert_eq!(screen.version, 4);
        assert_eq!(screen.rows(), 24);
        assert_eq!(screen.columns(), 80);
        assert_eq!(screen.top, 1);
        assert_eq!(screen.window_0_top, 1);
        assert!(screen.window_1_top.is_none());
        assert!(screen.window_1_bottom.is_none());
        assert_eq!(screen.selected_window(), 0);
        assert_eq!(screen.default_colors(), (Color::White, Color::Black));
        assert_eq!(screen.current_colors, (Color::White, Color::Black));
        assert_eq!(screen.current_style, CellStyle::new());
        assert_eq!(screen.font, 1);
        assert_eq!(screen.cursor_0, (24, 1));
        assert!(screen.cursor_1.is_none());
        assert_eq!(screen.lines_since_input, 0);
    }

    #[test]
    fn test_screen_new_v5() {
        let screen = assert_ok!(Screen::new_v5(Config::default()));
        assert_eq!(screen.version, 5);
        assert_eq!(screen.rows(), 24);
        assert_eq!(screen.columns(), 80);
        assert_eq!(screen.top, 1);
        assert_eq!(screen.window_0_top, 1);
        assert!(screen.window_1_top.is_none());
        assert!(screen.window_1_bottom.is_none());
        assert_eq!(screen.selected_window(), 0);
        assert_eq!(screen.default_colors(), (Color::White, Color::Black));
        assert_eq!(screen.current_colors, (Color::White, Color::Black));
        assert_eq!(screen.current_style, CellStyle::new());
        assert_eq!(screen.font, 1);
        assert_eq!(screen.cursor_0, (1, 1));
        assert!(screen.cursor_1.is_none());
        assert_eq!(screen.lines_since_input, 0);
    }

    #[test]
    fn test_screen_cursor_0() {
        let screen = assert_ok!(Screen::new_v5(Config::default()));
        assert_eq!(screen.cursor(), (1, 1));
    }

    #[test]
    fn test_screen_cursor_1() {
        let mut screen = assert_ok!(Screen::new_v5(Config::default()));
        screen.split_window(12);
        assert_eq!(screen.cursor(), (13, 1));
        assert!(screen.select_window(1).is_ok());
        assert_eq!(screen.cursor(), (1, 1));
    }

    #[test]
    fn test_screen_move_cursor_no_split() {
        let mut screen = assert_ok!(Screen::new_v5(Config::default()));
        assert_eq!(screen.cursor(), (1, 1));
        screen.move_cursor(12, 40);
        assert_eq!(screen.cursor(), (12, 40));
        screen.move_cursor(0, 0);
        assert_eq!(screen.cursor(), (1, 1));
        screen.move_cursor(25, 81);
        assert_eq!(screen.cursor(), (24, 80));
    }

    #[test]
    fn test_screen_move_cursor_split() {
        let mut screen = assert_ok!(Screen::new_v5(Config::default()));
        screen.split_window(12);
        assert_eq!(screen.cursor(), (13, 1));
        screen.move_cursor(18, 40);
        assert_eq!(screen.cursor(), (18, 40));
        screen.move_cursor(12, 0);
        assert_eq!(screen.cursor(), (13, 1));
        screen.move_cursor(25, 81);
        assert_eq!(screen.cursor(), (24, 80));
        assert!(screen.select_window(1).is_ok());
        assert_eq!(screen.cursor(), (1, 1));
        screen.move_cursor(6, 40);
        assert_eq!(screen.cursor(), (6, 40));
        screen.move_cursor(0, 0);
        assert_eq!(screen.cursor(), (1, 1));
        screen.move_cursor(13, 81);
        assert_eq!(screen.cursor(), (12, 80));
    }

    #[test]
    fn test_screen_map_color() {
        let screen = assert_ok!(Screen::new_v5(Config::default()));
        assert_ok_eq!(
            screen.map_color(0, Color::Black, Color::White),
            Color::Black
        );
        assert_ok_eq!(
            screen.map_color(1, Color::Black, Color::White),
            Color::White
        );
        assert_ok_eq!(screen.map_color(2, Color::Red, Color::White), Color::Black);
        assert_ok_eq!(screen.map_color(3, Color::Black, Color::White), Color::Red);
        assert_ok_eq!(
            screen.map_color(4, Color::Black, Color::White),
            Color::Green
        );
        assert_ok_eq!(
            screen.map_color(5, Color::Black, Color::White),
            Color::Yellow
        );
        assert_ok_eq!(screen.map_color(6, Color::Black, Color::White), Color::Blue);
        assert_ok_eq!(
            screen.map_color(7, Color::Black, Color::White),
            Color::Magenta
        );
        assert_ok_eq!(screen.map_color(8, Color::Black, Color::White), Color::Cyan);
        assert_ok_eq!(screen.map_color(9, Color::Black, Color::Red), Color::White);
        assert!(screen.map_color(10, Color::Black, Color::White).is_err());
    }

    #[test]
    fn test_screen_map_colors() {
        let screen = assert_ok!(Screen::new_v5(Config::default()));
        assert_ok_eq!(screen.map_colors(8, 5), (Color::Cyan, Color::Yellow));
        assert_ok_eq!(screen.map_colors(1, 1), (Color::White, Color::Black));
        assert_ok_eq!(screen.map_colors(0, 0), (Color::White, Color::Black));
        assert!(screen.map_colors(0, 10).is_err());
        assert!(screen.map_colors(10, 0).is_err());
    }

    #[test]
    fn test_screen_set_colors() {
        let mut screen = assert_ok!(Screen::new_v5(Config::default()));
        assert!(screen.set_colors(4, 6).is_ok());
        assert_eq!(screen.current_colors, (Color::Green, Color::Blue));
        assert_eq!(screen.default_colors(), (Color::White, Color::Black));
        assert_eq!(colors(), (4, 6));
        assert!(screen.set_colors(1, 1).is_ok());
        assert_eq!(screen.current_colors, (Color::White, Color::Black));
        assert_eq!(colors(), (9, 2));
    }

    #[test]
    fn test_screen_split_window_v3() {
        let mut screen = assert_ok!(Screen::new_v3(Config::default()));
        assert_eq!(screen.cursor_0, (24, 1));
        assert!(screen.cursor_1.is_none());
        screen.split_window(12);
        assert_eq!(screen.cursor_0, (24, 1));
        assert_some_eq!(screen.cursor_1, (1, 1));
        assert_some_eq!(screen.window_1_top, 2);
        assert_some_eq!(screen.window_1_bottom, 13);
        assert_eq!(screen.window_0_top, 14);
        assert_eq!(split(), 12);
    }

    #[test]
    fn test_screen_split_window_v4() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        assert_eq!(screen.cursor_0, (24, 1));
        assert!(screen.cursor_1.is_none());
        screen.split_window(12);
        assert_eq!(screen.cursor_0, (24, 1));
        assert_some_eq!(screen.cursor_1, (1, 1));
        assert_some_eq!(screen.window_1_top, 1);
        assert_some_eq!(screen.window_1_bottom, 12);
        assert_eq!(screen.window_0_top, 13);
        assert_eq!(split(), 12);
    }

    #[test]
    fn test_screen_split_window_v5() {
        let mut screen = assert_ok!(Screen::new_v5(Config::default()));
        assert_eq!(screen.cursor_0, (1, 1));
        assert!(screen.cursor_1.is_none());
        screen.split_window(12);
        assert_eq!(screen.cursor_0, (13, 1));
        assert_some_eq!(screen.cursor_1, (1, 1));
        assert_some_eq!(screen.window_1_top, 1);
        assert_some_eq!(screen.window_1_bottom, 12);
        assert_eq!(screen.window_0_top, 13);
        assert_eq!(split(), 12);
    }

    #[test]
    fn test_screen_unsplit_window_v3() {
        let mut screen = assert_ok!(Screen::new_v3(Config::default()));
        screen.split_window(12);
        screen.unsplit_window();
        assert_eq!(screen.cursor_0, (24, 1));
        assert!(screen.cursor_1.is_none());
        assert!(screen.window_1_top.is_none());
        assert!(screen.window_1_bottom.is_none());
        assert_eq!(screen.window_0_top, 2);
        assert_eq!(split(), 0);
    }

    #[test]
    fn test_screen_unsplit_window_v4() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        screen.split_window(12);
        screen.unsplit_window();
        assert_eq!(screen.cursor_0, (24, 1));
        assert!(screen.cursor_1.is_none());
        assert!(screen.window_1_top.is_none());
        assert!(screen.window_1_bottom.is_none());
        assert_eq!(screen.window_0_top, 1);
        assert_eq!(split(), 0);
    }

    #[test]
    fn test_screen_select_window_v3() {
        let mut screen = assert_ok!(Screen::new_v3(Config::default()));
        screen.split_window(12);
        assert_eq!(screen.selected_window(), 0);
        screen.cursor_1 = Some((12, 1));
        assert!(screen.select_window(1).is_ok());
        assert_eq!(screen.selected_window(), 1);
        assert_some_eq!(screen.cursor_1, (2, 1));
    }

    #[test]
    fn test_screen_select_window_v4() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        screen.split_window(12);
        assert_eq!(screen.selected_window(), 0);
        screen.cursor_1 = Some((12, 1));
        assert!(screen.select_window(1).is_ok());
        assert_eq!(screen.selected_window(), 1);
        assert_some_eq!(screen.cursor_1, (1, 1));
    }

    #[test]
    fn test_screen_select_window_1_not_split() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        assert!(screen.select_window(1).is_err());
    }

    #[test]
    fn test_screen_erase_window_0_v4() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        screen.split_window(12);
        assert!(screen.erase_window(0).is_ok());
        assert_print!(&vec![' '; 960].iter().collect::<String>());
        assert_eq!(screen.cursor_0, (24, 1));
    }

    #[test]
    fn test_screen_erase_window_0_v5() {
        let mut screen = assert_ok!(Screen::new_v5(Config::default()));
        screen.split_window(12);
        assert!(screen.erase_window(0).is_ok());
        assert_print!(&vec![' '; 960].iter().collect::<String>());
        assert_eq!(screen.cursor_0, (13, 1));
    }

    #[test]
    fn test_screen_erase_window_1() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        screen.split_window(10);
        assert!(screen.erase_window(1).is_ok());
        assert_print!(&vec![' '; 800].iter().collect::<String>());
        assert_eq!(screen.cursor_1.unwrap(), (1, 1));
    }

    #[test]
    fn test_screen_erase_window_minus_1_v4() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        screen.split_window(10);
        assert!(screen.erase_window(-1).is_ok());
        assert_print!(&vec![' '; 1920].iter().collect::<String>());
        assert_eq!(screen.cursor_0, (24, 1));
        assert!(screen.window_1_top.is_none());
        assert!(screen.window_1_bottom.is_none());
        assert!(screen.cursor_1.is_none());
        assert_eq!(screen.window_0_top, 1);
    }

    #[test]
    fn test_screen_erase_window_minus_1_v5() {
        let mut screen = assert_ok!(Screen::new_v5(Config::default()));
        screen.split_window(10);
        assert!(screen.erase_window(-1).is_ok());
        assert_print!(&vec![' '; 1920].iter().collect::<String>());
        assert_eq!(screen.cursor_0, (1, 1));
        assert!(screen.window_1_top.is_none());
        assert!(screen.window_1_bottom.is_none());
        assert!(screen.cursor_1.is_none());
        assert_eq!(screen.window_0_top, 1);
    }

    #[test]
    fn test_screen_erase_window_minus_2_v4() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        screen.split_window(10);
        assert!(screen.erase_window(-2).is_ok());
        assert_print!(&vec![' '; 1920].iter().collect::<String>());
        assert_eq!(screen.cursor_0, (24, 1));
        assert_some_eq!(screen.window_1_top, 1);
        assert_some_eq!(screen.window_1_bottom, 10);
        assert_some_eq!(screen.cursor_1, (1, 1));
        assert_eq!(screen.window_0_top, 11);
    }

    #[test]
    fn test_screen_erase_window_minus_2_v5() {
        let mut screen = assert_ok!(Screen::new_v5(Config::default()));
        screen.split_window(10);
        assert!(screen.erase_window(-2).is_ok());
        assert_print!(&vec![' '; 1920].iter().collect::<String>());
        assert_eq!(screen.cursor_0, (11, 1));
        assert_some_eq!(screen.window_1_top, 1);
        assert_some_eq!(screen.window_1_bottom, 10);
        assert_some_eq!(screen.cursor_1, (1, 1));
        assert_eq!(screen.window_0_top, 11);
    }

    #[test]
    fn test_screen_erase_window_invalid() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        screen.split_window(10);
        assert!(screen.erase_window(-3).is_err());
        assert_print!("");
        assert_eq!(screen.cursor_0, (24, 1));
        assert_some_eq!(screen.window_1_top, 1);
        assert_some_eq!(screen.window_1_bottom, 10);
        assert_some_eq!(screen.cursor_1, (1, 1));
        assert_eq!(screen.window_0_top, 11);
    }

    #[test]
    fn test_screen_erase_line_window_0() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        screen.split_window(10);
        screen.move_cursor(15, 5);
        screen.erase_line();
        assert_print!(&vec![' '; 75].iter().collect::<String>());
        assert_eq!(screen.cursor_0, (15, 5));
        assert_some_eq!(screen.window_1_top, 1);
        assert_some_eq!(screen.window_1_bottom, 10);
        assert_some_eq!(screen.cursor_1, (1, 1));
        assert_eq!(screen.window_0_top, 11);
    }

    #[test]
    fn test_screen_erase_line_window_1() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        screen.split_window(10);
        screen.move_cursor(15, 5);
        assert!(screen.select_window(1).is_ok());
        screen.move_cursor(5, 15);
        screen.erase_line();
        assert_print!(&vec![' '; 65].iter().collect::<String>());
        assert_eq!(screen.cursor_0, (15, 5));
        assert_some_eq!(screen.window_1_top, 1);
        assert_some_eq!(screen.window_1_bottom, 10);
        assert_some_eq!(screen.cursor_1, (5, 15));
        assert_eq!(screen.window_0_top, 11);
    }

    #[test]
    fn test_screen_next_line() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        screen.split_window(10);
        screen.move_cursor(15, 5);
        screen.next_line();
        assert_eq!(screen.cursor_0, (16, 1));
        assert_some_eq!(screen.window_1_top, 1);
        assert_some_eq!(screen.window_1_bottom, 10);
        assert_some_eq!(screen.cursor_1, (1, 1));
        assert_eq!(screen.window_0_top, 11);
        assert_eq!(scroll(), 0);
        assert_eq!(screen.lines_since_input, 1);
    }

    #[test]
    fn test_screen_next_line_scroll() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        screen.split_window(10);
        screen.move_cursor(24, 5);
        screen.next_line();
        assert_eq!(screen.cursor_0, (24, 1));
        assert_some_eq!(screen.window_1_top, 1);
        assert_some_eq!(screen.window_1_bottom, 10);
        assert_some_eq!(screen.cursor_1, (1, 1));
        assert_eq!(screen.window_0_top, 11);
        assert_eq!(scroll(), 11);
    }

    #[test]
    fn test_screen_next_line_scroll_prompt() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        screen.split_window(10);
        screen.move_cursor(24, 5);
        screen.lines_since_input = 13;
        input(&[' ']);
        screen.next_line();
        assert_eq!(screen.cursor_0, (24, 1));
        assert_some_eq!(screen.window_1_top, 1);
        assert_some_eq!(screen.window_1_bottom, 10);
        assert_some_eq!(screen.cursor_1, (1, 1));
        assert_eq!(screen.window_0_top, 11);
        assert_print!("[MORE]      ");
        assert_eq!(screen.lines_since_input, 0);
        assert_eq!(scroll(), 11);
    }

    #[test]
    fn test_screen_advance_cursor_window_0() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        screen.split_window(10);
        screen.move_cursor(24, 5);
        screen.advance_cursor();
        assert_eq!(screen.cursor_0, (24, 6));
        assert_some_eq!(screen.window_1_top, 1);
        assert_some_eq!(screen.window_1_bottom, 10);
        assert_some_eq!(screen.cursor_1, (1, 1));
        assert_eq!(screen.window_0_top, 11);
    }

    #[test]
    fn test_screen_advance_cursor_window_0_end_of_line() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        screen.split_window(10);
        screen.move_cursor(23, 80);
        screen.advance_cursor();
        assert_eq!(screen.cursor_0, (24, 1));
        assert_some_eq!(screen.window_1_top, 1);
        assert_some_eq!(screen.window_1_bottom, 10);
        assert_some_eq!(screen.cursor_1, (1, 1));
        assert_eq!(screen.window_0_top, 11);
        assert_eq!(screen.lines_since_input, 1);
    }

    #[test]
    fn test_screen_advance_cursor_window_0_end_of_line_scroll() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        screen.split_window(10);
        screen.move_cursor(24, 80);
        screen.advance_cursor();
        assert_eq!(screen.cursor_0, (24, 1));
        assert_some_eq!(screen.window_1_top, 1);
        assert_some_eq!(screen.window_1_bottom, 10);
        assert_some_eq!(screen.cursor_1, (1, 1));
        assert_eq!(screen.window_0_top, 11);
        assert_eq!(screen.lines_since_input, 1);
    }

    #[test]
    fn test_screen_advance_cursor_window_1() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        screen.split_window(10);
        screen.move_cursor(24, 80);
        assert!(screen.select_window(1).is_ok());
        screen.move_cursor(10, 5);
        screen.advance_cursor();
        assert_eq!(screen.cursor_0, (24, 80));
        assert_some_eq!(screen.window_1_top, 1);
        assert_some_eq!(screen.window_1_bottom, 10);
        assert_some_eq!(screen.cursor_1, (10, 6));
        assert_eq!(screen.window_0_top, 11);
    }

    #[test]
    fn test_screen_advance_cursor_window_1_end_of_line() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        screen.split_window(10);
        screen.move_cursor(24, 80);
        assert!(screen.select_window(1).is_ok());
        screen.move_cursor(9, 80);
        screen.advance_cursor();
        assert_eq!(screen.cursor_0, (24, 80));
        assert_some_eq!(screen.window_1_top, 1);
        assert_some_eq!(screen.window_1_bottom, 10);
        assert_some_eq!(screen.cursor_1, (10, 1));
        assert_eq!(screen.window_0_top, 11);
        assert_eq!(screen.lines_since_input, 0);
    }

    #[test]
    fn test_screen_advance_cursor_window_1_end_of_window() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        screen.split_window(10);
        screen.move_cursor(24, 80);
        assert!(screen.select_window(1).is_ok());
        screen.move_cursor(10, 80);
        screen.advance_cursor();
        assert_eq!(screen.cursor_0, (24, 80));
        assert_some_eq!(screen.window_1_top, 1);
        assert_some_eq!(screen.window_1_bottom, 10);
        assert_some_eq!(screen.cursor_1, (10, 80));
        assert_eq!(screen.window_0_top, 11);
        assert_eq!(screen.lines_since_input, 0);
    }

    #[test]
    fn test_screen_print_window_0() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        screen.split_window(10);
        screen.move_cursor(24, 1);
        screen.print(&vec!['a' as u16; 10]);
        assert_eq!(screen.cursor_0, (24, 11));
        assert_some_eq!(screen.window_1_top, 1);
        assert_some_eq!(screen.window_1_bottom, 10);
        assert_some_eq!(screen.cursor_1, (1, 1));
        assert_eq!(screen.window_0_top, 11);
        assert_eq!(screen.lines_since_input, 0);
        assert_print!("aaaaaaaaaa");
    }

    #[test]
    fn test_screen_print_window_0_wrap() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        screen.split_window(10);
        screen.move_cursor(23, 76);
        screen.print(&vec!['a' as u16; 10]);
        assert_eq!(screen.cursor_0, (24, 6));
        assert_some_eq!(screen.window_1_top, 1);
        assert_some_eq!(screen.window_1_bottom, 10);
        assert_some_eq!(screen.cursor_1, (1, 1));
        assert_eq!(screen.window_0_top, 11);
        assert_eq!(screen.lines_since_input, 1);
        assert_print!("aaaaaaaaaa");
    }

    #[test]
    fn test_screen_print_window_0_wrap_scroll() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        screen.split_window(10);
        screen.move_cursor(24, 76);
        screen.print(&vec!['a' as u16; 10]);
        assert_eq!(screen.cursor_0, (24, 6));
        assert_some_eq!(screen.window_1_top, 1);
        assert_some_eq!(screen.window_1_bottom, 10);
        assert_some_eq!(screen.cursor_1, (1, 1));
        assert_eq!(screen.window_0_top, 11);
        assert_eq!(screen.lines_since_input, 1);
        assert_print!("aaaaaaaaaa");
    }

    #[test]
    fn test_screen_print_window_1() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        screen.split_window(10);
        screen.move_cursor(24, 1);
        assert!(screen.select_window(1).is_ok());
        screen.move_cursor(10, 5);
        screen.print(&vec!['a' as u16; 10]);
        assert_eq!(screen.cursor_0, (24, 1));
        assert_some_eq!(screen.window_1_top, 1);
        assert_some_eq!(screen.window_1_bottom, 10);
        assert_some_eq!(screen.cursor_1, (10, 15));
        assert_eq!(screen.window_0_top, 11);
        assert_eq!(screen.lines_since_input, 0);
        assert_print!("aaaaaaaaaa");
    }

    #[test]
    fn test_screen_print_window_1_wrap() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        screen.split_window(10);
        screen.move_cursor(24, 1);
        assert!(screen.select_window(1).is_ok());
        screen.move_cursor(9, 76);
        screen.print(&vec!['a' as u16; 10]);
        assert_eq!(screen.cursor_0, (24, 1));
        assert_some_eq!(screen.window_1_top, 1);
        assert_some_eq!(screen.window_1_bottom, 10);
        assert_some_eq!(screen.cursor_1, (10, 6));
        assert_eq!(screen.window_0_top, 11);
        assert_eq!(screen.lines_since_input, 0);
        assert_print!("aaaaaaaaaa");
    }

    #[test]
    fn test_screen_print_window_1_end_of_screen() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        screen.split_window(10);
        screen.move_cursor(24, 1);
        assert!(screen.select_window(1).is_ok());
        screen.move_cursor(10, 76);
        screen.print(&vec!['a' as u16; 10]);
        assert_eq!(screen.cursor_0, (24, 1));
        assert_some_eq!(screen.window_1_top, 1);
        assert_some_eq!(screen.window_1_bottom, 10);
        assert_some_eq!(screen.cursor_1, (10, 80));
        assert_eq!(screen.window_0_top, 11);
        assert_eq!(screen.lines_since_input, 0);
        assert_print!("aaaaaaaaaa");
    }

    #[test]
    fn test_screen_print_char_window_0() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        screen.split_window(10);
        screen.move_cursor(24, 1);
        screen.print_char('b' as u16);
        assert_eq!(screen.cursor_0, (24, 2));
        assert_some_eq!(screen.window_1_top, 1);
        assert_some_eq!(screen.window_1_bottom, 10);
        assert_some_eq!(screen.cursor_1, (1, 1));
        assert_eq!(screen.window_0_top, 11);
        assert_eq!(screen.lines_since_input, 0);
        assert_print!("b");
    }

    #[test]
    fn test_screen_print_char_window_0_wrap() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        screen.split_window(10);
        screen.move_cursor(23, 80);
        screen.print_char('b' as u16);
        assert_eq!(screen.cursor_0, (24, 1));
        assert_some_eq!(screen.window_1_top, 1);
        assert_some_eq!(screen.window_1_bottom, 10);
        assert_some_eq!(screen.cursor_1, (1, 1));
        assert_eq!(screen.window_0_top, 11);
        assert_eq!(screen.lines_since_input, 1);
        assert_print!("b");
    }

    #[test]
    fn test_screen_print_char_window_0_wrap_scroll() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        screen.split_window(10);
        screen.move_cursor(24, 80);
        screen.print_char('b' as u16);
        assert_eq!(screen.cursor_0, (24, 1));
        assert_some_eq!(screen.window_1_top, 1);
        assert_some_eq!(screen.window_1_bottom, 10);
        assert_some_eq!(screen.cursor_1, (1, 1));
        assert_eq!(screen.window_0_top, 11);
        assert_eq!(screen.lines_since_input, 1);
        assert_print!("b");
    }

    #[test]
    fn test_screen_print_char_window_1() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        screen.split_window(10);
        screen.move_cursor(23, 80);
        assert!(screen.select_window(1).is_ok());
        screen.move_cursor(10, 5);
        screen.print_char('b' as u16);
        assert_eq!(screen.cursor_0, (23, 80));
        assert_some_eq!(screen.window_1_top, 1);
        assert_some_eq!(screen.window_1_bottom, 10);
        assert_some_eq!(screen.cursor_1, (10, 6));
        assert_eq!(screen.window_0_top, 11);
        assert_eq!(screen.lines_since_input, 0);
        assert_print!("b");
    }

    #[test]
    fn test_screen_print_char_window_1_wrap() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        screen.split_window(10);
        screen.move_cursor(23, 80);
        assert!(screen.select_window(1).is_ok());
        screen.move_cursor(9, 80);
        screen.print_char('b' as u16);
        assert_eq!(screen.cursor_0, (23, 80));
        assert_some_eq!(screen.window_1_top, 1);
        assert_some_eq!(screen.window_1_bottom, 10);
        assert_some_eq!(screen.cursor_1, (10, 1));
        assert_eq!(screen.window_0_top, 11);
        assert_eq!(screen.lines_since_input, 0);
        assert_print!("b");
    }

    #[test]
    fn test_screen_print_char_window_1_end_of_window() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        screen.split_window(10);
        screen.move_cursor(23, 80);
        assert!(screen.select_window(1).is_ok());
        screen.move_cursor(10, 80);
        screen.print_char('b' as u16);
        assert_eq!(screen.cursor_0, (23, 80));
        assert_some_eq!(screen.window_1_top, 1);
        assert_some_eq!(screen.window_1_bottom, 10);
        assert_some_eq!(screen.cursor_1, (10, 80));
        assert_eq!(screen.window_0_top, 11);
        assert_eq!(screen.lines_since_input, 0);
        assert_print!("b");
    }

    #[test]
    fn test_screen_print_at() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        screen.split_window(10);
        screen.move_cursor(23, 80);
        screen.print_at(&['c' as u16; 15], (12, 20), &CellStyle::new());
        assert_eq!(screen.cursor_0, (23, 80));
        assert_some_eq!(screen.window_1_top, 1);
        assert_some_eq!(screen.window_1_bottom, 10);
        assert_some_eq!(screen.cursor_1, (1, 1));
        assert_eq!(screen.window_0_top, 11);
        assert_eq!(screen.lines_since_input, 0);
        assert_print!("ccccccccccccccc");
    }

    #[test]
    fn test_screen_new_line_window_0() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        screen.split_window(10);
        screen.move_cursor(23, 80);
        screen.new_line();
        assert_eq!(screen.cursor_0, (24, 1));
        assert_some_eq!(screen.window_1_top, 1);
        assert_some_eq!(screen.window_1_bottom, 10);
        assert_some_eq!(screen.cursor_1, (1, 1));
        assert_eq!(screen.window_0_top, 11);
        assert_eq!(screen.lines_since_input, 1);
        assert_print!("");
    }

    #[test]
    fn test_screen_new_line_window_0_scroll() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        screen.split_window(10);
        screen.move_cursor(24, 80);
        screen.new_line();
        assert_eq!(screen.cursor_0, (24, 1));
        assert_some_eq!(screen.window_1_top, 1);
        assert_some_eq!(screen.window_1_bottom, 10);
        assert_some_eq!(screen.cursor_1, (1, 1));
        assert_eq!(screen.window_0_top, 11);
        assert_eq!(screen.lines_since_input, 1);
        assert_print!("");
    }

    #[test]
    fn test_screen_new_line_window_1() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        screen.split_window(10);
        screen.move_cursor(23, 80);
        assert!(screen.select_window(1).is_ok());
        screen.move_cursor(8, 7);
        screen.new_line();
        assert_eq!(screen.cursor_0, (23, 80));
        assert_some_eq!(screen.window_1_top, 1);
        assert_some_eq!(screen.window_1_bottom, 10);
        assert_some_eq!(screen.cursor_1, (9, 1));
        assert_eq!(screen.window_0_top, 11);
        assert_eq!(screen.lines_since_input, 0);
        assert_print!("");
    }

    #[test]
    fn test_screen_new_line_window_1_bottom() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        screen.split_window(10);
        screen.move_cursor(23, 80);
        assert!(screen.select_window(1).is_ok());
        screen.move_cursor(10, 7);
        screen.new_line();
        assert_eq!(screen.cursor_0, (23, 80));
        assert_some_eq!(screen.window_1_top, 1);
        assert_some_eq!(screen.window_1_bottom, 10);
        assert_some_eq!(screen.cursor_1, (10, 7));
        assert_eq!(screen.window_0_top, 11);
        assert_eq!(screen.lines_since_input, 0);
        assert_print!("");
    }

    #[test]
    fn test_screen_read_key_window_0() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        input(&[' ']);
        assert_eq!(screen.read_key(true), InputEvent::from_char(' ' as u16));
    }

    #[test]
    fn test_screen_read_key_window_1() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        screen.split_window(10);
        assert!(screen.select_window(1).is_ok());
        input(&[' ']);
        assert_eq!(screen.read_key(true), InputEvent::from_char(' ' as u16));
    }

    #[test]
    fn test_screen_backspace_window_0() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        screen.move_cursor(10, 10);
        assert!(screen.backspace().is_ok());
        assert_eq!(screen.cursor_0, (10, 9));
        assert_eq!(backspace(), (10, 9));
    }

    #[test]
    fn test_screen_backspace_window_0_left_edge() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        screen.move_cursor(10, 1);
        assert!(screen.backspace().is_ok());
        assert_eq!(screen.cursor_0, (10, 1));
        assert_eq!(backspace(), (0, 0));
    }

    #[test]
    fn test_screen_backspace_window_1() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        screen.move_cursor(18, 10);
        screen.split_window(12);
        assert!(screen.select_window(1).is_ok());
        screen.move_cursor(5, 10);
        assert!(screen.backspace().is_ok());
        assert_eq!(screen.cursor_0, (18, 10));
        assert_eq!(screen.cursor_1.unwrap(), (5, 9));
        assert_eq!(backspace(), (5, 9));
    }

    #[test]
    fn test_screen_backspace_window_1_left_edge() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        screen.move_cursor(18, 10);
        screen.split_window(12);
        assert!(screen.select_window(1).is_ok());
        screen.move_cursor(5, 1);
        assert!(screen.backspace().is_ok());
        assert_eq!(screen.cursor_0, (18, 10));
        assert_eq!(screen.cursor_1.unwrap(), (5, 1));
        assert_eq!(backspace(), (0, 0));
    }

    #[test]
    fn test_screen_set_style() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        assert!(screen.set_style(Style::Italic as u8).is_ok());
        assert_eq!(style(), Style::Italic as u8);
        assert!(screen.set_style(Style::Bold as u8).is_ok());
        assert_eq!(style(), Style::Italic as u8 + Style::Bold as u8);
    }

    #[test]
    fn test_screen_buffer_mode() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        screen.buffer_mode(1);
        assert_eq!(buffer_mode(), 1);
    }

    #[test]
    fn test_screen_beep() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        assert!(!beep());
        screen.beep();
        assert!(beep());
    }

    #[test]
    fn test_screen_reset_cursor_window_0() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        screen.cursor_0 = (23, 80);
        screen.reset_cursor();
        assert_eq!(cursor(), (23, 80));
    }

    #[test]
    fn test_screen_reset_cursor_window_1() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        screen.cursor_0 = (23, 80);
        screen.split_window(12);
        assert!(screen.select_window(1).is_ok());
        screen.cursor_1 = Some((10, 10));
        screen.reset_cursor();
        assert_eq!(cursor(), (10, 10));
    }

    #[test]
    fn test_screen_set_font() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        assert_eq!(screen.font, 1);
        assert_eq!(screen.set_font(3), 1);
        assert_eq!(screen.font, 3);
        assert_eq!(screen.set_font(0), 3);
        assert_eq!(screen.font, 3);
        assert_eq!(screen.set_font(4), 3);
        assert_eq!(screen.font, 4);
        assert_eq!(screen.set_font(1), 4);
        assert_eq!(screen.font, 1);
        assert_eq!(screen.set_font(2), 0);
        assert_eq!(screen.font, 1);
    }

    #[test]
    fn test_screen_ouptut_stream() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        screen.output_stream(5, Some(0x1234));
        assert_eq!(output_stream(), (5, Some(0x1234)));
    }

    #[test]
    fn test_screen_reset() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        screen.reset();
        assert!(reset());
    }

    #[test]
    fn test_screen_quit() {
        let mut screen = assert_ok!(Screen::new_v4(Config::default()));
        screen.quit();
        assert!(quit());
    }
}
