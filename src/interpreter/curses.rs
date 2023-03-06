use std::{
    fs::{self, File},
    io::Write,
    path::Path,
    thread,
    time::{self, SystemTime, UNIX_EPOCH},
};

use pancurses::{
    Attribute, Input, Window, ALL_MOUSE_EVENTS, BUTTON1_CLICKED, BUTTON1_DOUBLE_CLICKED,
    COLOR_BLACK, COLOR_BLUE, COLOR_CYAN, COLOR_GREEN, COLOR_MAGENTA, COLOR_RED, COLOR_WHITE,
    COLOR_YELLOW, REPORT_MOUSE_POSITION,
};

use super::{Interpreter, Spec};
use crate::executor::{
    header::{self, Flag},
    text,
};

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
    transcript_file: Option<File>,
    command_file: Option<File>,
    lines_since_input: i32,
}

impl Curses {
    pub fn new(version: u8, name: String) -> Curses {
        let window_0 = pancurses::initscr();
        window_0.keypad(true);
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
            command_file: None,
            lines_since_input: 0,
        }
    }

    fn current_window(&self) -> &Window {
        match self.selected_window {
            1 => &self.window_1,
            _ => &self.window_0,
        }
    }
    fn current_window_mut(&mut self) -> &mut Window {
        match self.selected_window {
            1 => &mut self.window_1,
            _ => &mut self.window_0,
        }
    }

    fn getch(&mut self) -> Option<super::Input> {
        let gc = self.current_window_mut().getch();
        pancurses::mousemask(ALL_MOUSE_EVENTS, None);
        self.lines_since_input = 0;
        match gc {
            Some(input) => {
                trace!("input: {:?}", input);
                match input {
                    Input::KeyUp => super::Input::from_u8(129, 129),
                    Input::KeyDown => super::Input::from_u8(130, 130),
                    Input::KeyLeft => super::Input::from_u8(131, 131),
                    Input::KeyRight => super::Input::from_u8(132, 132),
                    Input::KeyF1 => super::Input::from_u8(133, 133),
                    Input::KeyF2 => super::Input::from_u8(134, 134),
                    Input::KeyF3 => super::Input::from_u8(135, 135),
                    Input::KeyF4 => super::Input::from_u8(136, 136),
                    Input::KeyF5 => super::Input::from_u8(137, 137),
                    Input::KeyF6 => super::Input::from_u8(138, 138),
                    Input::KeyF7 => super::Input::from_u8(139, 139),
                    Input::KeyF8 => super::Input::from_u8(140, 140),
                    Input::KeyF9 => super::Input::from_u8(141, 141),
                    Input::KeyF10 => super::Input::from_u8(142, 142),
                    Input::KeyF11 => super::Input::from_u8(143, 143),
                    Input::KeyF12 => super::Input::from_u8(144, 144),
                    Input::KeyMouse => {
                        let e = pancurses::getmouse().unwrap();
                        trace!("{:?}", e);
                        match e.bstate {
                            BUTTON1_CLICKED => {
                                super::Input::from_mouse(254, e.x as u16, e.y as u16)
                            }
                            BUTTON1_DOUBLE_CLICKED => {
                                super::Input::from_mouse(253, e.x as u16, e.y as u16)
                            }
                            _ => None,
                        }
                    }
                    Input::Character(c) => match c as u16 as u32 {
                        // Cursor keys
                        0x1b => {
                            // Control character
                            let c1 = self.current_window_mut().getch().unwrap();
                            let c2 = self.current_window_mut().getch().unwrap();
                            match (c1, c2) {
                                (Input::Character('['), Input::Character('A')) => {
                                    super::Input::from_u8(129, 129)
                                }
                                (Input::Character('['), Input::Character('B')) => {
                                    super::Input::from_u8(130, 130)
                                }
                                (Input::Character('['), Input::Character('D')) => {
                                    super::Input::from_u8(131, 131)
                                }
                                (Input::Character('['), Input::Character('C')) => {
                                    super::Input::from_u8(132, 132)
                                }
                                _ => None,
                            }
                        }
                        0x7f => super::Input::from_char(c, 0x08),
                        0x0a => super::Input::from_char(c, 0x0d),
                        0xe4 => super::Input::from_char(c, 155),
                        0xf6 => super::Input::from_char(c, 156),
                        0xfc => super::Input::from_char(c, 157),
                        0xc4 => super::Input::from_char(c, 158),
                        0xd6 => super::Input::from_char(c, 159),
                        0xdc => super::Input::from_char(c, 160),
                        0xdf => super::Input::from_char(c, 161),
                        0xbb => super::Input::from_char(c, 162),
                        0xab => super::Input::from_char(c, 163),
                        0xeb => super::Input::from_char(c, 164),
                        0xef => super::Input::from_char(c, 165),
                        0xff => super::Input::from_char(c, 166),
                        0xcb => super::Input::from_char(c, 167),
                        0xcf => super::Input::from_char(c, 168),
                        0xe1 => super::Input::from_char(c, 169),
                        0xe9 => super::Input::from_char(c, 170),
                        0xed => super::Input::from_char(c, 171),
                        0xf3 => super::Input::from_char(c, 172),
                        0xfa => super::Input::from_char(c, 173),
                        0xfd => super::Input::from_char(c, 174),
                        0xc1 => super::Input::from_char(c, 175),
                        0xc9 => super::Input::from_char(c, 176),
                        0xcd => super::Input::from_char(c, 177),
                        0xd3 => super::Input::from_char(c, 178),
                        0xda => super::Input::from_char(c, 179),
                        0xdd => super::Input::from_char(c, 180),
                        0xe0 => super::Input::from_char(c, 181),
                        0xe8 => super::Input::from_char(c, 182),
                        0xec => super::Input::from_char(c, 183),
                        0xf2 => super::Input::from_char(c, 184),
                        0xf9 => super::Input::from_char(c, 185),
                        0xc0 => super::Input::from_char(c, 186),
                        0xc8 => super::Input::from_char(c, 187),
                        0xcc => super::Input::from_char(c, 188),
                        0xd2 => super::Input::from_char(c, 189),
                        0xd9 => super::Input::from_char(c, 190),
                        0xe2 => super::Input::from_char(c, 191),
                        0xea => super::Input::from_char(c, 192),
                        0xee => super::Input::from_char(c, 193),
                        0xf4 => super::Input::from_char(c, 194),
                        0xfb => super::Input::from_char(c, 195),
                        0xc2 => super::Input::from_char(c, 196),
                        0xca => super::Input::from_char(c, 197),
                        0xce => super::Input::from_char(c, 198),
                        0xd4 => super::Input::from_char(c, 199),
                        0xdb => super::Input::from_char(c, 200),
                        0xe5 => super::Input::from_char(c, 201),
                        0xc5 => super::Input::from_char(c, 202),
                        0xf8 => super::Input::from_char(c, 203),
                        0xd8 => super::Input::from_char(c, 204),
                        0xe3 => super::Input::from_char(c, 205),
                        0xf1 => super::Input::from_char(c, 206),
                        0xf5 => super::Input::from_char(c, 207),
                        0xc3 => super::Input::from_char(c, 208),
                        0xd1 => super::Input::from_char(c, 209),
                        0xd5 => super::Input::from_char(c, 210),
                        0xe6 => super::Input::from_char(c, 211),
                        0xc6 => super::Input::from_char(c, 212),
                        0xe7 => super::Input::from_char(c, 213),
                        0xc7 => super::Input::from_char(c, 214),
                        0xfe => super::Input::from_char(c, 215),
                        0xf0 => super::Input::from_char(c, 216),
                        0xde => super::Input::from_char(c, 217),
                        0xd0 => super::Input::from_char(c, 218),
                        0xa3 => super::Input::from_char(c, 219),
                        0x153 => super::Input::from_char(c, 220),
                        0x152 => super::Input::from_char(c, 221),
                        0xa1 => super::Input::from_char(c, 222),
                        0xbf => super::Input::from_char(c, 223),
                        _ => super::Input::from_char(c, c as u8),
                    },
                    _ => {
                        trace!("getch: {:?}", input);
                        None
                    }
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
        158 => 0xc4,
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

    // This might break accented characters in macos, which were working correctly.
    if ch > 0 {
        for o in char::decode_utf16([ch]) {
            match o {
                Ok(x) => {
                    window.addstr(x.to_string());
                }
                Err(_) => {}
            }
        }
    }
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
        self.pause_output();
        //let win = self.current_window_mut();
        addch(self.current_window_mut(), '\n' as u16);
        self.lines_since_input = self.lines_since_input + 1;
        // self.current_window_mut().addch('\n');
        self.current_window_mut().refresh();

        if self.selected_window == 0 && self.output_streams & 2 == 2 {
            self.transcript_file
                .as_mut()
                .unwrap()
                .write(&['\n' as u8])
                .unwrap();
        }
    }

    fn output_stream(&mut self, stream: i16, _table: usize) {
        if stream == 2 {
            match &mut self.transcript_file {
                Some(_) => {}
                None => {
                    self.transcript_file = Some(
                        fs::OpenOptions::new()
                            .create(true)
                            .write(true)
                            .open(self.transcript_filename())
                            .unwrap(),
                    );
                }
            }
        } else if stream == -2 {
            match &mut self.transcript_file {
                Some(f) => {
                    f.flush().unwrap();
                }
                None => {}
            }
        } else if stream == 4 {
            match &mut self.command_file {
                Some(_) => {}
                None => {
                    self.command_file = Some(
                        fs::OpenOptions::new()
                            .create(true)
                            .write(true)
                            .open(self.command_filename())
                            .unwrap(),
                    );
                }
            }
        } else if stream == -4 {
            match &mut self.command_file {
                Some(f) => f.flush().unwrap(),
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
        let (y, x) = self.current_window().get_cur_yx();
        trace!("Cursor @ {},{}", y, x);

        if self.output_streams & 1 == 1 && self.output_streams & 4 == 0 {
            if self.buffering || self.selected_window == 1 {
                trace!("Buffered printing");
                // Split the text string on spaces
                let frags = text.split_inclusive(&[' ']);
                // Iterate over the fragments
                for s in frags {
                    trace!("Fragment: {}", s);
                    let position = (
                        self.current_window_mut().get_cur_y() + 1,
                        self.current_window_mut().get_cur_x() + 1,
                    );
                    if self.columns as i32 - position.1 < s.len() as i32 {
                        addch(self.current_window_mut(), '\n' as u16);
                        self.lines_since_input = self.lines_since_input + 1;
                        self.pause_output();
                        addstr(self.current_window_mut(), s);
                        if self.selected_window == 0 && self.output_streams & 2 == 2 {
                            self.transcript_file
                                .as_mut()
                                .unwrap()
                                .write_fmt(format_args!("\n{}", s))
                                .unwrap();
                        }
                        // self.current_window_mut().addstr(s);
                    } else {
                        addstr(self.current_window_mut(), s);
                        if self.selected_window == 0 && self.output_streams & 2 == 2 {
                            self.transcript_file
                                .as_mut()
                                .unwrap()
                                .write_all(s.as_bytes())
                                .unwrap();
                        }
                        // self.current_window_mut().addstr(s);
                    }
                }
            } else {
                trace!("Printing");
                addstr(self.current_window_mut(), text.as_str());
                if self.selected_window == 0 && self.output_streams & 2 == 2 {
                    self.transcript_file
                        .as_mut()
                        .unwrap()
                        .write_all(text.as_bytes())
                        .unwrap();
                }
                // self.current_window_mut().addstr(text);
            }
        };
        self.current_window_mut().refresh();
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
        // self.window_0
        //     .mv(self.window_0.get_cur_y(), self.window_0.get_cur_x());
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
                let i = self.getch();
                match i {
                    Some(inp) => {
                        match inp.original_value {
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
                                if input.len() < length as usize
                                    && text::valid_input(inp.zscii_value)
                                {
                                    input.push(inp.zscii_value);
                                    addch(&mut self.window_0, inp.original_value as u16);
                                    self.window_0.refresh();
                                }
                                if input.len() < length as usize && inp.original_value == '\n' {
                                    input.push(inp.original_value);
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

        // Transcripting
        if self.selected_window == 0 && self.output_streams & 2 == 2 {
            let mut d = vec![];
            for c in input.clone() {
                d.push(c as u8);
            }
            self.transcript_file
                .as_mut()
                .unwrap()
                .write_all(&d)
                .unwrap();
        }

        // Stream 4
        if self.output_streams & 0x8 == 0x8 {
            let mut d = vec![];
            for c in input.clone() {
                d.push(c as u8);
            }
            self.command_file.as_mut().unwrap().write_all(&d).unwrap();
            self.command_file.as_mut().unwrap().flush().unwrap();
        }
        (input, false)
    }

    fn read_char(&mut self, time: u16) -> super::Input {
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
            let mut result = None;
            // While no (acceptable) keypress and 'time' seconds haven't elapsed
            while match result {
                Some(_) => false,
                None => true,
            } && SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                < end
            {
                result = self.getch();

                // Brief sleep
                thread::sleep(delay);
            }

            let result = match result {
                Some(r) => r,
                None => super::Input::from_u8(0, 0).unwrap(),
            };

            // Re-enable block on input
            pancurses::curs_set(0);

            // Send character to stream 4, if selected
            if self.output_streams & 0x8 == 0x8 {
                self.command_file
                    .as_mut()
                    .unwrap()
                    .write(format!("[{:03}]\n", result.zscii_value).as_bytes())
                    .unwrap();
                self.command_file.as_mut().unwrap().flush().unwrap();
            }

            result
        } else {
            self.current_window_mut().nodelay(false);
            let result = match self.getch() {
                Some(r) => r,
                None => super::Input::from_u8(0, 0).unwrap(),
            };
            pancurses::curs_set(0);

            if self.output_streams & 0x8 == 0x8 {
                self.command_file
                    .as_mut()
                    .unwrap()
                    .write(format!("[{:03}]\n", result.zscii_value).as_bytes())
                    .unwrap();
                self.command_file.as_mut().unwrap().flush().unwrap();
            }

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
        if self.selected_window == 1 {
            trace!(
                "Setting cursor in window {} to {},{}",
                self.selected_window,
                line,
                column
            );
            self.current_window_mut()
                .mv(line as i32 - 1, column as i32 - 1);
            trace!("{:?}", self.current_window().get_cur_yx());
        }
    }

    fn set_font(&mut self, font: u16) {}
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
            self.selected_window = 0;
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
                trace!("Splitting @ {}", lines);
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
        let default = self.save_filename();
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
        let default = self.restore_filename();
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
    fn pause_output(&mut self) {
        if self.selected_window == 0 {
            let max_lines = self.lines
                - match &self.status_window {
                    Some(_) => 1,
                    _ => 0,
                };
            trace!("Lines: {} / {}", self.lines_since_input, max_lines);
            if self.lines_since_input >= max_lines {
                trace!("MORE!");
                self.window_0.addstr("[MORE]");
                self.window_0.refresh();
                self.window_0.nodelay(false);
                self.getch();
                self.window_0.nodelay(true);
                self.window_0.mv(self.lines - 1, 0);
                self.window_0.addstr("      ");
                self.window_0.mv(self.lines - 1, 0);
                self.window_0.refresh();
            }
        }
    }
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
            1 | 2 | 3 => vec![Flag::StatusLineNotAvailable, Flag::VariablePitchDefaultFont, Flag::Transcripting],
            4 | 5 | 6 | 7 | 8 => vec![
                Flag::Transcripting,
                Flag::GameWantsSoundEffects,
                Flag::GameWantsPictures,
                Flag::GameWantsMenus,
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

    fn next_filename(&self, extension: &str) -> String {
        let mut index = 0;

        loop {
            index = index + 1;
            let name = format!("{}-{:02}{}", self.name, index, extension);
            if !Path::new(&name).exists() {
                return name;
            }
        }
    }

    fn last_filename(&self, extension: &str) -> String {
        let mut index = 1;

        loop {
            let name = format!("{}-{:02}{}", self.name, index, extension);
            if !Path::new(&name).exists() {
                return format!("{}-{:02}{}", self.name, index - 1, extension);
            }
            index = index + 1;
        }
    }

    fn save_filename(&self) -> String {
        self.next_filename(&".ifzs")
    }

    fn restore_filename(&self) -> String {
        self.last_filename(&".ifzs")
    }

    fn transcript_filename(&self) -> String {
        self.next_filename(&"-transcript.txt")
    }

    fn command_filename(&self) -> String {
        self.next_filename(&"-command.txt")
    }
}
