#![crate_name = "pancurses"]
extern crate log;

use std::env;
use std::fs::File;
use std::io::{Read, Write};
use std::panic;
use std::process::{exit, ExitCode};

use screen::Screen;
use zm::blorb::Blorb;
use zm::config::Config;
use zm::error::{ErrorCode, RuntimeError};
use zm::sound::Manager;
use zm::zmachine::{InterpreterResponse, Interrupt, RequestType, ZMachine};

use crate::log::*;

mod files;
mod screen;

struct Runtime {
    zmachine: ZMachine,
    screen: Screen,
    sound: Manager,
    stream_2: Option<File>,
}

impl Runtime {
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
        let sound = initialize_sound_engine(&zmachine, config.volume_factor(), blorb)?;

        Ok(Runtime {
            zmachine,
            screen,
            sound,
            stream_2: None,
        })
    }

    fn transcript(&mut self, text: &[u16]) {
        match self.stream_2.as_mut() {
            Some(f) => {
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
                let result = self.zmachine.execute(response.as_ref())?;
                // Reset the interpreter response
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
                            response = InterpreterResponse::get_cursor(row as u16, column as u16);
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
                            let effective_width =
                                req.request().width() as usize + req.request().skip() as usize;

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
                                || !req.request().terminators().contains(input.last().unwrap())
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

                                response = InterpreterResponse::read_complete(input, terminator);
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
                            let data = self.screen.prompt_and_read(
                                "Restore from: ",
                                req.request().name(),
                                "ifzs",
                            )?;
                            response = InterpreterResponse::restore(data)
                        }
                        RequestType::Save => {
                            let r = self.screen.prompt_and_write(
                                "Save to: ",
                                req.request().name(),
                                "ifzs",
                                req.request().save_data(),
                                false,
                            );
                            response = InterpreterResponse::save(r.is_ok())
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
                                self.screen.split_window(req.request().split_lines() as u32);
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
        }
    }

    fn quit(&mut self) {
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
        exit(-1);
    }
}

fn initialize_sound_engine(
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

fn initialize_config() -> Config {
    if let Some(filename) = files::config_file("config.yml") {
        match File::open(&filename) {
            Ok(f) => match Config::try_from(f) {
                Ok(config) => config,
                Err(e) => {
                    info!(target: "app::trace", "Error parsing configuration from {}: {}", filename, e);
                    Config::default()
                }
            },
            Err(e) => {
                info!(target: "app::trace", "Error reading configuration from {}: {}", filename, e);
                Config::default()
            }
        }
    } else {
        Config::default()
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let filename = &args[1];
    // full_name includes any path info and will be used to look for Blorb resources
    // co-located with the game file
    let full_name = filename.split('.').collect::<Vec<&str>>()[0].to_string();
    let name = full_name
        .split('/')
        .collect::<Vec<&str>>()
        .last()
        .unwrap()
        .to_string();
    let config = initialize_config();

    if config.logging() {
        if let Some(filename) = files::config_file("log4rs.yml") {
            if log4rs::init_file(filename, Default::default()).is_ok() {
                log_mdc::insert("instruction_count", format!("{:8x}", 0));
            }

            info!(target: "app::instruction", "Start instruction log for '{}'", name);
            info!(target: "app::resource", "Start resource log for '{}'", name);
            info!(target: "app::screen", "Start screen log for '{}'", name);
            info!(target: "app::sound", "Start sound log for '{}'", name);
            info!(target: "app::state", "Start state log for '{}'", name);
            info!(target: "app::stream", "Start stream log for '{}'", name);
            info!(target: "app::state", "Configuration: {:?}", config);
        }
    }

    let prev = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        debug!("{}", &info);
        // Reset the terminal because curses may not exit cleanly
        if cfg!(target_os = "macos") {
            let _ = std::process::Command::new("reset").status();
            let _ = std::process::Command::new("tput").arg("rmcup").status();
        } else if cfg!(target_os = "linux") {
            let _ = std::process::Command::new("reset").status();
        }
        prev(info);
    }));

    let mut data = Vec::new();
    match File::open(filename) {
        Ok(mut f) => match f.read_to_end(&mut data) {
            Ok(_) => {}
            Err(e) => {
                error!(target: "app::trace", "Error reading {}: {}", filename, e);
                println!("Error reading {}", filename);
                exit(-1);
            }
        },
        Err(e) => {
            error!(target: "app::trace", "Error reading {}: {}", filename, e);
            println!("Error reading {}", filename);
            exit(-1);
        }
    }

    let blorb = if data[0..4] == [b'F', b'O', b'R', b'M'] {
        info!(target: "app::trace", "Reading Blorb");
        match Blorb::try_from(data.clone()) {
            Ok(blorb) => Some(blorb),
            Err(e) => {
                error!(target: "app::trace", "Error reading blorb {}: {}", filename, e);
                exit(-1);
            }
        }
    } else if let Some(filename) = files::find_existing(&full_name, &["blorb", "blb"]) {
        info!(target: "app::sound", "Resource file: {}", filename);
        match File::open(&filename) {
            Ok(mut f) => match Blorb::try_from(&mut f) {
                Ok(blorb) => Some(blorb),
                Err(e) => {
                    error!(target: "app::trace", "Error reading blorb {}: {}", filename, e);
                    None
                }
            },
            Err(e) => {
                error!(target: "app::trace", "Error opening blorb {}: {}", filename, e);
                None
            }
        }
    } else {
        None
    };

    let zcode = match &blorb {
        Some(b) => match b.exec() {
            Some(d) => d.clone(),
            None => {
                if data[0] == b'F' {
                    error!(target: "app::trace", "No Exec chunk in blorb {}", filename);
                    exit(-1);
                } else {
                    data
                }
            }
        },
        None => data,
    };

    match Runtime::new(zcode, &config, &name, blorb) {
        Err(r) => {
            error!("{}", r);
            ExitCode::FAILURE
        }
        Ok(mut runtime) => {
            trace!("Begining execution");
            if let Err(r) = runtime.run() {
                error!("{}", r);
            }

            ExitCode::SUCCESS
        }
    }
}
