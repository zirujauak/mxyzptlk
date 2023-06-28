mod files;
mod input;
mod instruction;
mod io;
mod rng;
// mod save_restore;
pub mod sound;
pub mod state;

use std::fs;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::thread;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use crate::config::Config;
use crate::error::*;
use crate::zmachine::io::screen::Interrupt;
use rng::chacha_rng::ChaChaRng;
use rng::RNG;

use self::instruction::decoder;
use self::instruction::processor;
use self::io::IO;
use self::io::screen::InputEvent;
use self::sound::Sounds;
use self::state::header;
use self::state::header::Flags1v3;
use self::state::header::HeaderField;
use self::state::memory::Memory;
use self::state::object::property;
use self::state::text;
use self::state::State;

pub struct ZMachine {
    name: String,
    version: u8,
    state: State,
    io: IO,
    rng: Box<dyn RNG>,
    output_streams: u8,
    input_interrupt: Option<u16>,
    input_interrupt_print: bool,
    sounds: Option<Sounds>,
    sound_interrupt: Option<usize>,
}

impl ZMachine {
    pub fn new(
        memory: Memory,
        config: Config,
        sounds: Option<Sounds>,
        name: &str,
    ) -> Result<ZMachine, RuntimeError> {
        let version = memory.read_byte(HeaderField::Version as usize)?;

        if let Some(s) = sounds.as_ref() {
            info!(target: "app::sound", "{} sounds loaded", s.sounds().len())
        }
        let rng = ChaChaRng::new();

        let io = IO::new(version, config)?;

        let mut state = State::new(memory)?;

        let colors = io.default_colors();
        state.initialize(
            io.rows() as u8,
            io.columns() as u8,
            (colors.0 as u8, colors.1 as u8),
        )?;
        Ok(ZMachine {
            name: name.to_string(),
            version,
            state,
            io,
            rng: Box::new(rng),
            output_streams: 0x1,
            input_interrupt: None,
            input_interrupt_print: false,
            sounds,
            sound_interrupt: None,
        })
    }

