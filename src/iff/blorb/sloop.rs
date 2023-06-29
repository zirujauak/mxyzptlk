use std::fmt;

use super::super::*;

pub struct Entry {
    number: u32,
    repeats: u32,
}

impl From<Vec<u8>> for Entry {
    fn from(value: Vec<u8>) -> Entry {
        let number = vec_to_u32(&value, 0, 4);
        let repeats = vec_to_u32(&value, 4, 4);

        Entry::new(number, repeats)
    }
}

impl fmt::Display for Entry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "loop: {}, {}", self.number, self.repeats)
    }
}
impl Entry {
    pub fn new(number: u32, repeats: u32) -> Entry {
        Entry { number, repeats }
    }

    pub fn number(&self) -> u32 {
        self.number
    }

    pub fn repeats(&self) -> u32 {
        self.repeats
    }
}

pub struct Loop {
    entries: Vec<Entry>,
}

impl fmt::Display for Loop {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Sound loop data:")?;
        for entry in self.entries() {
            write!(f, "\n\t{}", entry)?;
        }
        write!(f, "")
    }
}

impl From<Chunk> for Loop {
    fn from(value: Chunk) -> Loop {
        let mut entries = Vec::new();

        for i in 0..value.length() as usize / 8 {
            let s = 8 * i;
            let index = Entry::from(value.data()[s..s + 8].to_vec());
            entries.push(index);
        }

        Loop { entries }
    }
}

impl Loop {
    pub fn new(entries: Vec<Entry>) -> Loop {
        Loop { entries }
    }

    pub fn entries(&self) -> &Vec<Entry> {
        &self.entries
    }
}
