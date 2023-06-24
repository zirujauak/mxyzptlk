use std::fmt;

use crate::state::{header::{self, HeaderField}, State};

use super::super::*;

#[derive(Debug)]
pub struct IFhd {
    pub release_number: u16,
    pub serial_number: Vec<u8>,
    pub checksum: u16,
    pub pc: u32,
}

impl fmt::Display for IFhd {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "IFhd:")?;
        writeln!(f, "\tRelease: {:04x}", self.release_number)?;
        write!(f, "\tSerial: ")?;
        for i in 0..6 {
            write!(f, "{}", self.serial_number[i as usize] as char)?;
        }
        writeln!(f, "")?;
        writeln!(f, "\tChecksum: {:04x}", self.checksum)?;
        write!(f, "\tPC: ${:06x}", self.pc)
    }
}

impl IFhd {
    pub fn from_state(state: &State, address: usize) -> IFhd {
        info!(target: "app::quetzal", "Building IFhd chunk from state");

        let release_number = header::field_word(state.memory(), HeaderField::Release)
            .expect("Error reading from header");
        let mut serial_number = Vec::new();
        for i in 0..6 {
            serial_number.push(state.read_byte(HeaderField::Serial as usize + i).expect("Error reading from header"));
        }
        let checksum = header::field_word(state.memory(), HeaderField::Checksum).expect("Error reading from header");
        let pc = address as u32 & 0xFFFFFF;

        IFhd {
            release_number,
            serial_number,
            checksum,
            pc,
        }
    }

    pub fn from_chunk(chunk: Chunk) -> IFhd {
        let release_number = vec_to_u32(&chunk.data, 0, 2) as u16;
        let serial_number = chunk.data[2..8].to_vec();
        let checksum = vec_to_u32(&chunk.data, 8, 2) as u16;
        let pc = vec_to_u32(&chunk.data, 10, 3);

        IFhd {
            release_number,
            serial_number,
            checksum,
            pc,
        }
    }

    pub fn from_vec(chunk: Vec<u8>) -> IFhd {
        let release_number = vec_as_usize(chunk[0..2].to_vec(), 2) as u16;
        let serial_number = chunk[2..8].to_vec();
        let checksum = vec_as_usize(chunk[8..10].to_vec(), 2) as u16;
        let pc = vec_as_usize(chunk[10..13].to_vec(), 3) as u32;

        IFhd {
            release_number,
            serial_number,
            checksum,
            pc,
        }
    }

    pub fn to_chunk(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.append(&mut usize_as_vec(self.release_number as usize, 2));
        data.append(&mut self.serial_number.clone());
        data.append(&mut usize_as_vec(self.checksum as usize, 2));
        data.append(&mut usize_as_vec(self.pc as usize, 3));

        chunk("IFhd", &mut data)
    }
}
