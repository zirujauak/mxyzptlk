use crate::error::*;

pub struct Memory {
    buffer: Vec<u8>,
}

pub fn word_value(hb: u8, lb: u8) -> u16 {
    (((hb as u16) << 8) & 0xFF00) + ((lb as u16) & 0xFF)
}

fn byte_values(w: u16) -> (u8, u8) {
    let hb = ((w >> 8) as u8) & 0xFF;
    let lb = (w as u8) & 0xFF;
    (hb, lb)
}

impl Memory {
    pub fn new(data: &Vec<u8>) -> Memory {
        let buffer = data.clone();
        Memory { buffer }
    }

    pub fn read_byte(&self, address: usize) -> Result<u8, RuntimeError> {
        if address < self.buffer.len() {
            Ok(self.buffer[address])
        } else {
            Err(RuntimeError::new(
                ErrorCode::InvalidAddress,
                format!(
                    "Byte address {:#06x} beyond end of memory ({:#06x})",
                    address,
                    self.buffer.len() - 1
                ),
            ))
        }
    }

    pub fn read_word(&self, address: usize) -> Result<u16, RuntimeError> {
        if address < self.buffer.len() - 1 {
            Ok(word_value(self.buffer[address], self.buffer[address + 1]))
        } else {
            Err(RuntimeError::new(
                ErrorCode::InvalidAddress,
                format!(
                    "Word address {:#06x} beyond end of memory ({:#06x})",
                    address,
                    self.buffer.len() - 1
                ),
            ))
        }
    }

    pub fn write_byte(&mut self, address: usize, value: u8) -> Result<(), RuntimeError> {
        if address < self.buffer.len() {
            info!(target: "app::memory", "Write {:#02x} to ${:04x}", value, address);
            self.buffer[address] = value;
            Ok(())
        } else {
            Err(RuntimeError::new(
                ErrorCode::InvalidAddress,
                format!(
                    "Byte address {:#06x} beyond end of memory ({:#06x})",
                    address,
                    self.buffer.len() - 1
                ),
            ))
        }
    }

    pub fn write_word(&mut self, address: usize, value: u16) -> Result<(), RuntimeError> {
        if address < self.buffer.len() - 2 {
            info!(target: "app::memory", "Write {:#04x} to ${:04x}", value, address);
            let (hb, lb) = byte_values(value);
            self.buffer[address] = hb;
            self.buffer[address + 1] = lb;
            Ok(())
        } else {
            Err(RuntimeError::new(
                ErrorCode::InvalidAddress,
                format!(
                    "Word address {:#06x} beyond end of memory ({:#06x})",
                    address,
                    self.buffer.len() - 1
                ),
            ))
        }
    }
}
