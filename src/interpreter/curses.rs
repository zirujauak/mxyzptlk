use std::{
    fs,
    io::Write,
    thread,
    time::{self, SystemTime, UNIX_EPOCH},
};

use pancurses::{
    Attribute, ColorPair, Input, Window, COLOR_BLACK, COLOR_BLUE, COLOR_CYAN, COLOR_GREEN,
    COLOR_MAGENTA, COLOR_RED, COLOR_WHITE, COLOR_YELLOW,
};

use super::{Interpreter, Spec};
use crate::executor::{header::Flag, text};

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
    top_line: i32,
    foreground: i16,
    background: i16,
}

impl Curses {
    pub fn new(version: u8) -> Curses {
        let window_0 = pancurses::initscr();
        let lines = window_0.get_max_y();
        let columns = window_0.get_max_x();
        let top_line = if version < 4 { 1 } else { 0 };
        window_0.setscrreg(top_line, lines - 1);
        let status_window = if version < 4 {
            Some(window_0.subwin(1, columns, 0, 0).unwrap())
        } else {
            None
        };
        window_0.scrollok(true);
        window_0.erase();

        let window_1 = window_0.subwin(0, 0, top_line, 0).unwrap();

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
            top_line,
            foreground: COLOR_GREEN,
            background: COLOR_BLACK,
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
                self.window_1.resize(0, 0);
                self.window_0.setscrreg(self.top_line, self.lines - 1);
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

    fn read(&mut self, length: u8, time: u16, existing_input: &Vec<char>, redraw: bool) -> (Vec<char>, bool) {
        self.window_1.refresh();
        self.window_0.refresh();
        self.window_0
            .mv(self.window_0.get_cur_y(), self.window_0.get_cur_x());
        pancurses::curs_set(1);
        pancurses::noecho();

        if redraw {
            for c in existing_input {
                self.current_window_mut().addch(*c);
            }
        }
        
        // Current time, in seconds
        let start = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        // Add the time offset (in seconds)
        let end = if time > 0 {
            start + (time as u128 * 1000)
        } else {
            0
        };
        let delay = time::Duration::from_millis(10);

        let mut input: Vec<char> = existing_input.clone();
        let mut done = false;
        self.current_window_mut().nodelay(true);
        while !done {
            if time > 0 && SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis()
                > end
            {
                return (input, true);
            } else {
                let c = self.window_0.getch();
                match c {
                    Some(ch) => {
                        match ch {
                            Input::Character(cx) => match cx {
                                // Backspace
                                '\u{7f}' => {
                                    if input.len() > 0 {
                                        // Remove from the input array
                                        input.pop();
                                        // Back cursor up and delete character
                                        self.window_0.mv(
                                            self.window_0.get_cur_y(),
                                            self.window_0.get_cur_x() - 1,
                                        );
                                        self.window_0.delch();
                                        self.window_0.refresh();
                                    }
                                }
                                //
                                _ => {
                                    if input.len() < length as usize && text::valid_input(cx) {
                                        input.push(cx);
                                        self.window_0.addstr(&format!("{}", cx));
                                        self.window_0.refresh();
                                    }
                                    if input.len() < length as usize && cx == '\n' {
                                        input.push(cx);
                                        done = true;
                                        self.window_0.addch('\n');
                                    }
                                }
                            },
                            _ => {
                                // Brief sleep
                                thread::sleep(delay);
                            }
                        }
                    }
                    None => {
                        // Brief sleep
                        thread::sleep(delay);
                    }
                }
            }
        }

        pancurses::curs_set(0);
        (input, false)
    }

    fn read_char(&mut self, time: u16) -> char {
        pancurses::noecho();
        if time > 0 {
            // Current time, in seconds
            let start = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            // Add the time offset (in seconds)
            let end = start + time as u64;
            // Add delay to getch() calls to avoid busy wait
            let delay = time::Duration::from_millis(10);
            // Don't block on input
            self.current_window_mut().nodelay(true);
            let mut ch = self.current_window_mut().getch();
            let mut result = 0 as char;
            // While no (acceptable) keypress and 'time' seconds haven't elapsed
            while result == 0 as char
                && SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    < end
            {
                ch = self.current_window_mut().getch();
                result = match ch {
                    Some(i) => match i {
                        // Valid input
                        Input::Character(c) => c,
                        _ => 0 as char,
                    },
                    _ => 0 as char,
                };

                // Brief sleep
                thread::sleep(delay);
            }

            // Re-enable block on input
            result
        } else {
            self.current_window_mut().nodelay(false);
            match self.current_window_mut().getch() {
                Some(ch) => match ch {
                    Input::Character(c) => c,
                    _ => ' ',
                },
                None => ' ',
            }
        }
    }

    fn set_colour(&mut self, foreground: u16, background: u16) {
        match foreground {
            2 => self.foreground = COLOR_BLACK,
            3 => self.foreground = COLOR_RED,
            1 | 4 => self.foreground = COLOR_GREEN,
            5 => self.foreground = COLOR_YELLOW,
            6 => self.foreground = COLOR_BLUE,
            7 => self.foreground = COLOR_MAGENTA,
            8 => self.foreground = COLOR_CYAN,
            9 => self.foreground = COLOR_WHITE,
            _ => {}
        };
        match background {
            1 | 2 => self.background = COLOR_BLACK,
            3 => self.background = COLOR_RED,
            4 => self.background = COLOR_GREEN,
            5 => self.background = COLOR_YELLOW,
            6 => self.background = COLOR_BLUE,
            7 => self.background = COLOR_MAGENTA,
            8 => self.background = COLOR_CYAN,
            9 => self.background = COLOR_WHITE,
            _ => {}
        };

        let pair = Curses::color_pair(self.foreground as i16, self.background as i16);
        self.window_0.color_set(pair);
        self.window_1.color_set(pair);
    }

    fn set_cursor(&mut self, line: u16, column: u16) {
        self.current_window_mut()
            .mv(line as i32 - 1, column as i32 - 1);
    }

    fn set_text_style(&mut self, style: u16) {
        let win = &mut self.current_window_mut();
        if style == 0 {
            win.attroff(Attribute::Reverse);
            win.attroff(Attribute::Bold);
            win.attroff(Attribute::Underline);
        } else {
            if style & 0x1 == 0x1 {
                win.attron(Attribute::Reverse);
            }
            if style & 0x2 == 0x2 {
                win.attron(Attribute::Bold);
            }
            if style & 0x4 == 0x4 {
                win.attron(Attribute::Underline);
            }
        }
    }

    fn set_window(&mut self, window: u16) {
        pancurses::curs_set(0);
        self.selected_window = window as u8;
        if window == 1 {
            self.current_window_mut().mv(0, 0);
        }
    }
    fn show_status(&mut self, location: &str, status: &str) {
        self.status_window.as_mut().unwrap().mv(0, 0);
        self.status_window
            .as_mut()
            .unwrap()
            .addstr(String::from_utf8(vec![32; self.columns as usize]).unwrap());
        self.status_window
            .as_mut()
            .unwrap()
            .mvaddstr(0, 1, location);
        let x = self.columns - 1 - status.len() as i32;
        self.status_window.as_mut().unwrap().mvaddstr(0, x, status);
        self.status_window.as_mut().unwrap().refresh();
    }

    fn sound_effect(&mut self, number: u16, effect: u16, volume: u8, repeats: u8) {
        todo!()
    }
    fn split_window(&mut self, lines: u16) {
        if lines == 0 {
            // Unsplit
            self.window_1.resize(0, 0);
            self.window_0.setscrreg(self.top_line, self.lines - 1);
        } else {
            if self.version < 4 {
                // Resize and move window 0
                self.window_0
                    .setscrreg(lines as i32 + self.top_line, self.lines - 1);

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

    fn save(&mut self, name: &String, data: &Vec<u8>) {
        let filename = format!("{}.ifzs", name);
        trace!("Save to: {}", filename);

        let mut file = fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(filename)
            .unwrap();

        file.write_all(&data).unwrap();
        file.flush().unwrap();
    }

    fn restore(&mut self, name: &String) -> Vec<u8> {
        let filename = format!("{}.ifzs", name);
        trace!("Restore from: {}", filename);

        fs::read(filename).unwrap()
    }
}

const COLOR_TABLE: [i16; 8] = [
    COLOR_BLACK,
    COLOR_RED,
    COLOR_GREEN,
    COLOR_YELLOW,
    COLOR_BLUE,
    COLOR_MAGENTA,
    COLOR_CYAN,
    COLOR_WHITE,
];

impl Curses {
    fn color_pair(fg: i16, bg: i16) -> i16 {
        (fg * 8) + bg
    }

    pub fn spec(&self, version: u8) -> Spec {
        let set_flags = match version {
            1 | 2 | 3 => vec![Flag::ScreenSplittingAvailable],
            4 | 5 | 6 | 7 | 8 => vec![
                Flag::BoldfaceAvailable,
                Flag::ItalicAvailable,
                Flag::FixedSpaceAvailable,
                Flag::TimedInputAvailable,
                Flag::ColoursAvailable,
            ],
            _ => vec![],
        };
        let clear_flags = match version {
            1 | 2 | 3 => vec![Flag::StatusLineNotAvailable, Flag::VariablePitchDefaultFont],
            4 | 5 | 6 | 7 | 8 => vec![
                Flag::GameWantsUndo,
                Flag::GameWantsSoundEffects,
                Flag::GameWantsPictures,
                Flag::GameWantsMenus,
                Flag::GameWantsMouse,
                Flag::PicturesAvailable,
                Flag::SoundEffectsAvailable,
            ],
            _ => vec![],
        };

        // Initialize color pairs for all fg/bg comobos
        pancurses::start_color();
        for i in 0..COLOR_TABLE.len() {
            for j in 0..COLOR_TABLE.len() {
                let pair = Curses::color_pair(i as i16, j as i16);
                pancurses::init_pair(pair as i16, COLOR_TABLE[i], COLOR_TABLE[j]);
            }
        }

        pancurses::curs_set(0);

        match &self.status_window {
            Some(w) => w.color_set(Curses::color_pair(COLOR_BLACK, COLOR_GREEN)),
            None => 0,
        };

        let pair = Curses::color_pair(COLOR_GREEN, COLOR_BLACK);
        self.window_0.color_set(pair);
        self.window_1.color_set(pair);

        self.window_0.setscrreg(self.top_line, 0);
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
