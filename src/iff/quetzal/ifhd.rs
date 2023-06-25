use std::fmt;

use super::super::*;

pub struct IFhd {
    release_number: u16,
    serial_number: Vec<u8>,
    checksum: u16,
    pc: u32,
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

impl From<Chunk> for IFhd {
    fn from(value: Chunk) -> IFhd {
        let release_number = vec_to_u32(&value.data(), 0, 2) as u16;
        let serial_number = value.data()[2..8].to_vec();
        let checksum = vec_to_u32(&value.data(), 8, 2) as u16;
        let pc = vec_to_u32(&value.data(), 10, 3);

        IFhd {
            release_number,
            serial_number,
            checksum,
            pc,
        }
    }
}

impl From<IFhd> for Chunk {
    fn from(value: IFhd) -> Chunk {
        let mut data = Vec::new();
        data.append(&mut usize_as_vec(value.release_number() as usize, 2));
        data.append(&mut value.serial_number().clone());
        data.append(&mut usize_as_vec(value.checksum() as usize, 2));
        data.append(&mut usize_as_vec(value.pc() as usize, 3));

        Chunk::new(0, None, "IFhd".to_string(), &data)
    }
}

impl From<&IFhd> for Vec<u8> {
    fn from(value: &IFhd) -> Vec<u8> {
        let mut data = Vec::new();
        data.append(&mut usize_as_vec(value.release_number() as usize, 2));
        data.append(&mut value.serial_number().clone());
        data.append(&mut usize_as_vec(value.checksum() as usize, 2));
        data.append(&mut usize_as_vec(value.pc() as usize, 3));

        chunk("IFhd", &mut data)
    }
}

impl PartialEq for IFhd {
    fn eq(&self, other: &Self) -> bool {
        self.release_number == other.release_number
            && self.serial_number == other.serial_number
            && self.checksum == other.checksum
    }
}

impl IFhd {
    pub fn new(release_number: u16, serial_number: &Vec<u8>, checksum: u16, pc: u32) -> IFhd {
        IFhd {
            release_number,
            serial_number: serial_number.clone(),
            checksum,
            pc,
        }
    }

    pub fn release_number(&self) -> u16 {
        self.release_number
    }

    pub fn serial_number(&self) -> &Vec<u8> {
        &self.serial_number
    }

    pub fn checksum(&self) -> u16 {
        self.checksum
    }

    pub fn pc(&self) -> u32 {
        self.pc
    }

    pub fn set_pc(&mut self, pc: u32) {
        self.pc = pc;
    }
}
