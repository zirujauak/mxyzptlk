mod frame_stack;
pub mod header;
mod input;
mod instruction;
pub mod memory;
mod object;
mod rng;
mod screen;
mod text;

use std::fs;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use crate::error::*;
use crate::iff::quetzal::Quetzal;
use frame_stack::*;
use header::*;
use instruction::*;
use memory::*;
use object::property;
use rng::chacha_rng::*;
use rng::RNG;
use screen::buffer::CellStyle;
use screen::*;

use self::frame_stack::frame::Frame;

pub struct Stream3 {
    table: usize,
    buffer: Vec<u16>,
}

pub struct State {
    name: String,
    version: u8,
    memory: Memory,
    dynamic: Vec<u8>,
    static_mark: usize,
    screen: Screen,
    frame_stack: FrameStack,
    rng: Box<dyn RNG>,
    output_streams: u8,
    stream_2: Option<File>,
    stream_3: Vec<Stream3>,
    undo_stack: Vec<Quetzal>,
    input_interrupt: Option<u16>,
    input_interrupt_print: bool,
    buffered: bool,
}

impl State {
    pub fn new(memory: Memory, name: &str) -> Result<State, RuntimeError> {
        let version = header::field_byte(&memory, HeaderField::Version)?;
        let static_mark = header::field_word(&memory, HeaderField::StaticMark)? as usize;
        let mut dynamic = Vec::new();
        for i in 0..static_mark {
            dynamic.push(memory.read_byte(i)?);
        }

        let rng = ChaChaRng::new();

        if version < 3 || version == 6 || version > 8 {
            Err(RuntimeError::new(
                ErrorCode::UnsupportedVersion,
                format!("Version {} is not currently supported", version),
            ))
        } else {
            let screen = match version {
                3 => Screen::new_v3(Color::White, Color::Black),
                4 => Screen::new_v4(Color::White, Color::Black),
                _ => Screen::new_v5(Color::White, Color::Black),
            };
            let frame_stack =
                FrameStack::new(header::field_word(&memory, HeaderField::InitialPC)? as usize);

            Ok(State {
                name: name.to_string(),
                version,
                memory,
                dynamic,
                static_mark: static_mark,
                screen,
                frame_stack,
                rng: Box::new(rng),
                output_streams: 0x1,
                stream_2: None,
                stream_3: Vec::new(),
                undo_stack: Vec::new(),
                input_interrupt: None,
                input_interrupt_print: false,
                buffered: true,
            })
        }
    }

    pub fn initialize(&mut self) -> Result<(), RuntimeError> {
        // Set V3 Flags 1
        if self.version < 4 {
            header::clear_flag1(&mut self.memory, Flags1v3::StatusLineNotAvailable as u8)?;
            header::set_flag1(&mut self.memory, Flags1v3::ScreenSplitAvailable as u8)?;
            header::clear_flag1(&mut self.memory, Flags1v3::VariablePitchDefault as u8)?;
        }

        // Set V4+ Flags 1
        if self.version > 3 {
            header::set_byte(
                &mut self.memory,
                HeaderField::DefaultBackground,
                Color::Black as u8,
            )?;
            header::set_byte(
                &mut self.memory,
                HeaderField::DefaultForeground,
                Color::White as u8,
            )?;
            header::set_byte(
                &mut self.memory,
                HeaderField::ScreenLines,
                self.screen.rows() as u8,
            )?;
            header::set_byte(
                &mut self.memory,
                HeaderField::ScreenColumns,
                self.screen.columns() as u8,
            )?;

            header::set_flag1(&mut self.memory, Flags1v4::SoundEffectsAvailable as u8)?;
        }

        // Set V5+ Flags 1
        if self.version > 4 {
            header::set_word(
                &mut self.memory,
                HeaderField::ScreenHeight,
                self.screen.rows() as u16,
            )?;
            header::set_word(
                &mut self.memory,
                HeaderField::ScreenWidth,
                self.screen.columns() as u16,
            )?;
            header::set_byte(&mut self.memory, HeaderField::FontWidth, 1)?;
            header::set_byte(&mut self.memory, HeaderField::FontHeight, 1)?;
            header::set_flag1(&mut self.memory, Flags1v4::ColoursAvailable as u8)?;
            header::set_flag1(&mut self.memory, Flags1v4::BoldfaceAvailable as u8)?;
            header::set_flag1(&mut self.memory, Flags1v4::ItalicAvailable as u8)?;
            header::set_flag1(&mut self.memory, Flags1v4::FixedSpaceAvailable as u8)?;
            header::set_flag1(&mut self.memory, Flags1v4::TimedInputAvailable as u8)?;
            header::clear_flag2(&mut self.memory, Flags2::RequestMouse)?;
        }

        // Interpreter # and version
        self.write_byte(0x1E, 6)?;
        self.write_byte(0x1F, 'Z' as u8)?;

        // Z-Machine standard compliance
        self.write_byte(0x32, 1)?;
        self.write_byte(0x33, 0)?;

        self.screen.reset();

        Ok(())
    }

