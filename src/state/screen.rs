mod buffer;
mod easy_curses;

use crate::error::*;
use buffer::Buffer;
use buffer::CellStyle;
use easy_curses::ECTerminal;

#[derive(Clone, Copy)]
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

pub struct Screen {
    version: u8,
    rows: u32,
    columns: u32,
    buffer: Buffer,
    status_line: bool,
    window_1_top: Option<u32>,
    window_1_bottom: Option<u32>,
    window_0_top: u32,
    selected_window: u8,
    // foreground, background
    default_colors: (Color, Color),
    current_colors: (Color, Color),
    current_style: CellStyle,
    // row, column with 1,1 as origin
    cursor_0: (u32, u32),
    cursor_1: Option<(u32, u32)>,
    buffered: bool,
    terminal: Box<dyn Terminal>,
}

impl Screen {
    pub fn new_v3(foreground: Color, background: Color) -> Screen {
        let terminal = Box::new(ECTerminal::new());
        let (rows, columns) = terminal.as_ref().size();
        let buffer = Buffer::new(rows, columns, (foreground, background));

        Screen {
            version: 3,
            rows,
            columns,
            buffer,
            status_line: true,
            window_0_top: 2,
            window_1_top: None,
            window_1_bottom: None,
            selected_window: 0,
            default_colors: (foreground.clone(), background.clone()),
            current_colors: (foreground.clone(), background.clone()),
            current_style: CellStyle::new(),
            cursor_0: (rows, 1),
            cursor_1: None,
            buffered: true,
            terminal,
        }
    }

    pub fn new_v4(foreground: Color, background: Color) -> Screen {
        let terminal = Box::new(ECTerminal::new());
        let (rows, columns) = terminal.as_ref().size();
        let buffer = Buffer::new(rows, columns, (foreground, background));

        Screen {
            version: 4,
            rows,
            columns,
            buffer,
            status_line: false,
            window_0_top: 1,
            window_1_top: None,
            window_1_bottom: None,
            selected_window: 0,
            default_colors: (foreground.clone(), background.clone()),
            current_colors: (foreground.clone(), background.clone()),
            current_style: CellStyle::new(),
            cursor_0: (rows, 1),
            cursor_1: None,
            buffered: true,
            terminal,
        }
    }

