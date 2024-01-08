#![crate_name = "pancurses"]
extern crate log;

use std::env;
use std::fs::File;
use std::io::Read;
use std::panic;
use std::process::exit;

use zm::blorb::Blorb;
use zm::config::Config;
use zm::error::{ErrorCode, RuntimeError};
use zm::files;
use zm::instruction::decoder;
use zm::instruction::processor::{processor_ext, processor_var};
use zm::sound::Manager;
use zm::types::Directive;
use zm::zmachine::ZMachine;

use crate::log::*;
use crate::screen::Screen;

mod screen;

fn initialize_sound_engine(
    zmachine: &ZMachine,
    volume_factor: f32,
    blorb: Option<Blorb>,
) -> Option<Manager> {
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
                return None;
            }
        }
        match Manager::new(volume_factor, blorb) {
            Ok(m) => Some(m),
            Err(e) => {
                info!(target: "app::sound", "Error initializing sound manager: {}", e);
                None
            }
        }
    } else {
        None
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
    let mut screen = match zcode[0] {
        3 => Screen::new_v3(&config),
        4 => Screen::new_v4(&config),
        5..=8 => Screen::new_v5(&config),
        _ => Err(RuntimeError::fatal(
            ErrorCode::UnsupportedVersion,
            format!("Version {} is not supported", zcode[0]).to_string(),
        )),
    }
    .expect("Error creating screen");

    let mut zmachine = ZMachine::new(
        zcode,
        config,
        &name,
        screen.rows() as u8,
        screen.columns() as u8,
    )
    .expect("Error creating zmachine");
    // let sound_manager = initialize_sound_engine(&zmac        hine, config.volume_factor(), blorb);

    trace!("Begining execution");

    let mut n = 1;
    loop {
        log_mdc::insert("instruction_count", format!("{:8x}", n));
        let instruction = decoder::decode_instruction(
            &zmachine,
            zmachine.pc().expect("Error fetching program counter"),
        )
        .expect("Error decoding instruction");
        match zmachine.execute(&instruction) {
            Ok(r) => match r.directive() {
                Some(d) => {
                    let request = r.request();
                    match d {
                        Directive::BufferMode => {
                            screen.buffer_mode(request.mode());
                            zmachine
                                .set_pc(r.next_instruction())
                                .expect("Error updatng program counter")
                        }
                        Directive::EraseWindow => {
                            screen
                                .erase_window(request.window_erase() as i8)
                                .expect("Error erasing window");
                            zmachine
                                .set_pc(r.next_instruction())
                                .expect("Error updatng program counter")
                        }
                        Directive::GetCursor => {
                            let (row, column) = screen.cursor();
                            match processor_var::get_cursor_post(
                                &mut zmachine,
                                &instruction,
                                row as u16,
                                column as u16,
                            ) {
                                Ok(r) => {
                                    zmachine
                                        .set_pc(r.next_instruction())
                                        .expect("Error updatng program counter");
                                }
                                Err(r) => {
                                    error!("{}", r);
                                    quit(&mut screen)
                                }
                            }
                        }
                        Directive::NewLine => {
                            screen.new_line();
                            zmachine
                                .set_pc(r.next_instruction())
                                .expect("Error updatng program counter")
                        }
                        Directive::Print => {
                            if zmachine.is_stream_enabled(1) && !zmachine.is_stream_enabled(3) {
                                if zmachine.is_input_interrupt().expect("Error checking input interrupt") {
                                    zmachine.set_redraw_input();
                                }
                                screen.print(request.text());
                            }
                            zmachine
                                .set_pc(r.next_instruction())
                                .expect("Error updatng program counter")
                        }
                        Directive::PrintRet => {
                            if zmachine.is_stream_enabled(1) && !zmachine.is_stream_enabled(3) {
                                if zmachine.is_input_interrupt().expect("Error checking input interrupt") {
                                    zmachine.set_redraw_input();
                                }
                                screen.print(request.text());
                                screen.new_line();
                            }
                            zmachine
                                .set_pc(r.next_instruction())
                                .expect("Error updatng program counter")
                        }
                        Directive::Quit => quit(&mut screen),
                        Directive::Read => {
                            if zmachine.version() == 3 {
                                let (mut left, mut right) =
                                    zmachine.status_line().expect("Error preparing status line");
                                screen
                                    .status_line(&mut left, &mut right)
                                    .expect("Error printing status line");
                            }
                            let input = screen
                                .read_line(
                                    request.preload(),
                                    request.length() as usize,
                                    request.terminators(),
                                    request.timeout(),
                                )
                                .expect("Error reading input");
                            // If no input was returned, or the last character in the buffer is not a terminator,
                            // then the read must have timed out.
                            if input.is_empty() || !request.terminators().contains(input.last().unwrap()) {
                                match processor_var::read_interrupted(&mut zmachine, &instruction, &input) {
                                    Ok(r) => {
                                        zmachine
                                            .set_pc(r.next_instruction())
                                            .expect("Error updatng program counter");
                                    }, 
                                    Err(r) => {
                                        error!("{}", r);
                                        quit(&mut screen)
                                    }
                                }
                            } else {
                                match processor_var::read_post(&mut zmachine, &instruction, input) {
                                    Ok(r) => {
                                        zmachine
                                            .set_pc(r.next_instruction())
                                            .expect("Error updatng program counter");
                                    }
                                    Err(r) => {
                                        error!("{}", r);
                                        quit(&mut screen)
                                    }
                                }
                            }
                        }
                        Directive::ReadChar => {
                            let key = screen
                                .read_key(request.timeout())
                                .expect("Error reading key");
                            match processor_var::read_char_post(&mut zmachine, &instruction, key) {
                                Ok(r) => {
                                    zmachine
                                        .set_pc(r.next_instruction())
                                        .expect("Error updatng program counter");
                                }
                                Err(r) => {
                                    error!("{}", r);
                                    quit(&mut screen)
                                }
                            }
                        }
                        Directive::ReadInterruptReturn => {
                            // Terminate input immedicately
                            if request.read_int_result() == 1 {
                                let instruction = decoder::decode_instruction(&zmachine, request.read_instruction()).expect("Error decoding instruction");
                                match processor_var::read_abort(&mut zmachine, &instruction) {
                                    Ok(r) => {
                                        zmachine.set_pc(r.next_instruction()).expect("Error updating program_counter")
                                    },
                                    Err(r) => {
                                        error!("{}", r);
                                        quit(&mut screen)
                                    }
                                }
                            } else if request.redraw_input() {
                                let instruction = decoder::decode_instruction(&zmachine, r.next_instruction()).expect("Error docoding instruction");
                                match processor_var::read_pre(&mut zmachine, &instruction) {
                                    Ok(r) => {
                                        let request = r.request();
                                        screen.print(&request.preload().to_vec());
                                        zmachine.set_pc(instruction.address()).expect("Error updating program_counter")
                                        
                                    }, 
                                    Err(r) => {
                                        error!("{}", r);
                                        quit(&mut screen)
                                    }
                                }
                            }
                        }
                        Directive::SetCursor => {
                            screen.move_cursor(request.row() as u32, request.column() as u32);
                            zmachine
                                .set_pc(r.next_instruction())
                                .expect("Error updatng program counter")
                        }
                        Directive::SetColour => {
                            screen
                                .set_colors(request.foreground(), request.background())
                                .expect("Error setting colours");
                            zmachine
                                .set_pc(r.next_instruction())
                                .expect("Error updatng program counter")
                        }
                        Directive::SetFont => {
                            let old_font = screen.set_font(request.font() as u8);
                            processor_ext::set_font_post(&mut zmachine, &instruction, old_font)
                                .expect("Error setting font");
                            zmachine
                                .set_pc(r.next_instruction())
                                .expect("Error updatng program counter")
                        }
                        Directive::SetTextStyle => {
                            screen
                                .set_style(request.style() as u8)
                                .expect("Error setting text tyle");
                            zmachine
                                .set_pc(r.next_instruction())
                                .expect("Error updatng program counter")
                        }
                        Directive::SetWindow => {
                            screen
                                .select_window(request.window_set() as u8)
                                .expect("Error selecting window");
                            zmachine
                                .set_pc(r.next_instruction())
                                .expect("Error updatng program counter")
                        }
                        Directive::SplitWindow => {
                            screen.split_window(request.split() as u32);
                            zmachine
                                .set_pc(r.next_instruction())
                                .expect("Error updatng program counter")
                        }
                        _ => {
                            debug!("Interpreter directive: {:?}", r.directive().unwrap());
                            quit(&mut screen)
                        }
                    }
                }
                None => zmachine
                    .set_pc(r.next_instruction())
                    .expect("Error updating program counter"),
            },
            Err(r) => {
                error!("{}", r);
                quit(&mut screen)
            }
        }

        n += 1;
    }
}