    pub fn version(&self) -> u8 {
        self.version
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut State {
        &mut self.state
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
    pub fn rows(&self) -> u16 {
        self.io.rows() as u16
    }

    pub fn columns(&self) -> u16 {
        self.io.columns() as u16
    }
    
    pub fn output_stream(&mut self, stream: i16, table: Option<usize>) -> Result<(), RuntimeError> {
        if stream > 0 {
            self.io.enable_output_stream(stream as u8, table)
        } else if stream < 0 {
            self.io.disable_output_stream(&mut self.state, i16::abs(stream) as u8)
        } else {
            Err(RuntimeError::new(ErrorCode::System, format!("Output stream {} is not valid: [-4..4]", stream)))
        }
    }

    pub fn print(&mut self, text: &Vec<u16>) -> Result<(), RuntimeError> {
        self.io.print_vec(text)?;

        if self.state.is_input_interrupt() {
            self.input_interrupt_print = true;
        }

        Ok(())
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

    pub fn status_line(&mut self) -> Result<(), RuntimeError> {
        let status_type = header::flag1(&self.state, Flags1v3::StatusLineType as u8)?;
        let object = self.state.variable(16)? as usize;
        let mut left = text::from_vec(&self.state, &property::short_name(&self.state, object)?)?;

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
            format!("{} ", format!("{}:{} {}", hour % 12, minute, suffix))
                .as_bytes()
                .iter()
                .map(|x| *x as u16)
                .collect()
        };

        self.io.status_line(&mut left, &mut right)
        // let width = self.screen.columns() as usize;
        // let available_for_left = width - right.len() - 1;
        // if left.len() > available_for_left {
        //     left.truncate(available_for_left - 4);
        //     left.push('.' as u16);
        //     left.push('.' as u16);
        //     left.push('.' as u16);
        // }

        // let mut spaces = vec![0x20 as u16; width - left.len() - right.len() - 1];
        // let mut status_line = vec![0x20 as u16];
        // status_line.append(&mut left);
        // status_line.append(&mut spaces);
        // status_line.append(&mut right);
        // let mut style = CellStyle::new();
        // style.set(Style::Reverse as u8);

        // self.screen.print_at(&status_line, (1, 1), &style);
        // self.screen.reset_cursor();
        // Ok(())
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
    pub fn read_key(&mut self, timeout: u16) -> Result<InputEvent, RuntimeError> {
        let end = if timeout > 0 {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Error getting system time")
                .as_millis()
                + (timeout as u128)
        } else {
            0
        };

        let check_sound = if let Some(_) = self.sound_interrupt {
            true
        } else {
            false
        };

        loop {
            // If a sound interrupt is set and there is no sound playing,
            // return buffer and clear any pending input_interrupt
            if let Some(_) = self.sound_interrupt {
                info!(target: "app::input", "Sound interrupt pending");
                if let Some(sounds) = self.sounds.as_mut() {
                    info!(target: "app::input", "Sound playing? {}", sounds.is_playing());
                    if !sounds.is_playing() {
                        info!(target: "app::input", "Sound interrupt firing");
                        self.input_interrupt = None;
                        return Ok(InputEvent::from_interrupt(Interrupt::Sound));
                    }
                }
            }

            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Error getting system time")
                .as_millis();
            if end > 0 && now > end {
                return Ok(InputEvent::from_interrupt(Interrupt::ReadTimeout));
            }

            let key = self.io.read_key(end == 0 && !check_sound);

            if let Some(c) = key.zchar() {
                if c == 253 || c == 254 {
                    info!(target: "app::input", "Storing mouse coordinates {},{}", key.column().unwrap(), key.row().unwrap());
                    header::set_extension(
                        &mut self.state,
                        1,
                        key.column().expect("Mouse click with no column data"),
                    )?;
                    header::set_extension(
                        &mut self.state,
                        2,
                        key.row().expect("Mouse click with no row data"),
                    )?;
                }

                return Ok(key);
            }

            thread::sleep(Duration::from_millis(10));
        }
    }

    pub fn read_line(
        &mut self,
        text: &Vec<u16>,
        len: usize,
        terminators: &Vec<u16>,
        timeout: u16,
    ) -> Result<Vec<u16>, RuntimeError> {
        let mut input_buffer = text.clone();

        let end = if timeout > 0 {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Error getting system time")
                .as_millis()
                + (timeout as u128)
        } else {
            0
        };

        let check_sound = if let Some(i) = self.state.interrupt() {
            match &i.interrupt_type() {
                state::InterruptType::Sound => true,
                _ => false,
            }
        } else {
            false
        };

        info!(target: "app::input", "Sound interrupt {:?}", check_sound);

        loop {
            // If a sound interrupt is set and there is no sound playing,
            // return buffer and clear any pending input_interrupt
            if let Some(i) = self.state.interrupt() {
                info!(target: "app::frame", "Interrupt pending");
                match &i.interrupt_type() {
                    state::InterruptType::Sound => {
                        if let Some(sounds) = self.sounds.as_mut() {
                            info!(target: "app::frame", "Sound playing? {}", sounds.is_playing());
                            if !sounds.is_playing() {
                                info!(target: "app::frame", "Sound interrupt firing");
                                return Ok(input_buffer);
                            }
                        }
                    }
                    _ => {}
                }
                // if let Some(sounds) = self.sounds.as_mut() {
                //     info!(target: "app::input", "Sound playing? {}", sounds.is_playing());
                //     if !sounds.is_playing() {
                //         info!(target: "app::input", "Sound interrupt firing");
                //         self.input_interrupt = None;
                //         return Ok(input_buffer);
                //     }
                // }
            }

            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Error getting system time")
                .as_millis();
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
                            header::set_extension(
                                &mut self.state,
                                1,
                                e.column().expect("Mouse click with no column data"),
                            )?;
                            header::set_extension(
                                &mut self.state,
                                2,
                                e.row().expect("Mouse click with no row data"),
                            )?;
                        }

                        input_buffer.push(key);
                        if key == 0x0d {
                            self.print(&vec![key])?;
                        }
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
                None => thread::sleep(Duration::from_millis(10)),
            }
        }

        Ok(input_buffer)
    }

    // Save/restore
    pub fn prompt_and_create(&mut self, prompt: &str, suffix: &str) -> Result<File, RuntimeError> {
        self.print(&prompt.chars().map(|c| c as u16).collect())?;
        let n = files::first_available(&self.name, suffix)?;
        self.print(&n)?;

        let f = self.read_line(&n, 32, &vec!['\r' as u16], 0)?;
        let filename = String::from_utf16(&f).unwrap();
        match fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(filename.trim())
        {
            Ok(f) => Ok(f),
            Err(e) => Err(RuntimeError::new(ErrorCode::System, format!("{}", e))),
        }
        // let file = fs::OpenOptions::new()
        //     .create(true)
        //     .truncate(true)
        //     .write(true)
        //     .open(filename.trim())
        //     .unwrap();

        // Ok(file)
    }

    pub fn prompt_and_write(
        &mut self,
        prompt: &str,
        suffix: &str,
        data: &Vec<u8>,
    ) -> Result<(), RuntimeError> {
        let mut file = self.prompt_and_create(prompt, suffix)?;

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
        let n = files::last_existing(&self.name, suffix)?;
        self.print(&n)?;

        let f = self.read_line(&n, 32, &vec!['\r' as u16], 0)?;
        let filename = String::from_utf16(&f).unwrap();
        let mut data = Vec::new();
        match File::open(filename.trim()) {
            Ok(mut file) => match file.read_to_end(&mut data) {
                Ok(_) => Ok(data),
                Err(e) => Err(RuntimeError::new(ErrorCode::System, format!("{}", e))),
            },
            Err(e) => Err(RuntimeError::new(ErrorCode::System, format!("{}", e))),
        }
    }

    // pub fn prepare_save(&self, pc: usize) -> Result<Vec<u8>, RuntimeError> {
    //     let quetzal = save_restore::quetzal(self, pc)?;
    //     Ok(Vec::from(quetzal))
    // }

    // pub fn restore(&mut self, save_data: Vec<u8>) -> Result<Option<usize>, RuntimeError> {
    //     let quetzal = Quetzal::try_from(save_data)?;
    //     // &(*self) ... ick
    //     let ifhd = IFhd::try_from((&(*self),0))?;
    //     if &ifhd != quetzal.ifhd() {
    //         error!(target: "app::quetzal", "Save file is not valid for the running story: {} != {}", quetzal.ifhd(), ifhd);
    //         self.print(
    //             &"Save was not created using the running story file\r"
    //                 .chars()
    //                 .map(|c| c as u16)
    //                 .collect(),
    //         )?;
    //         Ok(None)
    //     } else {
    //         // Rebuild frame stack
    //         let fs: Vec<Frame> = Vec::from(quetzal.stks());
    //         self.frames = fs;

    //         // Rebuild dynamic memory, preserving the contents of FLAGS2
    //         let flags2 = self.read_word(0x10)?;
    //         if let Some(umem) = quetzal.umem() {
    //             for i in 0..umem.data().len() {
    //                 self.memory.write_byte(i, umem.data()[i])?;
    //             }
    //         } else if let Some(cmem) = quetzal.cmem() {
    //             let data = save_restore::decompress(cmem, self.dynamic());
    //             for i in 0..data.len() {
    //                 self.memory.write_byte(i, data[i])?;
    //             }
    //         }

    //         // Re-initialize header
    //         self.initialize()?;

    //         // Restore FLAGS2
    //         self.memory.write_word(0x10, flags2)?;

    //         // Reset output stream 3
    //         self.stream_3 = Vec::new();

    //         Ok(Some(quetzal.ifhd().pc() as usize))
    //     }
    // }

    // pub fn save_undo(&mut self, instruction: &Instruction) -> Result<(), RuntimeError> {
    //     // let q = Quetzal::from_state(&self, instruction.store().unwrap().address());
    //     if let Some(r) = instruction.store() {
    //         debug!(target: "app::quetzal", "Saving undo state");
    //         match save_restore::quetzal(self, r.address()) {
    //             Ok(quetzal) => {
    //                 self.undo_stack.push(quetzal);
    //                 self.undo_stack.truncate(10);
    //                 Ok(())
    //             }
    //             Err(e) => {
    //                 error!(target: "app::quetzal", "Error saving undo state: {}", e);
    //                 Err(RuntimeError::new(
    //                     ErrorCode::Save,
    //                     format!("Error saving undo state: {}", e),
    //                 ))
    //             }
    //         }
    //     } else {
    //         error!(target: "app::quetzal", "SAVE_UNDO should be a store instruction");
    //         Err(RuntimeError::new(
    //             ErrorCode::Save,
    //             "SAVE_UNDO should be a store instruction".to_string(),
    //         ))
    //     }
    // }

    // pub fn restore_undo(&mut self) -> Result<Option<usize>, RuntimeError> {
    //     if let Some(q) = self.undo_stack.pop() {
    //         debug!(target: "app::quetzal", "Restoting undo state");
    //         self.restore(Vec::from(q))
    //     } else {
    //         debug!(target: "app::quetzal", "No undo state to restore");
    //         Ok(None)
    //     }
    // }

    // pub fn restart(&mut self) -> Result<usize, RuntimeError> {
    //     let f1 = self.read_byte(0x10)? & 0x3;
    //     for i in 0..self.dynamic.len() {
    //         let b = self.dynamic[i];
    //         self.write_byte(i, b)?;
    //     }
    //     self.write_byte(0x10, self.read_byte(0x10)? | f1)?;
    //     self.frames.clear();
    //     self.initialize()?;

    //     Ok(self.current_frame()?.pc())
    // }

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
        // if self.output_streams & 0x5 == 0x1 {
        //     self.screen.new_line();
        //     if self.output_streams & 0x2 == 0x2 {
        //         self.transcript(&vec![0x0a as u16].to_vec())?;
        //     }
        // }

        // Ok(())
    }

