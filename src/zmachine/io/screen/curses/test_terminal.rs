use std::thread;
use std::time::Duration;

use crate::instruction::processor::tests::{input_char, input_delay, print_char, set_colors};
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

    fn read_key(&mut self, _wait: bool) -> InputEvent {
        if input_delay() > 0 {
            thread::sleep(Duration::from_millis(input_delay()));
        }

        if let Some(c) = input_char() {
            InputEvent::from_char(c as u16)
        } else {
            InputEvent::from_char('\r' as u16)
        }
    }

    fn scroll(&mut self, _row: u32) {}

    fn backspace(&mut self, _at: (u32, u32)) {}

    fn beep(&mut self) {}

    fn move_cursor(&mut self, _at: (u32, u32)) {}

    fn reset(&mut self) {}

    fn quit(&mut self) {}

    fn set_colors(&mut self, colors: (Color, Color)) {
        set_colors((colors.0 as u8, colors.1 as u8))
    }
}
