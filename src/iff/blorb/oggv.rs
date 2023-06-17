use super::super::*;

pub struct OGGV {
    pub data: Vec<u8>,
}

impl OGGV {
    pub fn from_chunk(chunk: Chunk) -> OGGV {
        OGGV {
            data: chunk.data.clone()
        }
    }
}