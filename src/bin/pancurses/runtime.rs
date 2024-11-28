//! Interpeter runtime
use std::{collections::HashSet, fs::File, io::Write, process::exit};

use log::{debug, error, warn};
use zm::{
    blorb::Blorb,
    config::Config,
    error::{ErrorCode, RuntimeError},
    sound::Manager,
    zmachine::{ErrorHandling, InterpreterResponse, Interrupt, RequestType, ZMachine},
};

use crate::screen::Screen;

/// Initialize sound.
///
/// If a Blorb resource is provided, attempt to create a sound Manager.
///
/// # Arguments
/// * `zmachine` - Reference to the Z-Machine
/// * `volume_factor` - Volume normalization factor
/// * `blorb` - [Option] with a Blorb resource file
///
/// # Returns
/// If `blorb` is [Some], returns a [Result] with an initialized sound manager or a [RuntimeError].  
/// If `blorb` is none, returns a dummy sound manager.
fn initialize_sound(
    zmachine: &ZMachine,
    volume_factor: f32,
    blorb: Option<Blorb>,
) -> Result<Manager, RuntimeError> {
    if let Some(blorb) = blorb {
        if let Some(ifhd) = blorb.ifhd() {
            // TODO: Refactor this when adding Exec chunk support
            let release = zmachine.read_word(0x02).unwrap();
            let checksum = zmachine.read_word(0x1C).unwrap();
            let serial = [
                zmachine.read_byte(0x12).unwrap(),
                zmachine.read_byte(0x13).unwrap(),
                zmachine.read_byte(0x14).unwrap(),
                zmachine.read_byte(0x15).unwrap(),
                zmachine.read_byte(0x16).unwrap(),
                zmachine.read_byte(0x17).unwrap(),
            ]
            .to_vec();
            if release != ifhd.release_number()
                || checksum != ifhd.checksum()
                || &serial != ifhd.serial_number()
            {
                error!(target: "app::sound", "Resource file does not match the game");
                return Manager::none();
            }
        }
        Manager::new(volume_factor, blorb)
    } else {
        Manager::none()
    }
}

/// Runtime state
pub struct Runtime {
    /// The Z-Machine
    zmachine: ZMachine,
    /// Terminal I/O
    screen: Screen,
    /// Sound
    sound: Manager,
    /// Transcript file stream
    stream_2: Option<File>,
    /// Set of seen error codes
    errors: HashSet<ErrorCode>,
    /// Recoverable error handling strategy
    error_handling: ErrorHandling,
}

impl Runtime {
    /// Constructor
    ///
    /// # Arguments
    /// * `zcode` - ZCode program
    /// * `config` - Reference to configuration
    /// * `name` - Base filename
    /// * `blorb` - Optional Blorb resource file with sound resources
    ///
    /// # Returns
    /// [Result] with a new instance or a [RuntimeError]
    pub fn new(
        zcode: Vec<u8>,
        config: &Config,
        name: &str,
        blorb: Option<Blorb>,
    ) -> Result<Runtime, RuntimeError> {
        let screen = Screen::new(zcode[0], config)?;
        let zmachine = ZMachine::new(
            zcode,
            config,
            name,
            screen.rows() as u8,
            screen.columns() as u8,
        )?;
        let sound = initialize_sound(&zmachine, config.volume_factor(), blorb)?;

        Ok(Runtime {
            zmachine,
            screen,
            sound,
            stream_2: None,
            errors: HashSet::new(),
            error_handling: config.error_handling(),
        })
    }

    /// Write text to the transcript file
    ///
    /// Does nothing if the transcript file is not open.
    ///
    /// # Arguments
    /// * `test` - Array of text to write
    fn transcript(&mut self, text: &[u16]) {
        match self.stream_2.as_mut() {
            Some(f) => {
                // Translate u16 to u8, mapping carriage return (\r) to line feed (\n).
                let t: Vec<u8> = text
                    .iter()
                    .map(|x| if *x as u8 == b'\r' { b'\n' } else { *x as u8 })
                    .collect();
                if let Err(r) = f.write_all(&t) {
                    error!("Error writing to transcript: {}", r)
                }
            }
            None => {
                warn!("Transcript requested but no transcript file has been opened")
            }
        }
    }

