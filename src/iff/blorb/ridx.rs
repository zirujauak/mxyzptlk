use super::super::*;

pub struct Index {
    pub usage: String,
    pub number: u32,
    pub start: u32
}

impl Index {
    pub fn from_vec(v: &Vec<u8>, offset: usize) -> Index {
        let usage = vec_to_id(&v, offset);
        let number = vec_to_u32(&v, offset + 4, 4);
        let start = vec_to_u32(&v, offset + 8, 4);

        trace!("Resource Index: {} #{} @ {:#08x}", usage, number, start);

        Index {
            usage,
            number,
            start
        }
    }
}
pub struct RIdx {
    pub entries: Vec<Index>,
}

impl RIdx {
    pub fn from_chunk(chunk: Chunk) -> RIdx {
        let n = vec_to_u32(&chunk.data, 0, 4);
        let mut entries = Vec::new();

        for i in 0..n as usize {
            let index = Index::from_vec(&chunk.data, 4 + (12 * i));
            entries.push(index);
        }   

        trace!("RIdx: {} entries", n);

        RIdx {
            entries
        }
    }
}