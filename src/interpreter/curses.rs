use pancurses::{Attribute, Input, Window, COLOR_BLACK, COLOR_GREEN, ToChtype, chtype};

use super::{Interpreter, Spec};
use crate::executor::{
    header::Flag,
    text,
};

pub struct Curses {
    version: u8,
    window_0: Window,
    window_1: Window,
    status_window: Option<Window>,
    lines: i32,
    columns: i32,
    selected_window: u8,
    output_streams: Vec<bool>,
    buffering: bool,
}

impl Curses {
    pub fn new(version: u8) -> Curses {
        let window_0 = pancurses::initscr();
        let lines = window_0.get_max_y();
        let columns = window_0.get_max_x();
        let status_window = if version < 4 {
            window_0.setscrreg(1, lines);
            Some(window_0.subwin(1, columns, 0, 0).unwrap())
        } else {
            window_0.setscrreg(0, 0);
            None
        };
        window_0.scrollok(true);

        let window_1 = if version < 4 {
            window_0.subwin(0, 0, 1, 0).unwrap()
        } else {
            window_0.subwin(0, 0, 0, 0).unwrap()
        };

        window_0.erase();
        window_1.scrollok(false);

        let output_streams = if version < 3 {
            vec![true, false]
        } else {
            vec![true, false, false, false]
        };

        Self {
            version,
            window_0,
            window_1,
            status_window,
            lines,
            columns,
            selected_window: 0,
            output_streams,
            buffering: true,
        }
    }