    /// Run the ZCode program
    ///
    /// Runs a single instruction on the Z-Machine.  If an interpreter callback results, handles the callback
    /// and passes any return information to the Z-Machine, then proceeds to the next instruction.
    ///
    /// Between instructions, checks if an end-of-sound routine needs to be called.
    ///
    /// # Returns
    /// Empty [Result] when the program finishes or a [RuntimeError].  In most cases, the QUIT instruction
    /// will cause program termination before the `run` function returns.
    pub fn run(&mut self) -> Result<(), RuntimeError> {
        let mut n = 0;
        let mut response = None;
        loop {
            n += 1;
            if self.sound.routine() > 0 && !self.sound.is_playing() {
                debug!(target: "app::sound", "Sound interrupt ${:05x}", self.sound.routine());
                response = InterpreterResponse::sound_interrupt(self.sound.routine());
                self.sound.clear_routine();
            } else {
                log_mdc::insert("instruction_count", format!("{:8x}", n));
                // TODO: Recoverable error handling
                match self.zmachine.execute(response.as_ref()) {
                    Ok(result) => {
                        response = None;
                        if let Some(req) = result {
                            match req.request_type() {
                                RequestType::BufferMode => {
                                    self.screen.buffer_mode(req.request().buffer_mode());
                                }
                                RequestType::EraseLine => {
                                    self.screen.erase_line();
                                }
                                RequestType::EraseWindow => {
                                    self.screen
                                        .erase_window(req.request().window_erase() as i8)?;
                                }
                                RequestType::GetCursor => {
                                    let (row, column) = self.screen.cursor();
                                    response =
                                        InterpreterResponse::get_cursor(row as u16, column as u16);
                                }
                                RequestType::Message => {
                                    self.screen.print_str(req.request().message());
                                }
                                RequestType::NewLine => {
                                    self.screen.new_line();
                                    if req.request().transcript() {
                                        self.transcript(&[b'\n' as u16]);
                                    }
                                }
                                RequestType::OutputStream => {
                                    if req.request().stream() == 2 && self.stream_2.is_none() {
                                        let file = self.screen.prompt_and_create(
                                            "Transcript file name: ",
                                            req.request().name(),
                                            "txt",
                                            false,
                                        )?;
                                        self.stream_2 = Some(file);
                                    }
                                }
                                RequestType::Print => {
                                    self.screen.print(req.request().text());
                                    if req.request().transcript() {
                                        self.transcript(req.request().text());
                                    }
                                }
                                RequestType::PrintRet => {
                                    self.screen.print(req.request().text());
                                    self.screen.new_line();
                                    if req.request().transcript() {
                                        self.transcript(req.request().text());
                                        self.transcript(&[b'\n' as u16]);
                                    }
                                }
                                RequestType::PrintTable => {
                                    let origin = self.screen.cursor();
                                    let rows = self.screen.rows();
                                    let effective_width = req.request().width() as usize
                                        + req.request().skip() as usize;

                                    for i in 0..req.request().height() as usize {
                                        if origin.0 + i as u32 > rows {
                                            self.screen.new_line();
                                            self.screen.move_cursor(rows, origin.1);
                                        } else {
                                            self.screen.move_cursor(origin.0 + i as u32, origin.1);
                                        }
                                        let mut text = Vec::new();
                                        let offset = i * effective_width;
                                        for j in 0..req.request().width() as usize {
                                            text.push(req.request().table()[offset + j])
                                        }
                                        debug!(target: "app::screen", "PRINT_TABLE: '{}'", text.iter().map(|x| (*x as u8) as char).collect::<String>());
                                        self.screen.print(&text);
                                    }
                                }
                                RequestType::Quit => {
                                    self.quit();
                                    return Ok(());
                                }
                                RequestType::Read => {
                                    if self.zmachine.version() == 3 {
                                        let (left, right) = self.zmachine.status_line()?;
                                        self.screen.status_line(&left, &right)?;
                                    }
                                    let (input, terminator) = self.screen.read_line(
                                        req.request().input(),
                                        req.request().length() as usize,
                                        req.request().terminators(),
                                        req.request().timeout(),
                                        Some(&mut self.sound),
                                    )?;
                                    // If no input was returned, or the last character in the buffer is not a terminator,
                                    // then the read must have timed out.
                                    if input.is_empty()
                                        || !req
                                            .request()
                                            .terminators()
                                            .contains(input.last().unwrap())
                                    {
                                        if self.sound.routine() > 0 && !self.sound.is_playing() {
                                            debug!(target: "app::screen", "Sound playback finished, dispatching sound routine");
                                            response = InterpreterResponse::read_interrupted(
                                                input,
                                                Interrupt::Sound,
                                                self.sound.routine(),
                                            );
                                            self.sound.clear_routine();
                                            // zmachine.call_routine(sound.routine(), &Vec::new(), None, pc)?;
                                            // sound.clear_routine();
                                        } else {
                                            debug!(target: "app::screen", "Read timed out, dispatching read routine");
                                            response = InterpreterResponse::read_interrupted(
                                                input,
                                                Interrupt::ReadTimeout,
                                                0,
                                            );
                                        }
                                    } else {
                                        if req.request().transcript() {
                                            self.transcript(&input);
                                        }

                                        response =
                                            InterpreterResponse::read_complete(input, terminator);
                                    }
                                }
                                RequestType::ReadRedraw => {
                                    // Print the input
                                    self.screen.print(req.request().input());
                                }
                                RequestType::ReadChar => {
                                    let key = self.screen.read_key(req.request().timeout())?;
                                    if key.interrupt().is_some() {
                                        if self.sound.routine() > 0 && !self.sound.is_playing() {
                                            debug!(target: "app::screen", "Sound playback finished, dispatching sound routine");
                                            response = InterpreterResponse::read_char_interrupted(
                                                Interrupt::Sound,
                                            );
                                        } else {
                                            debug!(target: "app::screen", "Read character timed out, dispatching read routine");
                                            response = InterpreterResponse::read_char_interrupted(
                                                Interrupt::ReadTimeout,
                                            );
                                        }
                                    } else {
                                        response = InterpreterResponse::read_char_complete(key);
                                    }
                                }
                                RequestType::Restart => {
                                    self.screen.reset();
                                    self.sound.stop_sound();
                                }
                                RequestType::Restore => {
                                    // Prompt for filename and read data
                                    match self.screen.prompt_and_read(
                                        "Restore from: ",
                                        req.request().name(),
                                        "ifzs",
                                    ) {
                                        Ok(data) => response = InterpreterResponse::restore(data),
                                        Err(r) => {
                                            self.screen.print_str(r.message());
                                            self.screen.new_line();
                                        }
                                    }
                                }
                                RequestType::Save => {
                                    match self.screen.prompt_and_write(
                                        "Save to: ",
                                        req.request().name(),
                                        "ifzs",
                                        req.request().save_data(),
                                        false,
                                    ) {
                                        Ok(_) => {
                                            response = InterpreterResponse::save(true);
                                        }
                                        Err(r) => {
                                            self.screen.print_str(r.message());
                                            self.screen.new_line();
                                            response = InterpreterResponse::save(false);
                                        }
                                    }
                                }

                                RequestType::SetCursor => {
                                    self.screen.move_cursor(
                                        req.request().row() as u32,
                                        req.request().column() as u32,
                                    );
                                }
                                RequestType::SetColour => {
                                    self.screen.set_colors(
                                        req.request().foreground(),
                                        req.request().background(),
                                    )?;
                                }
                                RequestType::SetFont => {
                                    let old_font = self.screen.set_font(req.request().font() as u8);
                                    response = InterpreterResponse::set_font(old_font as u16);
                                }
                                RequestType::SetTextStyle => {
                                    self.screen.set_style(req.request().style() as u8)?;
                                }
                                RequestType::SetWindow => {
                                    self.screen
                                        .select_window(req.request().window_set() as u8)?;
                                }
                                RequestType::ShowStatus => {
                                    self.screen.status_line(
                                        req.request().status_left(),
                                        req.request().status_right(),
                                    )?;
                                }
                                RequestType::SoundEffect => match req.request().number() {
                                    1 | 2 => self.screen.beep(),
                                    _ => match req.request().effect() {
                                        1 => (),
                                        2 => {
                                            self.sound.play_sound(
                                                req.request().number(),
                                                req.request().volume(),
                                                Some(req.request().repeats()),
                                                req.request().routine(),
                                            )?;
                                        }
                                        3 | 4 => self.sound.stop_sound(),
                                        _ => (),
                                    },
                                },
                                RequestType::SplitWindow => {
                                    if req.request().split_lines() > 0 {
                                        self.screen
                                            .split_window(req.request().split_lines() as u32);
                                    } else {
                                        self.screen.unsplit_window();
                                    }
                                }
                                _ => {
                                    debug!("Interpreter directive: {:?}", req.request_type());
                                    return Err(RuntimeError::fatal(
                                        ErrorCode::UnimplementedInstruction,
                                        format!("{:?}", req.request_type()).to_string(),
                                    ));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        // If the error is fatal or error handling is abort
                        if !e.is_recoverable() || self.error_handling == ErrorHandling::Abort {
                            return Err(e);

                        // Error is not fatal

                        // If error handling is ignore, just continue to the next address
                        } else if self.error_handling == ErrorHandling::Ignore {
                            match e.next_address() {
                                Some(n) => {
                                    self.zmachine.set_next_pc(n.address())?;
                                }
                                _ => return Err(e),
                            }
                        // If error handling is warn always or the code hasn't been seen yet
                        } else if self.error_handling == ErrorHandling::ContinueWarnAlways
                            || !self.errors.contains(&e.code())
                        {
                            self.errors.insert(e.code());
                            if self.screen.error(
                                &format!("[Instruction #{}]: @ ${:05x}", n, self.zmachine.pc()?),
                                e.message(),
                                e.is_recoverable(),
                            ) {
                                match e.next_address() {
                                    Some(n) => {
                                        self.zmachine.set_next_pc(n.address())?;
                                    }
                                    _ => return Err(e),
                                }
                            } else {
                                // Print instruction details before returning an error
                                self.screen.print_str(&format!("\r{}", e));
                                self.screen.print_str(&format!(
                                    "\r[{}]: {:05x}\r",
                                    n,
                                    self.zmachine.pc()?
                                ));
                                return Err(e);
                            }
                        // Error handling is warn once and the code has been seen before
                        } else {
                            match e.next_address() {
                                Some(n) => {
                                    self.zmachine.set_next_pc(n.address())?;
                                }
                                _ => return Err(e),
                            }
                        }
                    }
                }
                // Reset the interpreter response
            }
        }
    }

    /// Quits the program.
    ///
    /// This function cleans up the terminal and attempts to clean up shell window on exit.  This works
    /// on Linux and Macos, but causes a SEGFAULT on Windows.  Room for improvement.
    pub fn quit(&mut self) {
        self.screen.quit();
        self.screen
            .print(&"Press any key".chars().map(|x| (x as u8) as u16).collect());
        self.screen.key(true);
        if cfg!(target_os = "macos") {
            let _ = std::process::Command::new("/usr/bin/reset").status();
            let _ = std::process::Command::new("/usr/bin/tput")
                .arg("rmcup")
                .status();
        } else if cfg!(target_os = "linux") {
            let _ = std::process::Command::new("/usr/bin/reset").status();
        }
        exit(0);
    }
}
