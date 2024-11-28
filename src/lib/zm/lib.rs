//! The Z-Machine as a library
#![crate_name = "zm"]

#[macro_use]
extern crate log;

pub mod blorb;
pub mod config;
pub mod error;
pub mod iff;
pub mod instruction;
pub mod object;
pub mod quetzal;
pub mod sound;
pub mod text;
pub mod zmachine;

#[cfg(test)]
pub mod test_util;
