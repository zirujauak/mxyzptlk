#![crate_name = "mxyzptlk"]
#[macro_use]
extern crate log;

use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::panic;
use std::path::Path;

pub mod config;
pub mod error;
pub mod iff;
pub mod state;

use crate::config::Config;
//use crate::iff::Chunk;
use crate::log::*;
use iff::blorb::Blorb;
use state::memory::*;
use state::sound::Sounds;
use state::State;

fn open_blorb(name: &str) -> Option<Sounds> {
    let filename = format!("{}.blorb", name);
    match Path::new(&filename).try_exists() {
        Ok(b) => {
            if b {
                match File::open(&filename) {
                    Ok(mut f) => {
                        let mut data = Vec::new();
                        match f.read_to_end(&mut data) {
                            Ok(_) => {
                                if let Ok(blorb) = Blorb::try_from(data) {
                                    Some(Sounds::from(blorb))
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
        },
        Err(e) => {
            error!(target: "app::blorb", "Error checking for {}: {}", &filename, e);
            None
        }
    }
}
fn main() {
    let args: Vec<String> = env::args().collect();

    let filename = &args[1];
    let name: Vec<&str> = filename.split(".").collect();
    let name = name[0].to_string();
    log4rs::init_file("log4rs.yml", Default::default()).unwrap();
    log_mdc::insert("instruction_count", format!("{:8x}", 0));
    info!(target: "app::blorb", "Start blorb log for '{}'", name);
    info!(target: "app::frame", "Start frame log for '{}'", name);
    info!(target: "app::input", "Start input log for '{}'", name);
    info!(target: "app::instruction", "Start instruction log for '{}'", name);
    info!(target: "app::memory", "Start memory log for '{}'", name);
    info!(target: "app::object", "Start object log for '{}'", name);
    info!(target: "app::quetzal", "Start quetzal log for '{}'", name);
    info!(target: "app::stack", "Start stack log for '{}'", name);
    info!(target: "app::trace", "Start trace log for '{}'", name);
    info!(target: "app::variable", "Start variable log for '{}'", name);

    let config_file = File::open("config.yml");
    let config = if let Ok(f) = config_file {
        if let Ok(c) = Config::from_file(f) {
            c
        } else {
            Config::default()
        }
    } else {
        Config::default()
    };

    let prev = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        trace!(target: "app::trace", "{}", &info);
        // Reset the terminal because curses may not exit cleanly
        let _ = std::process::Command::new("reset").status();
        prev(info);
    }));

    match File::open(filename) {
        Ok(mut f) => {
            let mut buffer = Vec::new();
            match f.read_to_end(&mut buffer) {
                Ok(_) => {
                    let memory = Memory::new(&buffer);
                    let blorb = open_blorb(&name);
                    let mut state =
                        State::new(memory, config, blorb, &name).expect("Error creating state");
                    if let Err(e) = state.initialize() {
                        panic!("{}", e);
                    }

                    trace!(target: "app::trace", "Begin execution");
                    if let Err(r) = state.run() {
                        let error: Vec<_> = format!("\r{}\rPress any key to exit", r)
                            .as_bytes()
                            .iter()
                            .map(|x| *x as u16)
                            .collect();
                        let _ = state.print(&error);
                        let _ = state.read_key(0);
                        let _ = state.quit();
                        panic!("{}", r)
                    }
                }
                Err(e) => {
                    panic!("Error reading file '{}': {}", filename, e);
                }
            }
        }
        Err(e) => {
            panic!("Error opening '{}': {}", filename, e);
        }
    }
}