    pub fn memory(&self) -> &Memory {
        &self.memory
    }

    pub fn frame_stack(&self) -> &FrameStack {
        &self.frame_stack
    }

    pub fn dynamic(&self) -> &Vec<u8> {
        &self.dynamic
    }

    pub fn screen(&self) -> &Screen {
        &self.screen
    }

    pub fn stream_2_mut(&mut self) -> Result<&mut File, RuntimeError> {
        if let Some(f) = self.stream_2.as_mut() {
            Ok(f)
        } else {
            Err(RuntimeError::new(
                ErrorCode::System,
                "Stream 2 not initialized".to_string(),
            ))
        }
    }

    pub fn input_interrupt(&mut self) -> Option<u16> {
        let v = self.input_interrupt;
        self.input_interrupt = None;
        v
    }

    pub fn checksum(&self) -> Result<u16, RuntimeError> {
        let mut checksum = 0 as u16;
        let size = header::field_word(self.memory(), HeaderField::FileLength)? as usize
            * match header::field_byte(self.memory(), HeaderField::Version)? {
                1 | 2 | 3 => 2,
                4 | 5 => 4,
                6 | 7 | 8 => 8,
                _ => 0,
            };
        for i in 0x40..self.dynamic().len() {
            checksum = u16::overflowing_add(checksum, self.dynamic[i] as u16).0;
        }

        for i in self.dynamic.len()..size {
            checksum = u16::overflowing_add(checksum, self.memory().read_byte(i)? as u16).0;
        }
        Ok(checksum)
    }

    // MMU
    pub fn read_byte(&self, address: usize) -> Result<u8, RuntimeError> {
        if address < 0x10000 {
            self.memory.read_byte(address)
        } else {
            Err(RuntimeError::new(
                ErrorCode::IllegalAccess,
                format!("Byte address {:#06x} is in high memory", address),
            ))
        }
    }

    pub fn read_word(&self, address: usize) -> Result<u16, RuntimeError> {
        if address < 0xFFFF {
            self.memory.read_word(address)
        } else {
            Err(RuntimeError::new(
                ErrorCode::IllegalAccess,
                format!("Word address {:#06x} is in high memory", address),
            ))
        }
    }

    pub fn write_byte(&mut self, address: usize, value: u8) -> Result<(), RuntimeError> {
        if address < self.static_mark {
            self.memory.write_byte(address, value)
        } else {
            Err(RuntimeError::new(
                ErrorCode::IllegalAccess,
                format!(
                    "Byte address {:#04x} is above the end of dynamic memory ({:#04x})",
                    address, self.static_mark
                ),
            ))
        }
    }

    pub fn write_word(&mut self, address: usize, value: u16) -> Result<(), RuntimeError> {
        if address < self.static_mark - 1 {
            if address == 0x10 {
                if value & 0x0001 == 0x0001 {
                    self.output_stream(2, None)?;
                } else {
                    self.output_stream(-2, None)?;
                }
            }
            self.memory.write_word(address, value)?;
            Ok(())
        } else {
            Err(RuntimeError::new(
                ErrorCode::IllegalAccess,
                format!(
                    "Word address {:#04x} is above the end of dynamic memory ({:#04x})",
                    address, self.static_mark
                ),
            ))
        }
    }

