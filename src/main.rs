#![crate_name = "mxyzptlk"]
#[macro_use]
extern crate log;

use std::env;
use std::fs::File;
use std::io::Read;
use std::panic;
use std::process::exit;

pub mod blorb;
pub mod config;
pub mod error;
pub mod files;
pub mod instruction;
pub mod object;
pub mod quetzal;
pub mod sound;
pub mod text;
pub mod zmachine;

#[cfg(test)]
pub mod test_util;

use crate::config::Config;
use crate::log::*;
use blorb::Blorb;
use sound::Manager;
use zmachine::state::memory::Memory;
use zmachine::ZMachine;

fn initialize_sound_engine(
    memory: &Memory,
    volume_factor: f32,
    blorb: Option<Blorb>,
) -> Option<Manager> {
    if let Some(blorb) = blorb {
        if let Some(ifhd) = blorb.ifhd() {
            // TODO: Refactor this when adding Exec chunk support
            let release = memory.read_word(0x02).unwrap();
            let checksum = memory.read_word(0x1C).unwrap();
            let serial = [
                memory.read_byte(0x12).unwrap(),
                memory.read_byte(0x13).unwrap(),
                memory.read_byte(0x14).unwrap(),
                memory.read_byte(0x15).unwrap(),
                memory.read_byte(0x16).unwrap(),
                memory.read_byte(0x17).unwrap(),
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

    let memory = Memory::new(zcode);
    let sound_manager = initialize_sound_engine(&memory, config.volume_factor(), blorb);
    let mut zmachine =
        ZMachine::new(memory, config, sound_manager, &name).expect("Error creating state");

    trace!("Begining execution");

    // If execution ended due to an error, print the error and quit
    if let Err(r) = zmachine.run() {
        let _ = zmachine.print_str(format!("\r{}\r", r));
        let _ = zmachine.quit();
    }

    // Clean up the terminal
    if cfg!(target_os = "macos") {
        let _ = std::process::Command::new("/usr/bin/reset").status();
        let _ = std::process::Command::new("/usr/bin/tput")
            .arg("rmcup")
            .status();
    } else if cfg!(target_os = "linux") {
        let _ = std::process::Command::new("/usr/bin/reset").status();
    }
}
