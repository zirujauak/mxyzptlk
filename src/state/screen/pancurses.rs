use pancurses::*;

use super::{buffer::CellStyle, Color, Style, Terminal, InputEvent};

pub struct PCTerminal {
    window: Window,
}

fn cp(fg: i16, bg: i16) -> i16 {
    // color range 0-7, so 3 bits each
    // color pair index is 6 bits, 00ff fbbb
    ((fg << 3) & 0x38) + (bg & 0x07)
}

impl PCTerminal {
    pub fn new() -> PCTerminal {
        info!(target: "app::input", "Initialize pancurses terminal");
        let window = pancurses::initscr();
        pancurses::curs_set(0);
        pancurses::noecho();
        pancurses::cbreak();
        info!(target: "app::input", "Mouse mask: {:08x}", pancurses::mousemask(BUTTON1_CLICKED | BUTTON1_DOUBLE_CLICKED | REPORT_MOUSE_POSITION, None));
        window.keypad(true);
        window.clear();
        window.refresh();

        // Initialize fg/bg color pairs
        for fg in 0..8 {
            for bg in 0..8 {
                pancurses::init_pair(cp(fg as i16, bg as i16), fg, bg);
            }
        }

        PCTerminal { window }
    }

    fn as_color(&self, color: Color) -> i16 {
        match color {
            Color::Black => COLOR_BLACK,
            Color::Red => COLOR_RED,
            Color::Green => COLOR_GREEN,
            Color::Yellow => COLOR_YELLOW,
            Color::Blue => COLOR_BLUE,
            Color::Magenta => COLOR_MAGENTA,
            Color::Cyan => COLOR_CYAN,
            Color::White => COLOR_WHITE,
        }
    }

    fn input_to_u16(&self, input: Input) -> InputEvent {
        match input {
            Input::Character(c) => match c {
                '\u{7f}' => InputEvent::from_char(0x08),
                '\u{0a}' => InputEvent::from_char(0x0d),
                ' '..='~' => InputEvent::from_char(c as u16),
                '\u{e4}' => InputEvent::from_char(0x9b),
                '\u{f6}' => InputEvent::from_char(0x9c),
                '\u{fc}' => InputEvent::from_char(0x9d),
                '\u{c4}' => InputEvent::from_char(0x9e),
                '\u{d6}' => InputEvent::from_char(0x9f),
                '\u{dc}' => InputEvent::from_char(0xa0),
                '\u{df}' => InputEvent::from_char(0xa1),
                '\u{bb}' => InputEvent::from_char(0xa2),
                '\u{ab}' => InputEvent::from_char(0xa3),
                '\u{eb}' => InputEvent::from_char(0xa4),
                '\u{ef}' => InputEvent::from_char(0xa5),
                '\u{ff}' => InputEvent::from_char(0xa6),
                '\u{cb}' => InputEvent::from_char(0xa7),
                '\u{cf}' => InputEvent::from_char(0xa8),
                '\u{e1}' => InputEvent::from_char(0xa9),
                '\u{e9}' => InputEvent::from_char(0xaa),
                '\u{ed}' => InputEvent::from_char(0xab),
                '\u{f3}' => InputEvent::from_char(0xac),
                '\u{fa}' => InputEvent::from_char(0xad),
                '\u{fd}' => InputEvent::from_char(0xae),
                '\u{c1}' => InputEvent::from_char(0xaf),
                '\u{c9}' => InputEvent::from_char(0xb0),
                '\u{cd}' => InputEvent::from_char(0xb1),
                '\u{d3}' => InputEvent::from_char(0xb2),
                '\u{da}' => InputEvent::from_char(0xb3),
                '\u{dd}' => InputEvent::from_char(0xb4),
                '\u{e0}' => InputEvent::from_char(0xb5),
                '\u{e8}' => InputEvent::from_char(0xb6),
                '\u{ec}' => InputEvent::from_char(0xb7),
                '\u{f2}' => InputEvent::from_char(0xb8),
                '\u{f9}' => InputEvent::from_char(0xb9),
                '\u{c0}' => InputEvent::from_char(0xba),
                '\u{c8}' => InputEvent::from_char(0xbb),
                '\u{cc}' => InputEvent::from_char(0xbc),
                '\u{d2}' => InputEvent::from_char(0xbd),
                '\u{d9}' => InputEvent::from_char(0xbe),
                '\u{e2}' => InputEvent::from_char(0xbf),
                '\u{ea}' => InputEvent::from_char(0xc0),
                '\u{ee}' => InputEvent::from_char(0xc1),
                '\u{f4}' => InputEvent::from_char(0xc2),
                '\u{fb}' => InputEvent::from_char(0xc3),
                '\u{c2}' => InputEvent::from_char(0xc4),
                '\u{ca}' => InputEvent::from_char(0xc5),
                '\u{ce}' => InputEvent::from_char(0xc6),
                '\u{d4}' => InputEvent::from_char(0xc7),
                '\u{db}' => InputEvent::from_char(0xc8),
                '\u{e5}' => InputEvent::from_char(0xc9),
                '\u{c5}' => InputEvent::from_char(0xca),
                '\u{f8}' => InputEvent::from_char(0xcb),
                '\u{d8}' => InputEvent::from_char(0xcc),
                '\u{e3}' => InputEvent::from_char(0xcd),
                '\u{f1}' => InputEvent::from_char(0xce),
                '\u{f5}' => InputEvent::from_char(0xcf),
                '\u{c3}' => InputEvent::from_char(0xd0),
                '\u{d1}' => InputEvent::from_char(0xd1),
                '\u{d5}' => InputEvent::from_char(0xd2),
                '\u{e6}' => InputEvent::from_char(0xd3),
                '\u{c6}' => InputEvent::from_char(0xd4),
                '\u{e7}' => InputEvent::from_char(0xd5),
                '\u{c7}' => InputEvent::from_char(0xd6),
                '\u{fe}' => InputEvent::from_char(0xd7),
                '\u{f0}' => InputEvent::from_char(0xd8),
                '\u{de}' => InputEvent::from_char(0xd9),
                '\u{d0}' => InputEvent::from_char(0xda),
                '\u{a3}' => InputEvent::from_char(0xdb),
                '\u{153}' => InputEvent::from_char(0xdc),
                '\u{152}' => InputEvent::from_char(0xdd),
                '\u{a1}' => InputEvent::from_char(0xde),
                '\u{bf}' => InputEvent::from_char(0xdf),
                _ => InputEvent::no_input(),
            },
            Input::KeyUp => InputEvent::from_char(129),
            Input::KeyDown => InputEvent::from_char(130),
            Input::KeyLeft => InputEvent::from_char(131),
            Input::KeyRight => InputEvent::from_char(132),
            Input::KeyF1 => InputEvent::from_char(133),
            Input::KeyF2 => InputEvent::from_char(134),
            Input::KeyF3 => InputEvent::from_char(135),
            Input::KeyF4 => InputEvent::from_char(136),
            Input::KeyF5 => InputEvent::from_char(137),
            Input::KeyF6 => InputEvent::from_char(138),
            Input::KeyF7 => InputEvent::from_char(139),
            Input::KeyF8 => InputEvent::from_char(140),
            Input::KeyF9 => InputEvent::from_char(141),
            Input::KeyF10 => InputEvent::from_char(142),
            Input::KeyF11 => InputEvent::from_char(143),
            Input::KeyF12 => InputEvent::from_char(144),
            Input::KeyMouse => {
                match pancurses::getmouse() {
                    Ok(event) => {
                        info!(target: "app::input", "Mouse: {:?}", event);
                        if event.bstate & BUTTON1_CLICKED == BUTTON1_CLICKED {
                            InputEvent::from_mouse(254, event.y as u16 + 1, event.x as u16 + 1)
                        } else if event.bstate & BUTTON1_DOUBLE_CLICKED == BUTTON1_DOUBLE_CLICKED {
                            InputEvent::from_mouse(253, event.y as u16 + 1, event.x as u16 + 1)
                        } else {
                            InputEvent::no_input()
                        }
                    },
                    Err(e) => {
                        error!(target: "app::input", "{}", e);
                        InputEvent::no_input()}
                }
            }
            _ => {
                info!(target: "app::input", "Input: {:?}", input);
                InputEvent::no_input()
            }
        }
    }

