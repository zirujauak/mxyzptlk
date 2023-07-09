pub mod io;
mod rng;
pub mod state;

use std::fs;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::thread;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use crate::config::Config;
use crate::error::*;
use crate::files;
use crate::instruction::decoder;
use crate::instruction::processor;
use crate::instruction::StoreResult;
use crate::object::property;
use crate::sound::Manager;
use crate::text;
use crate::zmachine::io::screen::Interrupt;
use regex::Regex;
use rng::chacha_rng::ChaChaRng;
use rng::ZRng;

use self::io::screen::InputEvent;
use self::io::IO;
use self::state::header;
use self::state::header::Flags1v3;
use self::state::header::HeaderField;
use self::state::memory::Memory;
use self::state::State;

pub struct ZMachine {
    name: String,
    version: u8,
    state: State,
    io: IO,
    rng: Box<dyn ZRng>,
    input_interrupt: Option<u16>,
    input_interrupt_print: bool,
    sound_manager: Option<Manager>,
    sound_interrupt: Option<usize>,
}

impl ZMachine {
    pub fn new(
        memory: Memory,
        config: Config,
        sound_manager: Option<Manager>,
        name: &str,
    ) -> Result<ZMachine, RuntimeError> {
        let version = memory.read_byte(HeaderField::Version as usize)?;

        let sounds = if let Some(s) = sound_manager.as_ref() {
            info!(target: "app::sound", "{} sounds loaded", s.sound_count());
            s.sound_count() > 0
        } else {
            false
        };

        let rng = ChaChaRng::new();

        let io = IO::new(version, config)?;

        let mut state = State::new(memory)?;

        let colors = io.default_colors();
        state.initialize(
            io.rows() as u8,
            io.columns() as u8,
            (colors.0 as u8, colors.1 as u8),
            sounds,
        )?;
        Ok(ZMachine {
            name: name.to_string(),
            version,
            state,
            io,
            rng: Box::new(rng),
            input_interrupt: None,
            input_interrupt_print: false,
            sound_manager,
            sound_interrupt: None,
        })
    }

    pub fn version(&self) -> u8 {
        self.version
    }

    pub fn input_interrupt_print(&self) -> bool {
        self.input_interrupt_print
    }

    pub fn clear_input_interrupt_print(&mut self) {
        self.input_interrupt_print = false
    }

    pub fn set_input_interrupt_print(&mut self) {
        self.input_interrupt_print = true
    }

    // Runtime state
    pub fn read_byte(&self, address: usize) -> Result<u8, RuntimeError> {
        self.state.read_byte(address)
    }

    pub fn read_word(&self, address: usize) -> Result<u16, RuntimeError> {
        self.state.read_word(address)
    }

    fn update_transcript_bit(&mut self, old: u16, new: u16) -> Result<(), RuntimeError> {
        if old & 0x1 != new & 0x1 {
            if new & 0x1 == 0x1 {
                if !self.io.is_stream_2_open() {
                    if let Err(e) = self.start_stream_2() {
                        self.print_str(format!("Error starting stream 2: {}\r", e))?;
                    }
                    self.io.enable_output_stream(2, None)
                } else {
                    self.io.enable_output_stream(2, None)
                }
            } else {
                self.io.disable_output_stream(&mut self.state, 2)
            }
        } else {
            Ok(())
        }
    }

    pub fn write_byte(&mut self, address: usize, value: u8) -> Result<(), RuntimeError> {
        // Check if the transcript bit is being changed in Flags 2
        if address == 0x11
            && self
                .update_transcript_bit(self.state.read_byte(0x11)? as u16, value as u16)
                .is_err()
        {
            // Starting the transcript failed, so skip writing to memory
            warn!(target: "app::memory", "Staring transcript failed, not writing data to Flags 2");
            return Ok(());
        }

        self.state.write_byte(address, value)
    }

