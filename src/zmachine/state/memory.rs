use std::{fs::File, io::Read};

use crate::error::*;

use super::header::HeaderField;

pub struct Memory {
    version: u8,
    map: Vec<u8>,
    dynamic: Vec<u8>,
}

pub fn word_value(hb: u8, lb: u8) -> u16 {
    (((hb as u16) << 8) & 0xFF00) + ((lb as u16) & 0xFF)
}

fn byte_values(w: u16) -> (u8, u8) {
    let hb = (w >> 8) as u8;
    let lb = w as u8;
    (hb, lb)
}

impl TryFrom<&mut File> for Memory {
    type Error = RuntimeError;

    fn try_from(value: &mut File) -> Result<Self, Self::Error> {
        let mut d = Vec::new();
        match value.read_to_end(&mut d) {
            Ok(_) => Ok(Memory::new(d)),
            Err(e) => Err(RuntimeError::new(
                ErrorCode::System,
                format!("Error reading file: {}", e),
            )),
        }
    }
}

impl Memory {
    pub fn new(map: Vec<u8>) -> Memory {
        let version = map[0];
        let static_mark = word_value(
            map[HeaderField::StaticMark as usize],
            map[HeaderField::StaticMark as usize + 1],
        ) as usize;
        let dynamic = map[0..static_mark].to_vec();
        Memory {
            version,
            map,
            dynamic,
        }
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn slice(&self, start: usize, length: usize) -> Vec<u8> {
        let end = usize::min(start + length, self.map.len());
        self.map[start..end].to_vec()
    }

    pub fn checksum(&self) -> Result<u16, RuntimeError> {
        let mut checksum = 0;
        let size = self.read_word(HeaderField::FileLength as usize)? as usize
            * match self.version {
                1 | 2 | 3 => 2,
                4 | 5 => 4,
                6 | 7 | 8 => 8,
                _ => 0,
            };
        for i in 0x40..self.dynamic.len() {
            checksum = u16::overflowing_add(checksum, self.dynamic[i] as u16).0;
        }

        for i in self.dynamic.len()..size {
            checksum = u16::overflowing_add(checksum, self.map[i] as u16).0;
        }
        Ok(checksum)
    }

    pub fn read_byte(&self, address: usize) -> Result<u8, RuntimeError> {
        if address < self.map.len() {
            Ok(self.map[address])
        } else {
            Err(RuntimeError::new(
                ErrorCode::InvalidAddress,
                format!(
                    "Byte address {:#06x} beyond end of memory ({:#06x})",
                    address,
                    self.map.len() - 1
                ),
            ))
        }
    }

    pub fn read_word(&self, address: usize) -> Result<u16, RuntimeError> {
        if address < self.map.len() - 1 {
            Ok(word_value(self.map[address], self.map[address + 1]))
        } else {
            Err(RuntimeError::new(
                ErrorCode::InvalidAddress,
                format!(
                    "Word address {:#06x} beyond end of memory ({:#06x})",
                    address,
                    self.map.len() - 1
                ),
            ))
        }
    }

    pub fn write_byte(&mut self, address: usize, value: u8) -> Result<(), RuntimeError> {
        if address < self.map.len() {
            info!(target: "app::memory", "Write {:#02x} to ${:04x}", value, address);
            self.map[address] = value;
            Ok(())
        } else {
            Err(RuntimeError::new(
                ErrorCode::InvalidAddress,
                format!(
                    "Byte address {:#06x} beyond end of memory ({:#06x})",
                    address,
                    self.map.len() - 1
                ),
            ))
        }
    }

    pub fn write_word(&mut self, address: usize, value: u16) -> Result<(), RuntimeError> {
        if address < self.map.len() - 2 {
            info!(target: "app::memory", "Write {:#04x} to ${:04x}", value, address);
            let (hb, lb) = byte_values(value);
            self.map[address] = hb;
            self.map[address + 1] = lb;
            Ok(())
        } else {
            Err(RuntimeError::new(
                ErrorCode::InvalidAddress,
                format!(
                    "Word address {:#06x} beyond end of memory ({:#06x})",
                    address,
                    self.map.len() - 1
                ),
            ))
        }
    }

    pub fn compress(&self) -> Vec<u8> {
        let mut cdata: Vec<u8> = Vec::new();
        let mut run_length = 0;
        let dynamic_len = self.dynamic.len();
        for i in 0..dynamic_len {
            let b = self.map[i] ^ self.dynamic[i];
            if b == 0 {
                if run_length == 255 {
                    cdata.push(0);
                    cdata.push(run_length);
                    run_length = 0;
                } else {
                    run_length += 1;
                }
            } else {
                if run_length > 0 {
                    cdata.push(0);
                    cdata.push(run_length - 1);
                    run_length = 0;
                }
                cdata.push(b);
            }
        }

        if run_length > 0 {
            cdata.push(0);
            cdata.push(run_length - 1);
        }

        cdata
    }

    pub fn reset(&mut self) {
        self.map[..][..self.dynamic.len()].copy_from_slice(&self.dynamic)
    }

    pub fn restore(&mut self, data: &Vec<u8>) -> Result<(), RuntimeError> {
        if data.len() != self.dynamic.len() {
            Err(RuntimeError::new(
                ErrorCode::Restore,
                format!(
                    "Dynamic memory size doesn't match: {:04x} != {:04x}",
                    self.dynamic.len(),
                    data.len()
                ),
            ))
        } else {
            self.map[..][..data.len()].copy_from_slice(data);
            Ok(())
        }
    }

    fn decompress(&self, cdata: &[u8]) -> Vec<u8> {
        let mut data = Vec::new();
        let mut iter = cdata.iter();
        let mut done = false;

        while !done {
            let b = iter.next();
            match b {
                Some(b) => {
                    let i = data.len();
                    if *b == 0 {
                        let l = *iter.next().expect("Incomplete CMem 0 run") as usize;
                        for j in 0..l + 1 {
                            data.push(self.dynamic[i + j]);
                        }
                    } else {
                        data.push(b ^ self.dynamic[i])
                    }
                }
                None => done = true,
            }
        }

        data
    }

    pub fn restore_compressed(&mut self, cdata: &[u8]) -> Result<(), RuntimeError> {
        let data = self.decompress(cdata);
        self.restore(&data)
    }
}
