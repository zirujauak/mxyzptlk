use std::fmt;

use super::super::*;

pub struct Index {
    usage: String,
    number: u32,
    start: u32,
}

impl fmt::Display for Index {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Index: {} [{}] @ {:08x}", self.number, self.usage, self.start)
    }
}

impl From<Vec<u8>> for Index {
    fn from(value: Vec<u8>) -> Index {
        let usage = vec_to_id(&value, 0);
        let number = vec_to_u32(&value, 4, 4);
        let start = vec_to_u32(&value, 8, 4);

        Index::new(usage, number, start)
    }
}

impl Index {
    pub fn new(usage: String, number: u32, start: u32) -> Index {
        Index {
            usage,
            number,
            start,
        }
    }

    pub fn usage(&self) -> &str {
        &self.usage
    }

    pub fn number(&self) -> u32 {
        self.number
    }

    pub fn start(&self) -> u32 {
        self.start
    }
}

pub struct RIdx {
    entries: Vec<Index>,
}

impl fmt::Display for RIdx {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Resource index:")?;
        for index in self.entries() {
            write!(f, "\n\t{}", index)?;
        }
        write!(f, "")
    }
}

impl From<Chunk> for RIdx {
    fn from(value: Chunk) -> RIdx {
        let n = vec_to_u32(&value.data(), 0, 4);
        let mut entries = Vec::new();

        for i in 0..n as usize {
            let s = 4 + (12 * i);
            let index = Index::from(value.data()[s..s + 12].to_vec());
            entries.push(index);
        }

        RIdx { entries }
    }
}

impl RIdx {
    pub fn entries(&self) -> &Vec<Index> {
        &self.entries
    }
}
