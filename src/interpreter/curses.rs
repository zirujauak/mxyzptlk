use std::{
    fs::{self, File},
    io::Write,
    path::Path,
    thread,
    time::{self, SystemTime, UNIX_EPOCH},
};

use pancurses::{
    Attribute, Input, Window, COLOR_BLACK, COLOR_BLUE, COLOR_CYAN, COLOR_GREEN,
    COLOR_MAGENTA, COLOR_RED, COLOR_WHITE, COLOR_YELLOW,
};

use super::{Interpreter, Spec};
use crate::executor::{header::Flag, text};

pub struct Curses {
    name: String,
    version: u8,
    window_0: Window,
    window_1: Window,
    status_window: Option<Window>,
    lines: i32,
    columns: i32,
    selected_window: u8,
    output_streams: u8,
    buffering: bool,
    top_line: i32,
    foreground: i16,
    background: i16,
    transcript_file: Option<File>
}

impl Curses {
    pub fn new(version: u8, name: String) -> Curses {
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

        Self {
            name,
            version,
            window_0,
            window_1,
            status_window,
            lines,
            columns,
            selected_window: 0,
            output_streams: 1,
            buffering: true,
            top_line,
            foreground: COLOR_GREEN,
            background: COLOR_BLACK,
            transcript_file: None,
        }
    }

    fn current_window_mut(&mut self) -> &mut Window {
        match self.selected_window {
            1 => &mut self.window_1,
            _ => &mut self.window_0,
        }
    }

