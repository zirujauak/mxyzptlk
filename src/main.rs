#![crate_name = "mxyzptlk"]
#[macro_use] extern crate log;

use std::io;
use std::io::prelude::*;
use std::fs::File;

pub mod executor;
// pub mod object_tree;
// pub mod object;
// pub mod text;

use executor::Executor;
// use executor::instruction::Instruction;

fn main() -> io::Result<()> {
    let mut f = File::open("curses-r10.z3")?;
    let mut buffer = Vec::new();
    f.read_to_end(&mut buffer)?;

    executor::log::init();
    let mut e = Executor::from_vec(buffer);
    e.run();
    Ok(())
}
