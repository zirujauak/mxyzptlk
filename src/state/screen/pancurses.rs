use pancurses::*;

use super::{Color, Terminal, buffer::CellStyle, Style};


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
        let window = pancurses::initscr();
        pancurses::curs_set(0);
        pancurses::noecho();
        pancurses::cbreak();
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

    fn input_to_u16(&self, input: Input) -> Option<u16> {
        match input {
            Input::Character(c) => match c {
                '\u{7f}' => Some(0x08),
                '\u{0a}' => Some(0x0d),
                ' '..='~' => Some(c as u16),
                '\u{e4}' => Some(0x9b),
                '\u{f6}' => Some(0x9c),
                '\u{fc}' => Some(0x9d),
                '\u{c4}' => Some(0x9e),
                '\u{d6}' => Some(0x9f),
                '\u{dc}' => Some(0xa0),
                '\u{df}' => Some(0xa1),
                '\u{bb}' => Some(0xa2),
                '\u{ab}' => Some(0xa3),
                '\u{eb}' => Some(0xa4),
                '\u{ef}' => Some(0xa5),
                '\u{ff}' => Some(0xa6),
                '\u{cb}' => Some(0xa7),
                '\u{cf}' => Some(0xa8),
                '\u{e1}' => Some(0xa9),
                '\u{e9}' => Some(0xaa),
                '\u{ed}' => Some(0xab),
                '\u{f3}' => Some(0xac),
                '\u{fa}' => Some(0xad),
                '\u{fd}' => Some(0xae),
                '\u{c1}' => Some(0xaf),
                '\u{c9}' => Some(0xb0),
                '\u{cd}' => Some(0xb1),
                '\u{d3}' => Some(0xb2),
                '\u{da}' => Some(0xb3),
                '\u{dd}' => Some(0xb4),
                '\u{e0}' => Some(0xb5),
                '\u{e8}' => Some(0xb6),
                '\u{ec}' => Some(0xb7),
                '\u{f2}' => Some(0xb8),
                '\u{f9}' => Some(0xb9),
                '\u{c0}' => Some(0xba),
                '\u{c8}' => Some(0xbb),
                '\u{cc}' => Some(0xbc),
                '\u{d2}' => Some(0xbd),
                '\u{d9}' => Some(0xbe),
                '\u{e2}' => Some(0xbf),
                '\u{ea}' => Some(0xc0),
                '\u{ee}' => Some(0xc1),
                '\u{f4}' => Some(0xc2),
                '\u{fb}' => Some(0xc3),
                '\u{c2}' => Some(0xc4),
                '\u{ca}' => Some(0xc5),
                '\u{ce}' => Some(0xc6),
                '\u{d4}' => Some(0xc7),
                '\u{db}' => Some(0xc8),
                '\u{e5}' => Some(0xc9),
                '\u{c5}' => Some(0xca),
                '\u{f8}' => Some(0xcb),
                '\u{d8}' => Some(0xcc),
                '\u{e3}' => Some(0xcd),
                '\u{f1}' => Some(0xce),
                '\u{f5}' => Some(0xcf),
                '\u{c3}' => Some(0xd0),
                '\u{d1}' => Some(0xd1),
                '\u{d5}' => Some(0xd2),
                '\u{e6}' => Some(0xd3),
                '\u{c6}' => Some(0xd4),
                '\u{e7}' => Some(0xd5),
                '\u{c7}' => Some(0xd6),
                '\u{fe}' => Some(0xd7),
                '\u{f0}' => Some(0xd8),
                '\u{de}' => Some(0xd9),
                '\u{d0}' => Some(0xda),
                '\u{a3}' => Some(0xdb),
                '\u{153}' => Some(0xdc),
                '\u{152}' => Some(0xdd),
                '\u{a1}' => Some(0xde),
                '\u{bf}' => Some(0xdf),
                _ => None,
            },
            Input::KeyUp => Some(129),
            Input::KeyDown => Some(130),
            Input::KeyLeft => Some(131),
            Input::KeyRight => Some(132),
            Input::KeyF1 => Some(133),
            Input::KeyF2 => Some(134),
            Input::KeyF3 => Some(135),
            Input::KeyF4 => Some(136),
            Input::KeyF5 => Some(137),
            Input::KeyF6 => Some(138),
            Input::KeyF7 => Some(139),
            Input::KeyF8 => Some(140),
            Input::KeyF9 => Some(141),
            Input::KeyF10 => Some(142),
            Input::KeyF11 => Some(143),
            Input::KeyF12 => Some(144),
            _ => None,
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
            }
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

    fn read_key(&mut self, timeout: u128) -> Option<u16> {
        pancurses::curs_set(1);
        pancurses::raw();
        if let Some(i) = self.window.getch() {
            pancurses::curs_set(0);
            self.input_to_u16(i)
        } else {
            pancurses::curs_set(0);
            None
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
}
