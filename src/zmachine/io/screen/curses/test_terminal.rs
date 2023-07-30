use std::thread;
use std::time::Duration;

use crate::{
    test_util::*,
    zmachine::io::screen::{CellStyle, Color, InputEvent, Terminal},
};

pub fn new_terminal() -> Box<dyn Terminal> {
    Box::new(TestTerminal {})
}

struct TestTerminal;

impl Terminal for TestTerminal {
    fn type_name(&self) -> &str {
        "TestTerminal"
    }

    fn size(&self) -> (u32, u32) {
        (24, 80)
    }

    fn print_at(
        &mut self,
        zchar: u16,
        _row: u32,
        _column: u32,
        _colors: (Color, Color),
        _style: &CellStyle,
        _font: u8,
    ) {
        print_char((zchar as u8) as char);
    }

    fn flush(&mut self) {}

    fn read_key(&mut self, wait: bool) -> InputEvent {
        if input_timeout() {
            InputEvent::no_input()
        } else {
            if input_delay() > 0 && !wait {
                thread::sleep(Duration::from_millis(input_delay()));
            }

            if let Some(c) = input_char() {
                if c == '\u{FD}' || c == '\u{FE}' {
                    InputEvent::from_mouse(c as u16, 18, 12)
                } else {
                    InputEvent::from_char(c as u16)
                }
            } else {
                InputEvent::from_char('\r' as u16)
            }
        }
    }

    fn scroll(&mut self, row: u32) {
        set_scroll(row);
    }

    fn backspace(&mut self, at: (u32, u32)) {
        set_backspace(at);
    }

    fn beep(&mut self) {
        set_beep();
    }

    fn move_cursor(&mut self, at: (u32, u32)) {
        set_cursor(at.0, at.1)
    }

    fn reset(&mut self) {
        set_reset();
    }

    fn quit(&mut self) {
        set_quit();
    }

    fn set_colors(&mut self, colors: (Color, Color)) {
        set_colors((colors.0 as u8, colors.1 as u8))
    }

    fn split_window(&mut self, lines: u32) {
        set_split(lines as u8)
    }

    fn set_window(&mut self, window: u8) {
        set_window(window);
    }

    fn erase_window(&mut self, window: i8) {
        set_erase_window(window);
    }

    fn erase_line(&mut self) {
        set_erase_line();
    }

    fn set_style(&mut self, style: u8) {
        set_style(style)
    }

    fn buffer_mode(&mut self, mode: u16) {
        set_buffer_mode(mode);
    }

    fn output_stream(&mut self, mask: u8, table: Option<usize>) {
        set_output_stream(mask, table);
    }

    fn error(&mut self, _instruction: &str, _message: &str, _recoverable: bool) -> bool {
        todo!()
    }
}