    // pub fn flush_screen(&mut self) -> Result<(), RuntimeError> {
    //     self.screen.flush_buffer()
    // }

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
        let repeats = if self.version == 5 {
            Some(repeats)
        } else {
            None
        };

        if let Some(sounds) = self.sounds.as_mut() {
            if let Some(address) = routine {
                self.state.sound_interrupt(address);
            }
            if sounds.current_effect() == effect {
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
        if let Some(sounds) = self.sounds.as_mut() {
            if let Some(i) = self.state.interrupt() {
                match i.interrupt_type() {
                    state::InterruptType::Sound => self.state.clear_interrupt(),
                    _ => {}
                }
            }
            sounds.stop_sound()
        }

        Ok(())
    }

    // Run
    pub fn run(&mut self) -> Result<(), RuntimeError> {
        let mut n = 1;
        loop {
            log_mdc::insert("instruction_count", format!("{:8x}", n));
            let pc = self.state.current_frame()?.pc();
            let instruction = decoder::decode_instruction(self.state(), pc)?;
            let pc = processor::dispatch(self, &instruction)?;
            if pc == 0 {
                return Ok(());
            }

            if let Some(i) = self.state.interrupt() {
                match &i.interrupt_type() {
                    state::InterruptType::Sound => {
                        info!(target: "app::sound", "Pending sound interrupt");
                        if let Some(sounds) = self.sounds.as_mut() {
                            info!(target: "app::sound", "Check for sound: {}", sounds.is_playing());
                            if !sounds.is_playing() {
                                let pc = self.state_mut().call_sound_interrupt(pc)?;
                                self.state.set_pc(pc)?;
                            } else {
                                self.state.set_pc(pc)?;
                            }
                        }
                    }
                    _ => self.state.set_pc(pc)?
                }
            } else {
                self.state.set_pc(pc)?;
            }
            n = n + 1;
        }
    }
}
