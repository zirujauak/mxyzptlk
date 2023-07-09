use std::thread;
use std::time::Duration;

use crate::instruction::processor::tests::{
    input_char, input_delay, input_timeout, print_char, set_beep, set_buffer_mode, set_colors,
    set_erase_line, set_erase_window, set_output_stream, set_split, set_style, set_window,
};
use crate::zmachine::io::screen::{CellStyle, Color, InputEvent, Terminal};

pub fn new_terminal() -> Box<dyn Terminal> {
    Box::new(TestTerminal {})
}

struct TestTerminal;

impl Terminal for TestTerminal {
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
                InputEvent::from_char(c as u16)
            } else {
                InputEvent::from_char('\r' as u16)
            }
        }
    }

    fn scroll(&mut self, _row: u32) {}

    fn backspace(&mut self, _at: (u32, u32)) {}

    fn beep(&mut self) {
        set_beep();
    }

    fn move_cursor(&mut self, _at: (u32, u32)) {}

    fn reset(&mut self) {}

    fn quit(&mut self) {}

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
}