    pub fn write_word(&mut self, address: usize, value: u16) -> Result<(), RuntimeError> {
        // Check if the transcript bit is being set in Flags 2
        if address == 0x10
            && self
                .update_transcript_bit(self.state.read_word(0x10)?, value)
                .is_err()
        {
            // Starting the transcript failed, so skip writing to memory
            warn!(target: "app::memory", "Staring transcript failed, not writing data to Flags 2");
            return Ok(());
        }

        self.state.write_word(address, value)
    }

    pub fn variable(&mut self, variable: u8) -> Result<u16, RuntimeError> {
        self.state.variable(variable)
    }

    pub fn peek_variable(&mut self, variable: u8) -> Result<u16, RuntimeError> {
        self.state.peek_variable(variable)
    }

    pub fn set_variable(&mut self, variable: u8, value: u16) -> Result<(), RuntimeError> {
        self.state.set_variable(variable, value)
    }

    pub fn set_variable_indirect(&mut self, variable: u8, value: u16) -> Result<(), RuntimeError> {
        self.state.set_variable_indirect(variable, value)
    }

    pub fn push(&mut self, value: u16) -> Result<(), RuntimeError> {
        self.state.push(value)
    }

    pub fn is_input_interrupt(&self) -> bool {
        self.state.is_input_interrupt()
    }

    pub fn string_literal(&self, address: usize) -> Result<Vec<u16>, RuntimeError> {
        self.state.string_literal(address)
    }

    pub fn packed_routine_address(&self, address: u16) -> Result<usize, RuntimeError> {
        self.state.packed_routine_address(address)
    }

    pub fn packed_string_address(&self, address: u16) -> Result<usize, RuntimeError> {
        self.state.packed_string_address(address)
    }

    pub fn instruction(&self, address: usize) -> Vec<u8> {
        self.state.instruction(address)
    }

    pub fn frame_count(&self) -> usize {
        self.state.frame_count()
    }

    pub fn checksum(&self) -> Result<u16, RuntimeError> {
        self.state.checksum()
    }

    pub fn save(&mut self, pc: usize) -> Result<(), RuntimeError> {
        let save_data = self.state.save(pc)?;
        self.prompt_and_write("Save to: ", "ifzs", &save_data, false)
    }

    pub fn restore(&mut self) -> Result<Option<usize>, RuntimeError> {
        match self.prompt_and_read("Restore from: ", "ifzs") {
            Ok(save_data) => self.state.restore(save_data),
            Err(e) => {
                error!(target: "app::quetzal", "Error restoring: {}", e);
                Err(e)
            }
        }
    }

    pub fn save_undo(&mut self, address: usize) -> Result<(), RuntimeError> {
        self.state.save_undo(address)
    }

    pub fn restore_undo(&mut self) -> Result<Option<usize>, RuntimeError> {
        self.state.restore_undo()
    }

    pub fn restart(&mut self) -> Result<usize, RuntimeError> {
        self.rng.seed(0);
        self.state.restart()
    }

    pub fn call_routine(
        &mut self,
        address: usize,
        arguments: &Vec<u16>,
        result: Option<StoreResult>,
        return_address: usize,
    ) -> Result<usize, RuntimeError> {
        self.state
            .call_routine(address, arguments, result, return_address)
    }

    pub fn call_read_interrupt(
        &mut self,
        address: usize,
        return_address: usize,
    ) -> Result<usize, RuntimeError> {
        self.state.call_read_interrupt(address, return_address)
    }

    pub fn read_interrupt_pending(&self) -> bool {
        self.state.read_interrupt_pending()
    }

    pub fn set_read_interrupt_pending(&mut self) {
        self.state.set_read_interrupt();
    }

    pub fn clear_read_interrupt(&mut self) {
        self.state.clear_read_interrupt();
    }

    pub fn read_interrupt_result(&mut self) -> Option<u16> {
        self.state.read_interrupt_result()
    }

