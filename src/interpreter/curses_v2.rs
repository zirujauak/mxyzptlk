use std::{
    cmp::{max, min},
    collections::HashMap,
    fs::{self, File},
    io::Write,
    path::Path,
    thread,
    time::{self, SystemTime, UNIX_EPOCH},
};

use pancurses::{
    Attribute, Input, Window, ALL_MOUSE_EVENTS, BUTTON1_CLICKED, BUTTON1_DOUBLE_CLICKED,
    COLOR_BLACK, COLOR_BLUE, COLOR_CYAN, COLOR_GREEN, COLOR_MAGENTA, COLOR_RED, COLOR_WHITE,
    COLOR_YELLOW,
};
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use tempfile::NamedTempFile;

use super::{Input as OtherInput, Interpreter, Sound, Spec};
use crate::executor::{header::Flag, text};

#[derive(Debug)]
struct Cursor {
    line: i32,
    column: i32,
}
pub struct CursesV2 {
    name: String,
    version: u8,
    window: Window,
    cursor: Vec<Cursor>,
    font: Vec<u16>,
    window_0_top: i32,
    window_1_bottom: i32,
    status_line: i32,
    screen_lines: i32,
    screen_columns: i32,
    selected_window: usize,
    output_streams: u8,
    buffering: bool,
    foreground_colour: i16,
    background_colour: i16,
    transcript_file: Option<File>,
    command_file: Option<File>,
    lines_since_input: i32,
    _output_stream: OutputStream,
    output_stream_handle: OutputStreamHandle,
    pub sounds: HashMap<u8, Sound>,
    current_effect: u8,
    sink: Option<Sink>,
}

impl CursesV2 {
    pub fn new(version: u8, name: String) -> CursesV2 {
        let window = pancurses::initscr();
        window.keypad(true);
        let screen_lines = window.get_max_y();
        let screen_columns = window.get_max_x();
        let status_line = if version < 4 { 1 } else { 0 };
        window.setscrreg(status_line, screen_lines - 1);
        window.scrollok(true);
        window.erase();

        let window_0_cursor = if version < 5 {
            Cursor {
                line: screen_lines,
                column: 1,
            }
        } else {
            Cursor { line: 1, column: 1 }
        };

        trace!(
            "Screen: {}x{} [top-left is 1,1]",
            screen_lines,
            screen_columns
        );
        trace!("Status line: {}", status_line);
        trace!("Window 1 bottom: {}", 0);
        trace!("Window 0 top: {}", status_line + 1);

        // Initialize color pairs for all fg/bg comobos
        pancurses::start_color();
        for i in 0..COLOR_TABLE.len() {
            for j in 0..COLOR_TABLE.len() {
                let pair = CursesV2::color_pair(i as i16, j as i16);
                pancurses::init_pair(pair as i16, COLOR_TABLE[i], COLOR_TABLE[j]);
            }
        }

        pancurses::curs_set(1);

        let cursor = vec![window_0_cursor, Cursor { line: 0, column: 0 }];

        let (stream, stream_handle) = OutputStream::try_default().unwrap();

        Self {
            name,
            version,
            window,
            cursor,
            font: vec![1, 1],
            window_0_top: status_line + 1,
            window_1_bottom: 0,
            status_line,
            screen_lines,
            screen_columns,
            selected_window: 0,
            output_streams: 1,
            buffering: true,
            foreground_colour: COLOR_GREEN,
            background_colour: COLOR_BLACK,
            transcript_file: None,
            command_file: None,
            lines_since_input: 0,
            _output_stream: stream,
            output_stream_handle: stream_handle,
            sounds: HashMap::new(),
            current_effect: 0,
            sink: None,
        }
    }

