use crate::instruction::processor::tests::{INPUT, PRINT};
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
        PRINT.lock().unwrap().push((zchar as u8) as char);
    }

    fn flush(&mut self) {}

    fn read_key(&mut self, _wait: bool) -> InputEvent {
        if let Some(c) = INPUT.lock().unwrap().pop_front() {
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

    fn set_colors(&mut self, _colors: (Color, Color)) {}
}
