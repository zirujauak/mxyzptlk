use std::fmt;

use super::super::*;

pub struct CMem {
    data: Vec<u8>,
}

impl fmt::Display for CMem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "compressed data size: {}", self.data.len())
    }
}

impl From<Vec<u8>> for CMem {
    fn from(value: Vec<u8>) -> CMem {
        CMem::new(&value)
    }
}

impl From<Chunk> for CMem {
    fn from(value: Chunk) -> CMem {
        CMem::new(value.data())
    }
}

impl From<&CMem> for Vec<u8> {
    fn from(value: &CMem) -> Vec<u8> {
        chunk("CMem", &mut value.data.clone())
    }
}

impl CMem {
    pub fn new(data: &[u8]) -> CMem {
        CMem { data: data.to_vec() }
    }

    pub fn data(&self) -> &Vec<u8> {
        &self.data
    }
}