    pub fn sound_interrupt(&self) -> Option<usize> {
        self.state.sound_interrupt()
    }

    pub fn set_sound_interrupt(&mut self, address: usize) {
        self.state.set_sound_interrupt(address);
    }

    pub fn call_sound_interrupt(&mut self, return_address: usize) -> Result<usize, RuntimeError> {
        self.state.call_sound_interrupt(return_address)
    }

    pub fn return_routine(&mut self, value: u16) -> Result<usize, RuntimeError> {
        self.state.return_routine(value)
    }

    pub fn throw(&mut self, depth: u16, result: u16) -> Result<usize, RuntimeError> {
        self.state.throw(depth, result)
    }

    pub fn argument_count(&self) -> Result<u8, RuntimeError> {
        self.state.argument_count()
    }

    // Header
    pub fn header_byte(&self, field: HeaderField) -> Result<u8, RuntimeError> {
        header::field_byte(&self.state, field)
    }

    pub fn header_word(&self, field: HeaderField) -> Result<u16, RuntimeError> {
        header::field_word(&self.state, field)
    }

    // RNG
    pub fn random(&mut self, range: u16) -> u16 {
        self.rng.random(range)
    }

    pub fn seed(&mut self, seed: u16) {
        self.rng.seed(seed)
    }

    pub fn predictable(&mut self, seed: u16) {
        self.rng.predictable(seed)
    }

    // Screen I/O
    pub fn rows(&self) -> u16 {
        self.io.rows() as u16
    }

    pub fn columns(&self) -> u16 {
        self.io.columns() as u16
    }

    fn start_stream_2(&mut self) -> Result<(), RuntimeError> {
        let file = self.prompt_and_create("Transcript file name: ", "txt", false)?;
        self.io.set_stream_2(file);
        Ok(())
    }

    pub fn output_stream(&mut self, stream: i16, table: Option<usize>) -> Result<(), RuntimeError> {
        match stream {
            1..=4 => {
                info!(target: "app::stream", "Enabling output stream {}", stream);
                if stream == 2 && !self.io.is_stream_2_open() {
                    if let Err(e) = self.start_stream_2() {
                        error!(target: "app::stream", "Error starting stream 2: {}", e);
                    }
                    self.io.enable_output_stream(stream as u8, table)
                } else {
                    self.io.enable_output_stream(stream as u8, table)
                }
            }
            -4..=-1 => {
                info!(target: "app::stream", "Disabling output stream {}", i16::abs(stream));
                self.io
                    .disable_output_stream(&mut self.state, i16::abs(stream) as u8)
            }
            _ => Err(RuntimeError::new(
                ErrorCode::System,
                format!("Output stream {} is not valid: [-4..4]", stream),
            )),
        }
    }

    pub fn print(&mut self, text: &Vec<u16>) -> Result<(), RuntimeError> {
        self.io.print_vec(text)?;

        if self.state.is_input_interrupt() {
            self.set_input_interrupt_print();
        }

        Ok(())
    }

    pub fn print_str(&mut self, text: String) -> Result<(), RuntimeError> {
        self.io.print_vec(&text.chars().map(|c| c as u16).collect())
    }
    pub fn split_window(&mut self, lines: u16) -> Result<(), RuntimeError> {
        self.io.split_window(lines)
    }

    pub fn set_window(&mut self, window: u16) -> Result<(), RuntimeError> {
        self.io.set_window(window)
    }

    pub fn erase_window(&mut self, window: i16) -> Result<(), RuntimeError> {
        self.io.erase_window(window)
    }

    pub fn erase_line(&mut self) -> Result<(), RuntimeError> {
        self.io.erase_line()
    }

