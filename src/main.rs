#![crate_name = "mxyzplex"]

use std::io;
use std::io::prelude::*;
use std::fs::File;

pub mod object;
pub mod object_tree;
pub mod text;

use object_tree::*;

fn main() -> io::Result<()> {
    let mut f = File::open("curses.z5")?;
    let mut buffer = Vec::new();
    f.read_to_end(&mut buffer)?;
    let version = buffer[0];

    print_object_tree(&buffer, version);
    Ok(())
}