    fn map_output(&self, zchar: u16, font: u8) -> char {
        match font {
            1 | 4 => match zchar {
                0x9b => '\u{e4}',
                0x9c => '\u{f6}',
                0x9d => '\u{fc}',
                0x9e => '\u{c4}',
                0x9f => '\u{d6}',
                0xa0 => '\u{dc}',
                0xa1 => '\u{df}',
                0xa2 => '\u{bb}',
                0xa3 => '\u{ab}',
                0xa4 => '\u{eb}',
                0xa5 => '\u{ef}',
                0xa6 => '\u{ff}',
                0xa7 => '\u{cb}',
                0xa8 => '\u{cf}',
                0xa9 => '\u{e1}',
                0xaa => '\u{e9}',
                0xab => '\u{ed}',
                0xac => '\u{f3}',
                0xad => '\u{fa}',
                0xae => '\u{fd}',
                0xaf => '\u{c1}',
                0xb0 => '\u{c9}',
                0xb1 => '\u{cd}',
                0xb2 => '\u{d3}',
                0xb3 => '\u{da}',
                0xb4 => '\u{dd}',
                0xb5 => '\u{e0}',
                0xb6 => '\u{e8}',
                0xb7 => '\u{ec}',
                0xb8 => '\u{f2}',
                0xb9 => '\u{f9}',
                0xba => '\u{c0}',
                0xbb => '\u{c8}',
                0xbc => '\u{cc}',
                0xbd => '\u{d2}',
                0xbe => '\u{d9}',
                0xbf => '\u{e2}',
                0xc0 => '\u{ea}',
                0xc1 => '\u{ee}',
                0xc2 => '\u{f4}',
                0xc3 => '\u{fb}',
                0xc4 => '\u{c2}',
                0xc5 => '\u{ca}',
                0xc6 => '\u{ce}',
                0xc7 => '\u{d4}',
                0xc8 => '\u{db}',
                0xc9 => '\u{e5}',
                0xca => '\u{c5}',
                0xcb => '\u{f8}',
                0xcc => '\u{d8}',
                0xcd => '\u{e3}',
                0xce => '\u{f1}',
                0xcf => '\u{f5}',
                0xd0 => '\u{c3}',
                0xd1 => '\u{d1}',
                0xd2 => '\u{d5}',
                0xd3 => '\u{e6}',
                0xd4 => '\u{c6}',
                0xd5 => '\u{e7}',
                0xd6 => '\u{c7}',
                0xd7 => '\u{fe}',
                0xd8 => '\u{f0}',
                0xd9 => '\u{de}',
                0xda => '\u{d0}',
                0xdb => '\u{a3}',
                0xdc => '\u{153}',
                0xdd => '\u{152}',
                0xde => '\u{a1}',
                0xdf => '\u{bf}',
                _ => (zchar as u8) as char,
            },
            3 => match zchar {
                0x20 => ' ',
                0x21 => '\u{2190}',
                0x22 => '\u{2192}',
                0x23 => '\u{2571}',
                0x24 => '\u{2572}',
                0x25 => ' ',
                0x26 => '\u{2500}',
                0x27 => '\u{2500}',
                0x28 => '\u{2502}',
                0x29 => '\u{2502}',
                0x2a => '\u{2534}',
                0x2b => '\u{252c}',
                0x2c => '\u{251c}',
                0x2d => '\u{2524}',
                0x2e => '\u{2514}',
                0x2f => '\u{250c}',
                0x30 => '\u{2510}',
                0x31 => '\u{2518}',
                0x32 => '\u{2514}',
                0x33 => '\u{250c}',
                0x34 => '\u{2510}',
                0x35 => '\u{2518}',
                0x36 => '\u{2588}',
                0x37 => '\u{2580}',
                0x38 => '\u{2584}',
                0x39 => '\u{258c}',
                0x3a => '\u{2590}',
                0x3b => '\u{2580}',
                0x3c => '\u{2584}',
                0x3d => '\u{258c}',
                0x3e => '\u{2514}',
                0x3f => '\u{250c}',
                0x40 => '\u{2510}',
                0x41 => '\u{2518}',
                0x42 => '\u{2514}',
                0x43 => '\u{250c}',
                0x44 => '\u{2510}',
                0x45 => '\u{2518}',
                0x46 => '\u{2598}',
                0x47 => '\u{259d}',
                0xb3 => '\u{2502}',
                0xbf => '\u{2510}',
                0xc0 => '\u{2514}',
                0xc4 => '\u{2500}',
                0xd9 => '\u{2518}',
                0xda => '\u{250c}',

                _ => {
                    trace!("Font 3 {:02x}", zchar as u8);
                    zchar as u8 as char
                }
            },
            _ => '@',
        }
    }
}