    // Variables
    pub fn variable(&mut self, variable: u8) -> Result<u16, RuntimeError> {
        self.frame_stack
            .variable(&mut self.memory, variable as usize)
    }

    pub fn peek_variable(&mut self, variable: u8) -> Result<u16, RuntimeError> {
        if variable == 0 {
            self.frame_stack.current_frame()?.peek()
        } else {
            self.frame_stack
                .variable(&mut self.memory, variable as usize)
        }
    }

    pub fn set_variable(&mut self, variable: u8, value: u16) -> Result<(), RuntimeError> {
        self.frame_stack
            .set_variable(&mut self.memory, variable as usize, value)
    }

    pub fn set_variable_indirect(&mut self, variable: u8, value: u16) -> Result<(), RuntimeError> {
        if variable == 0 {
            self.frame_stack.current_frame_mut()?.pop()?;
        }
        self.frame_stack
            .set_variable(&mut self.memory, variable as usize, value)
    }

    pub fn push(&mut self, value: u16) -> Result<(), RuntimeError> {
        Ok(self.frame_stack.current_frame_mut()?.push(value))
    }

    // Routines
    pub fn call_routine(
        &mut self,
        address: usize,
        arguments: &Vec<u16>,
        result: Option<StoreResult>,
        return_address: usize,
    ) -> Result<usize, RuntimeError> {
        self.frame_stack.call_routine(
            &mut self.memory,
            address,
            arguments,
            result,
            return_address,
        )?;
        self.frame_stack.pc()
    }

    pub fn read_interrupt(
        &mut self,
        address: usize,
        return_address: usize,
    ) -> Result<usize, RuntimeError> {
        self.input_interrupt = None;
        self.input_interrupt_print = false;
        self.frame_stack
            .input_interrupt(&mut self.memory, address, return_address)?;
        self.frame_stack.pc()
    }

    pub fn return_routine(&mut self, value: u16) -> Result<usize, RuntimeError> {
        if self.frame_stack.current_frame()?.input_interrupt() {
            self.input_interrupt = Some(value);
        }

        let result = self.frame_stack.return_routine(&mut self.memory, value)?;
        match result {
            Some(r) => self.set_variable(r.variable(), value)?,
            None => (),
        }

        self.frame_stack.pc()
    }

    pub fn throw(&mut self, depth: u16, result: u16) -> Result<usize, RuntimeError> {
        self.frame_stack.frames_mut().truncate(depth as usize);
        self.return_routine(result)
    }

    pub fn set_pc(&mut self, pc: usize) -> Result<(), RuntimeError> {
        self.frame_stack.current_frame_mut()?.set_pc(pc);
        Ok(())
    }

    // RNG
    pub fn random(&mut self, range: u16) -> u16 {
        let v = self.rng.random(range);
        v
    }

    pub fn seed(&mut self, seed: u16) {
        self.rng.seed(seed)
    }

    pub fn predictable(&mut self, seed: u16) {
        self.rng.predictable(seed)
    }

    // Screen
    pub fn output_stream(&mut self, stream: i16, table: Option<usize>) -> Result<(), RuntimeError> {
        if stream > 0 {
            let mask = (1 << stream - 1) & 0xF;
            if stream == 2 {
                if let None = self.stream_2 {
                    self.stream_2 = Some(self.prompt_and_create("Transcript file: ", "txt")?);
                }
                header::set_flag2(&mut self.memory, Flags2::Transcripting)?;
            } else if stream == 3 {
                self.stream_3.push(Stream3 {
                    table: table.unwrap(),
                    buffer: Vec::new(),
                });
            }
            self.output_streams = self.output_streams | mask;
        } else if stream == -2 {
            header::clear_flag2(&mut self.memory, Flags2::Transcripting)?;
        } else if stream == -3 {
            let stream3 = self.stream_3.pop().unwrap();
            let len = stream3.buffer.len();
            self.write_word(stream3.table, len as u16)?;
            for i in 0..len {
                self.write_byte(stream3.table + 2 + i, stream3.buffer[i] as u8)?;
            }

            if self.stream_3.is_empty() {
                self.output_streams = self.output_streams & 0xb;
            }
        } else if stream < 0 {
            let mask = !(1 << (stream.abs() - 1 & 0xF));
            self.output_streams = self.output_streams & mask;
        }

        Ok(())
    }

