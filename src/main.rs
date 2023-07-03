#![crate_name = "mxyzptlk"]
#[macro_use]
extern crate log;

use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::panic;

pub mod config;
pub mod error;
pub mod files;
pub mod iff;
pub mod instruction;
pub mod object;
pub mod sound;
pub mod text;
pub mod zmachine;

use crate::config::Config;
use crate::log::*;
use iff::blorb::Blorb;
use sound::Manager;
use zmachine::state::memory::Memory;
use zmachine::ZMachine;

fn initialize_sound_engine(name: &str) -> Option<Manager> {
    if let Some(filename) = files::find_existing(name, &["blorb", "blb"]) {
        match File::open(&filename) {
            Ok(mut f) => {
                let mut data = Vec::new();
                match f.read_to_end(&mut data) {
                    Ok(_) => {
                        if let Ok(blorb) = Blorb::try_from(data) {
                            info!(target: "app::sound", "{}", blorb);
                            if let Ok(manager) = Manager::new(blorb) {
                                Some(manager)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                    Err(e) => {
                        error!(target: "app::blorb", "Error reading {}: {}", &filename, e);
                        None
                    }
                }
            }
            Err(e) => {
                error!(target: "app::blorb", "Error opening {}: {}", &filename, e);
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
    let name: Vec<&str> = filename.split('.').collect();
    let name = name[0].to_string();
    let config = initialize_config();

    if config.logging() {
        if let Some(filename) = files::config_file("log4rs.yml") {
            if log4rs::init_file(filename, Default::default()).is_ok() {
                log_mdc::insert("instruction_count", format!("{:8x}", 0));
            }

            error!(target: "app::blorb", "Start blorb log for '{}'", name);
            error!(target: "app::frame", "Start frame log for '{}'", name);
            error!(target: "app::input", "Start input log for '{}'", name);
            error!(target: "app::instruction", "Start instruction log for '{}'", name);
            error!(target: "app::memory", "Start memory log for '{}'", name);
            error!(target: "app::object", "Start object log for '{}'", name);
            error!(target: "app::quetzal", "Start quetzal log for '{}'", name);
            error!(target: "app::sound", "Start sound log for '{}'", name);
            error!(target: "app::stack", "Start stack log for '{}'", name);
            error!(target: "app::trace", "Start trace log for '{}'", name);
            error!(target: "app::variable", "Start variable log for '{}'", name);
            info!(target: "app::trace", "{:?}", config);
        }
    }

    let prev = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        trace!(target: "app::trace", "{}", &info);
        // Reset the terminal because curses may not exit cleanly
        let _ = std::process::Command::new("reset").status();
        prev(info);
    }));

    match File::open(filename) {
        Ok(mut f) => match Memory::try_from(&mut f) {
            Ok(memory) => {
                let sound_manager = initialize_sound_engine(&name);
                let mut zmachine = ZMachine::new(memory, config, sound_manager, &name)
                    .expect("Error creating state");

                trace!(target: "app::trace", "Begin execution");
                if let Err(r) = zmachine.run() {
                    let error: Vec<_> = format!("\r{}\rPress any key to exit", r)
                        .as_bytes()
                        .iter()
                        .map(|x| *x as u16)
                        .collect();
                    let _ = zmachine.print(&error);
                    let _ = zmachine.read_key(0);
                    let _ = zmachine.quit();
                    panic!("{}", r)
                }
            }
            Err(e) => {
                panic!("Error reading file '{}': {}", filename, e);
            }
        },
        Err(e) => {
            panic!("Error opening '{}': {}", filename, e);
        }
    }
}
