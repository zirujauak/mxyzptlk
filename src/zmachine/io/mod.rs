use std::{fs::File, io::Write};

use crate::{
    config::Config,
    error::{ErrorCode, RuntimeError},
    fatal_error, recoverable_error,
};

use self::screen::{CellStyle, Color, InputEvent, Screen, Style};

use super::state::State;

pub mod screen;

#[derive(Debug)]
struct Stream3 {
    address: usize,
    buffer: Vec<u16>,
}

impl Stream3 {
    pub fn new(address: usize) -> Stream3 {
        Stream3 {
            address,
            buffer: Vec::new(),
        }
    }

    pub fn address(&self) -> usize {
        self.address
    }

    pub fn buffer(&self) -> &Vec<u16> {
        &self.buffer
    }

    pub fn push(&mut self, c: u16) {
        self.buffer.push(c);
    }
}

#[derive(Debug)]
pub struct IO {
    version: u8,
    screen: Screen,
    output_streams: u8,
    stream_2: Option<File>,
    stream_3: Vec<Stream3>,
    buffered: bool,
}

impl IO {
    pub fn new(version: u8, config: Config) -> Result<IO, RuntimeError> {
        let screen = match version {
            3 => Screen::new_v3(config)?,
            4 => Screen::new_v4(config)?,
            5 | 7 | 8 => Screen::new_v5(config)?,
            _ => {
                return fatal_error!(
                    ErrorCode::UnsupportedVersion,
                    "Version {} is not supported",
                    version
                )
            }
        };

        Ok(IO {
            version,
            screen,
            output_streams: 0x1,
            stream_2: None,
            stream_3: Vec::new(),
            buffered: true,
        })
    }

    pub fn rows(&self) -> u32 {
        self.screen.rows()
    }

    pub fn columns(&self) -> u32 {
        self.screen.columns()
    }

    pub fn default_colors(&self) -> (Color, Color) {
        self.screen.default_colors()
    }

    // Output streams
    pub fn is_stream_2_open(&self) -> bool {
        self.stream_2.is_some()
    }

    pub fn set_stream_2(&mut self, file: File) {
        self.stream_2 = Some(file)
    }

    pub fn is_stream_enabled(&self, stream: u8) -> bool {
        let mask = (1 << (stream - 1)) & 0xF;
        self.output_streams & mask == mask
    }

    pub fn enable_output_stream(
        &mut self,
        stream: u8,
        table: Option<usize>,
    ) -> Result<(), RuntimeError> {
        if (1..4).contains(&stream) {
            let mask = (1 << (stream - 1)) & 0xF;
            self.output_streams |= mask;
            debug!(target: "app::stream", "Enable output stream {} => {:04b}", stream, self.output_streams);
            self.screen.output_stream(self.output_streams, table);
        }
        match stream {
            1 | 2 => Ok(()),
            3 => {
                if let Some(address) = table {
                    self.stream_3.push(Stream3::new(address));
                    Ok(())
                } else {
                    fatal_error!(
                        ErrorCode::Stream3Table,
                        "Stream 3 enabled without a table to write to"
                    )
                }
            }
            4 => fatal_error!(
                ErrorCode::InvalidOutputStream,
                "Stream 4 is not implemented yet"
            ),
            _ => fatal_error!(
                ErrorCode::InvalidOutputStream,
                "Stream {} is not a valid stream [1..4]",
                stream
            ),
        }
    }

    pub fn disable_output_stream(
        &mut self,
        state: &mut State,
        stream: u8,
    ) -> Result<(), RuntimeError> {
        let mask = (1 << (stream - 1)) & 0xF;
        debug!(target: "app::stream", "Disable output stream {} => {:04b}", stream, self.output_streams);
        match stream {
            1 | 2 => {
                self.output_streams &= !mask;
                Ok(())
            }
            3 => {
                if let Some(s) = self.stream_3.pop() {
                    let len = s.buffer.len();
                    state.write_word(s.address(), len as u16)?;
                    for i in 0..len {
                        state.write_byte(s.address + 2 + i, s.buffer()[i] as u8)?;
                    }
                    if self.stream_3.is_empty() {
                        self.output_streams &= !mask;
                    }
                    Ok(())
                } else {
                    Ok(())
                }
            }
            4 => fatal_error!(
                ErrorCode::InvalidOutputStream,
                "Stream 4 is not implemented yet"
            ),
            _ => fatal_error!(
                ErrorCode::InvalidOutputStream,
                "Stream {} is not a valid stream [1..4]",
                stream
            ),
        }
    }

