use std::fmt;

use super::super::*;

pub struct UMem {
    data: Vec<u8>,
}

impl fmt::Display for UMem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "uncompressed data size: {}", self.data.len())
    }
}

impl From<Vec<u8>> for UMem {
    fn from(value: Vec<u8>) -> UMem {
        UMem::new(&value)
    }
}

impl From<Chunk> for UMem {
    fn from(value: Chunk) -> UMem {
        UMem::new(value.data())
    }
}

impl From<&UMem> for Vec<u8> {
    fn from(value: &UMem) -> Vec<u8> {
        chunk("UMem", value.data())
    }
}

impl UMem {
    pub fn new(data: &[u8]) -> UMem {
        UMem {
            data: data.to_vec(),
        }
    }

    pub fn data(&self) -> &Vec<u8> {
        &self.data
    }
}
