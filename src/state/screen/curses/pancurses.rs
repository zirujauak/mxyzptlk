use pancurses::*;

use crate::state::screen::Color;
use crate::state::screen::InputEvent;
use crate::state::screen::Style;
use crate::state::screen::Terminal;
use crate::state::screen::buffer::CellStyle;

pub struct PCTerminal {
    window: Window,
}

fn cp(fg: i16, bg: i16) -> i16 {
    // color range 0-7, so 3 bits each
    // color pair index is 6 bits, 00ff fbbb + 1
    // pairs 1 - 64 are used by the basic colors, leaving 191 for "true" colors
    let cp = ((fg << 3) & 0x38) + (bg & 0x07) + 1;
    cp
}

impl PCTerminal {
    pub fn new() -> PCTerminal {
        info!(target: "app::input", "Initialize pancurses terminal");
        let window = pancurses::initscr();
        pancurses::curs_set(0);
        pancurses::noecho();
        pancurses::cbreak();
        pancurses::start_color();
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
            Input::Character(c) => super::char_to_u16(c),
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
        let c = super::map_output(zchar, font);
        let cp = cp(self.as_color(colors.0), self.as_color(colors.1));
        let mut attributes = 0;
        if style.is_style(Style::Bold) {
            attributes = attributes | A_BOLD;
        }
        if style.is_style(Style::Italic) {
            attributes = attributes | A_UNDERLINE;
        }
        if style.is_style(Style::Reverse) {
            attributes = attributes | A_REVERSE;
        }
        trace!(target: "app::trace", "print @ {},{} '{}' {:06b} ({:?},{:?}) -> {:08x}", row, column, c, cp, colors.0, colors.1, attributes);
        self.window.mv(row as i32 - 1, column as i32 - 1);
        self.window.addstr(format!("{}", c));
        self.window.mv(row as i32 - 1, column as i32 - 1);
        self.window.chgat(1, attributes, cp);
    }

    fn flush(&mut self) {
        self.window.refresh();
    }

    fn read_key(&mut self, _timeout: u128) -> InputEvent {
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