    // Output
    pub fn transcript(&mut self, text: &[u16]) -> Result<(), RuntimeError> {
        if self.is_stream_enabled(2) {
            if let Some(f) = self.stream_2.as_mut() {
                let t: Vec<u8> = text
                    .iter()
                    .map(|c| if *c == 0x0d { 0x0a } else { *c as u8 })
                    .collect();
                if let Err(e) = f.write_all(&t) {
                    error!(target: "app::stream", "Error writing to transcript file: {}", e);
                }
                if let Err(e) = f.flush() {
                    error!(target: "app::stream", "Error writing to transcript file: {}", e);
                }
            } else {
                warn!(target: "app::stream", "Stream 2 is not open");
            }
        }

        Ok(())
    }

    pub fn print_vec(&mut self, text: &Vec<u16>) -> Result<(), RuntimeError> {
        // Stream 3 is exclusive
        if self.is_stream_enabled(3) {
            if let Some(s) = self.stream_3.last_mut() {
                for c in text {
                    match *c {
                        0 => {}
                        0xa => s.push(0xd),
                        _ => s.push(*c),
                    }
                }
            } else {
                return fatal_error!(
                    ErrorCode::Stream3Table,
                    "Stream 3 enabled, but no table to write to"
                );
            }
        } else if self.is_stream_enabled(1) {
            if self.screen.selected_window() == 1 || !self.buffered {
                self.screen.print(text);
                if self.screen.selected_window() == 0 {
                    self.transcript(text)?;
                }
            } else {
                let words = text.split_inclusive(|c| *c == 0x20);
                for word in words {
                    if self.screen.columns() - self.screen.cursor().1 < word.len() as u32 {
                        self.screen.new_line();
                        self.transcript(&[0x0a])?;
                    }

                    let w = word.to_vec();
                    self.screen.print(&w);
                    self.transcript(&w)?;
                }
            }
        }

        Ok(())
    }

    pub fn new_line(&mut self) -> Result<(), RuntimeError> {
        if self.is_stream_enabled(3) {
            if let Some(s) = self.stream_3.last_mut() {
                s.buffer.push(0xd);
            } else {
                return fatal_error!(
                    ErrorCode::Stream3Table,
                    "Stream 3 enabled, but no table to write to"
                );
            }
        } else {
            if self.is_stream_enabled(1) {
                self.screen.new_line();
            }
            if self.screen.selected_window() == 0 {
                self.transcript(&[0x0a])?;
            }
        }

        Ok(())
    }

    pub fn split_window(&mut self, lines: u16) -> Result<(), RuntimeError> {
        if lines == 0 {
            self.screen.unsplit_window();
            Ok(())
        } else {
            self.screen.split_window(lines as u32);
            if self.version == 3 {
                self.screen.erase_window(1)?;
            }
            Ok(())
        }
    }

    pub fn set_window(&mut self, window: u16) -> Result<(), RuntimeError> {
        if window > 1 {
            recoverable_error!(
                ErrorCode::InvalidWindow,
                "{} is not a valid window [0..1]",
                window
            )
        } else {
            self.screen.select_window(window as u8)
        }
    }

    pub fn erase_window(&mut self, window: i16) -> Result<(), RuntimeError> {
        match window {
            0 => self.screen.erase_window(0),
            1 => self.screen.erase_window(1),
            -1 => {
                self.screen.unsplit_window();
                self.screen.erase_window(0)
            }
            -2 => {
                self.screen.erase_window(1)?;
                self.screen.erase_window(0)
            }
            _ => recoverable_error!(
                ErrorCode::InvalidWindow,
                "{} is not a valid window to erase [-2, -1, 0, 1]",
                window
            ),
        }
    }

    pub fn erase_line(&mut self) -> Result<(), RuntimeError> {
        self.screen.erase_line();
        Ok(())
    }

    pub fn status_line(
        &mut self,
        left: &mut Vec<u16>,
        right: &mut Vec<u16>,
    ) -> Result<(), RuntimeError> {
        let width = self.screen.columns() as usize;
        let available_for_left = width - right.len() - 1;
        if left.len() > available_for_left {
            left.truncate(available_for_left - 4);
            left.push('.' as u16);
            left.push('.' as u16);
            left.push('.' as u16);
        }

        let mut spaces = vec![b' ' as u16; width - left.len() - right.len() - 2];
        let mut status_line = vec![b' ' as u16];
        status_line.append(left);
        status_line.append(&mut spaces);
        status_line.append(right);
        status_line.push(b' ' as u16);
        let mut style = CellStyle::new();
        style.set(Style::Reverse as u8);

        self.screen.print_at(&status_line, (1, 1), &style);
        self.screen.reset_cursor();
        Ok(())
    }

    pub fn set_font(&mut self, font: u16) -> Result<u16, RuntimeError> {
        Ok(self.screen.set_font(font as u8) as u16)
    }