    fn current_window_mut(&mut self) -> &mut Window {
        match self.selected_window {
            1 => &mut self.window_1,
            _ => &mut self.window_0,
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
                self.window_1.resize(0,0);
                // TODO: Account for status line window
                self.window_0.setscrreg(0, self.lines - 1);
                self.current_window_mut().erase();
            }
            -2 => {
                self.current_window_mut().erase();
            }
            _ => {
                trace!("TODO: ERASE_WINDOW {}", window)
            }
        }
    }

    fn get_cursor(&mut self) -> (u16, u16) {
        (
            self.current_window_mut().get_cur_y() as u16 + 1,
            self.current_window_mut().get_cur_x() as u16 + 1,
        )
    }

    fn input_stream(&mut self, stream: u16) {
        todo!()
    }
    fn new_line(&mut self) {
        self.current_window_mut().addch('\n');
        self.current_window_mut().refresh();
    }
    fn output_stream(&mut self, stream: i16, table: usize) {
        let stream_index = stream.abs() as usize - 1;
        self.output_streams[stream_index] = stream > 0;
    }
    fn print(&mut self, text: String) {
        if self.output_streams[0] {
            if self.buffering || self.selected_window == 1 {
                // Split the text string on spaces
                let frags = text.split_inclusive(&[' ']);
                // Iterate over the fragments
                for s in frags {
                    let position = (
                        self.current_window_mut().get_cur_y() + 1,
                        self.current_window_mut().get_cur_x() + 1,
                    );
                    trace!("buffer check: {} {}", position.1, s.len());
                    if self.columns as i32 - position.1 < s.len() as i32 {
                        self.current_window_mut().addch('\n');
                        self.current_window_mut().addstr(s);
                    } else {
                        self.current_window_mut().addstr(s);
                    }
                }
            } else {
                self.current_window_mut().addstr(text);
            }
            self.current_window_mut().refresh();
        };
    }

    fn print_table(&mut self, text: String, width: u16, height: u16, skip: u16) {
        todo!()
    }

    fn read(&mut self, length: u8, time: u16) -> Vec<char> {
        self.window_1.refresh();
        self.window_0.refresh();       
        self.window_0.mv(self.window_0.get_cur_y(), self.window_0.get_cur_x());
        pancurses::curs_set(1);

        let mut input: Vec<char> = Vec::new();
        let mut done = false;
        while !done {
            let c = self.window_0.getch().unwrap();
            match c {
                Input::Character(ch) => match ch {
                    // Backspace
                    '\u{7f}' => {
                        if input.len() > 0 {
                            // Remove from the input array
                            input.pop();
                            // Back cursor up and delete character
                            self.window_0
                                .mv(self.window_0.get_cur_y(), self.window_0.get_cur_x() - 1);
                            self.window_0.delch();
                            self.window_0.refresh();
                        }
                    }
                    // 
                    _ => {
                        if input.len() < length as usize && text::valid_input(ch) {
                            input.push(ch);
                            self.window_0.addstr(&format!("{}", ch));
                            self.window_0.refresh();
                        }
                        if ch == '\n' {
                            done = true;
                            self.window_0.addch('\n');
                        }
                    }
                },
                _ => {}
            }
        }

        pancurses::curs_set(0);
        input
    }

    fn read_char(&mut self, time: u16) -> char {
        pancurses::noecho();
        match self.current_window_mut().getch().unwrap() {
            Input::Character(c) => c,
            _ => ' ',
        }
    }

    fn set_colour(&mut self, foreground: u16, background: u16) {
        todo!()
    }

    fn set_cursor(&mut self, line: u16, column: u16) {
        self.current_window_mut().mv(line as i32 - 1, column as i32 - 1);
    }

    fn set_text_style(&mut self, style: u16) {
        let win = &mut self.current_window_mut();
        if style == 0 {
            win.attroff(Attribute::Reverse);
            win.attroff(Attribute::Bold);
            win.attroff(Attribute::Italic);
        } else {
            if style & 0x1 == 0x1 {
                win.attron(Attribute::Reverse);
            }
            if style & 0x2 == 0x2 {
                win.attron(Attribute::Bold);
            }
            if style & 0x4 == 0x4 {
                win.attron(Attribute::Italic);
            }
        }
    }

    fn set_window(&mut self, window: u16) {
        pancurses::curs_set(0);
        self.selected_window = window as u8;
        if window == 1 {
            self.current_window_mut().mv(0,0);
        }
    }
    fn show_status(&mut self, location: &str, status: &str) {}
    fn sound_effect(&mut self, number: u16, effect: u16, volume: u8, repeats: u8) {
        todo!()
    }
    fn split_window(&mut self, lines: u16) {
        if lines == 0 {
            // Unsplit
            self.window_1.resize(0, 0);
            // TODO: Account for status line window
            self.window_0.setscrreg(0, self.lines - 1);
        } else {
            if self.version == 3 {
                // Resize and move window 0
                self.window_0.setscrreg(lines as i32, self.lines - 1);

                // Resize windows 1
                self.window_1.resize(lines as i32, self.columns as i32);

                // Clear the upper window
                self.window_1.erase();
            } else {
                // Resize and move window 0
                self.window_0.setscrreg(lines as i32, self.lines - 1);
                // Resize window 1
                self.window_1.resize(lines as i32, self.columns as i32);
                // If cursor is in upper window, move cursor to first line
                // in lower window (0)
            }
        }

        self.window_0.refresh();
        self.window_1.refresh();

    }
}

impl Curses {
    pub fn spec(&self, version: u8) -> Spec {
        let set_flags = match version {
            1 | 2 | 3 => vec![Flag::ScreenSplittingAvailable],
            4 | 5 | 6 | 7 | 8 => vec![
                Flag::ColoursAvailable,
                Flag::BoldfaceAvailable,
                Flag::ItalicAvailable,
                Flag::FixedSpaceAvailable,
            ],
            _ => vec![],
        };
        let clear_flags = match version {
            1 | 2 | 3 => vec![Flag::StatusLineNotAvailable, Flag::VariablePitchDefaultFont],
            4 | 5 | 6 | 7 | 8 => vec![
                Flag::PicturesAvailable,
                Flag::SoundEffectsAvailable,
                Flag::TimedInputAvailable,
            ],
            _ => vec![],
        };

        pancurses::curs_set(0);
        pancurses::start_color();
        pancurses::init_pair(1, COLOR_GREEN, COLOR_BLACK);
        pancurses::init_pair(2, COLOR_GREEN, COLOR_BLACK);
        pancurses::init_pair(3, COLOR_BLACK, COLOR_GREEN);

        match &self.status_window {
            Some(w) => w.color_set(3),
            None => 0,
        };

        self.window_0.color_set(1);
        self.window_1.color_set(1);

        // TODO: Account for status line window
        self.window_0.setscrreg(0, 0);
        self.window_0.scrollok(true);

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
            foreground_color: 4,
        }
    }
}