    fn transcript(&mut self, text: &Vec<u16>) -> Result<(), RuntimeError> {
        match self.stream_2.as_mut() {
            Some(f) => match f.write_all(
                &text
                    .iter()
                    .map(|c| if *c == 0x0d { 0x0a } else { *c as u8 })
                    .collect::<Vec<u8>>(),
            ) {
                Ok(_) => (),
                Err(e) => error!(target: "app::trace", "Error writing transcript: {:?}", e),
            },
            None => error!(target: "app::trace", "Stream 2 not initialized"),
        }

        Ok(())
    }
    pub fn print(&mut self, text: &Vec<u16>) -> Result<(), RuntimeError> {
        // Only print to the screen if stream 3 is not selected and stream 1
        if self.output_streams & 0x5 == 0x1 {
            let s2 = self.output_streams & 0x2 == 0x2;
            if self.screen.selected_window() == 1 || !self.buffered {
                self.screen.print(text);
                if self.screen.selected_window() == 0 && s2 {
                    self.transcript(text)?;
                }
            } else {
                let words = text.split_inclusive(|c| *c == 0x20);
                for word in words {
                    if self.screen.columns() - self.screen.cursor().1 < word.len() as u32 {
                        self.screen.new_line();
                        if s2 {
                            self.transcript(&[0x0d as u16].to_vec())?;
                        }
                    }
                    self.screen.print(&word.to_vec());
                    if s2 {
                        self.transcript(&word.to_vec())?;
                    }
                }
            }

            if s2 {
                if let Err(e) = self.stream_2_mut()?.flush() {
                    error!(target: "app::trace", "Error flushing transcript: {}", e);
                }
            }
        }

        if self.frame_stack.current_frame()?.input_interrupt() {
            self.input_interrupt_print = true;
        }

        if self.output_streams & 0x4 == 0x4 {
            let stream3 = self.stream_3.last_mut().unwrap();
            for c in text {
                if *c != 0 {
                    stream3.buffer.push(*c);
                }
            }
        }
        Ok(())
    }

    pub fn split_window(&mut self, lines: u16) -> Result<(), RuntimeError> {
        Ok(self.screen.split_window(lines as u32))
    }

    pub fn set_window(&mut self, window: u16) -> Result<(), RuntimeError> {
        self.screen.select_window(window as u8)
    }

    pub fn erase_window(&mut self, window: i16) -> Result<(), RuntimeError> {
        self.screen.erase_window(window as i8)
    }

