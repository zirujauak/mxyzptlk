use pancurses::{Window, Input, COLOR_GREEN, COLOR_BLACK};

use super::{Interpreter, Spec};
use crate::executor::{state::State, header::{self, Flag}};

pub struct Curses {
    version: u8,
    window: Window,
    lines: u16,
    columns: u16,
    selected_window: u8,
    split_line: Option<u16>,
    cursor_position: Vec<(u16,u16)>,
    output_streams: Vec<bool>,
    buffering: bool
}

impl Curses {
    pub fn new(version: u8) -> Curses {
        let mut window = pancurses::initscr();
        window.erase();

        let cursor:(u16,u16) = if version < 5 {
            (24, 1)
        } else {
            (1,1)
        };

        let output_streams = if version < 3 {
            vec![true, false]
        } else {
            vec![true, false, false, false]
        };

        let cursor_position = vec![cursor, (1,1)];

        Self {
            version,
            window: pancurses::initscr(),
            lines: window.get_max_y() as u16,
            columns: window.get_max_x() as u16,
            selected_window: 0,
            split_line: None,
            cursor_position,
            output_streams,
            buffering: true
        }
    }
}

impl Interpreter for Curses {
    fn buffer_mode(&mut self, mode: bool) {
        self.buffering = mode
    }

    fn erase_line(&mut self, value: u16) {
        todo!()
    }

    fn erase_window(&mut self, window: i16) {
        match window {
                -1 => {
                self.selected_window = 0;
                self.split_line = None;
                self.window.erase();
            },
            -2 => {
                self.window.erase();
            }
            _ => {
                trace!("TODO: ERASE_WINDOW {}", window)
            }
        }
    }

    fn get_cursor(&mut self) -> (u16, u16) {
        self.cursor_position[self.selected_window as usize]
    }

    fn input_stream(&mut self, stream: u16) {
        todo!()
    }
    fn new_line(&mut self) {
        self.window.addch('\n');
    }
    fn output_stream(&mut self, stream: i16, table: usize) {
        let stream_index = stream.abs() as usize - 1;
        self.output_streams[stream_index] = stream > 0;
    }
    fn print(&mut self, text: String) {
        if self.output_streams[0] {
            if self.buffering {
                // Split the text string on spaces
                let frags = text.split_inclusive(&[' ']);
                // Iterate over the fragments
                for s in frags {
                    let position = (self.window.get_cur_y() + 1, self.window.get_cur_x() + 1);
                    trace!("buffer check: {} {}", position.1, s.len());
                    if self.columns as i32 - position.1 < s.len() as i32 {
                        self.window.addch('\n');
                        self.window.addstr(s);
                    } else {
                        self.window.addstr(s);
                    }
                }
            } else {
                self.window.addstr(text);
            }
            self.window.refresh();
            self.cursor_position[self.selected_window as usize] = (self.window.get_cur_y() as u16 + 1, self.window.get_cur_x() as u16 + 1);
        }

        trace!("cursor: {},{}", self.cursor_position[0].0, self.cursor_position[0].1)
    }

    fn print_table(&mut self, text: String, width: u16, height: u16, skip: u16) {
        todo!()
    }

    fn read(&mut self, length: u8, time: u16) -> String {
        todo!()
    }

    fn read_char(&mut self, time: u16) -> char {
        pancurses::noecho();
        match self.window.getch().unwrap() {
            Input::Character(c) => c,
            _ => ' '
        }
    }

    fn set_colour(&mut self, foreground: u16, background: u16) {
        todo!()
    }

    fn set_cursor(&mut self, line: u16, column: u16) {
        self.window.mv(line as i32 - 1, column as i32 - 1);
    }

    fn set_text_style(&mut self, style: u16) {
        todo!()
    }

    fn set_window(&mut self, window: u16) {
        todo!()
    }

    fn sound_effect(&mut self, number: u16, effect: u16, volume: u8, repeats: u8) {
        todo!()
    }

    fn split_window(&mut self, lines: u16) {
        if lines == 0 {
            // Unsplit
        } else {
            if version == 3 {
                // Split off {lines} lines
                // Clear the upper window (1)
            } else {
                // Split off {lines} lines
                // If cursor is in upper window, move cursor to first line
                // in lower window (0)
            }
        }
        todo!()
    }
}

impl Curses {
    pub fn spec(&self, version: u8) -> Spec {
        let set_flags = match version {
            1 | 2 | 3 => vec![Flag::ScreenSplittingAvailable],
            4 | 5 | 6 | 7 | 8 => vec![Flag::ColoursAvailable, Flag::BoldfaceAvailable, Flag::ItalicAvailable,
            Flag::FixedSpaceAvailable],
            _ => vec![]
        };
        let clear_flags = match version {
            1 | 2 | 3 => vec![Flag::StatusLineNotAvailable, Flag::VariablePitchDefaultFont],
            4 | 5 | 6 | 7 | 8 => vec![Flag::PicturesAvailable, Flag::SoundEffectsAvailable, Flag::TimedInputAvailable],
            _ => vec![]
        };

        pancurses::curs_set(0);
        pancurses::start_color();
        pancurses::init_pair(1, COLOR_GREEN, COLOR_BLACK);
        self.window.color_set(1);

        Spec {
            set_flags,
            clear_flags,
            interpreter_number: 6,
            interpreter_version: 'A' as u8,
            screen_lines: self.lines as u8,
            screen_columns: self.columns as u8,
            line_units: 1,
            column_units: 1,
            background_color: 2,
            foreground_color: 4
        }
    }
}