    fn getch(&mut self) -> Option<(char, char)> {
        let gc = self.current_window_mut().getch();
        match gc {
            Some(input) => {
                match input {
                    Input::Character(c) => match c as u16 as u32 {
                        // Cursor keys
                        0x1b => {
                            // Control character
                            let c1 = self.current_window_mut().getch().unwrap();
                            let c2 = self.current_window_mut().getch().unwrap();
                            match (c1, c2) {
                                (Input::Character('['), Input::Character('A')) => {
                                    Some((129 as char, 129 as char))
                                }
                                (Input::Character('['), Input::Character('B')) => {
                                    Some((130 as char, 130 as char))
                                }
                                (Input::Character('['), Input::Character('D')) => {
                                    Some((131 as char, 131 as char))
                                }
                                (Input::Character('['), Input::Character('C')) => {
                                    Some((132 as char, 132 as char))
                                }
                                _ => None,
                            }
                        }
                        0x7f => Some((c, 0x08 as char)),
                        0x0a => Some((c, 0x0d as char)),
                        0xe4 => Some((c, 155 as char)),
                        0xf6 => Some((c, 156 as char)),
                        0xfc => Some((c, 157 as char)),
                        0xc4 => Some((c, 158 as char)),
                        0xd6 => Some((c, 159 as char)),
                        0xdc => Some((c, 160 as char)),
                        0xdf => Some((c, 161 as char)),
                        0xbb => Some((c, 162 as char)),
                        0xab => Some((c, 163 as char)),
                        0xeb => Some((c, 164 as char)),
                        0xef => Some((c, 165 as char)),
                        0xff => Some((c, 166 as char)),
                        0xcb => Some((c, 167 as char)),
                        0xcf => Some((c, 168 as char)),
                        0xe1 => Some((c, 169 as char)),
                        0xe9 => Some((c, 170 as char)),
                        0xed => Some((c, 171 as char)),
                        0xf3 => Some((c, 172 as char)),
                        0xfa => Some((c, 173 as char)),
                        0xfd => Some((c, 174 as char)),
                        0xc1 => Some((c, 175 as char)),
                        0xc9 => Some((c, 176 as char)),
                        0xcd => Some((c, 177 as char)),
                        0xd3 => Some((c, 178 as char)),
                        0xda => Some((c, 179 as char)),
                        0xdd => Some((c, 180 as char)),
                        0xe0 => Some((c, 181 as char)),
                        0xe8 => Some((c, 182 as char)),
                        0xec => Some((c, 183 as char)),
                        0xf2 => Some((c, 184 as char)),
                        0xf9 => Some((c, 185 as char)),
                        0xc0 => Some((c, 186 as char)),
                        0xc8 => Some((c, 187 as char)),
                        0xcc => Some((c, 188 as char)),
                        0xd2 => Some((c, 189 as char)),
                        0xd9 => Some((c, 190 as char)),
                        0xe2 => Some((c, 191 as char)),
                        0xea => Some((c, 192 as char)),
                        0xee => Some((c, 193 as char)),
                        0xf4 => Some((c, 194 as char)),
                        0xfb => Some((c, 195 as char)),
                        0xc2 => Some((c, 196 as char)),
                        0xca => Some((c, 197 as char)),
                        0xce => Some((c, 198 as char)),
                        0xd4 => Some((c, 199 as char)),
                        0xdb => Some((c, 200 as char)),
                        0xe5 => Some((c, 201 as char)),
                        0xc5 => Some((c, 202 as char)),
                        0xf8 => Some((c, 203 as char)),
                        0xd8 => Some((c, 204 as char)),
                        0xe3 => Some((c, 205 as char)),
                        0xf1 => Some((c, 206 as char)),
                        0xf5 => Some((c, 207 as char)),
                        0xc3 => Some((c, 208 as char)),
                        0xd1 => Some((c, 209 as char)),
                        0xd5 => Some((c, 210 as char)),
                        0xe6 => Some((c, 211 as char)),
                        0xc6 => Some((c, 212 as char)),
                        0xe7 => Some((c, 213 as char)),
                        0xc7 => Some((c, 214 as char)),
                        0xfe => Some((c, 215 as char)),
                        0xf0 => Some((c, 216 as char)),
                        0xde => Some((c, 217 as char)),
                        0xd0 => Some((c, 218 as char)),
                        0xa3 => Some((c, 219 as char)),
                        0x153 => Some((c, 220 as char)),
                        0x152 => Some((c, 221 as char)),
                        0xa1 => Some((c, 222 as char)),
                        0xbf => Some((c, 223 as char)),
                        _ => Some((c, c)),
                    },
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

fn addch(window: &mut Window, c: u16) {
    let ch = match c {
        155 => 0xe4,
        156 => 0xf6,
        157 => 0xfc,
        158 => 0xc6,
        159 => 0xd6,
        160 => 0xdc,
        161 => 0xdf,
        162 => 0xbb,
        163 => 0xab,
        164 => 0xeb,
        165 => 0xef,
        166 => 0xff,
        167 => 0xcb,
        168 => 0xcf,
        169 => 0xe1,
        170 => 0xe9,
        171 => 0xed,
        172 => 0xf3,
        173 => 0xfa,
        174 => 0xfd,
        175 => 0xc1,
        176 => 0xc9,
        177 => 0xcd,
        178 => 0xd3,
        179 => 0xda,
        180 => 0xdd,
        181 => 0xe0,
        182 => 0xe8,
        183 => 0xec,
        184 => 0xf2,
        185 => 0xf9,
        186 => 0xc0,
        187 => 0xc8,
        188 => 0xcc,
        189 => 0xd2,
        190 => 0xd9,
        191 => 0xe2,
        192 => 0xea,
        193 => 0xee,
        194 => 0xf4,
        195 => 0xfb,
        196 => 0xc2,
        197 => 0xca,
        198 => 0xce,
        199 => 0xd4,
        200 => 0xdb,
        201 => 0xe5,
        202 => 0xc5,
        203 => 0xf8,
        204 => 0xd8,
        205 => 0xe3,
        206 => 0xf1,
        207 => 0xf5,
        208 => 0xc3,
        209 => 0xd1,
        210 => 0xd5,
        211 => 0xe6,
        212 => 0xc6,
        213 => 0xe7,
        214 => 0xc7,
        215 => 0xfe,
        216 => 0xf0,
        217 => 0xde,
        218 => 0xd0,
        219 => 0xa3,
        220 => 0x153,
        221 => 0x152,
        222 => 0xa1,
        223 => 0xbf,
        _ => c,
    } as u16;

    window.addstr(format!("{}", char::from_u32(ch as u32).unwrap()));
}

fn addstr(window: &mut Window, s: &str) {
    let chars: Vec<char> = s.chars().collect();
    for c in chars {
        addch(window, c as u16)
    }
}

impl Interpreter for Curses {
    fn buffer_mode(&mut self, mode: bool) {
        self.buffering = mode
    }

    fn erase_line(&mut self, _value: u16) {
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

    fn input_stream(&mut self, _stream: u16) {
        todo!()
    }
    fn new_line(&mut self) {
        //let win = self.current_window_mut();
        addch(self.current_window_mut(), '\n' as u16);
        // self.current_window_mut().addch('\n');
        self.current_window_mut().refresh();

        if self.selected_window == 0 && self.output_streams & 2 == 2 {
            self.transcript_file.as_mut().unwrap().write(&['\n' as u8]).unwrap();
        }
    }

    fn output_stream(&mut self, stream: i16, _table: usize) {
        if stream == 2 {
            match &mut self.transcript_file {
                Some(_) => {},
                None => {
                    self.transcript_file = Some(fs::OpenOptions::new()
                .create(true)
                .write(true)
                .open(format!("{}.txt", self.name)).unwrap()); }
            }
        } else if stream == -2 {
            match &mut self.transcript_file {
                Some(f) => {
                    f.flush().unwrap();
                },
                None => {}
            }
        }
        if stream < 0 {
            let bits = stream.abs() - 1;
            let mask = !((1 as u8) << bits);
            self.output_streams = self.output_streams & mask;
        } else if stream > 0 {
            let bits = stream - 1;
            let mask = (1 as u8) << bits;
            self.output_streams = self.output_streams | mask;
        }
    }

    fn print(&mut self, text: String) {
        if self.output_streams & 1 == 1 && self.output_streams & 4 == 0 {
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
                        addch(self.current_window_mut(), '\n' as u16);
                        addstr(self.current_window_mut(), s);
                        if self.selected_window == 0 && self.output_streams & 2 == 2 {
                            self.transcript_file.as_mut().unwrap().write_fmt(format_args!("\n{}", s)).unwrap();
                        }
                        // self.current_window_mut().addstr(s);
                    } else {
                        addstr(self.current_window_mut(), s);
                        if self.selected_window == 0 && self.output_streams & 2 == 2 {
                            self.transcript_file.as_mut().unwrap().write_all(s.as_bytes()).unwrap();
                        }
                        // self.current_window_mut().addstr(s);
                    }
                }
            } else {
                addstr(self.current_window_mut(), text.as_str());
                if self.selected_window == 0 && self.output_streams & 2 == 2 {
                    self.transcript_file.as_mut().unwrap().write_all(text.as_bytes()).unwrap();
                }
                // self.current_window_mut().addstr(text);
            }
            self.current_window_mut().refresh();
        };
    }

    fn print_table(&mut self, _text: String, _width: u16, _height: u16, _skip: u16) {
        todo!()
    }

    fn read(
        &mut self,
        length: u8,
        time: u16,
        existing_input: &Vec<char>,
        redraw: bool,
    ) -> (Vec<char>, bool) {
        self.window_1.refresh();
        self.window_0.refresh();
        self.window_0
            .mv(self.window_0.get_cur_y(), self.window_0.get_cur_x());
        pancurses::curs_set(1);
        pancurses::noecho();

        if redraw {
            for c in existing_input {
                addch(self.current_window_mut(), *c as u16);
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
            if time > 0
                && SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis()
                    > end
            {
                return (input, true);
            } else {
                let c = self.getch();
                match c {
                    Some(ch) => {
                        match ch {
                            // Backspace
                            ('\u{7f}', _) => {
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
                            (_, _) => {
                                if input.len() < length as usize && text::valid_input(ch.1) {
                                    input.push(ch.1);
                                    addch(&mut self.window_0, ch.1 as u16);
                                    self.window_0.refresh();
                                }
                                if input.len() < length as usize && ch.0 == '\n' {
                                    input.push(ch.0);
                                    done = true;
                                    addch(&mut self.window_0, '\n' as u16);
                                }
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
        if self.selected_window == 0 && self.output_streams & 2 == 2 {
            let mut d = vec![];
            for c in input.clone() {
                d.push(c as u8);
            }
            self.transcript_file.as_mut().unwrap().write_all(&d).unwrap();
        }
        (input, false)
    }

    fn read_char(&mut self, time: u16) -> char {
        pancurses::noecho();
        pancurses::curs_set(1);

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
            let mut result = 0 as char;
            // While no (acceptable) keypress and 'time' seconds haven't elapsed
            while result == 0 as char
                && SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    < end
            {
                let ch = self.getch();
                result = match ch {
                    Some(c) => c.1,
                    _ => 0 as char,
                };

                // Brief sleep
                thread::sleep(delay);
            }

            // Re-enable block on input
            pancurses::curs_set(0);
            result
        } else {
            self.current_window_mut().nodelay(false);
            let result = match self.getch() {
                Some(ch) => ch.1,
                None => ' ',
            };
            pancurses::curs_set(0);
            result
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

    fn sound_effect(&mut self, number: u16, _effect: u16, _volume: u8, _repeats: u8) {
        match number {
            1 => {
                pancurses::beep();
            }
            2 => {
                pancurses::beep();
                pancurses::beep();
            }
            _ => trace!("sound_effect > 2 not implemented yet."),
        }
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

    fn save(&mut self, data: &Vec<u8>) {
        let default = Curses::save_filename(&self.name);
        self.print("Save to: ".to_string());
        let filename = self
            .read(64, 0, &default.chars().collect(), true)
            .0
            .iter()
            .collect::<String>()
            .replace("\n", "");
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

    fn restore(&mut self) -> Vec<u8> {
        let default = Curses::restore_filename(&self.name);
        self.print("Restore from: ".to_string());
        let filename = self
            .read(64, 0, &default.chars().collect(), true)
            .0
            .iter()
            .collect::<String>()
            .replace("\n", "");
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

    fn save_filename(name: &String) -> String {
        let mut index = 0;
        let mut f = true;

        while f {
            index = index + 1;
            let name = format!("{}-{:02}.ifzs", name, index);
            f = Path::new(&name).exists();
            trace!("Checking {}: {}", name, f);
        }

        return format!("{}-{:02}.ifzs", name, index);
    }

    fn restore_filename(name: &String) -> String {
        let mut index = 0;
        let mut f = true;

        while f {
            index = index + 1;
            let name = format!("{}-{:02}.ifzs", name, index);
            f = Path::new(&name).exists();
            trace!("Checking {}: {}", name, f);
        }

        return format!("{}-{:02}.ifzs", name, index - 1);
    }
}
