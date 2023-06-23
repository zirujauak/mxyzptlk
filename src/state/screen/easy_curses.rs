use easycurses::Color;
use easycurses::ColorPair;
use easycurses::*;

use super::super::screen;
use super::InputEvent;
use super::buffer::CellStyle;
use super::Style;
use super::Terminal;

pub struct ECTerminal {
    easycurses: EasyCurses,
}

impl ECTerminal {
    pub fn new() -> ECTerminal {
        info!(target: "app::input", "Initialize easycurses terminal");
        let mut easycurses = EasyCurses::initialize_system().unwrap();
        easycurses.set_cursor_visibility(CursorVisibility::Invisible);
        easycurses.set_echo(false);
        easycurses.set_keypad_enabled(true);
        easycurses.refresh();

        ECTerminal { easycurses }
    }

    fn as_color(&self, color: screen::Color) -> Color {
        match color {
            screen::Color::Black => Color::Black,
            screen::Color::Red => Color::Red,
            screen::Color::Green => Color::Green,
            screen::Color::Yellow => Color::Yellow,
            screen::Color::Blue => Color::Blue,
            screen::Color::Magenta => Color::Magenta,
            screen::Color::Cyan => Color::Cyan,
            screen::Color::White => Color::White,
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
            _ => InputEvent::no_input(),
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

impl Terminal for ECTerminal {
    fn size(&self) -> (u32, u32) {
        let (rows, columns) = self.easycurses.get_row_col_count();
        (rows as u32, columns as u32)
    }

    fn print_at(
        &mut self,
        zchar: u16,
        row: u32,
        column: u32,
        colors: (screen::Color, screen::Color),
        style: &CellStyle,
        font: u8,
    ) {
        let c = self.map_output(zchar, font);
        self.easycurses.move_rc(row as i32 - 1, column as i32 - 1);
        let fg = self.as_color(colors.0);
        let bg = self.as_color(colors.1);
        self.easycurses.set_bold(style.is_style(Style::Bold));
        self.easycurses
            .set_underline(zchar != 0x20 && style.is_style(Style::Italic));
        let colors = if style.is_style(Style::Reverse) {
            colorpair!(bg on fg)
        } else {
            colorpair!(fg on bg)
        };
        self.easycurses.set_color_pair(colors);
        self.easycurses.print(c.to_string());
    }

    fn flush(&mut self) {
        self.easycurses.refresh();
    }

    fn read_key(&mut self, timeout: u128) -> InputEvent {
        self.easycurses
            .set_cursor_visibility(CursorVisibility::Visible);

        self.easycurses.set_input_mode(InputMode::RawCharacter);
        let mode = if timeout > 0 {
            TimeoutMode::WaitUpTo(timeout as i32)
        } else {
            TimeoutMode::Never
        };
        self.easycurses.set_input_timeout(mode);
        if let Some(i) = self.easycurses.get_input() {
            self.easycurses
                .set_cursor_visibility(CursorVisibility::Invisible);
            self.input_to_u16(i)
        } else {
            self.easycurses
                .set_cursor_visibility(CursorVisibility::Invisible);
            InputEvent::no_input()
        }
    }

    fn scroll(&mut self, row: u32) {
        self.easycurses.move_rc(row as i32 - 1, 0);
        self.easycurses.delete_line();
    }

    fn backspace(&mut self, at: (u32, u32)) {
        self.easycurses.move_rc(at.0 as i32 - 1, at.1 as i32 - 1);
        self.easycurses.print_char(' ');
        self.easycurses.move_rc(at.0 as i32 - 1, at.1 as i32 - 1);
    }

    fn beep(&mut self) {
        self.easycurses.beep()
    }

    fn move_cursor(&mut self, at: (u32, u32)) {
        self.easycurses.move_rc(at.0 as i32 - 1, at.1 as i32 - 1);
    }

    fn reset(&mut self) {
        self.easycurses.clear();
    }

    fn quit(&mut self) {
        
    }
}
