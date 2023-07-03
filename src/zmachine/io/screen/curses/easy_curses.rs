use easycurses::Color;
use easycurses::ColorPair;
use easycurses::*;
use pancurses::ALL_MOUSE_EVENTS;
use pancurses::BUTTON1_CLICKED;
use pancurses::BUTTON1_DOUBLE_CLICKED;

use crate::zmachine::io::screen::Style;
use crate::zmachine::io::screen::{self, CellStyle, InputEvent, Terminal};

pub struct ECTerminal {
    easycurses: EasyCurses,
}

pub fn new_terminal() -> Box<dyn Terminal> {
    Box::new(ECTerminal::new())
}

impl ECTerminal {
    pub fn new() -> ECTerminal {
        info!(target: "app::input", "Initialize easycurses terminal");
        let mut easycurses =
            EasyCurses::initialize_system().expect("Error initializing easycurses");
        pancurses::mousemask(ALL_MOUSE_EVENTS, None);
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
            Input::KeyBackspace => InputEvent::from_char(8),
            Input::KeyMouse => match pancurses::getmouse() {
                Ok(event) => {
                    if event.bstate & BUTTON1_CLICKED == BUTTON1_CLICKED {
                        InputEvent::from_mouse(254, event.y as u16 + 1, event.x as u16 + 1)
                    } else if event.bstate & BUTTON1_DOUBLE_CLICKED == BUTTON1_DOUBLE_CLICKED {
                        InputEvent::from_mouse(253, event.y as u16 + 1, event.x as u16 + 1)
                    } else {
                        InputEvent::no_input()
                    }
                }
                Err(e) => {
                    error!(target: "app::input", "{}", e);
                    InputEvent::no_input()
                }
            },
            _ => {
                info!(target: "app::input", "Input: {:?}", input);
                InputEvent::no_input()
            }
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
        let c = super::map_output(zchar, font);
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

    fn read_key(&mut self, wait: bool) -> InputEvent {
        self.easycurses
            .set_cursor_visibility(CursorVisibility::Visible);

        self.easycurses.set_input_mode(InputMode::RawCharacter);
        let mode = if wait {
            TimeoutMode::Never
        } else {
            TimeoutMode::Immediate
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
        let curs = self.easycurses.get_row_col_count();
        self.easycurses.move_rc(curs.0 - 1, 0);
        for _ in 0..curs.1 {
            self.easycurses.print_char(' ');
        }
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

    fn quit(&mut self) {}

    fn set_colors(&mut self, _colors: (screen::Color, screen::Color)) {}
}