impl Terminal for PCTerminal {
    fn size(&self) -> (u32, u32) {
        let (rows, columns) = self.window.get_max_yx();
        (rows as u32, columns as u32)
    }

    fn print_at(
        &mut self,
        zchar: u16,
        row: u32,
        column: u32,
        colors: (Color, Color),
        style: &CellStyle,
        font: u8,
    ) {
        let mut c = self.map_output(zchar, font).to_chtype();
        let cp = cp(self.as_color(colors.0), self.as_color(colors.1));
        let mut attributes = 0;
        if style.is_style(Style::Bold) {
            attributes = attributes | A_BOLD;
        }
        if style.is_style(Style::Italic) {
            attributes = attributes | A_ITALIC;
        }
        if style.is_style(Style::Reverse) {
            attributes = attributes | A_REVERSE;
        }
        self.window.mv(row as i32 - 1, column as i32 - 1);
        c = c | attributes | (cp << 8) as u32;
        self.window.addch(c);
    }

    fn flush(&mut self) {
        self.window.refresh();
    }

    fn read_key(&mut self, timeout: u128) -> InputEvent {
        pancurses::curs_set(1);
        pancurses::raw();
        if let Some(i) = self.window.getch() {
            pancurses::curs_set(0);
            self.input_to_u16(i)
        } else {
            pancurses::curs_set(0);
            InputEvent::no_input()
        }
    }

    fn scroll(&mut self, row: u32) {
        self.window.mv(row as i32 - 1, 0);
        self.window.deleteln();
    }

    fn backspace(&mut self, at: (u32, u32)) {
        self.window.mv(at.0 as i32 - 1, at.1 as i32 - 1);
        self.window.delch();
        self.window.mv(at.0 as i32 - 1, at.1 as i32 - 1);
    }

    fn beep(&mut self) {
        pancurses::beep();
    }

    fn move_cursor(&mut self, at: (u32, u32)) {
        self.window.mv(at.0 as i32 - 1, at.1 as i32 - 1);
    }

    fn reset(&mut self) {
        self.window.clear();
    }

    fn quit(&mut self) {
        info!(target: "app::input", "Closing pancurses terminal");
        pancurses::curs_set(2);
        pancurses::endwin();
        pancurses::doupdate();
        pancurses::reset_shell_mode();
    }
}