    pub fn set_text_style(&mut self, style: u16) -> Result<(), RuntimeError> {
        self.screen.set_style(style as u8)
    }

    pub fn cursor(&mut self) -> Result<(u16, u16), RuntimeError> {
        let c = self.screen.cursor();
        Ok((c.0 as u16, c.1 as u16))
    }

    pub fn set_cursor(&mut self, row: u16, column: u16) -> Result<(), RuntimeError> {
        self.screen.move_cursor(row as u32, column as u32);
        Ok(())
    }

    pub fn buffer_mode(&mut self, mode: u16) -> Result<(), RuntimeError> {
        self.buffered = mode != 0;
        self.screen.buffer_mode(mode);
        Ok(())
    }

    pub fn beep(&mut self) -> Result<(), RuntimeError> {
        self.screen.beep();
        Ok(())
    }

    pub fn set_colors(&mut self, foreground: u16, background: u16) -> Result<(), RuntimeError> {
        self.screen.set_colors(foreground, background)
    }

    // Input
    pub fn read_key(&mut self, wait: bool) -> InputEvent {
        self.screen.read_key(wait)
    }

    pub fn backspace(&mut self) -> Result<(), RuntimeError> {
        self.screen.backspace()
    }

    // Housekeeping
    pub fn quit(&mut self) {
        self.screen.quit()
    }

