#![crate_name = "pancurses"]
extern crate log;

use std::env;
use std::fs::File;
use std::io::Read;
use std::panic;
use std::process::{exit, ExitCode};

use zm::blorb::Blorb;
use zm::config::Config;

use crate::log::*;
use crate::runtime::Runtime;

mod files;
mod runtime;
mod screen;

/// Initalize configuration.
///
/// If `config.yml` exists in the current working directory in in ${HOME}/.mmxyzpltk, it will
/// be used.  Otherwise, the defaults from [Config] are used.
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

    // Initialize logging
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

    // Add a panic handler to try and reset the terminal before exiting
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

    // Read the ZCode (or Blorb) argument
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

    // If the argument was a Blorb file, process it.  Otherwise, look for a Blorb file
    // the the same basename as the argument and a `.blorb` or `blb` extension.
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

    // Get the ZCode from the Blorb Exec chunk, if present, else from the argument file
    let zcode = match &blorb {
        Some(b) => match b.exec() {
            Some(d) => d.clone(),
            None => {
                // If the Blorb was not the argument, then `data` should start with the ZCode
                // version.  If the Blorb _was_ the argument, then it will start with 'F'
                if data[0] == b'F' {
                    // The argument was a Blorb with no Exec chunk
                    error!(target: "app::trace", "No Exec chunk in blorb {}", filename);
                    exit(-1);
                } else {
                    data
                }
            }
        },
        None => data,
    };

    // Initialize the runtime
    match Runtime::new(zcode, &config, &name, blorb) {
        Err(r) => {
            error!("{}", r);
            ExitCode::FAILURE
        }
        Ok(mut runtime) => {
            trace!("Begining execution");
            // Start execution.  `run` should only return with an error
            if let Err(r) = runtime.run() {
                // Log the error and quit
                error!("{}", r);
                runtime.quit();
            }

            ExitCode::SUCCESS
        }
    }
}
