pub mod io;
mod rng;
pub mod state;

use std::collections::HashSet;
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
use crate::recoverable_error;
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorHandling {
    ContinueWarnAlways,
    ContinueWarnOnce,
    Ignore,
    Abort,
}

#[derive(Debug)]
pub struct ZMachine {
    name: String,
    version: u8,
    state: State,
    io: IO,
    rng: Box<dyn ZRng>,
    input_interrupt: Option<u16>,
    input_interrupt_print: bool,
    sound_manager: Option<Manager>,
    errors: HashSet<ErrorCode>,
    error_handling: ErrorHandling,
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

        let error_handling = config.error_handling();
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
            errors: HashSet::new(),
            error_handling,
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
                        Err(e)
                    } else {
                        self.io.enable_output_stream(2, None)
                    }
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
            warn!(target: "app::stream", "Staring transcript failed, not setting transcript bit");
            return self.state.write_byte(address, value & 0xFE);
        }

        self.state.write_byte(address, value)
    }

    pub fn write_word(&mut self, address: usize, value: u16) -> Result<(), RuntimeError> {
        // Check if the transcript bit is being set in Flags 2 when writing to 0x10 or 0x11
        if address == 0x10
            && self
                .update_transcript_bit(self.state.read_word(0x10)?, value)
                .is_err()
        {
            // Starting the transcript failed, so skip writing to memory
            warn!(target: "app::stream", "Staring transcript failed, not setting transcript bit");
            return self.state.write_word(0x10, value & 0xFFFE);
        } else if address == 0x11
            && self
                .update_transcript_bit(self.state.read_byte(0x11)? as u16, value >> 8)
                .is_err()
        {
            // Starting the transcript failed, so skip writing to memory
            warn!(target: "app::stream", "Staring transcript failed, not setting transcript bit");
            return self.state.write_word(0x11, value & 0xFEFF);
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
                error!(target: "app::state", "Error restoring state: {}", e);
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
                debug!(target: "app::stream", "Enabling output stream {}", stream);
                if stream == 2 {
                    if !self.io.is_stream_2_open() {
                        if let Err(e) = self.start_stream_2() {
                            error!(target: "app::stream", "Error starting stream 2: {}", e);
                            return recoverable_error!(
                                ErrorCode::Transcript,
                                "Error creating transcript file: {}",
                                e
                            );
                        }
                    }
                    // Set the transcript bit
                    let f2 = self.state.read_word(0x10)?;
                    self.state.write_word(0x10, f2 | 1)?;
                    self.io.enable_output_stream(stream as u8, table)
                } else {
                    self.io.enable_output_stream(stream as u8, table)
                }
            }
            -4..=-1 => {
                debug!(target: "app::stream", "Disabling output stream {}", i16::abs(stream));
                if stream == -2 {
                    // Unset the transcript bit
                    let f2 = self.state.read_word(0x10)?;
                    self.state.write_word(0x10, f2 & 0xFFFE)?;
                }
                self.io
                    .disable_output_stream(&mut self.state, i16::abs(stream) as u8)
            }
            _ => recoverable_error!(
                ErrorCode::InvalidOutputStream,
                "Output stream {} is not valid: [-4..4]",
                stream
            ),
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
        let mut left = text::from_vec(self, &property::short_name(self, object)?, false)?;
        let mut right: Vec<u16> = if status_type == 0 {
            // Score is between -99 and 999 inclusive
            let score = i16::min(999, i16::max(-99, self.state.variable(17)? as i16));
            // Turns is between 0 and 9999 inclusive
            let turns = u16::min(9999, self.state.variable(18)?);
            format!("{:<8}", format!("{:}/{:}", score, turns))
                .as_bytes()
                .iter()
                .map(|x| *x as u16)
                .collect()
        } else {
            // Hour is between 0 and 23, inclusive
            let hour = u16::min(23, self.state.variable(17)?);
            // Minute is between 0 and 59, inclusive
            let minute = u16::min(59, self.state.variable(18)?);
            let suffix = if hour > 11 { "PM" } else { "AM" };
            // 0-24 -> 1-12
            let h = if hour == 0 {
                12
            } else if hour > 12 {
                hour - 12
            } else {
                hour
            };

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
                error!(target: "app::state", "Error getting current system time: {}", e);
                0
            }
        }
    }

    fn mouse_data(&mut self, event: &InputEvent) -> Result<(), RuntimeError> {
        let column = match event.column() {
            Some(col) => col,
            _ => {
                error!(target: "app::screen", "Input event missing mouse column data");
                0
            }
        };
        let row = match event.row() {
            Some(row) => row,
            _ => {
                error!(target: "app::screen", "Input event missing mouse row data");
                0
            }
        };

        debug!(target: "app::screen", "Storing mouse coordinates ({},{})", column, row);
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

        let check_sound = self.state.sound_interrupt().is_some();
        loop {
            // If a sound interrupt is set and there is no sound playing,
            // return buffer and clear any pending input_interrupt
            if self.state.sound_interrupt().is_some() {
                if let Some(sounds) = self.sound_manager.as_mut() {
                    if !sounds.is_playing() {
                        debug!(target: "app::screen", "Read interrupted: sound interrupt firing");
                        self.input_interrupt = None;
                        return Ok(InputEvent::from_interrupt(Interrupt::Sound));
                    }
                }
            }

            let now = self.now(None);
            if end > 0 && now > end {
                debug!(target: "app::screen", "Read interrupted: timed out");
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

        debug!(target: "app::screen", "Pending sound interrupt? {}", check_sound);

        loop {
            // If a sound interrupt is set and there is no sound playing,
            // return buffer and clear any pending input_interrupt
            if self.state.sound_interrupt().is_some() {
                debug!(target: "app::screen", "Soundinterrupt pending");
                if let Some(sounds) = self.sound_manager.as_mut() {
                    debug!(target: "app::screen", "Sound playing? {}", sounds.is_playing());
                    if !sounds.is_playing() {
                        debug!(target: "app::screen", "Read interrupted: sound interrupt firing");
                        self.state.clear_read_interrupt();
                        return Ok(input_buffer);
                    }
                }
            }

            let now = self.now(None);
            if end > 0 && now > end {
                debug!(target: "app::screen", "Read interrupted: timed out");
                return Ok(input_buffer);
            }

            let timeout = if end > 0 { end - now } else { 0 };

            trace!(target: "app::screen", "Now: {}, End: {}, Timeout: {}", now, end, timeout);

            let e = self.io.read_key(end == 0 && !check_sound);
            match e.zchar() {
                Some(key) => {
                    if terminators.contains(&key)
                        // Terminator 255 means "any function key"
                        || (terminators.contains(&255) && ((129..155).contains(&key) || key > 251))
                    {
                        if key == 254 || key == 253 {
                            self.mouse_data(&e)?;
                        }

                        input_buffer.push(key);
                        // Only print the terminator if it was the return key
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
                return recoverable_error!(
                    ErrorCode::InvalidInput,
                    "Error parsing user input: {}",
                    e
                )
            }
        };

        if !overwrite {
            match Path::new(&filename).try_exists() {
                Ok(b) => match b {
                    true => {
                        return recoverable_error!(
                            ErrorCode::FileExists,
                            "'{}' already exists.",
                            filename
                        )
                    }
                    false => {}
                },
                Err(e) => {
                    return recoverable_error!(
                        ErrorCode::Interpreter,
                        "Error checking if '{}' exists: {}",
                        filename,
                        e
                    )
                }
            }
        }

        match Regex::new(r"^((.*\.z\d)|(.*\.blb)|(.*\.blorb))$") {
            Ok(r) => {
                if r.is_match(&filename) {
                    recoverable_error!(
                        ErrorCode::InvalidFilename,
                        "Filenames ending in '.z#', '.blb', or '.blorb' are not allowed"
                    )
                } else {
                    Ok(filename)
                }
            }
            Err(e) => recoverable_error!(
                ErrorCode::Interpreter,
                "Interal error with regex checking filename: {}",
                e
            ),
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
                Err(e) => recoverable_error!(ErrorCode::FileError, "{}", e),
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
            Err(e) => return recoverable_error!(ErrorCode::FileError, "{}", e),
        };
        match file.flush() {
            Ok(_) => Ok(()),
            Err(e) => recoverable_error!(ErrorCode::FileError, "{}", e),
        }
    }

    pub fn prompt_and_read(&mut self, prompt: &str, suffix: &str) -> Result<Vec<u8>, RuntimeError> {
        let filename = self.prompt_filename(prompt, suffix, true, false)?;
        let mut data = Vec::new();
        match File::open(filename.trim()) {
            Ok(mut file) => match file.read_to_end(&mut data) {
                Ok(_) => Ok(data),
                Err(e) => recoverable_error!(ErrorCode::FileError, "{}", e),
            },
            Err(e) => recoverable_error!(ErrorCode::FileError, "{}: {}", filename, e),
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
        let r = if self.version > 4 && repeats > 0 {
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
                sounds.play_sound(effect, volume, r)
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
            match processor::dispatch(self, &instruction) {
                Ok(pc) => {
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
                }
                Err(e) => {
                    // If the error is fatal or error handling is abort
                    if !e.is_recoverable() || self.error_handling == ErrorHandling::Abort {
                        return Err(e);
                    // Error is not fatal
                    // If error handling is ignore
                    } else if self.error_handling == ErrorHandling::Ignore {
                        self.state.set_pc(instruction.next_address())?;
                    // If error handling is warn always or the code hasn't been seen yet
                    } else if self.error_handling == ErrorHandling::ContinueWarnAlways
                        || !self.errors.contains(&e.code())
                    {
                        self.errors.insert(e.code());
                        if self.io.error(
                            &format!("[{}]: {}", n, instruction),
                            e.message(),
                            e.is_recoverable(),
                        ) {
                            self.state.set_pc(instruction.next_address())?;
                        } else {
                            // Print instruction details before returning an error
                            self.print_str(format!("\r[{}]: {}", n, instruction))?;
                            return Err(e);
                        }
                    // Error handling is warn once and the code has been seen before
                    } else {
                        self.state.set_pc(instruction.next_address())?
                    }
                }
            }
            n += 1;
        }
    }
}

#[cfg(test)]
mod tests {

    use std::collections::HashMap;

    use iff::Chunk;

    use crate::{
        assert_ok, assert_ok_eq, assert_print, assert_some, assert_some_eq,
        blorb::{Blorb, Entry, Index, Loop, RIdx},
        test_util::{
            backspace, beep, buffer_mode, colors, cursor, erase_line, erase_window, input,
            mock_blorb, mock_object, mock_routine, play_sound, quit, scroll, set_input_delay,
            set_input_timeout, split, style, test_map, window,
        },
        zmachine::{io::screen::Style, state::header::Flags2},
    };

    use super::*;

    #[test]
    fn test_constructor() {
        let map = test_map(3);
        let m = Memory::new(map);
        let zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert_eq!(zmachine.name, "test");
        assert_eq!(zmachine.version(), 3);
        assert_eq!(zmachine.state.version(), 3);
        assert_eq!(zmachine.io.columns(), 80);
        assert_eq!(zmachine.io.rows(), 24);
        assert!(zmachine.input_interrupt.is_none());
        assert!(!zmachine.input_interrupt_print);
        assert!(zmachine.sound_manager.is_none());
        assert!(zmachine.sound_interrupt().is_none());
    }

    #[test]
    fn test_input_interrupt_print() {
        let map = test_map(3);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(!zmachine.input_interrupt_print());
        zmachine.set_input_interrupt_print();
        assert!(zmachine.input_interrupt_print());
        zmachine.clear_input_interrupt_print();
        assert!(!zmachine.input_interrupt_print());
    }

    #[test]
    fn test_read_byte() {
        let mut map = test_map(3);
        map.append(&mut vec![0; 0x10000]);
        for (i, b) in (0x40..0x10800).enumerate() {
            map[i + 0x40] = b as u8;
        }

        let m = Memory::new(map);
        let zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert_ok_eq!(zmachine.read_byte(0), 3);
        assert_ok_eq!(zmachine.read_byte(0x401), 1);
        assert!(zmachine.read_byte(0x10000).is_err());
    }

    #[test]
    fn test_read_word() {
        let mut map = test_map(3);
        map.append(&mut vec![0; 0x10000]);
        for (i, b) in (0x40..0x10800).enumerate() {
            map[i + 0x40] = b as u8;
        }

        let m = Memory::new(map);
        let zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        // Flags1 is modified, so $01 is #20
        assert_ok_eq!(zmachine.read_word(0), 0x320);
        assert_ok_eq!(zmachine.read_word(0x401), 0x0102);
        assert!(zmachine.read_word(0xFFFF).is_err());
    }

    #[test]
    fn test_write_byte() {
        let mut map = test_map(3);
        map.append(&mut vec![0; 0x10000]);
        for (i, b) in (0x40..0x10800).enumerate() {
            map[i + 0x40] = b as u8;
        }

        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(zmachine.write_byte(0x200, 0xFF).is_ok());
        assert_ok_eq!(zmachine.read_byte(0x200), 0xFF);
        assert!(zmachine.write_byte(0x400, 0xFF).is_err());
        assert_ok_eq!(zmachine.read_byte(0x400), 0);
        assert!(zmachine.write_byte(0x10000, 0xFF).is_err());
    }

    #[test]
    fn test_write_word() {
        let mut map = test_map(3);
        map.append(&mut vec![0; 0x10000]);
        for (i, b) in (0x40..0x10800).enumerate() {
            map[i + 0x40] = b as u8;
        }

        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(zmachine.write_word(0x200, 0x1234).is_ok());
        assert_ok_eq!(zmachine.read_word(0x200), 0x1234);
        assert!(zmachine.write_word(0x3FF, 0x1234).is_err());
        assert!(zmachine.write_word(0xFFFF, 0x1234).is_err());
    }

    #[test]
    fn test_write_byte_transcript_1() {
        let mut map = test_map(3);
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }

        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        input(&[
            '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}', 'z', '1', '.', 't', 'x',
            't',
        ]);
        let f1 = assert_ok!(zmachine.read_byte(0x11));
        assert!(zmachine.write_byte(0x11, f1 | 1).is_ok());
        assert!(Path::new("test-z1.txt").exists());
        assert!(fs::remove_file("test-z1.txt").is_ok());
        assert_ok_eq!(zmachine.read_byte(0x11), f1 | 1);
        assert!(zmachine.io.is_stream_enabled(2));
    }

    #[test]
    fn test_write_transcript_1_already_1() {
        let mut map = test_map(3);
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }

        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        input(&[
            '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}', 'z', '5', '.', 't', 'x',
            't',
        ]);
        let f1 = assert_ok!(zmachine.read_byte(0x11));
        assert!(zmachine.write_byte(0x11, f1 | 1).is_ok());
        assert!(Path::new("test-z5.txt").exists());
        assert!(fs::remove_file("test-z5.txt").is_ok());
        assert!(zmachine.write_byte(0x11, f1 | 1).is_ok());
        assert!(!Path::new("test-z5.txt").exists());
        assert_ok_eq!(zmachine.read_byte(0x11), f1 | 1);
        assert!(zmachine.io.is_stream_enabled(2));
    }

    #[test]
    fn test_write_byte_transcript_1_error() {
        let mut map = test_map(3);
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }

        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        input(&[
            '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}',
            '\u{08}', '\u{08}', '\u{08}', '/', 'x', '/', 'f', 'o', 'o',
        ]);
        let f1 = assert_ok!(zmachine.read_byte(0x11));
        assert!(zmachine.write_byte(0x11, f1 | 1).is_ok());
        assert_ok_eq!(zmachine.read_byte(0x11), f1);
        assert!(!zmachine.io.is_stream_enabled(2));
    }

    #[test]
    fn test_write_byte_transcript_0() {
        let mut map = test_map(3);
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }

        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        input(&[
            '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}', 'z', '2', '.', 't', 'x',
            't',
        ]);
        let f1 = assert_ok!(zmachine.read_byte(0x11));
        assert!(zmachine.write_byte(0x11, f1 | 1).is_ok());
        assert!(Path::new("test-z2.txt").exists());
        assert!(fs::remove_file("test-z2.txt").is_ok());
        assert!(zmachine.io.is_stream_enabled(2));
        assert!(zmachine.write_byte(0x11, f1).is_ok());
        assert_ok_eq!(zmachine.read_byte(0x11), f1);
        assert!(!zmachine.io.is_stream_enabled(2));
    }

    #[test]
    fn test_write_word_transcript_1() {
        let mut map = test_map(3);
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }

        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        input(&[
            '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}', 'z', '3', '.', 't', 'x',
            't',
        ]);
        let f1 = assert_ok!(zmachine.read_word(0x10));
        assert!(zmachine.write_word(0x10, f1 | 1).is_ok());
        assert!(Path::new("test-z3.txt").exists());
        assert!(fs::remove_file("test-z3.txt").is_ok());
        assert_ok_eq!(zmachine.read_word(0x10), f1 | 1);
        assert!(zmachine.io.is_stream_enabled(2));
    }

    #[test]
    fn test_write_word_transcript_0() {
        let mut map = test_map(3);
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }

        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        input(&[
            '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}', 'z', '4', '.', 't', 'x',
            't',
        ]);
        let f1 = assert_ok!(zmachine.read_word(0x11));
        assert!(zmachine.write_word(0x11, f1 | 0x100).is_ok());
        assert!(Path::new("test-z4.txt").exists());
        assert!(fs::remove_file("test-z4.txt").is_ok());
        assert!(zmachine.io.is_stream_enabled(2));
        assert!(zmachine.write_word(0x11, f1).is_ok());
        assert_ok_eq!(zmachine.read_word(0x11), f1);
        assert!(!zmachine.io.is_stream_enabled(2));
    }

    #[test]
    fn test_write_word_0x10_transcript_1_error() {
        let mut map = test_map(3);
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }

        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        input(&[
            '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}',
            '\u{08}', '\u{08}', '\u{08}', '/', 'x', '/', 'f', 'o', 'o',
        ]);
        let f1 = assert_ok!(zmachine.read_word(0x10));
        assert!(zmachine.write_word(0x10, f1 | 1).is_ok());
        assert_ok_eq!(zmachine.read_word(0x10), f1);
        assert!(!zmachine.io.is_stream_enabled(2));
    }

    #[test]
    fn test_write_word_0x11_transcript_1_error() {
        let mut map = test_map(3);
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }

        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        input(&[
            '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}',
            '\u{08}', '\u{08}', '\u{08}', '/', 'x', '/', 'f', 'o', 'o',
        ]);
        let f1 = assert_ok!(zmachine.read_word(0x11));
        assert!(zmachine.write_word(0x11, f1 | 0x100).is_ok());
        assert_ok_eq!(zmachine.read_word(0x11), f1);
        assert!(!zmachine.io.is_stream_enabled(2));
    }

    #[test]
    fn test_variable() {
        let mut map = test_map(3);
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        mock_routine(&mut map, 0x600, &[0x1122, 0x3344, 0x5566]);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(zmachine
            .state
            .call_routine(0x600, &vec![0x8888], None, 0x400)
            .is_ok());
        assert!(zmachine.push(0x1234).is_ok());
        assert!(zmachine.push(0x5678).is_ok());
        assert_ok_eq!(zmachine.variable(0), 0x5678);
        assert_ok_eq!(zmachine.variable(0), 0x1234);
        assert!(zmachine.variable(0).is_err());
        assert_ok_eq!(zmachine.variable(1), 0x8888);
        assert_ok_eq!(zmachine.variable(2), 0x3344);
        assert_ok_eq!(zmachine.variable(3), 0x5566);
        assert!(zmachine.variable(4).is_err());
        assert_ok_eq!(zmachine.variable(0x80), 0xE0E1);
    }

    #[test]
    fn test_peek_variable() {
        let mut map = test_map(3);
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        mock_routine(&mut map, 0x600, &[0x1122, 0x3344, 0x5566]);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(zmachine
            .state
            .call_routine(0x600, &vec![0x8888], None, 0x400)
            .is_ok());
        assert!(zmachine.push(0x1234).is_ok());
        assert!(zmachine.push(0x5678).is_ok());
        assert_ok_eq!(zmachine.peek_variable(0), 0x5678);
        assert_ok_eq!(zmachine.peek_variable(0), 0x5678);
        assert_ok_eq!(zmachine.peek_variable(1), 0x8888);
        assert_ok_eq!(zmachine.peek_variable(2), 0x3344);
        assert_ok_eq!(zmachine.peek_variable(3), 0x5566);
        assert!(zmachine.peek_variable(4).is_err());
        assert_ok_eq!(zmachine.peek_variable(0x80), 0xE0E1);
    }

    #[test]
    fn test_set_variable() {
        let mut map = test_map(3);
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        mock_routine(&mut map, 0x600, &[0x1122, 0x3344, 0x5566]);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(zmachine
            .state
            .call_routine(0x600, &vec![0x8888], None, 0x400)
            .is_ok());
        assert!(zmachine.set_variable(0, 0x1234).is_ok());
        assert!(zmachine.set_variable(0, 0x5678).is_ok());
        assert!(zmachine.set_variable(1, 0x9988).is_ok());
        assert!(zmachine.set_variable(2, 0x7766).is_ok());
        assert!(zmachine.set_variable(3, 0x5544).is_ok());
        assert!(zmachine.set_variable(4, 0x3322).is_err());
        assert!(zmachine.set_variable(0x80, 0x1100).is_ok());
        assert_ok_eq!(zmachine.variable(0), 0x5678);
        assert_ok_eq!(zmachine.variable(0), 0x1234);
        assert!(zmachine.variable(0).is_err());
        assert_ok_eq!(zmachine.variable(1), 0x9988);
        assert_ok_eq!(zmachine.variable(2), 0x7766);
        assert_ok_eq!(zmachine.variable(3), 0x5544);
        assert!(zmachine.variable(4).is_err());
        assert_ok_eq!(zmachine.variable(0x80), 0x1100);
    }

    #[test]
    fn test_set_variable_indirect() {
        let mut map = test_map(3);
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        mock_routine(&mut map, 0x600, &[0x1122, 0x3344, 0x5566]);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(zmachine
            .state
            .call_routine(0x600, &vec![0x8888], None, 0x400)
            .is_ok());
        assert!(zmachine.set_variable_indirect(0, 0x1234).is_err());
        assert!(zmachine.push(0).is_ok());
        assert!(zmachine.set_variable_indirect(0, 0x5678).is_ok());
        assert!(zmachine.set_variable_indirect(1, 0x9988).is_ok());
        assert!(zmachine.set_variable_indirect(2, 0x7766).is_ok());
        assert!(zmachine.set_variable_indirect(3, 0x5544).is_ok());
        assert!(zmachine.set_variable_indirect(4, 0x3322).is_err());
        assert!(zmachine.set_variable_indirect(0x80, 0x1100).is_ok());
        assert_ok_eq!(zmachine.variable(0), 0x5678);
        assert!(zmachine.variable(0).is_err());
        assert_ok_eq!(zmachine.variable(1), 0x9988);
        assert_ok_eq!(zmachine.variable(2), 0x7766);
        assert_ok_eq!(zmachine.variable(3), 0x5544);
        assert!(zmachine.variable(4).is_err());
        assert_ok_eq!(zmachine.variable(0x80), 0x1100);
    }

    #[test]
    fn test_push() {
        let mut map = test_map(3);
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        mock_routine(&mut map, 0x600, &[0x1122, 0x3344, 0x5566]);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(zmachine
            .state
            .call_routine(0x600, &vec![0x8888], None, 0x400)
            .is_ok());
        assert!(zmachine.push(0x1234).is_ok());
        assert!(zmachine.push(0x5678).is_ok());
        assert_ok_eq!(zmachine.variable(0), 0x5678);
        assert_ok_eq!(zmachine.variable(0), 0x1234);
        assert!(zmachine.variable(0).is_err());
    }

    #[test]
    fn test_is_input_interrupt() {
        let mut map = test_map(3);
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        let m = Memory::new(map);
        let zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(!zmachine.is_input_interrupt());
    }

    #[test]
    fn test_string_literal() {
        let mut map = test_map(3);
        map.append(&mut vec![0; 0x10000]);
        for (i, b) in (0..0xF).enumerate() {
            map[0x10000 + (i * 2)] = (b + 1) * 0x11;
            map[0x10001 + (i * 2)] = (b + 1) * 0x11;
        }
        let m = Memory::new(map);
        let zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(zmachine.string_literal(0x10000).is_ok_and(
            |x| x == vec![0x1111, 0x2222, 0x3333, 0x4444, 0x5555, 0x6666, 0x7777, 0x8888]
        ));
    }

    #[test]
    fn test_packed_routine_address_v3() {
        let map = test_map(3);
        let m = Memory::new(map);
        let zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(zmachine
            .packed_routine_address(0x400)
            .is_ok_and(|x| x == 0x800));
    }

    #[test]
    fn test_packed_routine_address_v4() {
        let map = test_map(4);
        let m = Memory::new(map);
        let zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(zmachine
            .packed_routine_address(0x200)
            .is_ok_and(|x| x == 0x800));
    }

    #[test]
    fn test_packed_routine_address_v5() {
        let map = test_map(5);
        let m = Memory::new(map);
        let zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(zmachine
            .packed_routine_address(0x200)
            .is_ok_and(|x| x == 0x800));
    }

    #[test]
    fn test_packed_routine_address_v7() {
        let mut map = test_map(7);
        // Routine offset is 0x100;
        map[0x28] = 0x1;
        let m = Memory::new(map);
        let zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(zmachine
            .packed_routine_address(0x200)
            .is_ok_and(|x| x == 0x1000));
    }

    #[test]
    fn test_packed_routine_address_v8() {
        let map = test_map(8);
        let m = Memory::new(map);
        let zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(zmachine
            .packed_routine_address(0x100)
            .is_ok_and(|x| x == 0x800));
    }

    #[test]
    fn test_packed_string_address_v3() {
        let map = test_map(3);
        let m = Memory::new(map);
        let zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(zmachine
            .packed_string_address(0x400)
            .is_ok_and(|x| x == 0x800));
    }

    #[test]
    fn test_packed_string_address_v4() {
        let map = test_map(4);
        let m = Memory::new(map);
        let zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(zmachine
            .packed_string_address(0x200)
            .is_ok_and(|x| x == 0x800));
    }

    #[test]
    fn test_packed_string_address_v5() {
        let map = test_map(5);
        let m = Memory::new(map);
        let zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(zmachine
            .packed_string_address(0x200)
            .is_ok_and(|x| x == 0x800));
    }

    #[test]
    fn test_packed_string_address_v7() {
        let mut map = test_map(7);
        // String offset is 0x100;
        map[0x2A] = 0x1;
        let m = Memory::new(map);
        let zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(zmachine
            .packed_string_address(0x200)
            .is_ok_and(|x| x == 0x1000));
    }

    #[test]
    fn test_packed_string_address_v8() {
        let map = test_map(8);
        let m = Memory::new(map);
        let zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(zmachine
            .packed_string_address(0x100)
            .is_ok_and(|x| x == 0x800));
    }

    #[test]
    fn test_instruction() {
        let mut map = test_map(3);
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        let m = Memory::new(map);
        let zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert_eq!(
            zmachine.instruction(0x400),
            &[
                0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
                0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16
            ]
        )
    }

    #[test]
    fn test_frame_count() {
        let mut map = test_map(3);
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        mock_routine(&mut map, 0x400, &[]);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert_eq!(zmachine.frame_count(), 1);
        assert!(zmachine.call_routine(0x400, &vec![], None, 0x500).is_ok());
        assert_eq!(zmachine.frame_count(), 2);
    }

    #[test]
    fn test_checksum() {
        let mut map = test_map(3);
        map[0x1a] = 0x4;
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        let m = Memory::new(map);
        let zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert_ok_eq!(zmachine.checksum(), 0xf420);
    }

    #[test]
    fn test_save() {
        let mut map = test_map(3);
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        map[0x02] = 0x12;
        map[0x03] = 0x34;
        map[0x12] = b'2';
        map[0x13] = b'3';
        map[0x14] = b'0';
        map[0x15] = b'7';
        map[0x16] = b'1';
        map[0x17] = b'5';
        map[0x1C] = 0x56;
        map[0x1D] = 0x78;

        mock_routine(&mut map, 0x600, &[]);
        let m = Memory::new(map.clone());
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(zmachine.call_routine(0x600, &vec![], None, 0x500).is_ok());
        // See state.rs tests ... change dynamic memory a little bit
        assert!(zmachine.write_byte(0x200, 0xFC).is_ok());
        assert!(zmachine.write_byte(0x280, 0x10).is_ok());
        assert!(zmachine.write_byte(0x300, 0xFD).is_ok());

        input(&[
            '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}', 'z', '1', '.',
            'i', 'f', 'z', 's',
        ]);
        assert!(zmachine.save(0x9876).is_ok());
        assert!(Path::new("test-z1.ifzs").exists());
        let d = fs::read("test-z1.ifzs");
        assert!(fs::remove_file("test-z1.ifzs").is_ok());
        assert_ok_eq!(
            d,
            [
                b'F', b'O', b'R', b'M', 0x00, 0x00, 0x00, 0x50, b'I', b'F', b'Z', b'S', b'I', b'F',
                b'h', b'd', 0x00, 0x00, 0x00, 0x0D, 0x12, 0x34, 0x32, 0x33, 0x30, 0x37, 0x31, 0x35,
                0x56, 0x78, 0x00, 0x98, 0x76, 0x00, b'C', b'M', b'e', b'm', 0x00, 0x00, 0x00, 0x17,
                0x00, 0x00, 0x20, 0x00, 0x1B, 0x06, 0x5A, 0x00, 0x11, 0x01, 0x00, 0xFF, 0x00, 0xCC,
                0xFC, 0x00, 0x7E, 0x90, 0x00, 0x7E, 0xFD, 0x00, 0xFE, 0x00, b'S', b't', b'k', b's',
                0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05,
                0x00, 0x10, 0x00, 0x00, 0x00, 0x00
            ]
        );
    }

    #[test]
    fn test_restore() {
        let mut map = test_map(5);
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        map[0x02] = 0x12;
        map[0x03] = 0x34;
        map[0x12] = b'2';
        map[0x13] = b'3';
        map[0x14] = b'0';
        map[0x15] = b'7';
        map[0x16] = b'1';
        map[0x17] = b'5';
        map[0x1C] = 0x56;
        map[0x1D] = 0x78;
        mock_routine(&mut map, 0x600, &[]);
        let m = Memory::new(map.clone());
        let mut zmachine = assert_ok!(ZMachine::new(
            m,
            Config::new(3, 6, false, ErrorHandling::Ignore, 128.0),
            None,
            "test"
        ));
        // Turn on transcripting ... it should survive the restore
        assert!(header::set_flag2(&mut zmachine.state, Flags2::Transcripting).is_ok());

        let restore_data = vec![
            b'F', b'O', b'R', b'M', 0x00, 0x00, 0x00, 0x56, b'I', b'F', b'Z', b'S', b'I', b'F',
            b'h', b'd', 0x00, 0x00, 0x00, 0x0D, 0x12, 0x34, 0x32, 0x33, 0x30, 0x37, 0x31, 0x35,
            0x56, 0x78, 0x00, 0x9a, 0xbc, 0x00, b'C', b'M', b'e', b'm', 0x00, 0x00, 0x00, 0x0D,
            0x00, 0xFF, 0x00, 0xFF, 0xFC, 0x00, 0x7E, 0x90, 0x00, 0x7E, 0xFD, 0x00, 0xFE, 0x00,
            b'S', b't', b'k', b's', 0x00, 0x00, 0x00, 0x1E, 0x00, 0x04, 0x8E, 0x03, 0x80, 0x03,
            0x00, 0x02, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x11, 0x11, 0x22, 0x22, 0x00, 0x06,
            0x23, 0x12, 0x00, 0x00, 0x00, 0x00, 0x88, 0x99, 0xaa, 0xbb,
        ];
        let mut file = assert_ok!(fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open("test-z3.ifzs"));
        assert!(file.write_all(&restore_data).is_ok());
        assert!(file.flush().is_ok());
        input(&[
            '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}', '-', 'z', '3', '.', 'i', 'f', 'z',
            's',
        ]);
        assert_eq!(zmachine.frame_count(), 1);
        let r = zmachine.restore();
        assert!(fs::remove_file("test-z3.ifzs").is_ok());
        let pc = assert_ok!(r);
        assert_some_eq!(pc, 0x9abc);
        assert_ok_eq!(header::flag2(&zmachine.state, Flags2::Transcripting), 1);
        assert_ok_eq!(
            header::field_byte(&zmachine.state, HeaderField::DefaultForeground),
            3
        );
        assert_ok_eq!(
            header::field_byte(&zmachine.state, HeaderField::DefaultBackground),
            6
        );
        assert_ok_eq!(
            header::field_byte(&zmachine.state, HeaderField::ScreenLines),
            24
        );
        assert_ok_eq!(
            header::field_byte(&zmachine.state, HeaderField::ScreenColumns),
            80
        );
        assert_ok_eq!(zmachine.read_byte(0x200), 0xFC);
        assert_ok_eq!(zmachine.read_byte(0x280), 0x10);
        assert_ok_eq!(zmachine.read_byte(0x300), 0xFD);
        assert_eq!(zmachine.frame_count(), 2);
    }

    #[test]
    fn test_undo() {
        let mut map = test_map(3);
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        map[0x02] = 0x12;
        map[0x03] = 0x34;
        map[0x12] = b'2';
        map[0x13] = b'3';
        map[0x14] = b'0';
        map[0x15] = b'7';
        map[0x16] = b'1';
        map[0x17] = b'5';
        map[0x1C] = 0x56;
        map[0x1D] = 0x78;

        let m = Memory::new(map.clone());
        let mut zmachine = assert_ok!(ZMachine::new(
            m,
            Config::new(3, 6, false, ErrorHandling::Ignore, 128.0),
            None,
            "test"
        ));
        // Just test save/restore ... there are state.rs tests for the innards
        assert!(zmachine.save_undo(0x9867).is_ok());
        let pc = assert_ok!(zmachine.restore_undo());
        assert_some_eq!(pc, 0x9867);
    }

    #[test]
    fn test_restart() {
        let mut map = test_map(4);
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        map[0x02] = 0x12;
        map[0x03] = 0x34;
        map[0x12] = b'2';
        map[0x13] = b'3';
        map[0x14] = b'0';
        map[0x15] = b'7';
        map[0x16] = b'1';
        map[0x17] = b'5';
        map[0x1C] = 0x56;
        map[0x1D] = 0x78;

        let m = Memory::new(map.clone());
        let mut zmachine = assert_ok!(ZMachine::new(
            m,
            Config::new(3, 6, false, ErrorHandling::Ignore, 128.0),
            None,
            "test"
        ));
        // Set a predictable RNG that will always return 1
        zmachine.rng.predictable(1);
        assert!(zmachine.rng.random(1000) == 1 && zmachine.random(1000) == 1);
        assert!(zmachine.state.set_pc(0x401).is_ok());
        assert_ok_eq!(zmachine.state.pc(), 0x401);
        assert_ok_eq!(zmachine.restart(), 0x400);
        assert_ok_eq!(zmachine.state.pc(), 0x400);
        // Test the RNG is in random mode ... this _could_ fail
        assert!(zmachine.rng.random(1000) != 1 && zmachine.random(1000) != 1);
    }

    #[test]
    fn test_call_routine() {
        let mut map = test_map(3);
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        map[0x02] = 0x12;
        map[0x03] = 0x34;
        map[0x12] = b'2';
        map[0x13] = b'3';
        map[0x14] = b'0';
        map[0x15] = b'7';
        map[0x16] = b'1';
        map[0x17] = b'5';
        map[0x1C] = 0x56;
        map[0x1D] = 0x78;

        mock_routine(&mut map, 0x600, &[0x1111, 0x2222]);
        let m = Memory::new(map.clone());
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert_eq!(zmachine.frame_count(), 1);
        assert_ok_eq!(zmachine.call_routine(0x600, &vec![], None, 0x500), 0x605);
        assert_eq!(zmachine.frame_count(), 2);
    }

    #[test]
    fn test_call_read_interrupt() {
        let mut map = test_map(3);
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        map[0x02] = 0x12;
        map[0x03] = 0x34;
        map[0x12] = b'2';
        map[0x13] = b'3';
        map[0x14] = b'0';
        map[0x15] = b'7';
        map[0x16] = b'1';
        map[0x17] = b'5';
        map[0x1C] = 0x56;
        map[0x1D] = 0x78;

        mock_routine(&mut map, 0x600, &[0x1111, 0x2222]);
        let m = Memory::new(map.clone());
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert_eq!(zmachine.frame_count(), 1);
        zmachine.state.set_read_interrupt();
        assert_ok_eq!(zmachine.call_read_interrupt(0x600, 0x500), 0x605);
        assert_eq!(zmachine.frame_count(), 2);
        assert_some_eq!(zmachine.state.read_interrupt_result(), 0);
        // Test clear_read_interrupt() clears the state read_interrupt_result
        // because it's convenient to do so here
        zmachine.clear_read_interrupt();
        assert!(zmachine.read_interrupt_result().is_none());
    }

    #[test]
    fn test_read_interrupt() {
        let map = test_map(5);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(!zmachine.read_interrupt_pending());
        zmachine.set_read_interrupt_pending();
        assert!(zmachine.read_interrupt_pending());
        zmachine.clear_read_interrupt();
        assert!(!zmachine.read_interrupt_pending());
        assert!(zmachine.read_interrupt_result().is_none());
    }

    #[test]
    fn test_sound_interrupt() {
        let map = test_map(5);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(zmachine.sound_interrupt().is_none());
        zmachine.set_sound_interrupt(0x1234);
        assert_some_eq!(zmachine.sound_interrupt(), 0x1234);
    }

    #[test]
    fn test_call_sound_interrupt() {
        let mut map = test_map(5);
        mock_routine(&mut map, 0x600, &[0x1122, 0x3344]);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        zmachine.set_sound_interrupt(0x600);
        assert_eq!(zmachine.frame_count(), 1);
        assert_ok_eq!(zmachine.call_sound_interrupt(0x500), 0x601);
        assert_eq!(zmachine.frame_count(), 2);
        assert!(zmachine.sound_interrupt().is_none());
    }

    #[test]
    fn test_return_routine() {
        let mut map = test_map(5);
        mock_routine(&mut map, 0x500, &[0x1122, 0x3344]);
        mock_routine(&mut map, 0x600, &[]);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert_ok_eq!(
            zmachine.call_routine(0x500, &vec![0x1111, 0x2222, 0x3333], None, 0x40B),
            0x501
        );
        assert_ok_eq!(
            zmachine.call_routine(0x600, &vec![], Some(StoreResult::new(0x40A, 2)), 0x50B),
            0x601
        );
        assert_eq!(zmachine.frame_count(), 3);
        assert_ok_eq!(zmachine.return_routine(0x1234), 0x50B);
        assert_eq!(zmachine.frame_count(), 2);
        assert_ok_eq!(zmachine.variable(2), 0x1234);
    }

    #[test]
    fn test_return_routine_read_interrupt() {
        let mut map = test_map(5);
        mock_routine(&mut map, 0x500, &[0, 0, 0]);
        mock_routine(&mut map, 0x600, &[]);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert_ok_eq!(
            zmachine.call_routine(0x500, &vec![0x1111, 0x2222, 0x3333], None, 0x40B),
            0x501
        );
        zmachine.set_read_interrupt_pending();
        assert_ok_eq!(zmachine.call_read_interrupt(0x600, 0x50B), 0x601);
        assert_some_eq!(zmachine.state.read_interrupt_result(), 0);
        assert_eq!(zmachine.frame_count(), 3);
        assert_ok_eq!(zmachine.return_routine(0x1234), 0x50B);
        assert_eq!(zmachine.frame_count(), 2);
        assert_some_eq!(zmachine.state.read_interrupt_result(), 0x1234);
        assert_ok_eq!(zmachine.variable(2), 0x2222);
    }

    #[test]
    fn test_throw() {
        let mut map = test_map(5);
        mock_routine(&mut map, 0x500, &[0, 0, 0]);
        mock_routine(&mut map, 0x600, &[]);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert_ok_eq!(
            zmachine.call_routine(
                0x500,
                &vec![0x1111, 0x2222, 0x3333],
                Some(StoreResult::new(0x40A, 0)),
                0x40B
            ),
            0x501
        );
        assert_ok_eq!(zmachine.call_routine(0x600, &vec![], None, 0x50B), 0x601);
        assert_eq!(zmachine.frame_count(), 3);
        assert_ok_eq!(zmachine.throw(2, 0x1234), 0x40B);
        assert_eq!(zmachine.frame_count(), 1);
        assert_ok_eq!(zmachine.variable(0), 0x1234);
    }

    #[test]
    fn test_header_byte() {
        let mut map = test_map(3);
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        map[0x02] = 0x12;
        map[0x03] = 0x34;
        map[0x12] = b'2';
        map[0x13] = b'3';
        map[0x14] = b'0';
        map[0x15] = b'7';
        map[0x16] = b'1';
        map[0x17] = b'5';
        map[0x1C] = 0x56;
        map[0x1D] = 0x78;

        let m = Memory::new(map.clone());
        let zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert_ok_eq!(zmachine.header_byte(HeaderField::Version), 3);
    }

    #[test]
    fn test_header_word() {
        let mut map = test_map(3);
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        map[0x02] = 0x12;
        map[0x03] = 0x34;
        map[0x12] = b'2';
        map[0x13] = b'3';
        map[0x14] = b'0';
        map[0x15] = b'7';
        map[0x16] = b'1';
        map[0x17] = b'5';
        map[0x1C] = 0x56;
        map[0x1D] = 0x78;

        let m = Memory::new(map.clone());
        let zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert_ok_eq!(zmachine.header_word(HeaderField::Release), 0x1234);
    }

    #[test]
    fn test_random_random() {
        let map = test_map(3);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        for _ in 0..10 {
            assert!((1..=32767).contains(&zmachine.random(0x7FFF)));
        }
    }

    #[test]
    fn test_random_seeded() {
        let map = test_map(3);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        zmachine.seed(1024);
        assert_eq!(zmachine.random(100), 99);
        assert_eq!(zmachine.random(100), 93);
        assert_eq!(zmachine.random(100), 69);
        assert_eq!(zmachine.random(100), 89);
        assert_eq!(zmachine.random(100), 82);
        assert_eq!(zmachine.random(100), 26);
        assert_eq!(zmachine.random(100), 22);
        assert_eq!(zmachine.random(100), 40);
        assert_eq!(zmachine.random(100), 23);
        assert_eq!(zmachine.random(100), 76);
    }

    #[test]
    fn test_random_predictable() {
        let map = test_map(3);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        zmachine.predictable(5);
        for i in 1..4 {
            assert_eq!(zmachine.random(3), i)
        }
        for i in 1..3 {
            assert_eq!(zmachine.random(3), i)
        }
        assert_eq!(zmachine.random(50), 1);
    }

    #[test]
    fn test_rows() {
        let map = test_map(3);
        let m = Memory::new(map);
        let zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert_eq!(zmachine.rows(), 24);
    }

    #[test]
    fn test_columns() {
        let map = test_map(3);
        let m = Memory::new(map);
        let zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert_eq!(zmachine.columns(), 80);
    }

    #[test]
    fn test_output_stream_1_enable() {
        let map = test_map(3);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(zmachine.output_stream(1, None).is_ok());
        assert!(zmachine.io.is_stream_enabled(1));
    }

    #[test]
    fn test_output_stream_1_disable() {
        let map = test_map(3);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(zmachine.io.is_stream_enabled(1));
        assert!(zmachine.output_stream(-1, None).is_ok());
        assert!(!zmachine.io.is_stream_enabled(1));
    }

    #[test]
    fn test_output_stream_2_enable() {
        let map = test_map(3);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(!zmachine.io.is_stream_enabled(2));
        input(&[
            '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}', 'z', 'o', '1', '.', 't',
            'x', 't',
        ]);
        assert!(zmachine.output_stream(2, None).is_ok());
        assert!(Path::new("test-zo1.txt").exists());
        assert!(fs::remove_file("test-zo1.txt").is_ok());
        assert_ok_eq!(zmachine.read_byte(0x11), 1);
        assert!(zmachine.io.is_stream_enabled(2));
    }

    #[test]
    fn test_output_stream_2_disable() {
        let map = test_map(3);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(!zmachine.io.is_stream_enabled(2));
        input(&[
            '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}', 'z', 'o', '2', '.', 't',
            'x', 't',
        ]);
        assert!(zmachine.output_stream(2, None).is_ok());
        assert!(Path::new("test-zo2.txt").exists());
        assert!(fs::remove_file("test-zo2.txt").is_ok());
        assert_ok_eq!(zmachine.read_byte(0x11), 1);
        assert!(zmachine.io.is_stream_enabled(2));
        assert!(zmachine.output_stream(-2, None).is_ok());
        assert_ok_eq!(zmachine.read_byte(0x11), 0);
        assert!(!zmachine.io.is_stream_enabled(2));
    }

    #[test]
    fn test_output_stream_2_reenable() {
        let map = test_map(3);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(!zmachine.io.is_stream_enabled(2));
        input(&[
            '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}', 'z', 'o', '3', '.', 't',
            'x', 't',
        ]);
        assert!(zmachine.output_stream(2, None).is_ok());
        assert!(Path::new("test-zo3.txt").exists());
        assert_ok_eq!(zmachine.read_byte(0x11), 1);
        assert!(zmachine.io.is_stream_enabled(2));
        assert!(zmachine.output_stream(-2, None).is_ok());
        assert_ok_eq!(zmachine.read_byte(0x11), 0);
        assert!(!zmachine.io.is_stream_enabled(2));
        assert!(zmachine.output_stream(2, None).is_ok());
        assert!(fs::remove_file("test-zo3.txt").is_ok());
        assert_ok_eq!(zmachine.read_byte(0x11), 1);
        assert!(zmachine.io.is_stream_enabled(2));
    }

    #[test]
    fn test_output_stream_2_error() {
        let map = test_map(3);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(!zmachine.io.is_stream_enabled(2));
        input(&[
            '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}',
            '\u{08}', '\u{08}', '\u{08}', '/', 'x', '/', 'f',
        ]);
        assert!(zmachine.output_stream(2, None).is_err());
        assert_ok_eq!(zmachine.read_byte(0x11), 0);
        assert!(!zmachine.io.is_stream_enabled(2));
    }

    #[test]
    fn test_output_stream_3_enable() {
        let map = test_map(3);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(!zmachine.io.is_stream_enabled(3));
        assert!(zmachine.output_stream(3, Some(0x300)).is_ok());
        assert!(zmachine.io.is_stream_enabled(3));
    }

    #[test]
    fn test_output_stream_3_disable() {
        let map = test_map(3);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(!zmachine.io.is_stream_enabled(3));
        assert!(zmachine.output_stream(3, Some(0x300)).is_ok());
        assert!(zmachine.io.is_stream_enabled(3));
        assert!(zmachine.print_str("Test stream 3".to_string()).is_ok());
        assert!(zmachine.output_stream(-3, None).is_ok());
        assert!(!zmachine.io.is_stream_enabled(3));
        assert_print!("");
        assert_ok_eq!(zmachine.read_word(0x300), 13);
        assert_ok_eq!(zmachine.read_byte(0x302), b'T');
        assert_ok_eq!(zmachine.read_byte(0x303), b'e');
        assert_ok_eq!(zmachine.read_byte(0x304), b's');
        assert_ok_eq!(zmachine.read_byte(0x305), b't');
        assert_ok_eq!(zmachine.read_byte(0x306), b' ');
        assert_ok_eq!(zmachine.read_byte(0x307), b's');
        assert_ok_eq!(zmachine.read_byte(0x308), b't');
        assert_ok_eq!(zmachine.read_byte(0x309), b'r');
        assert_ok_eq!(zmachine.read_byte(0x30a), b'e');
        assert_ok_eq!(zmachine.read_byte(0x30b), b'a');
        assert_ok_eq!(zmachine.read_byte(0x30c), b'm');
        assert_ok_eq!(zmachine.read_byte(0x30d), b' ');
        assert_ok_eq!(zmachine.read_byte(0x30e), b'3');
    }

    #[test]
    fn test_output_stream_4_enable() {
        let map = test_map(3);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(!zmachine.io.is_stream_enabled(4));
        assert!(zmachine.output_stream(4, None).is_err());
        assert!(!zmachine.io.is_stream_enabled(4));
    }

    #[test]
    fn test_output_stream_4_disable() {
        let map = test_map(3);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(!zmachine.io.is_stream_enabled(4));
        assert!(zmachine.output_stream(-4, None).is_err());
        assert!(!zmachine.io.is_stream_enabled(4));
    }

    #[test]
    fn test_output_stream_invalid() {
        let map = test_map(3);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(zmachine.output_stream(5, None).is_err());
    }

    #[test]
    fn test_print() {
        let map = test_map(3);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(zmachine
            .print(&vec![b'T' as u16, b'e' as u16, b's' as u16, b't' as u16])
            .is_ok(),);
        assert_print!("Test");
    }

    #[test]
    fn test_print_in_input_interrupt() {
        let mut map = test_map(5);
        mock_routine(&mut map, 0x400, &[]);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        zmachine.state.set_read_interrupt();
        assert_ok_eq!(zmachine.call_read_interrupt(0x400, 0x500), 0x401);
        assert!(zmachine
            .print(&vec![b'T' as u16, b'e' as u16, b's' as u16, b't' as u16])
            .is_ok(),);
        assert_print!("Test");
        assert!(zmachine.input_interrupt_print());
    }

    #[test]
    fn test_print_str() {
        let map = test_map(3);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(zmachine.print_str("Test".to_string()).is_ok(),);
        assert_print!("Test");
    }

    #[test]
    fn test_split_window() {
        let map = test_map(3);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(zmachine.split_window(10).is_ok());
        assert_eq!(split(), 10);
    }

    #[test]
    fn test_set_window() {
        let map = test_map(3);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(zmachine.split_window(10).is_ok());
        assert!(zmachine.set_window(1).is_ok());
        assert_eq!(window(), 1);
    }

    #[test]
    fn test_erase_window() {
        let map = test_map(3);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(zmachine.erase_window(0).is_ok());
        assert_eq!(erase_window(), &[0]);
    }

    #[test]
    fn test_erase_line() {
        let map = test_map(4);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(zmachine.erase_line().is_ok());
        assert!(erase_line());
    }

    #[test]
    fn test_status_line_score_min() {
        let mut map = test_map(3);
        // Status Object
        //   4     18    19       6     19    1A       18    0     4        14    7     F        A      8     19
        // 0 00100 11000 11001  0 00110 11001 11010  0 11000 00000 00100  0 10100 00111 01111  1 01010  01000 11001
        // 1319                 1B3A                 6004                 50EF                 A919
        mock_object(
            &mut map,
            1,
            vec![0x1319, 0x1B3A, 0x6004, 0x50EF, 0xA919],
            (0, 0, 0),
        );
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(zmachine.set_variable(16, 1).is_ok());
        assert!(zmachine.set_variable(17, 0xFF0A).is_ok());
        assert!(zmachine.set_variable(18, 0).is_ok());
        assert!(header::clear_flag1(&mut zmachine.state, Flags1v3::StatusLineType as u8).is_ok());
        assert!(zmachine.status_line().is_ok());
        assert_print!(
            " Status Object                                                         -99/0    "
        );
    }

    #[test]
    fn test_status_line_score_max() {
        let mut map = test_map(3);
        // Status Object
        //   4     18    19       6     19    1A       18    0     4        14    7     F        A      8     19
        // 0 00100 11000 11001  0 00110 11001 11010  0 11000 00000 00100  0 10100 00111 01111  1 01010  01000 11001
        // 1319                 1B3A                 6004                 50EF                 A919
        mock_object(
            &mut map,
            1,
            vec![0x1319, 0x1B3A, 0x6004, 0x50EF, 0xA919],
            (0, 0, 0),
        );
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(zmachine.set_variable(16, 1).is_ok());
        assert!(zmachine.set_variable(17, 999).is_ok());
        assert!(zmachine.set_variable(18, 9999).is_ok());
        assert!(header::clear_flag1(&mut zmachine.state, Flags1v3::StatusLineType as u8).is_ok());
        assert!(zmachine.status_line().is_ok());
        assert_print!(
            " Status Object                                                         999/9999 "
        );
    }

    #[test]
    fn test_status_line_time_12_am() {
        let mut map = test_map(3);
        mock_object(
            &mut map,
            1,
            vec![0x1319, 0x1B3A, 0x6004, 0x50EF, 0xA919],
            (0, 0, 0),
        );
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(zmachine.set_variable(16, 1).is_ok());
        assert!(zmachine.set_variable(17, 0).is_ok());
        assert!(zmachine.set_variable(18, 0).is_ok());
        assert!(header::set_flag1(&mut zmachine.state, Flags1v3::StatusLineType as u8).is_ok());
        assert!(zmachine.status_line().is_ok());
        assert_print!(
            " Status Object                                                         12:00 AM "
        );
    }

    #[test]
    fn test_status_line_time_6_59_am() {
        let mut map = test_map(3);
        mock_object(
            &mut map,
            1,
            vec![0x1319, 0x1B3A, 0x6004, 0x50EF, 0xA919],
            (0, 0, 0),
        );
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(zmachine.set_variable(16, 1).is_ok());
        assert!(zmachine.set_variable(17, 6).is_ok());
        assert!(zmachine.set_variable(18, 59).is_ok());
        assert!(header::set_flag1(&mut zmachine.state, Flags1v3::StatusLineType as u8).is_ok());
        assert!(zmachine.status_line().is_ok());
        assert_print!(
            " Status Object                                                          6:59 AM "
        );
    }

    #[test]
    fn test_status_line_time_12_00_pm() {
        let mut map = test_map(3);
        mock_object(
            &mut map,
            1,
            vec![0x1319, 0x1B3A, 0x6004, 0x50EF, 0xA919],
            (0, 0, 0),
        );
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(zmachine.set_variable(16, 1).is_ok());
        assert!(zmachine.set_variable(17, 12).is_ok());
        assert!(zmachine.set_variable(18, 00).is_ok());
        assert!(header::set_flag1(&mut zmachine.state, Flags1v3::StatusLineType as u8).is_ok());
        assert!(zmachine.status_line().is_ok());
        assert_print!(
            " Status Object                                                         12:00 PM "
        );
    }

    #[test]
    fn test_status_line_time_6_30_pm() {
        let mut map = test_map(3);
        mock_object(
            &mut map,
            1,
            vec![0x1319, 0x1B3A, 0x6004, 0x50EF, 0xA919],
            (0, 0, 0),
        );
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(zmachine.set_variable(16, 1).is_ok());
        assert!(zmachine.set_variable(17, 18).is_ok());
        assert!(zmachine.set_variable(18, 30).is_ok());
        assert!(header::set_flag1(&mut zmachine.state, Flags1v3::StatusLineType as u8).is_ok());
        assert!(zmachine.status_line().is_ok());
        assert_print!(
            " Status Object                                                          6:30 PM "
        );
    }

    #[test]
    fn test_status_line_time_invalid() {
        let mut map = test_map(3);
        mock_object(
            &mut map,
            1,
            vec![0x1319, 0x1B3A, 0x6004, 0x50EF, 0xA919],
            (0, 0, 0),
        );
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(zmachine.set_variable(16, 1).is_ok());
        assert!(zmachine.set_variable(17, 24).is_ok());
        assert!(zmachine.set_variable(18, 60).is_ok());
        assert!(header::set_flag1(&mut zmachine.state, Flags1v3::StatusLineType as u8).is_ok());
        assert!(zmachine.status_line().is_ok());
        assert_print!(
            " Status Object                                                         11:59 PM "
        );
    }

    #[test]
    fn test_set_font() {
        let map = test_map(3);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert_ok_eq!(zmachine.set_font(3), 1);
        assert_ok_eq!(zmachine.set_font(0), 3);
    }

    #[test]
    fn test_set_text_style() {
        let map = test_map(3);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(zmachine.set_text_style(Style::Bold as u16).is_ok());
        assert_eq!(style(), Style::Bold as u8);
    }

    #[test]
    fn test_cursor() {
        let map = test_map(3);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert_ok_eq!(zmachine.cursor(), (24, 1));
    }

    #[test]
    fn test_set_cursor() {
        let map = test_map(3);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(zmachine.set_cursor(12, 40).is_ok());
        assert_eq!(cursor(), (12, 40));
    }

    #[test]
    fn test_buffer_mode() {
        let map = test_map(3);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(zmachine.buffer_mode(1).is_ok());
        assert_eq!(buffer_mode(), 1);
        assert!(zmachine.buffer_mode(0).is_ok());
        assert_eq!(buffer_mode(), 0);
    }

    #[test]
    fn test_beep() {
        let map = test_map(3);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(zmachine.beep().is_ok());
        assert!(beep());
    }

    #[test]
    fn test_set_colors() {
        let map = test_map(3);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(zmachine.set_colors(6, 3).is_ok());
        assert_eq!(colors(), (6, 3));
    }

    #[test]
    fn test_read_key() {
        let map = test_map(5);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        input(&[' ']);
        assert_ok_eq!(zmachine.read_key(0), InputEvent::from_char(' ' as u16));
    }

    #[test]
    fn test_read_key_with_timeout() {
        let map = test_map(5);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        input(&[' ']);
        set_input_delay(50);
        assert_ok_eq!(zmachine.read_key(100), InputEvent::from_char(' ' as u16));
    }

    #[test]
    fn test_read_key_timeout() {
        let map = test_map(5);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        input(&[' ']);
        set_input_timeout();
        assert_ok_eq!(
            zmachine.read_key(100),
            InputEvent::from_interrupt(Interrupt::ReadTimeout)
        );
    }

    #[test]
    fn test_read_key_sound_interrupt() {
        let map = test_map(5);
        let m = Memory::new(map);
        let blorb = mock_blorb();
        let manager = assert_ok!(Manager::new(128.0, blorb));
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), Some(manager), "test"));
        zmachine.set_sound_interrupt(0x1234);
        let manager = assert_some!(zmachine.sound_manager.as_mut());
        assert!(!manager.is_playing());
        assert_ok_eq!(
            zmachine.read_key(0),
            InputEvent::from_interrupt(Interrupt::Sound)
        );
    }

    #[test]
    fn test_read_key_mouse_click() {
        let mut map = test_map(5);
        map[0x101] = 2;
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(header::set_word(&mut zmachine.state, HeaderField::ExtensionTable, 0x100).is_ok());
        input(&['\u{FD}']);
        // test_terminal returns fixed mouse coordinates 12,18
        assert_ok_eq!(zmachine.read_key(0), InputEvent::from_mouse(0xFD, 18, 12));
        assert_ok_eq!(zmachine.read_word(0x102), 12);
        assert_ok_eq!(zmachine.read_word(0x104), 18);
    }

    #[test]
    fn test_read_key_mouse_double_click() {
        let mut map = test_map(5);
        map[0x101] = 2;
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(header::set_word(&mut zmachine.state, HeaderField::ExtensionTable, 0x100).is_ok());
        input(&['\u{FE}']);
        // test_terminal returns fixed mouse coordinates 12,18
        assert_ok_eq!(zmachine.read_key(0), InputEvent::from_mouse(0xFE, 18, 12));
        assert_ok_eq!(zmachine.read_word(0x102), 12);
        assert_ok_eq!(zmachine.read_word(0x104), 18);
    }

    #[test]
    fn test_read_line() {
        let map = test_map(5);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        // Tests length limit
        // Tests backspace
        // Tests terminator
        input(&['T', 'e', 's', 't', 'i', 'n', 'g', '\u{08}', '\u{0d}']);
        assert_ok_eq!(
            zmachine.read_line(&[], 6, &['\r' as u16], 0),
            [
                b'T' as u16,
                b'e' as u16,
                b's' as u16,
                b't' as u16,
                b'i' as u16,
                b'\r' as u16
            ]
        );
        assert_print!("Testin");
    }

    #[test]
    fn test_read_line_fn_terminator() {
        let mut map = test_map(5);
        map[0x101] = 2;
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(header::set_word(&mut zmachine.state, HeaderField::ExtensionTable, 0x100).is_ok());
        input(&['T', 'e', 's', 't', 'i', 'n', 'g', 'x', '\u{08}', '\u{FD}']);
        assert_ok_eq!(
            zmachine.read_line(&[], 16, &['\r' as u16, 255], 0),
            [
                b'T' as u16,
                b'e' as u16,
                b's' as u16,
                b't' as u16,
                b'i' as u16,
                b'n' as u16,
                b'g' as u16,
                0xFD
            ]
        );
        // The x is printed, even though it is erased by the backspace
        assert_print!("Testingx");
        assert_ok_eq!(zmachine.read_word(0x102), 12);
        assert_ok_eq!(zmachine.read_word(0x104), 18);
    }

    #[test]
    fn test_read_line_with_timeout() {
        let map = test_map(5);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        input(&['T', 'e', 's', 't', 'i', 'n', 'g']);
        assert_ok_eq!(
            zmachine.read_line(&[], 16, &['\r' as u16], 100),
            [
                b'T' as u16,
                b'e' as u16,
                b's' as u16,
                b't' as u16,
                b'i' as u16,
                b'n' as u16,
                b'g' as u16,
                b'\r' as u16
            ]
        );
        assert_print!("Testing");
    }
    #[test]
    fn test_read_line_timeout() {
        let map = test_map(5);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        input(&['T', 'e', 's', 't', 'i', 'n', 'g']);
        set_input_delay(350);
        let r = assert_ok!(zmachine.read_line(&[], 16, &['\r' as u16], 1000));
        assert_eq!(r, [b'T' as u16, b'e' as u16, b's' as u16]);
        assert_print!("Tes");
    }

    #[test]
    fn test_read_line_sound_interrupt() {
        let map = test_map(5);
        let m = Memory::new(map);
        let blorb = mock_blorb();
        let manager = assert_ok!(Manager::new(128.0, blorb));
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), Some(manager), "test"));
        zmachine.set_sound_interrupt(0x1234);
        let manager = assert_some!(zmachine.sound_manager.as_mut());
        assert!(!manager.is_playing());
        let input = assert_ok!(zmachine.read_line(&[], 16, &[b'\r' as u16], 0));
        assert!(input.is_empty());
    }

    #[test]
    fn test_prompt_filename_first() {
        let map = test_map(5);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        input(&['\r']);
        assert_ok_eq!(
            zmachine.prompt_filename("Filename? ", "pf01", false, true),
            "test-01.pf01"
        );
        assert_print!("Filename? test-01.pf01");
    }

    #[test]
    fn test_prompt_filename_first_existing() {
        let map = test_map(5);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        input(&['\r']);
        let f = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open("test-01.pf02");
        assert!(f.is_ok());
        assert!(Path::new("test-01.pf02").exists());
        let r = zmachine.prompt_filename("Filename? ", "pf02", false, true);
        assert!(fs::remove_file("test-01.pf02").is_ok());
        assert_ok_eq!(r, "test-02.pf02");
        assert_print!("Filename? test-02.pf02");
    }

    #[test]
    fn test_prompt_filename_first_last_none() {
        let map = test_map(5);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        input(&['\r']);
        let r = zmachine.prompt_filename("Filename? ", "pf03", true, false);
        assert_ok_eq!(r, "test.pf03");
        assert_print!("Filename? test.pf03");
    }

    #[test]
    fn test_prompt_filename_first_last_existing() {
        let map = test_map(5);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        input(&['\r']);
        let f = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open("test-01.pf04");
        assert!(f.is_ok());
        assert!(Path::new("test-01.pf04").exists());
        let f = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open("test-02.pf04");
        assert!(f.is_ok());
        assert!(Path::new("test-02.pf04").exists());
        let r = zmachine.prompt_filename("Filename? ", "pf04", true, false);
        assert!(fs::remove_file("test-01.pf04").is_ok());
        assert!(fs::remove_file("test-02.pf04").is_ok());
        assert_ok_eq!(r, "test-02.pf04");
        assert_print!("Filename? test-02.pf04");
    }

    #[test]
    fn test_prompt_filename_overwrite_existing() {
        let map = test_map(5);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        input(&['\r']);
        let f = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open("test-01.pf05");
        assert!(f.is_ok());
        assert!(Path::new("test-01.pf05").exists());
        let r = zmachine.prompt_filename("Filename? ", "pf05", false, false);
        assert!(fs::remove_file("test-01.pf05").is_ok());
        assert!(r.is_err());
        assert_print!("Filename? test-01.pf05");
    }

    #[test]
    fn test_prompt_filename_invalid_filename_z() {
        let map = test_map(5);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        input(&['\u{08}', '\u{08}', '\u{08}', '\u{08}', 'z', '5', '\r']);
        let r = zmachine.prompt_filename("Filename? ", "pf06", false, false);
        assert!(r.is_err());
        assert_print!("Filename? test.pf06z5");
    }

    #[test]
    fn test_prompt_filename_invalid_filename_blb() {
        let map = test_map(5);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        input(&['\u{08}', '\u{08}', '\u{08}', '\u{08}', 'b', 'l', 'b', '\r']);
        let r = zmachine.prompt_filename("Filename? ", "pf06", false, false);
        assert!(r.is_err());
        assert_print!("Filename? test.pf06blb");
    }

    #[test]
    fn test_prompt_filename_invalid_filename_blorb() {
        let map = test_map(5);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        input(&[
            '\u{08}', '\u{08}', '\u{08}', '\u{08}', 'b', 'l', 'o', 'r', 'b', '\r',
        ]);
        let r = zmachine.prompt_filename("Filename? ", "pf06", false, false);
        assert!(r.is_err());
        assert_print!("Filename? test.pf06blorb");
    }

    #[test]
    fn test_prompt_and_create() {
        let map = test_map(5);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        input(&['\r']);
        let r = zmachine.prompt_and_create("Filename? ", "pc01", false);
        assert!(Path::new("test-01.pc01").exists());
        assert!(fs::remove_file("test-01.pc01").is_ok());
        assert!(r.is_ok());
    }

    #[test]
    fn test_prompt_and_create_exists() {
        let map = test_map(5);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        let f = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open("test-01.pc02");
        assert!(f.is_ok());
        assert!(Path::new("test-01.pc02").exists());
        input(&[
            '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}', '-',
            '0', '1', '.', 'p', 'c', '0', '2', '\r',
        ]);
        let r = zmachine.prompt_and_create("Filename? ", "pc02", false);
        assert!(Path::new("test-01.pc02").exists());
        assert!(fs::remove_file("test-01.pc02").is_ok());
        assert!(r.is_err());
    }

    #[test]
    fn test_prompt_and_create_exists_overwrite() {
        let map = test_map(5);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        let f = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open("test-01.pc03");
        assert!(f.is_ok());
        assert!(Path::new("test-01.pc03").exists());
        input(&[
            '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}', '-',
            '0', '1', '.', 'p', 'c', '0', '3', '\r',
        ]);
        let r = zmachine.prompt_and_create("Filename? ", "pc03", true);
        assert!(Path::new("test-01.pc03").exists());
        assert!(fs::remove_file("test-01.pc03").is_ok());
        assert!(r.is_ok());
    }

    #[test]
    fn test_prompt_and_write() {
        let map = test_map(5);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        input(&['\r']);
        let r = zmachine.prompt_and_write("Filename? ", "pw01", &[1, 2, 3, 4], false);
        assert!(Path::new("test-01.pw01").exists());
        let data = fs::read("test-01.pw01");
        assert!(fs::remove_file("test-01.pw01").is_ok());
        assert!(r.is_ok());
        assert_ok_eq!(data, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_prompt_and_write_exists() {
        let map = test_map(5);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        let mut f = assert_ok!(fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open("test-01.pw02"));
        assert!(f.write_all(&[1, 2, 3, 4]).is_ok());
        assert!(Path::new("test-01.pw02").exists());
        input(&[
            '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}', '-',
            '0', '1', '.', 'p', 'w', '0', '2', '\r',
        ]);
        let r = zmachine.prompt_and_write("Filename? ", "pw02", &[5, 6, 7, 8], false);
        assert!(Path::new("test-01.pw02").exists());
        let data = fs::read("test-01.pw02");
        assert!(fs::remove_file("test-01.pw02").is_ok());
        assert!(r.is_err());
        assert_ok_eq!(data, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_prompt_and_write_exists_overwrite() {
        let map = test_map(5);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        let mut f = assert_ok!(fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open("test-01.pw04"));
        assert!(f.write_all(&[1, 2, 3, 4]).is_ok());
        assert!(Path::new("test-01.pw04").exists());
        input(&[
            '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}', '\u{08}', '-',
            '0', '1', '.', 'p', 'w', '0', '4', '\r',
        ]);
        let r = zmachine.prompt_and_write("Filename? ", "pw04", &[5, 6, 7, 8], true);
        assert!(Path::new("test-01.pw04").exists());
        let data = fs::read("test-01.pw04");
        assert!(fs::remove_file("test-01.pw04").is_ok());
        assert!(r.is_ok());
        assert_ok_eq!(data, vec![5, 6, 7, 8]);
    }

    #[test]
    fn test_prompt_and_read() {
        let map = test_map(5);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        let mut f = assert_ok!(fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open("test-01.pr01"));
        assert!(f.write_all(&[1, 2, 3, 4]).is_ok());
        assert!(Path::new("test-01.pr01").exists());
        input(&['\r']);
        let r = zmachine.prompt_and_read("Filename? ", "pr01");
        assert!(fs::remove_file("test-01.pr01").is_ok());
        assert_ok_eq!(r, [1, 2, 3, 4]);
    }

    #[test]
    fn test_prompt_and_read_error() {
        let map = test_map(5);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        input(&['\r']);
        let r = zmachine.prompt_and_read("Filename? ", "pr02");
        assert!(r.is_err());
    }

    #[test]
    fn test_quit() {
        let map = test_map(5);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        input(&['\r']);
        assert!(zmachine.quit().is_ok());
        assert_print!("Press any key to exit");
        assert!(quit());
    }

    #[test]
    fn test_new_line() {
        let map = test_map(3);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(zmachine.new_line().is_ok());
        assert_eq!(scroll(), 2);
    }

    #[test]
    fn test_backspace() {
        let map = test_map(3);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(zmachine.set_cursor(23, 2).is_ok());
        assert!(zmachine.backspace().is_ok());
        assert_eq!(backspace(), (23, 1));
    }

    #[test]
    fn test_play_sound_v3() {
        let map = test_map(3);
        let m = Memory::new(map);
        let blorb = mock_blorb();
        let manager = assert_ok!(Manager::new(128.0, blorb));
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), Some(manager), "test"));
        assert!(zmachine.play_sound(1, 8, 0, None).is_ok());
        assert_eq!(play_sound(), (4, 8, 10));
        assert!(zmachine.is_sound_playing());
    }

    #[test]
    fn test_play_sound_v5() {
        let map = test_map(5);
        let m = Memory::new(map);
        let ridx = RIdx::new(vec![
            Index::new("Snd ".to_string(), 1, 0x100),
            Index::new("Snd ".to_string(), 2, 0x200),
            Index::new("Pic ".to_string(), 1, 0x300),
            Index::new("Snd ".to_string(), 4, 0x400),
        ]);
        let sloop = Loop::new(vec![Entry::new(1, 0), Entry::new(2, 20)]);
        let mut sounds = HashMap::new();
        sounds.insert(0x100, Chunk::new_chunk(0x100, "OGGV", vec![1, 1, 1, 1]));
        sounds.insert(0x400, Chunk::new_chunk(0x400, "OGGV", vec![4, 4, 4, 4]));
        sounds.insert(0x200, Chunk::new_form(0x200, "AIFF", vec![]));
        let blorb = Blorb::new(ridx, None, sounds, Some(sloop), None);
        let manager = assert_ok!(Manager::new(128.0, blorb));
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), Some(manager), "test"));
        assert!(zmachine.play_sound(1, 8, 0, None).is_ok());
        assert_eq!(play_sound(), (4, 8, 0));
        assert!(zmachine.is_sound_playing());
    }

    #[test]
    fn test_play_sound_v5_with_repeats() {
        let map = test_map(5);
        let m = Memory::new(map);
        let blorb = mock_blorb();
        let manager = assert_ok!(Manager::new(128.0, blorb));
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), Some(manager), "test"));
        assert!(zmachine.play_sound(4, 8, 5, None).is_ok());
        assert_eq!(play_sound(), (4, 8, 5));
        assert!(zmachine.is_sound_playing());
    }

    #[test]
    fn test_play_sound_v5_with_interrupt() {
        let map = test_map(5);
        let m = Memory::new(map);
        let blorb = mock_blorb();
        let manager = assert_ok!(Manager::new(128.0, blorb));
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), Some(manager), "test"));
        assert!(zmachine.play_sound(4, 8, 5, Some(0x500)).is_ok());
        assert_some_eq!(zmachine.sound_interrupt(), 0x500);
        assert_eq!(play_sound(), (4, 8, 5));
        assert!(zmachine.is_sound_playing());
    }

    #[test]
    fn test_play_sound_change_volume() {
        let map = test_map(5);
        let m = Memory::new(map);
        let blorb = mock_blorb();
        let manager = assert_ok!(Manager::new(128.0, blorb));
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), Some(manager), "test"));
        assert!(zmachine.play_sound(4, 8, 5, None).is_ok());
        assert_eq!(play_sound(), (4, 8, 5));
        assert!(zmachine.is_sound_playing());
        assert!(zmachine.play_sound(4, 4, 5, None).is_ok());
        assert_eq!(play_sound(), (0, 4, 0));
        assert!(zmachine.is_sound_playing());
    }

    #[test]
    fn test_play_sound_no_effect() {
        let map = test_map(5);
        let m = Memory::new(map);
        let ridx = RIdx::new(vec![
            Index::new("Snd ".to_string(), 1, 0x100),
            Index::new("Snd ".to_string(), 2, 0x200),
            Index::new("Pic ".to_string(), 1, 0x300),
            Index::new("Snd ".to_string(), 4, 0x400),
        ]);
        let sloop = Loop::new(vec![Entry::new(1, 0), Entry::new(2, 20)]);
        let mut sounds = HashMap::new();
        sounds.insert(0x100, Chunk::new_chunk(0x100, "OGGV", vec![1, 1, 1, 1]));
        sounds.insert(0x400, Chunk::new_chunk(0x400, "OGGV", vec![4, 4, 4, 4]));
        sounds.insert(0x200, Chunk::new_form(0x200, "AIFF", vec![]));
        let blorb = Blorb::new(ridx, None, sounds, Some(sloop), None);
        let manager = assert_ok!(Manager::new(128.0, blorb));
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), Some(manager), "test"));
        assert!(zmachine.play_sound(2, 8, 5, None).is_ok());
        assert_eq!(play_sound(), (0, 0, 0));
        assert!(!zmachine.is_sound_playing());
    }

    #[test]
    fn test_play_sound_no_manager() {
        let map = test_map(5);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(zmachine.play_sound(2, 8, 5, None).is_ok());
        assert_eq!(play_sound(), (0, 0, 0));
        assert!(!zmachine.is_sound_playing());
    }

    #[test]
    fn test_stop_sound() {
        let map = test_map(5);
        let m = Memory::new(map);
        let blorb = mock_blorb();
        let manager = assert_ok!(Manager::new(128.0, blorb));
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), Some(manager), "test"));
        assert!(zmachine.play_sound(4, 8, 5, None).is_ok());
        assert_eq!(play_sound(), (4, 8, 5));
        assert!(zmachine.is_sound_playing());
        assert!(zmachine.stop_sound().is_ok());
        assert_eq!(play_sound(), (0, 0, 0));
        assert!(!zmachine.is_sound_playing());
    }

    #[test]
    fn test_stop_sound_not_playing() {
        let map = test_map(5);
        let m = Memory::new(map);
        let blorb = mock_blorb();
        let manager = assert_ok!(Manager::new(128.0, blorb));
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), Some(manager), "test"));
        assert!(!zmachine.is_sound_playing());
        assert!(zmachine.stop_sound().is_ok());
        assert_eq!(play_sound(), (0, 0, 0));
        assert!(!zmachine.is_sound_playing());
    }

    #[test]
    fn test_stop_sound_no_manager() {
        let map = test_map(5);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(zmachine.stop_sound().is_ok());
        assert_eq!(play_sound(), (0, 0, 0));
        assert!(!zmachine.is_sound_playing());
    }

    #[test]
    fn test_is_sound_playing() {
        let map = test_map(5);
        let m = Memory::new(map);
        let blorb = mock_blorb();
        let manager = assert_ok!(Manager::new(128.0, blorb));
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), Some(manager), "test"));
        assert!(!zmachine.is_sound_playing());
        assert!(zmachine.play_sound(4, 5, 5, None).is_ok());
        assert!(zmachine.is_sound_playing());
    }

    #[test]
    fn test_is_sound_playing_no_manager() {
        let map = test_map(5);
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        assert!(!zmachine.is_sound_playing());
    }

    #[test]
    fn test_run() {
        let mut map = test_map(5);
        // NOP and QUIT
        map[0x400] = 0xB4;
        map[0x401] = 0xBA;
        let m = Memory::new(map);
        let mut zmachine = assert_ok!(ZMachine::new(m, Config::default(), None, "test"));
        input(&[' ']);
        assert!(zmachine.run().is_ok());
    }
}
