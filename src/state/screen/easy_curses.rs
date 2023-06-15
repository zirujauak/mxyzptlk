use easycurses::*;
use easycurses::Color;
use easycurses::ColorPair;

use super::Terminal;
use super::super::screen;

pub struct ECTerminal {
    easycurses: EasyCurses,
}

impl ECTerminal {
    pub fn new() -> ECTerminal {
        let mut easycurses = EasyCurses::initialize_system().unwrap();
        easycurses.set_cursor_visibility(CursorVisibility::Invisible);
        easycurses.set_echo(false);
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
}

impl Terminal for ECTerminal {
    fn size(&self) -> (u32,u32) {
        let (rows, columns) = self.easycurses.get_row_col_count();
        (rows as u32, columns as u32)
    }

    fn print_at(&mut self, c: char, row: u32, column: u32, colors: (screen::Color, screen::Color)) {
        self.easycurses.move_rc(row as i32 - 1, column as i32 - 1);
        let fg = self.as_color(colors.0);
        let bg = self.as_color(colors.1);
        self.easycurses.set_color_pair(colorpair!(fg on bg));
        self.easycurses.print_char(c);
    }

    fn flush(&mut self) {
        self.easycurses.refresh();
    }

    fn read_key(&mut self) {
        self.easycurses.get_input();
    }

    fn scroll(&mut self, row: u32) {
        self.easycurses.move_rc(row as i32 - 1, 0);
        self.easycurses.delete_line();
    }
}