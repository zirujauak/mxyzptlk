use super::super::*;

pub struct Entry {
    number: u32,
    repeats: u32
}

impl Entry {
    pub fn from_vec(v: &Vec<u8>, offset: usize) -> Entry {
        let number = vec_to_u32(&v, offset, 4);
        let repeats = vec_to_u32(&v, offset + 4, 4);

        trace!("Loop Entry: Sound #{} repeat {}", number, repeats);

        Entry {
            number,
            repeats,
        }
    }
}
pub struct Loop {
    pub entries: Vec<Entry>,
}

impl Loop {
    pub fn from_chunk(chunk: Chunk) -> Loop {
        let mut entries = Vec::new();

        for i in 0..chunk.length as usize / 8 {
            let index = Entry::from_vec(&chunk.data, (8 * i as usize));
            entries.push(index);
        }   

        trace!("Loop: {} entries", entries.len());
        
        Loop {
            entries
        }
    }
}