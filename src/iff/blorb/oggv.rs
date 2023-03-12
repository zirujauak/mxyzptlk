use super::super::*;

pub struct OGGV {
    pub data: Vec<u8>,
}

impl OGGV {
    pub fn from_chunk(chunk: Chunk) -> OGGV {
        trace!("OGGV: {:#05x}, {:#05x} bytes", chunk.offset, chunk.data.len());
        OGGV {
            data: chunk.data.clone()
        }
    }
}