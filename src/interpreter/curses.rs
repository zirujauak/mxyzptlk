use pancurses::{Attribute, Input, Window, COLOR_BLACK, COLOR_GREEN, ToChtype, chtype};

use super::{Interpreter, Spec};
use crate::executor::{
    header::Flag,
    text,
};

pub struct Curses {
    version: u8,
    window: Window,
    status_window: Option<Window>,
    sub_windows: Vec<Window>,
    lines: u16,
    columns: u16,
    selected_window: u8,
    split_line: Option<u16>,
    output_streams: Vec<bool>,
    buffering: bool,
}

impl Curses {
    pub fn new(version: u8) -> Curses {
        let window = pancurses::initscr();
        let mut sub_windows = Vec::new();
        let status_window = if version < 4 {
            Some(window.subwin(1, window.get_max_x(), 0, 0).unwrap())
        } else {
            None
        };

        if version < 4 {
            let window_0 = window
                .subwin(window.get_max_y() - 1, window.get_max_x(), 1, 0)
                .unwrap();
            let window_1 = window.subwin(0, 0, 1, 0).unwrap();
            sub_windows.push(window_0);
            sub_windows.push(window_1);
        } else {
            let window_0 = window
                .subwin(window.get_max_y(), window.get_max_x(), 0, 0)
                .unwrap();
            let window_1 = window.subwin(0, 0, 0, 0).unwrap();
            sub_windows.push(window_0);
            sub_windows.push(window_1);
        };

        window.erase();

        let output_streams = if version < 3 {
            vec![true, false]
        } else {
            vec![true, false, false, false]
        };

        let lines = window.get_max_y() as u16;
        let columns = window.get_max_x() as u16;
        Self {
            version,
            window,
            status_window,
            sub_windows,
            lines,
            columns,
            selected_window: 0,
            split_line: None,
            output_streams,
            buffering: true,
        }
    }

    fn current_window(&mut self) -> &mut Window {
        &mut self.sub_windows[self.selected_window as usize]
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
                self.current_window().erase();
            }
            -2 => {
                self.current_window().erase();
            }
            _ => {
                trace!("TODO: ERASE_WINDOW {}", window)
            }
        }
    }

    fn get_cursor(&mut self) -> (u16, u16) {
        (
            self.current_window().get_cur_y() as u16 + 1,
            self.current_window().get_cur_x() as u16 + 1,
        )
    }

    fn input_stream(&mut self, stream: u16) {
        todo!()
    }
    fn new_line(&mut self) {
        self.current_window().addch('\n');
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
                        self.current_window().get_cur_y() + 1,
                        self.current_window().get_cur_x() + 1,
                    );
                    trace!("buffer check: {} {}", position.1, s.len());
                    if self.columns as i32 - position.1 < s.len() as i32 {
                        self.current_window().addch('\n');
                        self.current_window().addstr(s);
                    } else {
                        self.current_window().addstr(s);
                    }
                }
            } else {
                self.current_window().addstr(text);
            }
            self.current_window().refresh();
        };

        trace!(
            "cursor: {},{}",
            self.current_window().get_cur_y() + 1,
            self.current_window().get_cur_x() + 1
        )
    }

    fn print_table(&mut self, text: String, width: u16, height: u16, skip: u16) {
        todo!()
    }

    fn read(&mut self, length: u8, time: u16) -> Vec<char> {
        pancurses::curs_set(1);
        self.window.mv(
            self.sub_windows[0].get_cur_y(),
            self.sub_windows[0].get_cur_x(),
        );

        let mut input: Vec<char> = Vec::new();
        let mut done = false;
        while !done {
            let c = self.window.getch().unwrap();
            match c {
                Input::Character(ch) => match ch {
                    // Backspace
                    '\u{7f}' => {
                        if input.len() > 0 {
                            // Remove from the input array
                            input.pop();
                            // Back cursor up and delete character
                            self.window
                                .mv(self.window.get_cur_y(), self.window.get_cur_x() - 1);
                            self.window.delch();
                        }
                    }
                    // 
                    _ => {
                        if input.len() < length as usize && text::valid_input(ch) {
                            input.push(ch);
                            self.window.addstr(&format!("{}", ch));
                        }
                        done = ch == '\n';
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
        match self.current_window().getch().unwrap() {
            Input::Character(c) => c,
            _ => ' ',
        }
    }

    fn set_colour(&mut self, foreground: u16, background: u16) {
        todo!()
    }

    fn set_cursor(&mut self, line: u16, column: u16) {
        self.current_window().mv(line as i32 - 1, column as i32 - 1);
    }

    fn set_text_style(&mut self, style: u16) {
        let win = &mut self.sub_windows[self.selected_window as usize];
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
        self.selected_window = window as u8;
        self.window.mv(
            self.sub_windows[window as usize].get_cur_y(),
            self.sub_windows[window as usize].get_cur_x(),
        );
    }
    fn show_status(&mut self, location: &str, status: &str) {}
    fn sound_effect(&mut self, number: u16, effect: u16, volume: u8, repeats: u8) {
        todo!()
    }
    fn split_window(&mut self, lines: u16) {
        if lines == 0 {
            self.sub_windows[1].resize(0, 0);
            // Unsplit
        } else {
            if self.version == 3 {
                let win = &mut self.sub_windows[1];
                // Resize windows 1
                win.resize(lines as i32, self.columns as i32);
                // Clear the upper window
                win.clear();
            } else {
                trace!(
                    "Window 0 cursor before split: {}, {}",
                    self.sub_windows[0].get_cur_y(),
                    self.sub_windows[0].get_cur_x()
                );
                self.sub_windows[1].resize(lines as i32, self.columns as i32);
                trace!(
                    "Window 0 cursor after split: {}, {}",
                    self.sub_windows[0].get_cur_y(),
                    self.sub_windows[0].get_cur_x()
                );
                // Resize window 1

                // If cursor is in upper window, move cursor to first line
                // in lower window (0)
            }
        }
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

        self.window.color_set(1);
        self.sub_windows[0].color_set(1);
        self.sub_windows[1].color_set(1);

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