    pub fn error(&mut self, instruction: &str, message: &str, recoverable: bool) -> bool {
        self.screen.error(instruction, message, recoverable)
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use crate::{
        assert_ok, assert_ok_eq, assert_print,
        test_util::{
            backspace, beep, buffer_mode, colors, cursor, input, mock_state, quit, split, style,
            test_map,
        },
    };

    use super::*;

    #[test]
    fn test_stream3_constructor() {
        let s3 = Stream3::new(0x1234);
        assert_eq!(s3.address(), 0x1234);
        assert_eq!(s3.buffer().len(), 0);
    }

    #[test]
    fn test_stream3_push() {
        let mut s3 = Stream3::new(0x1234);
        assert_eq!(s3.buffer().len(), 0);
        s3.push(0x5678);
        assert_eq!(s3.buffer(), &[0x5678]);
        s3.push(0x9abc);
        assert_eq!(s3.buffer(), &[0x5678, 0x9abc]);
    }

    #[test]
    fn test_io_constructor_v3() {
        let mut io = assert_ok!(IO::new(3, Config::default()));
        assert_eq!(io.version, 3);
        assert_eq!(io.screen.cursor(), (24, 1));
        // Version 3 top is 2
        io.screen.move_cursor(0, 0);
        assert_eq!(io.screen.cursor(), (2, 1));
        assert_eq!(io.output_streams, 0x1);
        assert!(io.stream_2.is_none());
        assert!(io.stream_3.is_empty());
        assert!(io.buffered);
        assert_eq!(io.rows(), 24);
        assert_eq!(io.columns(), 80);
        assert_eq!(io.default_colors(), (Color::White, Color::Black));
    }

    #[test]
    fn test_io_constructor_v4() {
        let mut io = assert_ok!(IO::new(4, Config::default()));
        assert_eq!(io.version, 4);
        assert_eq!(io.screen.cursor(), (24, 1));
        // Version 4 top is 1
        io.screen.move_cursor(0, 0);
        assert_eq!(io.screen.cursor(), (1, 1));
        assert_eq!(io.output_streams, 0x1);
        assert!(io.stream_2.is_none());
        assert!(io.stream_3.is_empty());
        assert!(io.buffered);
        assert_eq!(io.rows(), 24);
        assert_eq!(io.columns(), 80);
        assert_eq!(io.default_colors(), (Color::White, Color::Black));
    }

    #[test]
    fn test_io_constructor_v5() {
        let io = assert_ok!(IO::new(5, Config::default()));
        assert_eq!(io.version, 5);
        // Version 5 starts the cursor at 1,1
        assert_eq!(io.screen.cursor(), (1, 1));
        assert_eq!(io.output_streams, 0x1);
        assert!(io.stream_2.is_none());
        assert!(io.stream_3.is_empty());
        assert!(io.buffered);
        assert_eq!(io.rows(), 24);
        assert_eq!(io.columns(), 80);
        assert_eq!(io.default_colors(), (Color::White, Color::Black));
    }

    #[test]
    fn test_io_stream_2() {
        let mut io = assert_ok!(IO::new(5, Config::default()));
        assert!(!io.is_stream_2_open());
        let f = assert_ok!(File::create(Path::new("test-io.txt")));
        io.set_stream_2(f);
        assert!(Path::new("test-io.txt").exists());
        assert!(fs::remove_file(Path::new("test-io.txt")).is_ok());
        assert!(io.is_stream_2_open());
    }

    #[test]
    fn test_is_stream_enabled() {
        let io = assert_ok!(IO::new(5, Config::default()));
        assert!(io.is_stream_enabled(1));
        assert!(!io.is_stream_enabled(2));
        assert!(!io.is_stream_enabled(3));
        assert!(!io.is_stream_enabled(4));
    }

    #[test]
    fn test_enable_output_stream_2() {
        let mut io = assert_ok!(IO::new(5, Config::default()));
        assert!(io.is_stream_enabled(1));
        assert!(!io.is_stream_enabled(2));
        assert!(!io.is_stream_enabled(3));
        assert!(!io.is_stream_enabled(4));
        assert!(io.enable_output_stream(2, None).is_ok());
        assert!(io.is_stream_enabled(1));
        assert!(io.is_stream_enabled(2));
        assert!(!io.is_stream_enabled(3));
        assert!(!io.is_stream_enabled(4));
    }

    #[test]
    fn test_enable_output_stream_3() {
        let mut io = assert_ok!(IO::new(5, Config::default()));
        assert!(io.is_stream_enabled(1));
        assert!(!io.is_stream_enabled(2));
        assert!(!io.is_stream_enabled(3));
        assert!(!io.is_stream_enabled(4));
        assert!(io.enable_output_stream(3, Some(0x1234)).is_ok());
        assert!(io.is_stream_enabled(1));
        assert!(!io.is_stream_enabled(2));
        assert!(io.is_stream_enabled(3));
        assert!(!io.is_stream_enabled(4));
        assert_eq!(io.stream_3.len(), 1);
        assert_eq!(io.stream_3[0].address(), 0x1234);
        assert!(io.stream_3[0].buffer().is_empty())
    }

    #[test]
    fn test_enable_output_stream_4() {
        let mut io = assert_ok!(IO::new(5, Config::default()));
        assert!(io.is_stream_enabled(1));
        assert!(!io.is_stream_enabled(2));
        assert!(!io.is_stream_enabled(3));
        assert!(!io.is_stream_enabled(4));
        assert!(io.enable_output_stream(4, None).is_err());
        assert!(io.is_stream_enabled(1));
        assert!(!io.is_stream_enabled(2));
        assert!(!io.is_stream_enabled(3));
        assert!(!io.is_stream_enabled(4));
    }

    #[test]
    fn test_enable_output_stream_already_enabled() {
        let mut io = assert_ok!(IO::new(5, Config::default()));
        assert!(io.is_stream_enabled(1));
        assert!(!io.is_stream_enabled(2));
        assert!(!io.is_stream_enabled(3));
        assert!(!io.is_stream_enabled(4));
        assert!(io.enable_output_stream(1, None).is_ok());
        assert!(io.is_stream_enabled(1));
        assert!(!io.is_stream_enabled(2));
        assert!(!io.is_stream_enabled(3));
        assert!(!io.is_stream_enabled(4));
    }

    #[test]
    fn test_disable_output_stream_1() {
        let map = test_map(3);
        let mut state = mock_state(map);
        let mut io = assert_ok!(IO::new(3, Config::default()));
        assert!(io.is_stream_enabled(1));
        assert!(!io.is_stream_enabled(2));
        assert!(!io.is_stream_enabled(3));
        assert!(!io.is_stream_enabled(4));
        assert!(io.disable_output_stream(&mut state, 1).is_ok());
        assert!(!io.is_stream_enabled(1));
        assert!(!io.is_stream_enabled(2));
        assert!(!io.is_stream_enabled(3));
        assert!(!io.is_stream_enabled(4));
    }

    #[test]
    fn test_disable_output_stream_2() {
        let map = test_map(3);
        let mut state = mock_state(map);
        let mut io = assert_ok!(IO::new(3, Config::default()));
        assert!(io.enable_output_stream(2, None).is_ok());
        assert!(io.is_stream_enabled(1));
        assert!(io.is_stream_enabled(2));
        assert!(!io.is_stream_enabled(3));
        assert!(!io.is_stream_enabled(4));
        assert!(io.disable_output_stream(&mut state, 2).is_ok());
        assert!(io.is_stream_enabled(1));
        assert!(!io.is_stream_enabled(2));
        assert!(!io.is_stream_enabled(3));
        assert!(!io.is_stream_enabled(4));
    }

    #[test]
    fn test_disable_output_stream_3() {
        let map = test_map(3);
        let mut state = mock_state(map);
        let mut io = assert_ok!(IO::new(3, Config::default()));
        assert!(io.enable_output_stream(3, Some(0x200)).is_ok());
        assert!(io.is_stream_enabled(1));
        assert!(!io.is_stream_enabled(2));
        assert!(io.is_stream_enabled(3));
        assert!(!io.is_stream_enabled(4));
        assert!(io.stream_3.last().is_some());
        let s3 = io.stream_3.last_mut().unwrap();
        s3.push(0x20);
        s3.push(0x31);
        s3.push(0x32);
        assert!(io.enable_output_stream(3, Some(0x300)).is_ok());
        let s3 = io.stream_3.last_mut().unwrap();
        s3.push(0x40);
        s3.push(0x56);
        assert!(io.disable_output_stream(&mut state, 3).is_ok());
        assert!(io.is_stream_enabled(1));
        assert!(!io.is_stream_enabled(2));
        assert!(io.is_stream_enabled(3));
        assert!(!io.is_stream_enabled(4));
        assert_ok_eq!(state.read_word(0x300), 0x02);
        assert_ok_eq!(state.read_byte(0x302), 0x40);
        assert_ok_eq!(state.read_byte(0x303), 0x56);
        assert!(io.disable_output_stream(&mut state, 3).is_ok());
        assert!(io.is_stream_enabled(1));
        assert!(!io.is_stream_enabled(2));
        assert!(!io.is_stream_enabled(3));
        assert!(!io.is_stream_enabled(4));
        assert_ok_eq!(state.read_word(0x200), 0x03);
        assert_ok_eq!(state.read_byte(0x202), 0x20);
        assert_ok_eq!(state.read_byte(0x203), 0x31);
        assert_ok_eq!(state.read_byte(0x204), 0x32);
    }

    #[test]
    fn test_disable_output_stream_4() {
        let map = test_map(3);
        let mut state = mock_state(map);
        let mut io = assert_ok!(IO::new(3, Config::default()));
        assert!(io.is_stream_enabled(1));
        assert!(!io.is_stream_enabled(2));
        assert!(!io.is_stream_enabled(3));
        assert!(!io.is_stream_enabled(4));
        assert!(io.disable_output_stream(&mut state, 4).is_err());
        assert!(io.is_stream_enabled(1));
        assert!(!io.is_stream_enabled(2));
        assert!(!io.is_stream_enabled(3));
        assert!(!io.is_stream_enabled(4));
    }

    #[test]
    fn test_transcript_stream_2_enabled() {
        let mut io = assert_ok!(IO::new(5, Config::default()));
        assert!(!io.is_stream_2_open());
        let f = assert_ok!(File::create(Path::new("test-transcript.txt")));
        io.set_stream_2(f);
        assert!(io.enable_output_stream(2, None).is_ok());
        assert!(io.is_stream_2_open());
        assert!(Path::new("test-transcript.txt").exists());
        assert!(io
            .transcript(
                &"This is transcripting"
                    .bytes()
                    .map(|x| x as u16)
                    .collect::<Vec<u16>>()
            )
            .is_ok());
        let s = fs::read_to_string(Path::new("test-transcript.txt"));
        assert!(fs::remove_file(Path::new("test-transcript.txt")).is_ok());
        assert_ok_eq!(s, "This is transcripting");
    }

    #[test]
    fn test_transcript_stream_2_disabled() {
        let mut io = assert_ok!(IO::new(5, Config::default()));
        assert!(!io.is_stream_2_open());
        assert!(io
            .transcript(
                &"This is transcripting"
                    .bytes()
                    .map(|x| x as u16)
                    .collect::<Vec<u16>>()
            )
            .is_ok());
    }

    #[test]
    fn test_print_vec_window_0_no_buffering() {
        let mut io = assert_ok!(IO::new(5, Config::default()));
        io.buffered = false;
        assert!(io.print_vec(&"This is a very long string greater than 80 characters in length that will not be wrapped because buffering is not turned on".bytes().map(|x| x as u16).collect::<Vec<u16>>()).is_ok());
        assert_print!("This is a very long string greater than 80 characters in length that will not be wrapped because buffering is not turned on");
        assert_ok_eq!(io.cursor(), (2, 44));
    }

    #[test]
    fn test_print_vec_window_0_no_buffering_transcript() {
        let mut io = assert_ok!(IO::new(5, Config::default()));
        io.buffered = false;
        let f = assert_ok!(File::create(Path::new("test-nobuffer.txt")));
        io.set_stream_2(f);
        assert!(io.enable_output_stream(2, None).is_ok());
        assert!(io.is_stream_2_open());
        assert!(Path::new("test-nobuffer.txt").exists());
        assert!(io.print_vec(&"This is a very long string greater than 80 characters in length that will not be wrapped because buffering is not turned on".bytes().map(|x| x as u16).collect::<Vec<u16>>()).is_ok());
        let s = fs::read_to_string(Path::new("test-nobuffer.txt"));
        assert!(fs::remove_file(Path::new("test-nobuffer.txt")).is_ok());
        assert_ok_eq!(s, "This is a very long string greater than 80 characters in length that will not be wrapped because buffering is not turned on");
        assert_print!("This is a very long string greater than 80 characters in length that will not be wrapped because buffering is not turned on");
        assert_ok_eq!(io.cursor(), (2, 44));
    }

    #[test]
    fn test_print_vec_window_0_buffering_() {
        let mut io = assert_ok!(IO::new(5, Config::default()));
        io.buffered = true;
        assert!(io.print_vec(&"This is a very long string greater than 80 characters in length that will not be wrapped because buffering is not turned on".bytes().map(|x| x as u16).collect::<Vec<u16>>()).is_ok());
        assert_print!("This is a very long string greater than 80 characters in length that will not be wrapped because buffering is not turned on");
        assert_ok_eq!(io.cursor(), (2, 46));
    }

    #[test]
    fn test_print_vec_window_0_buffering_transcript() {
        let mut io = assert_ok!(IO::new(5, Config::default()));
        let f = assert_ok!(File::create(Path::new("test-buffer.txt")));
        io.set_stream_2(f);
        assert!(io.enable_output_stream(2, None).is_ok());
        assert!(io.is_stream_2_open());
        assert!(Path::new("test-buffer.txt").exists());
        io.buffered = true;
        assert!(io.print_vec(&"This is a very long string greater than 80 characters in length that will not be wrapped because buffering is not turned on".bytes().map(|x| x as u16).collect::<Vec<u16>>()).is_ok());
        let s = fs::read_to_string(Path::new("test-buffer.txt"));
        assert!(fs::remove_file(Path::new("test-buffer.txt")).is_ok());
        assert_ok_eq!(s, "This is a very long string greater than 80 characters in length that will not \nbe wrapped because buffering is not turned on");
        assert_print!("This is a very long string greater than 80 characters in length that will not be wrapped because buffering is not turned on");
        assert_ok_eq!(io.cursor(), (2, 46));
    }

    #[test]
    fn test_print_vec_window_1() {
        let mut io = assert_ok!(IO::new(5, Config::default()));
        io.screen.split_window(12);
        assert!(io.screen.select_window(1).is_ok());
        io.buffered = true;
        assert!(io.print_vec(&"This is a very long string greater than 80 characters in length that will not be wrapped because buffering is not turned on".bytes().map(|x| x as u16).collect::<Vec<u16>>()).is_ok());
        assert_print!("This is a very long string greater than 80 characters in length that will not be wrapped because buffering is not turned on");
        assert_ok_eq!(io.cursor(), (2, 44));
    }

    #[test]
    fn test_print_vec_stream_3() {
        let map = test_map(5);
        let mut state = mock_state(map);
        let mut io = assert_ok!(IO::new(5, Config::default()));
        assert!(io.enable_output_stream(3, Some(0x200)).is_ok());
        io.buffered = false;
        assert!(io
            .print_vec(
                &"This will not print to stream 1"
                    .bytes()
                    .map(|x| x as u16)
                    .collect::<Vec<u16>>()
            )
            .is_ok());
        assert_print!("");
        assert_ok_eq!(io.cursor(), (1, 1));
        assert!(io.disable_output_stream(&mut state, 3).is_ok());
        assert_ok_eq!(state.read_word(0x200), 31);
        assert_ok_eq!(state.read_byte(0x202), b'T');
        assert_ok_eq!(state.read_byte(0x203), b'h');
        assert_ok_eq!(state.read_byte(0x204), b'i');
        assert_ok_eq!(state.read_byte(0x205), b's');
        assert_ok_eq!(state.read_byte(0x206), b' ');
        assert_ok_eq!(state.read_byte(0x207), b'w');
        assert_ok_eq!(state.read_byte(0x208), b'i');
        assert_ok_eq!(state.read_byte(0x209), b'l');
        assert_ok_eq!(state.read_byte(0x20A), b'l');
        assert_ok_eq!(state.read_byte(0x20B), b' ');
        assert_ok_eq!(state.read_byte(0x20C), b'n');
        assert_ok_eq!(state.read_byte(0x20D), b'o');
        assert_ok_eq!(state.read_byte(0x20E), b't');
        assert_ok_eq!(state.read_byte(0x20F), b' ');
        assert_ok_eq!(state.read_byte(0x210), b'p');
        assert_ok_eq!(state.read_byte(0x211), b'r');
        assert_ok_eq!(state.read_byte(0x212), b'i');
        assert_ok_eq!(state.read_byte(0x213), b'n');
        assert_ok_eq!(state.read_byte(0x214), b't');
        assert_ok_eq!(state.read_byte(0x215), b' ');
        assert_ok_eq!(state.read_byte(0x216), b't');
        assert_ok_eq!(state.read_byte(0x217), b'o');
        assert_ok_eq!(state.read_byte(0x218), b' ');
        assert_ok_eq!(state.read_byte(0x219), b's');
        assert_ok_eq!(state.read_byte(0x21A), b't');
        assert_ok_eq!(state.read_byte(0x21B), b'r');
        assert_ok_eq!(state.read_byte(0x21C), b'e');
        assert_ok_eq!(state.read_byte(0x21D), b'a');
        assert_ok_eq!(state.read_byte(0x21E), b'm');
        assert_ok_eq!(state.read_byte(0x21F), b' ');
        assert_ok_eq!(state.read_byte(0x220), b'1');
    }

    #[test]
    fn test_new_line_stream_1() {
        let mut io = assert_ok!(IO::new(5, Config::default()));
        assert!(io.new_line().is_ok());
        assert_ok_eq!(io.cursor(), (2, 1));
    }

    #[test]
    fn test_new_line_stream_2() {
        let mut io = assert_ok!(IO::new(5, Config::default()));
        let f = assert_ok!(File::create(Path::new("test-newline.txt")));
        io.set_stream_2(f);
        assert!(io.enable_output_stream(2, None).is_ok());
        assert!(io.is_stream_2_open());
        assert!(Path::new("test-newline.txt").exists());
        assert!(io.new_line().is_ok());
        let s = fs::read_to_string(Path::new("test-newline.txt"));
        assert!(fs::remove_file(Path::new("test-newline.txt")).is_ok());
        assert_ok_eq!(s, "\n");
        assert_ok_eq!(io.cursor(), (2, 1));
    }

    #[test]
    fn test_new_line_stream_3() {
        let map = test_map(3);
        let mut state = mock_state(map);
        let mut io = assert_ok!(IO::new(5, Config::default()));
        assert!(io.enable_output_stream(3, Some(0x200)).is_ok());
        assert!(io.new_line().is_ok());
        assert_ok_eq!(io.cursor(), (1, 1));
        assert!(io.disable_output_stream(&mut state, 3).is_ok());
        assert_ok_eq!(state.read_word(0x200), 1);
        assert_ok_eq!(state.read_byte(0x202), 0xd);
    }

    #[test]
    fn test_new_split_window() {
        let mut io = assert_ok!(IO::new(3, Config::default()));
        assert!(io.split_window(12).is_ok());
        assert_eq!(split(), 12);
        // V3 erases the upper window
        assert_print!(&vec![' '; 960].iter().collect::<String>());
    }

    #[test]
    fn test_new_split_window_v4() {
        let mut io = assert_ok!(IO::new(4, Config::default()));
        assert!(io.split_window(12).is_ok());
        assert_eq!(split(), 12);
        assert_print!("");
    }

    #[test]
    fn test_new_split_window_unsplit() {
        let mut io = assert_ok!(IO::new(3, Config::default()));
        assert!(io.split_window(12).is_ok());
        assert_eq!(split(), 12);
        assert!(io.split_window(0).is_ok());
        assert_eq!(split(), 0);
    }

    #[test]
    fn test_set_window() {
        let mut io = assert_ok!(IO::new(3, Config::default()));
        assert!(io.split_window(12).is_ok());
        assert_eq!(io.screen.selected_window(), 0);
        assert!(io.set_window(1).is_ok());
        assert_eq!(io.screen.selected_window(), 1);
        assert!(io.set_window(0).is_ok());
        assert_eq!(io.screen.selected_window(), 0);
    }

    #[test]
    fn test_set_window_no_window_1() {
        let mut io = assert_ok!(IO::new(3, Config::default()));
        assert!(io.set_window(1).is_err());
        assert_eq!(io.screen.selected_window(), 0);
    }

    #[test]
    fn test_set_window_no_window_invalid() {
        let mut io = assert_ok!(IO::new(3, Config::default()));
        assert!(io.set_window(2).is_err());
        assert_eq!(io.screen.selected_window(), 0);
    }

    #[test]
    fn test_erase_window_0() {
        let mut io = assert_ok!(IO::new(5, Config::default()));
        assert!(io.split_window(10).is_ok());
        assert!(io.erase_window(0).is_ok());
        assert_print!(&vec![' '; 80 * 14].iter().collect::<String>());
    }

    #[test]
    fn test_erase_window_1() {
        let mut io = assert_ok!(IO::new(5, Config::default()));
        assert!(io.split_window(10).is_ok());
        assert!(io.erase_window(1).is_ok());
        assert_print!(&vec![' '; 80 * 10].iter().collect::<String>());
    }

    #[test]
    fn test_erase_window_minus_1() {
        let mut io = assert_ok!(IO::new(5, Config::default()));
        assert!(io.split_window(10).is_ok());
        assert!(io.erase_window(-1).is_ok());
        assert_print!(&vec![' '; 1920].iter().collect::<String>());
        assert_eq!(split(), 0);
    }

    #[test]
    fn test_erase_window_minus_2() {
        let mut io = assert_ok!(IO::new(5, Config::default()));
        assert!(io.split_window(10).is_ok());
        assert!(io.erase_window(-2).is_ok());
        assert_print!(&vec![' '; 1920].iter().collect::<String>());
        assert_eq!(split(), 10);
    }

    #[test]
    fn test_erase_window_invalid() {
        let mut io = assert_ok!(IO::new(5, Config::default()));
        assert!(io.split_window(10).is_ok());
        assert!(io.erase_window(-3).is_err());
        assert_print!("");
        assert_eq!(split(), 10);
    }

    #[test]
    fn test_erase_line() {
        let mut io = assert_ok!(IO::new(5, Config::default()));
        assert!(io.split_window(10).is_ok());
        assert!(io.set_cursor(15, 5).is_ok());
        assert!(io.erase_line().is_ok());
        assert_print!(&vec![' '; 75].iter().collect::<String>());
        assert_eq!(split(), 10);
    }

    #[test]
    fn test_status_line() {
        let mut io = assert_ok!(IO::new(3, Config::default()));
        assert!(io
            .status_line(
                &mut "(Darkness)".bytes().map(|x| x as u16).collect::<Vec<u16>>(),
                &mut "    0/999".bytes().map(|x| x as u16).collect::<Vec<u16>>()
            )
            .is_ok());
        assert_ok_eq!(io.cursor(), (24, 1));
        assert_print!(
            " (Darkness)                                                               0/999 "
        );
    }

    #[test]
    fn test_set_font() {
        let mut io = assert_ok!(IO::new(5, Config::default()));
        assert_ok_eq!(io.set_font(3), 1);
    }

    #[test]
    fn test_set_text_style() {
        let mut io = assert_ok!(IO::new(5, Config::default()));
        assert!(io.set_text_style(1).is_ok());
        assert_eq!(style(), 1);
    }

    #[test]
    fn test_cursor() {
        let mut io = assert_ok!(IO::new(3, Config::default()));
        assert_ok_eq!(io.cursor(), (24, 1));
        assert!(io.split_window(10).is_ok());
        assert!(io.set_window(1).is_ok());
        assert_ok_eq!(io.cursor(), (2, 1));
    }

    #[test]
    fn test_set_cursor() {
        let mut io = assert_ok!(IO::new(3, Config::default()));
        assert!(io.split_window(10).is_ok());
        assert!(io.set_cursor(13, 14).is_ok());
        assert_eq!(cursor(), (13, 14));
        assert!(io.set_window(1).is_ok());
        assert!(io.set_cursor(6, 7).is_ok());
        assert_eq!(cursor(), (6, 7));
    }

    #[test]
    fn test_set_buffer_mode_on() {
        let mut io = assert_ok!(IO::new(3, Config::default()));
        io.buffered = false;
        assert!(io.buffer_mode(1).is_ok());
        assert!(io.buffered);
        assert_eq!(buffer_mode(), 1);
    }

    #[test]
    fn test_set_buffer_mode_off() {
        let mut io = assert_ok!(IO::new(3, Config::default()));
        io.buffered = true;
        assert!(io.buffer_mode(0).is_ok());
        assert!(!io.buffered);
        assert_eq!(buffer_mode(), 0);
    }

    #[test]
    fn test_beep() {
        let mut io = assert_ok!(IO::new(3, Config::default()));
        assert!(io.beep().is_ok());
        assert!(beep());
    }

    #[test]
    fn test_set_colors() {
        let mut io = assert_ok!(IO::new(5, Config::default()));
        assert!(io.set_colors(8, 3).is_ok());
        assert_eq!(colors(), (8, 3));
    }

    #[test]
    fn test_read_key() {
        let mut io = assert_ok!(IO::new(4, Config::default()));
        input(&[' ']);
        assert_eq!(io.read_key(true), InputEvent::from_char(0x20));
    }

    #[test]
    fn test_backspace() {
        let mut io = assert_ok!(IO::new(3, Config::default()));
        assert!(io.set_cursor(10, 12).is_ok());
        assert!(io.backspace().is_ok());
        assert_eq!(backspace(), (10, 11));
    }

    #[test]
    fn test_quit() {
        let mut io = assert_ok!(IO::new(3, Config::default()));
        io.quit();
        assert!(quit());
    }
}
