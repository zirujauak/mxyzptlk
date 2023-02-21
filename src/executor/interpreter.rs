pub trait Interpreter {
    fn buffer_mode(&mut self, mode: bool);
    fn erase_line(&mut self, value: u16);
    fn erase_window(&mut self, window: i16);
    fn get_cursor(&mut self) -> (u16, u16);
    fn input_stream(&mut self, stream: u16);
    fn new_line(&mut self);
    fn output_stream(&mut self, stream: i16, table: usize);
    fn print(&mut self, text: String);
    fn print_table(&mut self, text: String, width: u16, height: u16, skip: u16);
    fn read(&mut self, length: u8, time: u16) -> String;
    fn read_char(&mut self, time: u16) -> char;
    fn set_colour(&mut self, foreground: u16, background: u16);
    fn set_cursor(&mut self, line: u16, column: u16);
    fn set_text_style(&mut self, style: u16);
    fn set_window(&mut self, window: u16);
    fn sound_effect(&mut self, number: u16, effect: u16, volume: u8, repeats: u8);
    fn split_window(&mut self, lines: u16);
}