    pub fn status_line(&mut self) -> Result<(), RuntimeError> {
        let status_type = header::flag1(self.memory(), Flags1v3::StatusLineType as u8)?;
        let object = self.variable(16)? as usize;
        let mut left = text::from_vec(self, &property::short_name(self, object)?)?;

        let mut right: Vec<u16> = if status_type == 0 {
            let score = self.variable(17)? as i16;
            let turns = self.variable(18)?;
            format!("{:<8}", format!("{}/{}", score, turns))
                .as_bytes()
                .iter()
                .map(|x| *x as u16)
                .collect()
        } else {
            let hour = self.variable(17)?;
            let minute = self.variable(18)?;
            let suffix = if hour > 11 { "PM" } else { "AM" };
            format!("{} ", format!("{}:{} {}", hour % 12, minute, suffix))
                .as_bytes()
                .iter()
                .map(|x| *x as u16)
                .collect()
        };

        let width = self.screen.columns() as usize;
        let mut spaces = vec![0x20 as u16; width - left.len() - right.len() - 1];
        let mut status_line = vec![0x20 as u16];
        status_line.append(&mut left);
        status_line.append(&mut spaces);
        status_line.append(&mut right);
        let mut style = CellStyle::new();
        style.set(Style::Reverse as u8);

        self.screen.print_at(&status_line, (1, 1), &style);
        self.screen.reset_cursor();
        Ok(())
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
    pub fn read_key(&mut self, timeout: u16) -> Result<Option<u16>, RuntimeError> {
        trace!(target: "app::trace.log", "read_key timeout {:?}", timeout);
        let key = self.screen.read_key(timeout as u128 * 1000);
        info!(target: "app::input", "read_key -> {:?}", key);
        Ok(key)
    }

    pub fn read_line(
        &mut self,
        text: &Vec<u16>,
        len: usize,
        terminators: &Vec<u16>,
        timeout: u16,
    ) -> Result<Vec<u16>, RuntimeError> {
        let mut input_buffer = text.clone();

        // TODO: Set a timeout
        let end = if timeout > 0 {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Error getting system time")
                .as_millis()
                + (timeout as u128 * 1000)
        } else {
            0
        };

        loop {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Error getting system time")
                .as_millis();
            if end > 0 && now > end {
                return Ok(input_buffer);
            }

            let timeout = if end > 0 { end - now } else { 0 };

            info!(target: "app::input", "Now: {}, End: {}, Timeout: {}", now, end, timeout);

            match self.screen.read_key(timeout) {
                Some(key) => {
                    if terminators.contains(&key) {
                        input_buffer.push(key);
                        self.print(&vec![key])?;
                        break;
                    } else {
                        if key == 0x08 {
                            if input_buffer.len() > 0 {
                                input_buffer.pop();
                                self.backspace()?;
                            }
                        } else if input_buffer.len() < len && key >= 0x1f && key <= 0x7f {
                            input_buffer.push(key);
                            self.print(&vec![key])?;
                        }
                    }
                }
                None => break,
            }
        }

        Ok(input_buffer)
    }

    // Save/restore
    pub fn prompt_and_create(&mut self, prompt: &str, suffix: &str) -> Result<File, RuntimeError> {
        self.print(&prompt.chars().map(|c| c as u16).collect())?;
        let n = format!("{}.{}", self.name, suffix)
            .chars()
            .map(|c| c as u16)
            .collect();
        self.print(&n)?;

        let f = self.read_line(&n, 32, &vec!['\r' as u16], 0)?;
        let filename = String::from_utf16(&f).unwrap();
        trace!(target: "app::trace", "Save '{}'", filename.trim());
        let file = fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(filename.trim())
            .unwrap();

        Ok(file)
    }

    pub fn prompt_and_write(
        &mut self,
        prompt: &str,
        suffix: &str,
        data: &Vec<u8>,
    ) -> Result<(), RuntimeError> {
        let mut file = self.prompt_and_create(prompt, suffix)?;
        // self.print(&prompt.chars().map(|c| c as u16).collect())?;
        // let n = format!("{}.{}", self.name, suffix)
        //     .chars()
        //     .map(|c| c as u16)
        //     .collect();
        // self.print(&n)?;

        // let f = self.read_line(&n, 32, &vec!['\r' as u16], 0)?;
        // let filename = String::from_utf16(&f).unwrap();
        // trace!(target: "app::trace", "Save '{}'", filename.trim());
        // let mut file = fs::OpenOptions::new()
        //     .create(true)
        //     .truncate(true)
        //     .write(true)
        //     .open(filename.trim())
        //     .unwrap();

        match file.write_all(data) {
            Ok(_) => (),
            Err(e) => return Err(RuntimeError::new(ErrorCode::System, format!("{}", e))),
        };
        match file.flush() {
            Ok(_) => Ok(()),
            Err(e) => Err(RuntimeError::new(ErrorCode::System, format!("{}", e))),
        }
    }

    pub fn prompt_and_read(&mut self, prompt: &str, suffix: &str) -> Result<Vec<u8>, RuntimeError> {
        self.print(&prompt.chars().map(|c| c as u16).collect())?;
        let n = format!("{}.{}", self.name, suffix)
            .chars()
            .map(|c| c as u16)
            .collect();
        self.print(&n)?;

        let f = self.read_line(&n, 32, &vec!['\r' as u16], 0)?;
        let filename = String::from_utf16(&f).unwrap();
        trace!(target: "app::trace", "Restore '{}'", filename.trim());
        let mut data = Vec::new();
        match File::open(filename.trim()) {
            Ok(mut file) => match file.read_to_end(&mut data) {
                Ok(_) => Ok(data),
                Err(e) => Err(RuntimeError::new(ErrorCode::System, format!("{}", e))),
            },
            Err(e) => Err(RuntimeError::new(ErrorCode::System, format!("{}", e))),
        }
    }

    pub fn restore(&mut self, quetzal: Quetzal) -> Result<usize, RuntimeError> {
        let mut fs = FrameStack::new(0);
        for stackframe in quetzal.stks.stks {
            let result = if stackframe.flags & 0x10 == 0x00 {
                Some(StoreResult::new(0, stackframe.result_variable))
            } else {
                None
            };
            let f = Frame::new(
                0,
                0,
                &stackframe.local_variables,
                stackframe.flags & 0xF,
                &stackframe.stack,
                result,
                stackframe.return_address as usize,
            );
            fs.frames_mut().push(f);
        }

        self.frame_stack = fs;

        let dynamic = if let Some(cmem) = quetzal.cmem {
            cmem.to_vec(&self)
        } else if let Some(umem) = quetzal.umem {
            umem.data
        } else {
            return Err(RuntimeError::new(
                ErrorCode::System,
                "No CMEM or UMEM chunk".to_string(),
            ));
        };
        for i in 0..dynamic.len() {
            if i != 0x10 && i != 0x11 {
                self.memory.write_byte(i, dynamic[i])?;
            }
        }

        // Reset stream 3
        self.stream_3 = Vec::new();

        Ok(quetzal.ifhd.pc as usize)
    }

    pub fn save_undo(&mut self, instruction: &Instruction) -> Result<(), RuntimeError> {
        let q = Quetzal::from_state(&self, instruction.store().unwrap().address());
        self.undo_stack.push(q);
        self.undo_stack.truncate(10);
        Ok(())
    }

    pub fn restore_undo(&mut self) -> Result<usize, RuntimeError> {
        if let Some(q) = self.undo_stack.pop() {
            self.restore(q)
        } else {
            Ok(0)
        }
    }
    pub fn restart(&mut self) -> Result<usize, RuntimeError> {
        let f1 = self.read_byte(0x10)? & 0x3;
        for i in 0..self.dynamic.len() {
            let b = self.dynamic[i];
            self.write_byte(i, b)?;
        }
        self.write_byte(0x10, self.read_byte(0x10)? | f1)?;
        self.initialize()?;
        self.frame_stack =
            FrameStack::new(header::field_word(&self.memory(), HeaderField::InitialPC)? as usize);

        Ok(self.frame_stack.pc()?)
    }

    // pub fn print_char(&mut self, char: u16) -> Result<(),RuntimeError> {
    //     self.screen.print_char(char);
    //     Ok(())
    // }

    // pub fn print_num(&mut self, n: i16) -> Result<(), RuntimeError> {
    //     let s = format!("{}", n);
    //     let mut text = Vec::new();
    //     for c in s.chars() {
    //         text.push(c as u16);
    //     }

    //     self.screen.print(&text);
    //     Ok(())
    // }

    pub fn new_line(&mut self) -> Result<(), RuntimeError> {
        if self.output_streams & 0x5 == 0x1 {
            self.screen.new_line();
            if self.output_streams & 0x2 == 0x2 {
                self.transcript(&vec![0x0a as u16].to_vec())?;
            }
        }

        Ok(())
    }

    pub fn flush_screen(&mut self) -> Result<(), RuntimeError> {
        self.screen.flush_buffer()
    }

    pub fn backspace(&mut self) -> Result<(), RuntimeError> {
        self.screen.backspace()
    }

    // Run
    pub fn run(&mut self) -> Result<(), RuntimeError> {
        let mut n = 1;
        loop {
            log_mdc::insert("instruction_count", format!("{:8x}", n));
            let pc = self.frame_stack.pc()?;
            let instruction = decoder::decode_instruction(&self.memory, pc)?;
            let pc = processor::dispatch(self, &instruction)?;
            if pc == 0 {
                return Ok(());
            }
            self.set_pc(pc)?;
            n = n + 1;
        }
    }
}