    pub fn new_v5(foreground: Color, background: Color) -> Screen {
        let terminal = Box::new(ECTerminal::new());
        let (rows, columns) = terminal.as_ref().size();
        let buffer = Buffer::new(rows, columns, (foreground, background));

        Screen {
            version: 5,
            rows,
            columns,
            buffer,
            status_line: false,
            window_1_top: None,
            window_1_bottom: None,
            window_0_top: 1,
            selected_window: 0,
            default_colors: (foreground.clone(), background.clone()),
            current_colors: (foreground.clone(), background.clone()),
            current_style: CellStyle::new(),
            cursor_0: (1, 1),
            cursor_1: None,
            buffered: true,
            terminal,
        }
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
            self.cursor_1.unwrap()
        }
    }

    pub fn move_cursor(&mut self, row: u32, column: u32) {
        if self.selected_window == 0 {
            self.cursor_0 = (row, column)
        } else {
            self.cursor_1 = Some((row, column))
        }
    }

    pub fn set_colors(&mut self, foreground: Color, background: Color) {
        self.current_colors = (foreground, background);
    }

    pub fn split_window(&mut self, lines: u32) {
        let top = match self.status_line {
            true => 2,
            false => 1,
        };
        if lines == 0 {
            self.window_0_top = top;
            self.window_1_top = None;
            self.window_1_bottom = None;
            self.cursor_1 = None;
        } else {
            let bottom = top + lines - 1;
            self.window_1_top = Some(top);
            self.window_1_bottom = Some(bottom);
            self.cursor_1 = Some((1, 1));
            self.window_0_top = bottom + 1;
            if self.cursor_0.0 < self.window_0_top {
                self.cursor_0 = (self.window_0_top, self.cursor_0.1)
            }
            if self.version == 3 {
                for i in self.window_1_top.unwrap()..self.window_1_bottom.unwrap() {
                    for j in 1..self.columns {
                        self.buffer
                            .clear(&mut self.terminal, self.current_colors, (i, j));
                    }
                }
            }
        }
    }

    pub fn select_window(&mut self, window: u8) {
        if window == 0 {
            self.selected_window = 0;
        } else if let Some(_) = self.cursor_1 {
            self.selected_window = 1;
        } // Else error
    }

    pub fn erase_window(&mut self, window: i8) {
        match window {
            0 => {
                for i in self.window_0_top..self.rows {
                    for j in 1..self.columns {
                        self.buffer
                            .clear(&mut self.terminal, self.current_colors, (i, j));
                    }
                }
                self.cursor_0 = if self.version == 4 {
                    (self.rows, 1)
                } else {
                    (self.window_0_top, 1)
                };
            }
            1 => {
                if let Some(start) = self.window_1_top {
                    if let Some(end) = self.window_1_bottom {
                        for i in start..end {
                            for j in 1..self.columns {
                                self.buffer
                                    .clear(&mut self.terminal, self.current_colors, (i, j));
                            }
                        }
                        self.cursor_1 = Some((start, 1))
                    }
                }
            }
            -1 => {
                self.window_1_top = None;
                self.window_1_bottom = None;
                self.cursor_1 = None;
                self.window_0_top = 1;
                for i in self.window_0_top..self.rows {
                    for j in 1..self.columns {
                        self.buffer
                            .clear(&mut self.terminal, self.current_colors, (i, j));
                    }
                }
                self.cursor_0 = if self.version == 4 {
                    (self.rows, 1)
                } else {
                    (self.window_0_top, 1)
                };
                self.selected_window = 0;
            }
            -2 => {
                for i in 1..self.rows {
                    for j in 1..self.columns {
                        self.buffer
                            .clear(&mut self.terminal, self.current_colors, (i, j));
                    }
                    if let Some(_) = self.cursor_1 {
                        self.cursor_1 = Some((1, 1))
                    }
                    self.cursor_0 = if self.version == 4 {
                        (self.rows, 1)
                    } else {
                        (self.window_0_top, 1)
                    };
                }
            }
            _ => (), // This is an error
        }
    }

    pub fn erase_line(&mut self) {
        let (row, col) = if self.selected_window == 0 {
            self.cursor_0
        } else {
            self.cursor_1.unwrap()
        };
        for i in col..self.columns {
            self.buffer
                .clear(&mut self.terminal, self.current_colors, (row, i));
        }
    }

    fn advance_cursor(&mut self) {
        if self.selected_window == 0 {
            if self.cursor_0.1 == self.columns {
                // At the end of the row
                if self.cursor_0.0 == self.rows {
                    // At bottom of screen, scroll window 0 up 1 row and set the cursor to the bottom left
                    self.buffer.scroll(&mut self.terminal, self.window_0_top, self.current_colors);
                    self.cursor_0 = (self.rows, 1);
                } else {
                    // Not at the bottom, so just move the cursor to the start of the next line
                    self.cursor_0 = (self.cursor_0.0 + 1, 1)
                }
            } else {
                // Just move the cursor to the right
                self.cursor_0 = (self.cursor_0.0, self.cursor_0.1 + 1)
            }
        } else {
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

    pub fn print_word(&mut self, word: &Vec<u16>) {
        for c in word {
            self.print_char(*c);
        }
    }

    pub fn print(&mut self, text: &Vec<u16>) {
        if self.selected_window == 1 || !self.buffered {
            self.print_word(text);
        } else {
            let words = text.split_inclusive(|c| *c == 0x20);
            for word in words {
                if self.columns - self.cursor_0.1 < word.len() as u32 {
                    self.new_line();
                }
                self.print_word(&word.to_vec());
            }
        }

        self.terminal.flush();
    }

    fn print_char(&mut self, zchar: u16) {
        if zchar == 0xd {
            self.new_line();
        } else {
            if self.selected_window == 0 {
                self.buffer.print(
                    &mut self.terminal,
                    zchar,
                    self.current_colors,
                    &self.current_style,
                    self.cursor_0,
                );
            } else {
                self.buffer.print(
                    &mut self.terminal,
                    zchar,
                    self.current_colors,
                    &self.current_style,
                    self.cursor_1.unwrap(),
                );
            }
            self.advance_cursor();
        }
    }

    pub fn new_line(&mut self) {
        if self.selected_window == 0 {
            if self.cursor_0.0 == self.rows {
                self.buffer.scroll(&mut self.terminal, self.window_0_top, self.current_colors);
                self.cursor_0 = (self.rows, 1)
            } else {
                self.cursor_0 = (self.cursor_0.0 + 1, 1);
            }
        } else {
            if self.cursor_1.unwrap().0 < self.window_1_bottom.unwrap() {
                self.cursor_1 = Some((self.cursor_1.unwrap().0 + 1, 1));
            }
        }
    }

    pub fn flush_buffer(&mut self) -> Result<(), RuntimeError> {
        self.terminal.flush();
        Ok(())
    }

    pub fn read_key(&mut self) {
        self.terminal.read_key();
    }
}

pub trait Terminal {
    fn size(&self) -> (u32, u32);
    fn print_at(&mut self, c: char, row: u32, cursor: u32, colors: (Color, Color));
    fn flush(&mut self);
    fn read_key(&mut self);
    fn scroll(&mut self, row: u32);
}
