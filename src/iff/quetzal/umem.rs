use crate::executor::{state::State, header};
use super::super::*;

pub struct UMem {
    pub data: Vec<u8>,
}

impl UMem {
    pub fn from_state(state: &State) -> UMem {
        UMem {
            data: state.memory_map()[0..header::static_memory_base(state) as usize].to_vec(),
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

