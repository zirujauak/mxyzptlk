mod curses;

use crate::config::Config;
use crate::error::*;

#[cfg(feature = "easycurses")]
use curses::easy_curses::*;

#[cfg(all(feature = "pancurses", not(feature = "easycurses"), not(test)))]
use curses::pancurses::*;

#[cfg(test)]
use curses::test_terminal::new_terminal;

#[derive(Clone, Copy, Debug)]
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

#[derive(Clone, Copy)]
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

#[derive(Debug)]
pub enum Interrupt {
    ReadTimeout,
    Sound,
}

#[derive(Debug)]
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
        if self.selected_window == 0 {
            self.cursor_0 = (row, column);
        } else {
            self.cursor_1 = Some((row, column));
        }
        self.terminal.move_cursor((row, column));
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
                    for j in 1..self.columns {
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
                            for j in 1..self.columns {
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
                for i in 1..self.rows {
                    for j in 1..self.columns {
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
            if self.selected_window == 0 {
                self.terminal.print_at(
                    zchar,
                    self.cursor_0.0,
                    self.cursor_0.1,
                    self.current_colors,
                    &self.current_style,
                    self.font,
                );
            } else {
                // unwrap() should be safe here because when selected_window
                // is 1, cursor_1 is Some
                self.terminal.print_at(
                    zchar,
                    self.cursor_1.unwrap().0,
                    self.cursor_1.unwrap().1,
                    self.current_colors,
                    &self.current_style,
                    self.font,
                );
            }
            self.advance_cursor();
        }
    }

    pub fn print_at(&mut self, text: &[u16], at: (u32, u32), style: &CellStyle) {
        for (i, c) in text.iter().enumerate() {
            self.terminal.print_at(
                *c,
                at.0,
                at.1 + i as u32,
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
        self.terminal
            .backspace((self.cursor_0.0, self.cursor_0.1 - 1));
        self.cursor_0 = (self.cursor_0.0, self.cursor_0.1 - 1);
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
