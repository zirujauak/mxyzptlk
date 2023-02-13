pub mod header;

use self::header::Header;

use std::fmt;

pub struct Memory {
    pub memory_map: Vec<u8>,
    pub header: Header    
}

impl fmt::Display for Memory {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Memory size: ${:x}", self.memory_map.len())?;
        write!(f, "Header:\n{}", self.header)
    }
}

impl Memory {
    pub fn from_vec(v: Vec<u8>) -> Self {
        Self {
            memory_map: v.clone(),
            header: Header::from_vec(&v)
        }
    }

    pub fn byte(&self, a: usize) -> u8 {
        if a > 0x3f && a < self.header.base_high_memory as usize{
            self.memory_map[a]
        } else {
            /* TODO: error */
            0 
        }
    }

    pub fn word(&self, a: usize) -> u16 {
        if a > 0x3f && a < (self.header.base_high_memory - 1) as usize {
            let hb = self.memory_map[a];
            let lb = self.memory_map[a + 1];
            (((hb as u16) << 8) & 0xFF00) + (lb & 0xFF) as u16
        } else {
            /* TODO: error */
            0
        }
    }

    pub fn set_byte(&mut self, a: usize, v: u8) {
        if a > 0x3f && a < self.header.base_static_memory as usize {
            self.memory_map[a] = v
        }
    }

    pub fn set_word(&mut self, a: usize, v: u16) {
        if a > 0x3f && a < (self.header.base_static_memory - 1) as usize {
            self.memory_map[a] = ((v >> 8) & 0xFF) as u8;
            self.memory_map[a + 1] = (v & 0xFF) as u8
        }
    }

    pub fn checksum(&self) -> u16 {
        let mut c:usize = 0;
        for i in 40..self.memory_map.len() {
            c = c + self.memory_map[i] as usize;
        }

        (c & 0xFFFF) as u16
    }
}
