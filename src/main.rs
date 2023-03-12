#![crate_name = "mxyzptlk"]
#[macro_use]
extern crate log;

use std::env;
use std::fs::File;
use std::io::prelude::*;

pub mod executor;
pub mod iff;
pub mod interpreter;

use executor::Executor;
use iff::blorb::Blorb;

fn main() {
    let args: Vec<String> = env::args().collect();

    let filename = &args[1];
    match File::open(filename) {
        Ok(mut f) => {
            let mut buffer = Vec::new();
            match f.read_to_end(&mut buffer) {
                Ok(_) => {
                    let name: Vec<&str> = filename.split(".").collect();
                    let mut e = Executor::from_vec(name[0].to_string(), buffer);
                    // blorb::rebuild_blorb(name[0].to_string());
                    let rs = format!("{}-new.blorb", name[0].to_string());
                    match File::open(rs) {
                        Ok(mut rf) => {
                            let mut rbuf = Vec::new();
                            match rf.read_to_end(&mut rbuf) {
                                Ok(_) => match Blorb::from_vec(rbuf) {
                                    Some(b) => e.state.resources(b),
                                    None => (),
                                },
                                Err(_) => (),
                            }
                        }
                        Err(_) => (),
                    };
                    e.run();
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
