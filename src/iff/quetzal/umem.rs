use crate::state::{State, header::{self, HeaderField}};

use super::super::*;

pub struct UMem {
    pub data: Vec<u8>,
}

impl UMem {
    pub fn from_state(state: &State) -> UMem {
        let static_mark = header::field_word(state.memory(), HeaderField::StaticMark).expect("Error reading from header") as usize;
        let mut data = Vec::new();
        for i in 0..static_mark {
            data.push(state.read_byte(i).expect("Error reading dynamic memory"));
        }
        UMem {
            data
        }
    }

    pub fn from_vec(chunk: Vec<u8>) -> UMem {
        UMem {
            data: chunk.clone(),
        }
    }

    pub fn from_chunk(chunk: Chunk) -> UMem {
        UMem {
            data: chunk.data.clone()
        }
    }
    
    pub fn to_chunk(&self) -> Vec<u8> {
        chunk("UMem", &mut self.data.clone())
    }
}