    pub fn status_line(&mut self) -> Result<(), RuntimeError> {
        let status_type = header::flag1(&self.state, Flags1v3::StatusLineType as u8)?;
        let object = self.state.variable(16)? as usize;
        let mut left = text::from_vec(self, &property::short_name(self, object)?)?;

        let mut right: Vec<u16> = if status_type == 0 {
            let score = self.state.variable(17)? as i16;
            let turns = self.state.variable(18)?;
            format!("{:<8}", format!("{}/{}", score, turns))
                .as_bytes()
                .iter()
                .map(|x| *x as u16)
                .collect()
        } else {
            let hour = self.state.variable(17)?;
            let minute = self.state.variable(18)?;
            let suffix = if hour > 11 { "PM" } else { "AM" };
            let h = if hour % 12 == 0 { 12 } else { hour };

            format!("{:2}:{:02} {}", h, minute, suffix)
                .as_bytes()
                .iter()
                .map(|x| *x as u16)
                .collect()
        };

        self.io.status_line(&mut left, &mut right)
    }

    pub fn set_font(&mut self, font: u16) -> Result<u16, RuntimeError> {
        self.io.set_font(font)
    }

    pub fn set_text_style(&mut self, style: u16) -> Result<(), RuntimeError> {
        self.io.set_text_style(style)
    }

    pub fn cursor(&mut self) -> Result<(u16, u16), RuntimeError> {
        self.io.cursor()
    }

    pub fn set_cursor(&mut self, row: u16, column: u16) -> Result<(), RuntimeError> {
        self.io.set_cursor(row, column)
    }

    pub fn buffer_mode(&mut self, mode: u16) -> Result<(), RuntimeError> {
        self.io.buffer_mode(mode)
    }

    pub fn beep(&mut self) -> Result<(), RuntimeError> {
        self.io.beep()
    }

    pub fn set_colors(&mut self, foreground: u16, background: u16) -> Result<(), RuntimeError> {
        self.io.set_colors(foreground, background)
    }

