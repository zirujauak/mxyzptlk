use std::fs::File;

use crate::{
    config::Config,
    error::{ErrorCode, RuntimeError},
};

use self::screen::{buffer::CellStyle, Color, InputEvent, Screen, Style};

use super::state::State;

pub mod screen;

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
}

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
                return Err(RuntimeError::new(
                    ErrorCode::UnsupportedVersion,
                    format!("Version {} is unsupported", version),
                ))
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
    pub fn is_stream_enabled(&self, stream: u8) -> bool {
        let mask = (1 << stream - 1) & 0xF;
        self.output_streams & mask == mask
    }

    pub fn enable_output_stream(
        &mut self,
        stream: u8,
        table: Option<usize>,
    ) -> Result<(), RuntimeError> {
        let mask = (1 << stream - 1) & 0xF;
        self.output_streams = self.output_streams | mask;
        match stream {
            1 => Ok(()),
            2 => todo!("Implement stream 2"),
            3 => {
                if let Some(address) = table {
                    self.stream_3.push(Stream3::new(address));
                    Ok(())
                } else {
                    Err(RuntimeError::new(
                        ErrorCode::System,
                        "Stream 3 enabled without a table to write to".to_string(),
                    ))
                }
            }
            4 => todo!("Implement stream 4"),
            _ => Err(RuntimeError::new(
                ErrorCode::System,
                format!("Stream {} is not a valid stream [1..4]", stream),
            )),
        }
    }

    pub fn disable_output_stream(
        &mut self,
        state: &mut State,
        stream: u8,
    ) -> Result<(), RuntimeError> {
        let mask = (1 << stream - 1) & 0xF;
        self.output_streams = self.output_streams & !mask;
        match stream {
            1 => Ok(()),
            2 => todo!("Implement stream 2"),
            3 => {
                if let Some(s) = self.stream_3.pop() {
                    let len = s.buffer.len();
                    state.write_word(s.address, len as u16)?;
                    for i in 0..len {
                        state.write_byte(s.address + 2 + i, s.buffer[i] as u8)?;
                    }
                    Ok(())
                } else {
                    Ok(())
                }
            }
            4 => todo!("Implement stream 4"),
            _ => Err(RuntimeError::new(
                ErrorCode::System,
                format!("Stream {} is not a valid stream [1..4]", stream),
            )),
        }
    }

    // Output
    pub fn print_vec(&mut self, text: &Vec<u16>) -> Result<(), RuntimeError> {
        // Only print to the screen if stream 3 is not selected and stream 1
        if self.is_stream_enabled(3) {
            if let Some(s) = self.stream_3.last_mut() {
                for c in text {
                    match *c {
                        0 => {},
                        0xa => s.buffer.push(0xd),
                        _ => s.buffer.push(*c)
                    }
                }
            } else {
                return Err(RuntimeError::new(
                    ErrorCode::System,
                    "Stream 3 enabled, but no table to write to".to_string(),
                ));
            }
        } else {
            if self.is_stream_enabled(1) {
                if self.screen.selected_window() == 1 || !self.buffered {
                    self.screen.print(text);
                } else {
                    let words = text.split_inclusive(|c| *c == 0x20);
                    for word in words {
                        if self.screen.columns() - self.screen.cursor().1 < word.len() as u32 {
                            self.screen.new_line();
                        }
                        self.screen.print(&word.to_vec());
                    }
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
                return Err(RuntimeError::new(
                    ErrorCode::System,
                    "Stream 3 enabled, but no table to write to".to_string(),
                ));
            }
        } else {
            if self.is_stream_enabled(1) {
                self.screen.new_line();
            }
        }
        // if self.output_streams & 0x5 == 0x1 {
        //     self.screen.new_line();
        //     if self.output_streams & 0x2 == 0x2 {
        //         self.transcript(&vec![0x0a as u16].to_vec())?;
        //     }
        // }

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
            Err(RuntimeError::new(
                ErrorCode::System,
                format!("{} is not a valid window [0..1]", window),
            ))
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
            _ => Err(RuntimeError::new(
                ErrorCode::System,
                format!("{} is not a valid window to erase [-2, -1, 0, 1]", window),
            )),
        }
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

        let mut spaces = vec![0x20 as u16; width - left.len() - right.len() - 1];
        let mut status_line = vec![0x20 as u16];
        status_line.append(left);
        status_line.append(&mut spaces);
        status_line.append(right);
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
        Ok(self.screen.move_cursor(row as u32, column as u32))
    }

    pub fn buffer_mode(&mut self, mode: u16) -> Result<(), RuntimeError> {
        self.buffered = mode != 0;
        Ok(())
    }

    pub fn beep(&mut self) -> Result<(), RuntimeError> {
        Ok(self.screen.beep())
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
}