    fn getch(&mut self) -> Option<super::Input> {
        let gc = self.window.getch();
        pancurses::mousemask(ALL_MOUSE_EVENTS, None);
        self.window.mv(
            self.cursor[self.selected_window].line - 1,
            self.cursor[self.selected_window].column - 1,
        );
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
                        // 0x1b => {
                        //     // Control character
                        //     let c1 = self.window.getch().unwrap();
                        //     let c2 = self.window.getch().unwrap();
                        //     match (c1, c2) {
                        //         (Input::Character('['), Input::Character('A')) => {
                        //             super::Input::from_u8(129, 129)
                        //         }
                        //         (Input::Character('['), Input::Character('B')) => {
                        //             super::Input::from_u8(130, 130)
                        //         }
                        //         (Input::Character('['), Input::Character('D')) => {
                        //             super::Input::from_u8(131, 131)
                        //         }
                        //         (Input::Character('['), Input::Character('C')) => {
                        //             super::Input::from_u8(132, 132)
                        //         }
                        //         _ => None,
                        //     }
                        // }
                        0x0a => super::Input::from_char(c, 0x0d),
                        0x7f => super::Input::from_char(c, 0x08),
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

    fn addch(&mut self, c: u16) {
        let ch = if self.font[self.selected_window] != 3 {
            match c {
                0x0d => 0x0a,
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
            }
        } else {
            match c {
                0xb0 => 0x2591,
                0xb1 => 0x2592,
                0xb2 => 0x2593,
                0xb3 => 0x2502,
                0xb4 => 0x2524,
                0xb5 => 0x2561,
                0xb6 => 0x2562,
                0xbf => 0x2510,
                0xc0 => 0x2514,
                0xc4 => 0x2500,
                0xd9 => 0x2518,
                0xda => 0x250C,
                _ => c,
            }
        } as u16;

        // This might break accented characters in macos, which were working correctly.
        if ch > 0 {
            for o in char::decode_utf16([ch]) {
                let cursor = &self.cursor[self.selected_window];
                // if this is a newline, advance the cursor to the next line
                if ch == '\n' as u16 {
                    match self.selected_window {
                        0 => {
                            self.window
                                .mvaddch(cursor.line - 1, cursor.column - 1, '\n');
                            self.cursor[0].column = 1;
                            self.cursor[0].line = min(self.cursor[0].line + 1, self.screen_lines);
                        }
                        // Except for window 1 - the cursor stays in the lower right corner
                        1 => {
                            if self.cursor[1].line < self.window_1_bottom {
                                self.cursor[1].line = self.cursor[1].line + 1;
                                self.cursor[1].column = 1;
                            }
                        }
                        _ => {}
                    }
                } else {
                    match o {
                        Ok(x) => {
                            self.window
                                .mvaddstr(cursor.line - 1, cursor.column - 1, x.to_string());
                            self.advance_cursor();
                        }
                        Err(_) => {}
                    }
                }
            }
        }
    }

    fn advance_cursor(&mut self) {
        let mut cursor = &mut self.cursor[self.selected_window];
        let new_col = cursor.column + 1;
        if new_col > self.screen_columns {
            cursor.line = min(cursor.line + 1, self.screen_lines);
            match self.selected_window {
                0 => {
                    if cursor.line >= self.screen_lines {
                        self.window
                            .mvaddch(self.screen_lines - 1, self.screen_columns - 1, '\n');
                        cursor.column = 1;
                    }
                }
                _ => {}
            }
        } else {
            cursor.column = new_col;
        }

        trace!("advance_cursor: {:?}", self.cursor[self.selected_window]);
    }

    fn addstr(&mut self, s: &str) {
        let chars: Vec<char> = s.chars().collect();
        for c in chars {
            self.addch(c as u16)
        }
    }

    fn is_terminator(&self, terminators: &Vec<u8>, c: u8) -> bool {
        trace!("Terminator? {} => {}", c, terminators.contains(&c));
        c == '\n' as u8
            || c == '\r' as u8
            || terminators.contains(&c)
            || (terminators.contains(&255) && c >= 129 && c <= 144)
    }
}

impl Interpreter for CursesV2 {
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
                self.window_1_bottom = 0;
                self.window_0_top = self.status_line;
                self.window
                    .setscrreg(self.window_0_top - 1, self.screen_lines - 1);
                self.window.erase();
            }
            -2 => {
                self.window.erase();
            }
            0 => {
                self.window.mv(self.window_0_top - 1, 0);
                self.window.clrtobot();
                if self.version < 5 {
                    self.cursor[0].line = self.screen_lines;
                } else {
                    self.cursor[0].line = self.window_0_top;
                }
                self.cursor[0].column = 1;
            }
            1 => {
                trace!(
                    "Erasing from line {} to {}",
                    self.status_line,
                    self.window_1_bottom
                );
                for i in self.status_line..self.window_1_bottom {
                    self.window.mv(i - 1, 0);
                    self.window.clrtoeol();
                }
                self.cursor[1].column = 1;
                self.cursor[1].line = 1;
            }
            _ => {}
        }
    }

    fn get_cursor(&mut self) -> (u16, u16) {
        (
            self.cursor[self.selected_window].line as u16,
            self.cursor[self.selected_window].column as u16,
        )
    }

    fn input_stream(&mut self, _stream: u16) {
        todo!()
    }
    fn new_line(&mut self) {
        self.pause_output();
        //let win = self.current_window_mut();
        self.addch('\n' as u16);
        self.lines_since_input = self.lines_since_input + 1;
        // self.current_window_mut().addch('\n');
        self.window.refresh();

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
        if self.output_streams & 1 == 1 && self.output_streams & 4 == 0 {
            if self.buffering || self.selected_window == 1 {
                trace!("Buffered printing");
                // Split the text string on spaces
                let frags = text.split_inclusive(&[' ']);
                // Iterate over the fragments
                for s in frags {
                    trace!("Fragment: {}", s);
                    if self.screen_columns as i32 - self.cursor[self.selected_window].column
                        < s.len() as i32
                    {
                        self.addch('\n' as u16);
                        self.lines_since_input = self.lines_since_input + 1;
                        self.pause_output();
                        self.addstr(s);
                        if self.selected_window == 0 && self.output_streams & 2 == 2 {
                            self.transcript_file
                                .as_mut()
                                .unwrap()
                                .write_fmt(format_args!("\n{}", s.replace("\r", "\n")))
                                .unwrap();
                        }
                        // self.current_window_mut().addstr(s);
                    } else {
                        self.addstr(s);
                        if self.selected_window == 0 && self.output_streams & 2 == 2 {
                            self.transcript_file
                                .as_mut()
                                .unwrap()
                                .write_all(s.replace("\r", "\n").as_bytes())
                                .unwrap();
                        }
                        // self.current_window_mut().addstr(s);
                    }
                }
            } else {
                trace!("Printing: {}", text);
                self.addstr(text.as_str());
                if self.selected_window == 0 && self.output_streams & 2 == 2 {
                    self.transcript_file
                        .as_mut()
                        .unwrap()
                        .write_all(text.replace("\r", "\n").as_bytes())
                        .unwrap();
                }
                // self.current_window_mut().addstr(text);
            }
        };
        self.window.refresh();
    }

    fn print_table(&mut self, data: Vec<u8>, width: u16, height: u16, skip: u16) {
        let column = self.cursor[self.selected_window].column;
        for i in 0..height as usize {
            // Debugging
            let offset = i * (width as usize + skip as usize);
            let end = offset + width as usize;
            let row: Vec<char> = data[offset..end]
                .to_vec()
                .iter()
                .map(|x| *x as char)
                .collect();
            trace!("Row {}: '{:?}'", i, row);
            for j in 0..width as usize {
                self.addch(data[(i * (width as usize + skip as usize)) + j] as u16);
            }
            self.addch('\n' as u8 as u16);
            self.cursor[self.selected_window].column = column;
        }
    }

    fn read(
        &mut self,
        length: u8,
        time: u16,
        existing_input: &Vec<char>,
        redraw: bool,
        terminators: Vec<u8>,
    ) -> (Vec<char>, bool, OtherInput) {
        self.window.refresh();
        // self.window_0
        //     .mv(self.window_0.get_cur_y(), self.window_0.get_cur_x());
        pancurses::noecho();
        pancurses::curs_set(1);

        if redraw {
            for c in existing_input {
                self.addch(*c as u16);
            }

            trace!("Last input: {:#02x}", *existing_input.last().unwrap() as u8)
        }

        self.window.refresh();

        match existing_input.last() {
            Some(x) => {
                if self.is_terminator(&terminators, *x as u8) {
                    return (
                        existing_input.clone(),
                        false,
                        super::Input::from_char(*x, *x as u8).unwrap(),
                    );
                }
            }
            _ => {}
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
        let mut i = super::Input::from_u8(0, 0);
        self.window.nodelay(true);
        while !done {
            if time > 0
                && SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis()
                    > end
            {
                i = super::Input::from_u8(0, 0);
            } else {
                i = self.getch();
                match i {
                    Some(inp) => {
                        trace!("Input: {:?}", inp);
                        match inp.original_value {
                            // Backspace
                            '\u{7f}' => {
                                if input.len() > 0 {
                                    // Remove from the input array
                                    input.pop();

                                    // Back cursor up and delete character
                                    self.cursor[self.selected_window].column =
                                        self.cursor[self.selected_window].column - 1;
                                    self.window.mv(
                                        self.cursor[self.selected_window].line - 1,
                                        self.cursor[self.selected_window].column - 1,
                                    );
                                    self.window.delch();
                                    self.window.refresh();
                                }
                            }
                            //
                            _ => {
                                if input.len() < length as usize
                                    && self.is_terminator(&terminators, inp.zscii_value as u8)
                                {
                                    input.push(inp.original_value);
                                    done = true;
                                    if inp.original_value == '\n' {
                                        self.addch('\n' as u16);
                                    }
                                } else if input.len() < length as usize
                                    && text::valid_input(inp.zscii_value)
                                {
                                    input.push(inp.zscii_value);
                                    self.addch(inp.original_value as u16);
                                    self.window.refresh();
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

        pancurses::curs_set(2);

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
        (input, false, i.unwrap())
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
            self.window.nodelay(true);
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
            pancurses::curs_set(1);

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
            self.window.nodelay(false);
            let result = match self.getch() {
                Some(r) => r,
                None => super::Input::from_u8(0, 0).unwrap(),
            };
            pancurses::curs_set(1);

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
            2 => self.foreground_colour = COLOR_BLACK,
            3 => self.foreground_colour = COLOR_RED,
            1 | 4 => self.foreground_colour = COLOR_GREEN,
            5 => self.foreground_colour = COLOR_YELLOW,
            6 => self.foreground_colour = COLOR_BLUE,
            7 => self.foreground_colour = COLOR_MAGENTA,
            8 => self.foreground_colour = COLOR_CYAN,
            9 => self.foreground_colour = COLOR_WHITE,
            _ => {}
        };
        match background {
            1 | 2 => self.background_colour = COLOR_BLACK,
            3 => self.background_colour = COLOR_RED,
            4 => self.background_colour = COLOR_GREEN,
            5 => self.background_colour = COLOR_YELLOW,
            6 => self.background_colour = COLOR_BLUE,
            7 => self.background_colour = COLOR_MAGENTA,
            8 => self.background_colour = COLOR_CYAN,
            9 => self.background_colour = COLOR_WHITE,
            _ => {}
        };

        let pair =
            CursesV2::color_pair(self.foreground_colour as i16, self.background_colour as i16);
        self.window.color_set(pair);
    }

    fn set_cursor(&mut self, line: u16, column: u16) {
        if self.selected_window == 1 {
            trace!(
                "Setting cursor in window {} to {},{}",
                self.selected_window,
                line,
                column
            );
            self.cursor[1].line = min(line as i32, self.window_1_bottom);
            self.cursor[1].column = column as i32;
        }
    }

    fn set_font(&mut self, font: u16) -> u16 {
        let current_font = self.font[self.selected_window];

        match font {
            0 => self.font[self.selected_window],
            1 | 3 | 4 => {
                self.font[self.selected_window] = font;
                current_font
            }
            _ => 0,
        }
    }

    fn set_text_style(&mut self, style: u16) {
        if style == 0 {
            self.window.attroff(Attribute::Reverse);
            self.window.attroff(Attribute::Bold);
            self.window.attroff(Attribute::Underline);
        } else {
            if style & 0x1 == 0x1 {
                self.window.attron(Attribute::Reverse);
            }
            if style & 0x2 == 0x2 {
                self.window.attron(Attribute::Bold);
            }
            if style & 0x4 == 0x4 {
                self.window.attron(Attribute::Underline);
            }
        }
    }

    fn set_window(&mut self, window: u16) {
        pancurses::curs_set(1);
        self.selected_window = window as usize;
        if window == 1 {
            self.cursor[1].line = 1;
            self.cursor[1].column = 1;
        }
    }
    fn show_status(&mut self, location: &str, status: &str) {
        self.window.color_set(CursesV2::color_pair(
            self.background_colour,
            self.foreground_colour,
        ));
        self.window.mv(0, 0);
        self.window.mvaddstr(
            0,
            0,
            String::from_utf8(vec![32; self.screen_columns as usize]).unwrap(),
        );
        self.window.mvaddstr(0, 1, location);
        let x = self.screen_columns - 1 - status.len() as i32;
        self.window.mvaddstr(0, x, status);
        self.window.refresh();
        self.window.color_set(CursesV2::color_pair(
            self.foreground_colour,
            self.background_colour,
        ));
    }

    fn sound_effect(&mut self, number: u16, effect: u16, volume: u8, repeats: u8) {
        match number {
            1 => {
                pancurses::beep();
            }
            2 => {
                pancurses::beep();
                pancurses::beep();
            }
            _ => {
                trace!("Current effect: {}", self.current_effect);
                if number == 0 || self.sounds.contains_key(&(number as u8)) {
                    match effect {
                        2 => {
                            if number == 0 {
                                match &self.sink {
                                    Some(sink) => sink.stop(),
                                    None => (),
                                }
                            } else if number as u8 == self.current_effect {
                                let vol = volume as f32 / 128.0;
                                trace!("Adjusting volume to {}", vol);
                                match self.sink.as_ref() {
                                    Some(sink) => sink.set_volume(vol),
                                    None => error!("Nothing currently playing"),
                                }
                            } else {
                                self.current_effect = 0;
                                match self.sink.as_ref() {
                                    Some(sink) => sink.stop(),
                                    None => (),
                                }

                                match NamedTempFile::new() {
                                    Ok(mut write) => match write.reopen() {
                                        Ok(read) => match self.sounds.get(&(number as u8)) {
                                            Some(s) => match write.write_all(&s.data) {
                                                Ok(_) => match Decoder::new(read) {
                                                    Ok(source) => {
                                                        match self.sink {
                                                                    None => match Sink::try_new(&self.output_stream_handle) {
                                                                        Ok(sink) => self.sink = Some(sink),
                                                                        Err(e) => error!("Error creating playback sink: {}", e)
                                                                    }
                                                                    Some(_) => ()
                                                                }
                                                        match self.sink.as_ref() {
                                                            Some(sink) => {
                                                                sink.set_volume(
                                                                    volume as f32 / 128.0,
                                                                );
                                                                match s.repeat {
                                                                    Some(repeats) => {
                                                                        match repeats {
                                                                            0 => sink.append(source.repeat_infinite()),
                                                                            _ => for _ in 0..repeats {
                                                                                sink.append(Decoder::new(write.reopen().unwrap()).unwrap());
                                                                            }
                                                                        }
                                                                    },
                                                                    None => {
                                                                        match repeats {
                                                                            255 => sink.append(source.repeat_infinite()),
                                                                            _ => for _ in 0..repeats {
                                                                                sink.append(Decoder::new(write.reopen().unwrap()).unwrap());
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                                sink.play();
                                                                self.current_effect = number as u8;
                                                            }
                                                            None => (),
                                                        }
                                                    }
                                                    Err(e) => error!(
                                                        "Error getting playback source: {}",
                                                        e
                                                    ),
                                                },
                                                Err(e) => {
                                                    error!("Error getting playback decoder: {}", e)
                                                }
                                            },
                                            None => error!("Sound effect {} not found", number),
                                        },
                                        Err(e) => {
                                            error!("Error reopening temp file for reading: {}", e)
                                        }
                                    },
                                    Err(e) => error!("Error opening temp file for writing: {}", e),
                                }
                                // let mut tf = NamedTempFile::new().unwrap();
                                // let tfr = tf.reopen().unwrap();
                                // let s = self.sounds.get(&(number as u8)).unwrap();
                                // tf.write_all(&s.data).unwrap();
                                // let source = Decoder::new(tfr).unwrap();
                                // let sink = Sink::try_new(&self.output_stream_handle).unwrap();
                                // sink.set_volume(volume as f32 / 128.0);
                                // if s.repeat == 0 {
                                //     sink.append(source.repeat_infinite());
                                // }
                                // sink.play();
                                // self.current_effect = number as u8;
                                // self.sink = Some(sink);
                            }
                        },
                        3 | 4 => {
                            match &self.sink {
                                Some(sink) => {
                                    trace!("Stopping playback");
                                    sink.stop()
                                },
                                None => (),
                            }
                            self.current_effect = 0;
                        }
                        _ => (),
                    }
                }
            }
        }
    }

    fn split_window(&mut self, lines: u16) {
        if lines == 0 {
            // Unsplit
            self.window_1_bottom = 0;
            self.window_0_top = self.status_line;
            self.window
                .setscrreg(self.window_0_top - 1, self.screen_lines - 1);
            self.selected_window = 0;
            trace!("Window 1 bottom: {}", self.window_1_bottom);
            trace!("Window 0 top: {}", self.window_0_top);
        } else {
            // Resize and move window 0
            self.window_1_bottom = lines as i32 + self.status_line;
            self.window_0_top = lines as i32 + self.status_line + 1;
            self.window
                .setscrreg(self.window_0_top - 1, self.screen_lines - 1);

            if self.version < 4 {
                // Clear the upper window
                for i in self.status_line..self.screen_lines {
                    self.window.mv(i - 1, 0);
                    self.window.clrtoeol();
                }
            } else {
                self.cursor[0].line = max(self.window_1_bottom + 1, self.cursor[0].line)
            }

            trace!("Window 1 bottom: {}", self.window_1_bottom);
            trace!("Window 0 top: {}", self.window_0_top);
        }

        self.window.refresh();
    }

    fn save(&mut self, data: &Vec<u8>) {
        let default = self.save_filename();
        self.print("Save to: ".to_string());
        let filename = self
            .read(64, 0, &default.chars().collect(), true, vec![])
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
            .read(64, 0, &default.chars().collect(), true, vec![])
            .0
            .iter()
            .collect::<String>()
            .replace("\n", "");
        trace!("Restore from: {}", filename);

        fs::read(filename).unwrap()
    }

    fn resources(&mut self, sounds: HashMap<u8, super::Sound>) {
        self.sounds = sounds;
    }

    fn spec(&mut self, version: u8) -> Spec {
        let set_flags = match version {
            1 | 2 | 3 => vec![Flag::ScreenSplittingAvailable],
            4 | 5 | 6 | 7 | 8 => vec![
                Flag::BoldfaceAvailable,
                Flag::ItalicAvailable,
                Flag::FixedSpaceAvailable,
                Flag::TimedInputAvailable,
                Flag::PicturesAvailable,
                Flag::ColoursAvailable,
                Flag::SoundEffectsAvailable,
            ],
            _ => vec![],
        };
        let clear_flags = match version {
            1 | 2 | 3 => vec![
                Flag::StatusLineNotAvailable,
                Flag::VariablePitchDefaultFont,
            ],
            4 | 5 | 6 | 7 | 8 => vec![
                Flag::GameWantsSoundEffects,
                Flag::GameWantsPictures,
                Flag::GameWantsMenus,
            ],
            _ => vec![],
        };

        // Unsplit the window
        self.window_1_bottom = 0;
        self.window_0_top = self.status_line;
        self.cursor[1].column = 0;
        self.cursor[1].line = 0;
        self.window.color_set(CursesV2::color_pair(COLOR_GREEN, COLOR_BLACK));
        self.window
            .setscrreg(self.window_0_top - 1, self.screen_lines - 1);
        self.window.scrollok(true);

        Spec {
            set_flags,
            clear_flags,
            interpreter_number: 10,
            interpreter_version: 'A' as u8,
            screen_lines: self.screen_lines as u8,
            screen_columns: self.screen_columns as u8,
            line_units: 1,
            column_units: 1,
            background_color: 2,
            foreground_color: 4,
        }
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

impl CursesV2 {
    fn pause_output(&mut self) {
        if self.selected_window == 0 {
            let max_lines = self.screen_lines - self.status_line;
            trace!("Lines: {} / {}", self.lines_since_input, max_lines);
            if self.lines_since_input >= max_lines {
                trace!("MORE!");
                self.window.mvaddstr(self.screen_lines - 1, 0, "[MORE]");
                self.window.refresh();
                self.window.nodelay(false);
                self.getch();
                self.window.nodelay(true);
                self.window.mv(self.screen_lines - 1, 0);
                self.window.clrtoeol();
                self.window.refresh();
            }
        }
    }
    fn color_pair(fg: i16, bg: i16) -> i16 {
        (fg * 8) + bg
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
