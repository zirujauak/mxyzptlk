#![crate_name = "mxyzptlk"]
#[macro_use]
extern crate log;

use std::fs::File;
use std::io::prelude::*;
use std::{env};

pub mod executor;
pub mod interpreter;
pub mod quetzal;

use executor::Executor;

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
                    e.run();                
                },
                Err(e) => {
                    panic!("Error reading file '{}': {}", filename, e);
                }
            }  
        },
        Err(e) => {
            panic!("Error opening '{}': {}", filename, e);
        }
    }
}
