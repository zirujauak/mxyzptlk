use crate::state::{
    header::{self, HeaderField},
    State,
};

use super::super::*;

pub struct CMem {
    pub data: Vec<u8>,
}

impl CMem {
    pub fn from_state(state: &State) -> CMem {
        let mut data: Vec<u8> = Vec::new();
        let mut run_length = 0;
        let dynamic_len = header::field_word(state.memory(), HeaderField::StaticMark)
            .expect("Error reading static memory mark from header")
            as usize;
        let dynamic = state.dynamic();
        for i in 0..dynamic_len {
            let b = state
                .read_byte(i)
                .expect("Error reading byte from dynamic memory")
                ^ dynamic[i];
            if b == 0 {
                if run_length == 255 {
                    data.push(0);
                    data.push(run_length);
                    run_length = 0;
                } else {
                    run_length = run_length + 1;
                }
            } else {
                if run_length > 0 {
                    data.push(0);
                    data.push(run_length - 1);
                    run_length = 0;
                }
                data.push(b);
            }
        }

        if run_length > 0 {
            data.push(0);
            data.push(run_length - 1);
        }

        CMem { data }
    }

    pub fn from_vec(chunk: Vec<u8>) -> CMem {
        CMem {
            data: chunk.clone(),
        }
    }

    pub fn from_chunk(chunk: Chunk) -> CMem {
        CMem {
            data: chunk.data.clone(),
        }
    }

    pub fn to_chunk(&self) -> Vec<u8> {
        chunk("CMem", &mut self.data.clone())
    }

    pub fn to_vec(&self, state: &State) -> Vec<u8> {
        let mut data = Vec::new();
        let mut iter = self.data.iter();
        let mut done = false;

        let dynamic = state.dynamic();
        while !done {
            let b = iter.next();
            match b {
                Some(b) => {
                    let i = data.len();
                    if *b == 0 {
                        let l = *iter.next().unwrap() as usize;
                        for j in 0..l + 1 {
                            data.push(dynamic[i + j]);
                        }
                    } else {
                        data.push(b ^ dynamic[i])
                    }
                }
                None => done = true,
            }
        }

        // FLAGS2 in the header is preserved from the current play state
        data[0x10] = state.read_byte(0x10).expect("Error reading from header");
        data[0x11] = state.read_byte(0x11).expect("Error reading from header");
        data
    }
}
