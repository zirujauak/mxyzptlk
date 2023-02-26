#![crate_name = "mxyzptlk"]
#[macro_use] extern crate log;

use std::{io, env};
use std::io::prelude::*;
use std::fs::File;

pub mod executor;
pub mod interpreter;
pub mod quetzal;

// pub mod object_tree;
// pub mod object;
// pub mod text;

use executor::Executor;
// use executor::instruction::Instruction;

fn main() -> io::Result<()> {
    let args:Vec<String> = env::args().collect();
     
    let filename = &args[1];
    let mut f = File::open(filename)?;
    let mut buffer = Vec::new();
    f.read_to_end(&mut buffer)?;

    executor::log::init();
    let name:Vec<&str> = filename.split(".").collect();
    let mut e = Executor::from_vec(name[0].to_string(), buffer);
    e.run();
    Ok(())
}
