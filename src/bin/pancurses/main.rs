#![crate_name = "pancurses"]
extern crate log;

use std::env;
use std::fs::File;
use std::io::Read;
use std::panic;
use std::process::exit;

use screen::Screen;
use zm::blorb::Blorb;
use zm::config::Config;
use zm::error::{ErrorCode, RuntimeError};
use zm::sound::Manager;
use zm::zmachine::{InterpreterResponse, Interrupt, RequestType, ZMachine};

use crate::log::*;

mod files;
mod screen;

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

fn quit(screen: &mut Screen) {
    screen.quit();
    screen.print(&"Press any key".chars().map(|x| (x as u8) as u16).collect());
    screen.key(true);
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

fn run(
    zmachine: &mut ZMachine,
    screen: &mut Screen,
    sound: &mut Manager,
) -> Result<(), RuntimeError> {
    let mut n = 0;
    let mut response = None;
    loop {
        n += 1;
        if sound.routine() > 0 && !sound.is_playing() {
            debug!(target: "app::sound", "Sound interrupt ${:05x}", sound.routine());
            response = InterpreterResponse::sound_interrupt(sound.routine());
            sound.clear_routine();
        } else {
            log_mdc::insert("instruction_count", format!("{:8x}", n));
            // TODO: Recoverable error handling
            let result = zmachine.execute(response.as_ref())?;
            // Reset the interpreter response
            response = None;
            if let Some(req) = result {
                match req.request_type() {
                    RequestType::BufferMode => {
                        screen.buffer_mode(req.request().buffer_mode());
                    }
                    RequestType::EraseLine => {
                        screen.erase_line();
                    }
                    RequestType::EraseWindow => {
                        screen.erase_window(req.request().window_erase() as i8)?;
                    }
                    RequestType::GetCursor => {
                        let (row, column) = screen.cursor();
                        response = InterpreterResponse::get_cursor(row as u16, column as u16);
                    }
                    RequestType::Message => {
                        screen.print_str(req.request().message());
                    }
                    RequestType::NewLine => {
                        screen.new_line();
                    }
                    RequestType::OutputStream => {
                        // TBD: Remove this RequestType?
                    }
                    RequestType::Print => {
                        screen.print(req.request().text());
                    }
                    RequestType::PrintRet => {
                        screen.print(req.request().text());
                        screen.new_line();
                    }
                    RequestType::PrintTable => {
                        let origin = screen.cursor();
                        let rows = screen.rows();
                        let effective_width =
                            req.request().width() as usize + req.request().skip() as usize;

                        for i in 0..req.request().height() as usize {
                            if origin.0 + i as u32 > rows {
                                screen.new_line();
                                screen.move_cursor(rows, origin.1);
                            } else {
                                screen.move_cursor(origin.0 + i as u32, origin.1);
                            }
                            let mut text = Vec::new();
                            let offset = i * effective_width;
                            for j in 0..req.request().width() as usize {
                                text.push(req.request().table()[offset + j])
                            }
                            debug!(target: "app::screen", "PRINT_TABLE: '{}'", text.iter().map(|x| (*x as u8) as char).collect::<String>());
                            screen.print(&text);
                        }
                    }
                    RequestType::Quit => {
                        quit(screen);
                        return Ok(());
                    }
                    RequestType::Read => {
                        if zmachine.version() == 3 {
                            let (left, right) = zmachine.status_line()?;
                            screen.status_line(&left, &right)?;
                        }
                        let (input, terminator) = screen.read_line(
                            req.request().input(),
                            req.request().length() as usize,
                            req.request().terminators(),
                            req.request().timeout(),
                            Some(sound),
                        )?;
                        // If no input was returned, or the last character in the buffer is not a terminator,
                        // then the read must have timed out.
                        if input.is_empty()
                            || !req.request().terminators().contains(input.last().unwrap())
                        {
                            if sound.routine() > 0 && !sound.is_playing() {
                                debug!(target: "app::screen", "Sound playback finished, dispatching sound routine");
                                response = InterpreterResponse::read_interrupted(
                                    input,
                                    Interrupt::Sound,
                                    sound.routine(),
                                );
                                sound.clear_routine();
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
                            response = InterpreterResponse::read_complete(input, terminator);
                        }
                    }
                    RequestType::ReadRedraw => {
                        // Print the input
                        screen.print(req.request().input());
                    }
                    RequestType::ReadChar => {
                        let key = screen.read_key(req.request().timeout())?;
                        if key.interrupt().is_some() {
                            if sound.routine() > 0 && !sound.is_playing() {
                                debug!(target: "app::screen", "Sound playback finished, dispatching sound routine");
                                response =
                                    InterpreterResponse::read_char_interrupted(Interrupt::Sound);
                                // zmachine.call_routine(sound.routine(), &Vec::new(), None, pc)?;
                                // sound.clear_routine();
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
                        screen.reset();
                        sound.stop_sound();
                    }
                    RequestType::Restore => {
                        // Prompt for filename and read data
                        let data = screen.prompt_and_read(
                            "Restore from: ",
                            req.request().name(),
                            "ifzs",
                        )?;
                        response = InterpreterResponse::restore(data)
                    }
                    RequestType::Save => {
                        let r = screen.prompt_and_write(
                            "Save to: ",
                            req.request().name(),
                            "ifzs",
                            req.request().save_data(),
                            false,
                        );
                        response = InterpreterResponse::save(r.is_ok())
                    }

                    RequestType::SetCursor => {
                        screen
                            .move_cursor(req.request().row() as u32, req.request().column() as u32);
                    }
                    RequestType::SetColour => {
                        screen
                            .set_colors(req.request().foreground(), req.request().background())?;
                    }
                    RequestType::SetFont => {
                        let old_font = screen.set_font(req.request().font() as u8);
                        response = InterpreterResponse::set_font(old_font as u16);
                    }
                    RequestType::SetTextStyle => {
                        screen.set_style(req.request().style() as u8)?;
                    }
                    RequestType::SetWindow => {
                        screen.select_window(req.request().window_set() as u8)?;
                    }
                    RequestType::ShowStatus => {
                        screen.status_line(
                            req.request().status_left(),
                            req.request().status_right(),
                        )?;
                    }
                    RequestType::SoundEffect => match req.request().number() {
                        1 | 2 => screen.beep(),
                        _ => match req.request().effect() {
                            1 => (),
                            2 => {
                                sound.play_sound(
                                    req.request().number(),
                                    req.request().volume(),
                                    Some(req.request().repeats()),
                                    req.request().routine(),
                                )?;
                            }
                            3 | 4 => sound.stop_sound(),
                            _ => (),
                        },
                    },
                    RequestType::SplitWindow => {
                        if req.request().split_lines() > 0 {
                            screen.split_window(req.request().split_lines() as u32);
                        } else {
                            screen.unsplit_window();
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

fn main() {
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

    // let memory = Memory::new(zcode);
    let mut screen = Screen::new(zcode[0], &config).expect("Error creating screen");
    let mut zmachine = ZMachine::new(
        zcode,
        &config,
        &name,
        screen.rows() as u8,
        screen.columns() as u8,
    )
    .expect("Error creating zmachine");
    let mut sound = initialize_sound_engine(&zmachine, config.volume_factor(), blorb)
        .expect("Error initializing sound engine");

    trace!("Begining execution");
    if let Err(r) = run(&mut zmachine, &mut screen, &mut sound) {
        error!("{}", r);
        quit(&mut screen);
    }
}
