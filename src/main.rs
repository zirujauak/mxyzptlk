#![crate_name = "mxyzplex"]

use std::io;
use std::io::prelude::*;
use std::fs::File;

pub mod object_tree;
pub mod text;

use object_tree::*;
use text::*;

fn main() -> io::Result<()> {
    let mut f = File::open("curses.z5")?;
    let version = 5;
    let mut buffer = Vec::new();
    f.read_to_end(&mut buffer)?;

    let o = 2;
    println!("Parent/sibling/child for object {}: {} / {} / {}", o, parent(&buffer, version, o), sibling(&buffer, version, o), child(&buffer, version, o));
    print!("Property 1 for object {}: [", o);
    for b in property(&buffer, version, o, 1) {
        print!(" ${:02x}", b)
    }
    println!(" ]");
    println!("Short name for object {}: {}", o, from_vec(&buffer, version, &short_name(&buffer, version, o)));
    Ok(())
}