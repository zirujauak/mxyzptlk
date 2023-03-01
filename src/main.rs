#![crate_name = "mxyzptlk"]
#[macro_use]
extern crate log;

use std::fs::File;
use std::io::prelude::*;
use std::{env, io};

pub mod executor;
pub mod interpreter;
pub mod quetzal;

use executor::Executor;

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    let filename = &args[1];
    let mut f = File::open(filename)?;
    let mut buffer = Vec::new();
    f.read_to_end(&mut buffer)?;

    executor::log::init();
    let name: Vec<&str> = filename.split(".").collect();
    let mut e = Executor::from_vec(name[0].to_string(), buffer);
    e.run();
    Ok(())
}
