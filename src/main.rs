#![crate_name = "mxyzptlk"]

use std::io;
use std::io::prelude::*;
use std::fs::File;

pub mod object;
pub mod object_tree;
pub mod text;
pub mod instruction;

use object_tree::*;
use instruction::*;

fn word_value(v: &Vec<u8>, a: usize) -> u16 {
    let hb: u16 = (((v[a] as u16) << 8) as u16 & 0xFF00) as u16;
    let lb: u16 = (v[a + 1] & 0xFF) as u16;
    hb + lb
}

fn main() -> io::Result<()> {
    let mut f = File::open("curses.z5")?;
    let mut buffer = Vec::new();
    f.read_to_end(&mut buffer)?;
    let version = buffer[0];

    let mut pc = word_value(&buffer, 6) as usize;
    pc = 0xa325;
    for i in 0..5 {
        let inst = decode_instruction(&buffer, version, pc);
        pc = inst.next_pc;
        println!("{}", inst);
    }
    Ok(())
}
