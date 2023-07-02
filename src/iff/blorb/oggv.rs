use std::fmt;

use super::super::*;

pub struct OGGV {
    data: Vec<u8>,
}

impl fmt::Display for OGGV {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "OGG Vorbis data size: {}", self.data.len())
    }
}

impl From<Chunk> for OGGV {
    fn from(value: Chunk) -> OGGV {
        OGGV::new(value.data())
    }
}

impl OGGV {
    pub fn new(data: &[u8]) -> OGGV {
        OGGV {
            data: data.to_vec(),
        }
    }

    pub fn data(&self) -> &Vec<u8> {
        &self.data
    }
}
