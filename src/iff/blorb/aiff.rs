use std::fmt;

use crate::iff;

use super::super::*;

pub struct AIFF {
    _id: String,
    _length: u32,
    data: Vec<u8>,
}

impl fmt::Display for AIFF {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "AIFF data size: {}", self.data.len())
    }
}

impl From<Chunk> for AIFF {
    fn from(value: Chunk) -> AIFF {
        AIFF::new(value.id(), value.length(), value.data())
    }
}

impl From<&AIFF> for Vec<u8> {
    fn from(value: &AIFF) -> Vec<u8> {
        // Reconstitute the AIFF FORM
        let mut v = Vec::new();
        v.append(&mut iff::id_as_vec("FORM"));
        v.append(&mut iff::u32_to_vec(value.data.len() as u32, 4));
        v.append(&mut value.data.clone());
        if v.len() % 2 == 1 {
            v.push(0);
        }
        v
    }
}

impl AIFF {
    pub fn new(id: &str, length: u32, data: &[u8]) -> AIFF {
        AIFF {
            _id: id.to_string(),
            _length: length,
            data: data.to_vec(),
        }
    }

    pub fn data(&self) -> &Vec<u8> {
        &self.data
    }
}
