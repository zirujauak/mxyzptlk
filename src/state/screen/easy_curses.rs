use easycurses::Color;
use easycurses::ColorPair;
use easycurses::*;

use super::super::screen;
use super::buffer::CellStyle;
use super::Style;
use super::Terminal;

pub struct ECTerminal {
    easycurses: EasyCurses,
}

impl ECTerminal {
    pub fn new() -> ECTerminal {
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

    fn input_to_u16(&self, input: Input) -> u16 {
        match input {
            Input::Character(c) => {
                match c as u8 {
                    0x0a => 0x0d,
                    0x7f => 0x08,
                    _ => c as u16,
                }
            }
            Input::KeyUp => 129,
            Input::KeyDown => 130,
            Input::KeyLeft => 131,
            Input::KeyRight => 132,
            _ => 0,
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
        c: char,
        row: u32,
        column: u32,
        colors: (screen::Color, screen::Color),
        style: &CellStyle,
    ) {
        self.easycurses.move_rc(row as i32 - 1, column as i32 - 1);
        let fg = self.as_color(colors.0);
        let bg = self.as_color(colors.1);
        self.easycurses.set_bold(style.is_style(Style::Bold));
        self.easycurses.set_underline(c != ' ' && style.is_style(Style::Bold));
        let colors = if style.is_style(Style::Reverse) {
            colorpair!(bg on fg)
        } else {
            colorpair!(fg on bg)
        };
        self.easycurses.set_color_pair(colors);
        self.easycurses.print_char(c);
    }

    fn flush(&mut self) {
        self.easycurses.refresh();
    }

    fn read_key(&mut self, timeout: u128) -> Option<u16> {
        self.easycurses.set_input_mode(InputMode::RawCharacter);
        let mode = if timeout > 0 {
            TimeoutMode::WaitUpTo(timeout as i32)
        } else {
            TimeoutMode::Never
        };
        self.easycurses.set_input_timeout(mode);
        if let Some(i) = self.easycurses.get_input() {
            trace!(target: "app::trace", "curses input: {:?}", i);
            Some(self.input_to_u16(i))
        } else {
            None
        }
    }

    fn scroll(&mut self, row: u32) {
        trace!(target: "app::trace", "Scroll up from row {}", row);
        self.easycurses.move_rc(row as i32 - 1, 0);
        self.easycurses.delete_line();
    }

    fn backspace(&mut self, at: (u32, u32)) {
        self.easycurses.move_rc(at.0 as i32 - 1, at.1 as i32 - 2);
        self.easycurses.delete_char();
    }
}