    // Input
    fn now(&self, timeout: Option<u16>) -> u128 {
        match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(t) => {
                if let Some(d) = timeout {
                    t.as_millis() + d as u128
                } else {
                    t.as_millis()
                }
            }
            Err(e) => {
                error!(target: "app::trace", "Error getting current system time: {}", e);
                0
            }
        }
    }

    fn mouse_data(&mut self, event: &InputEvent) -> Result<(), RuntimeError> {
        let column = match event.column() {
            Some(col) => col,
            _ => {
                error!(target: "app::input", "Input event missing mouse column data");
                0
            }
        };
        let row = match event.row() {
            Some(row) => row,
            _ => {
                error!(target: "app::input", "Input event missing mouse row data");
                0
            }
        };

        debug!(target: "app::input", "Storing mouse coordinates {},{}", column, row);
        header::set_extension(&mut self.state, 1, column)?;
        header::set_extension(&mut self.state, 2, row)?;

        Ok(())
    }

    pub fn read_key(&mut self, timeout: u16) -> Result<InputEvent, RuntimeError> {
        let end = if timeout > 0 {
            self.now(Some(timeout))
        } else {
            0
        };

        let check_sound = self.sound_interrupt.is_some();

        loop {
            // If a sound interrupt is set and there is no sound playing,
            // return buffer and clear any pending input_interrupt
            if self.sound_interrupt.is_some() {
                if let Some(sounds) = self.sound_manager.as_mut() {
                    if !sounds.is_playing() {
                        info!(target: "app::input", "Sound interrupt firing");
                        self.input_interrupt = None;
                        return Ok(InputEvent::from_interrupt(Interrupt::Sound));
                    }
                }
            }

            let now = self.now(None);
            if end > 0 && now > end {
                return Ok(InputEvent::from_interrupt(Interrupt::ReadTimeout));
            }

            let key = self.io.read_key(end == 0 && !check_sound);

            if let Some(c) = key.zchar() {
                if c == 253 || c == 254 {
                    self.mouse_data(&key)?;
                }

                return Ok(key);
            }

            thread::sleep(Duration::from_millis(10));
        }
    }

    pub fn read_line(
        &mut self,
        text: &[u16],
        len: usize,
        terminators: &[u16],
        timeout: u16,
    ) -> Result<Vec<u16>, RuntimeError> {
        let mut input_buffer = text.to_vec();

        let end = if timeout > 0 {
            self.now(Some(timeout))
        } else {
            0
        };

        let check_sound = self.state.sound_interrupt().is_some();

        info!(target: "app::input", "Sound interrupt {:?}", check_sound);

        loop {
            // If a sound interrupt is set and there is no sound playing,
            // return buffer and clear any pending input_interrupt
            if self.state.sound_interrupt().is_some() {
                info!(target: "app::frame", "Interrupt pending");
                if let Some(sounds) = self.sound_manager.as_mut() {
                    info!(target: "app::frame", "Sound playing? {}", sounds.is_playing());
                    if !sounds.is_playing() {
                        info!(target: "app::frame", "Sound interrupt firing");
                        self.state.clear_read_interrupt();
                        return Ok(input_buffer);
                    }
                }
            }

            let now = self.now(None);
            if end > 0 && now > end {
                info!(target: "app::input", "read_line timed out");
                return Ok(input_buffer);
            }

            let timeout = if end > 0 { end - now } else { 0 };

            info!(target: "app::input", "Now: {}, End: {}, Timeout: {}", now, end, timeout);

            let e = self.io.read_key(end == 0 && !check_sound);
            match e.zchar() {
                Some(key) => {
                    if terminators.contains(&key) || (terminators.contains(&255) && (key > 128)) {
                        if key == 254 || key == 253 {
                            self.mouse_data(&e)?;
                        }

                        input_buffer.push(key);
                        if key == 0x0d {
                            self.io.print_vec(&vec![key])?;
                        }
                        break;
                    } else if key == 0x08 {
                        if !input_buffer.is_empty() {
                            input_buffer.pop();
                            self.backspace()?;
                        }
                    } else if input_buffer.len() < len && (0x20..0x7f).contains(&key) {
                        input_buffer.push(key);
                        self.io.print_vec(&vec![key])?;
                    }
                }
                None => thread::sleep(Duration::from_millis(10)),
            }
        }

        Ok(input_buffer)
    }

    pub fn prompt_filename(
        &mut self,
        prompt: &str,
        suffix: &str,
        overwrite: bool,
        first: bool,
    ) -> Result<String, RuntimeError> {
        self.print_str(prompt.to_string())?;
        let n = if first {
            files::first_available(&self.name, suffix)?
        } else {
            files::last_existing(&self.name, suffix)?
        };

        self.print(&n)?;

        let f = self.read_line(&n, 32, &['\r' as u16], 0)?;
        let filename = match String::from_utf16(&f) {
            Ok(s) => s.trim().to_string(),
            Err(e) => {
                return Err(RuntimeError::new(
                    ErrorCode::System,
                    format!("Error parsing user input: {}", e),
                ))
            }
        };

        if !overwrite {
            match Path::new(&filename).try_exists() {
                Ok(b) => match b {
                    true => {
                        return Err(RuntimeError::new(
                            ErrorCode::System,
                            format!("'{}' already exists.", filename),
                        ))
                    }
                    false => {}
                },
                Err(e) => {
                    return Err(RuntimeError::new(
                        ErrorCode::System,
                        format!("Error checking if '{}' exists: {}", filename, e),
                    ))
                }
            }
        }

        match Regex::new(r".*\.z\d") {
            Ok(r) => {
                if r.is_match(&filename) {
                    Err(RuntimeError::new(
                        ErrorCode::System,
                        "Filenames ending in '.z#' are not allowed".to_string(),
                    ))
                } else {
                    Ok(filename)
                }
            }
            Err(e) => Err(RuntimeError::new(
                ErrorCode::System,
                format!("Interal error with regex checking filename: {}", e),
            )),
        }
    }

    pub fn prompt_and_create(
        &mut self,
        prompt: &str,
        suffix: &str,
        overwrite: bool,
    ) -> Result<File, RuntimeError> {
        match self.prompt_filename(prompt, suffix, overwrite, true) {
            Ok(filename) => match fs::OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(filename.trim())
            {
                Ok(f) => Ok(f),
                Err(e) => Err(RuntimeError::new(ErrorCode::System, format!("{}", e))),
            },
            Err(e) => {
                self.print_str(format!("Error creating file: {}\r", e))?;
                Err(e)
            }
        }
    }

    pub fn prompt_and_write(
        &mut self,
        prompt: &str,
        suffix: &str,
        data: &[u8],
        overwrite: bool,
    ) -> Result<(), RuntimeError> {
        let mut file = self.prompt_and_create(prompt, suffix, overwrite)?;

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
        let filename = self.prompt_filename(prompt, suffix, true, false)?;
        let mut data = Vec::new();
        match File::open(filename.trim()) {
            Ok(mut file) => match file.read_to_end(&mut data) {
                Ok(_) => Ok(data),
                Err(e) => Err(RuntimeError::new(ErrorCode::System, format!("{}", e))),
            },
            Err(e) => Err(RuntimeError::new(ErrorCode::System, format!("{}", e))),
        }
    }

    // Save/restore
    // Also quit/restart
    pub fn quit(&mut self) -> Result<(), RuntimeError> {
        self.print(
            &"Press any key to exit"
                .as_bytes()
                .iter()
                .map(|x| *x as u16)
                .collect(),
        )?;
        self.read_key(0)?;

        self.io.quit();
        Ok(())
    }

    pub fn new_line(&mut self) -> Result<(), RuntimeError> {
        self.io.new_line()
    }

    pub fn backspace(&mut self) -> Result<(), RuntimeError> {
        self.io.backspace()
    }

    // Sound
    pub fn play_sound(
        &mut self,
        effect: u16,
        volume: u8,
        repeats: u8,
        routine: Option<usize>,
    ) -> Result<(), RuntimeError> {
        let repeats = if self.version > 4 && repeats > 0 {
            Some(repeats)
        } else {
            None
        };

        if let Some(sounds) = self.sound_manager.as_mut() {
            if let Some(address) = routine {
                self.state.set_sound_interrupt(address);
            }
            // Sound is already playing, possibly repeating, so just
            // adjust the volume, if possible, without interrupting
            // the loop
            if sounds.current_effect() as u16 == effect {
                sounds.change_volume(volume);
                Ok(())
            } else {
                sounds.play_sound(effect, volume, repeats)
            }
        } else {
            Ok(())
        }
    }

    pub fn stop_sound(&mut self) -> Result<(), RuntimeError> {
        if let Some(sounds) = self.sound_manager.as_mut() {
            self.state.clear_sound_interrupt();
            sounds.stop_sound()
        }

        Ok(())
    }

    pub fn is_sound_playing(&mut self) -> bool {
        if let Some(sounds) = self.sound_manager.as_mut() {
            sounds.is_playing()
        } else {
            false
        }
    }

    // Run
    pub fn run(&mut self) -> Result<(), RuntimeError> {
        let mut n = 1;
        loop {
            log_mdc::insert("instruction_count", format!("{:8x}", n));
            let pc = self.state.pc()?;
            let instruction = decoder::decode_instruction(self, pc)?;
            let pc = processor::dispatch(self, &instruction)?;
            if pc == 0 {
                return Ok(());
            }

            if self.state.sound_interrupt().is_some() {
                if let Some(sounds) = self.sound_manager.as_mut() {
                    if !sounds.is_playing() {
                        let pc = self.state.call_sound_interrupt(pc)?;
                        self.state.set_pc(pc)?;
                    } else {
                        self.state.set_pc(pc)?;
                    }
                }
            } else {
                self.state.set_pc(pc)?;
            }
            n += 1;
        }
    }
}
