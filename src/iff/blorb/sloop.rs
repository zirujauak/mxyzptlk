use std::fmt;

use super::super::*;

#[derive(Clone, Debug)]
pub struct Entry {
    number: u32,
    repeats: u32,
}

impl PartialEq for Entry {
    fn eq(&self, other: &Self) -> bool {
        self.number == other.number && self.repeats == other.repeats
    }
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
    pub fn new(entries: &[Entry]) -> Loop {
        Loop {
            entries: entries.to_vec(),
        }
    }

    pub fn entries(&self) -> &Vec<Entry> {
        &self.entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entry_new() {
        let e = Entry::new(1, 2);
        assert_eq!(e.number(), 1);
        assert_eq!(e.repeats(), 2);
    }

    #[test]
    fn test_entry_partial_eq() {
        let e1 = Entry::new(1, 2);
        let e2 = Entry::new(1, 2);
        let e3 = Entry::new(3, 2);
        let e4 = Entry::new(1, 3);
        assert_eq!(e1, e2);
        assert_eq!(e2, e1);
        assert_ne!(e1, e3);
        assert_ne!(e1, e4);
    }

    #[test]
    fn test_entry_from_vec_u8() {
        let v = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let e = Entry::from(v);
        assert_eq!(e.number(), 0x01020304);
        assert_eq!(e.repeats(), 0x05060708);
    }

    #[test]
    fn test_loop_new() {
        let e1 = Entry::new(1, 2);
        let e2 = Entry::new(3, 4);
        let l = Loop::new(&[e1.clone(), e2.clone()]);
        assert_eq!(l.entries(), &vec![e1, e2]);
    }

    #[test]
    fn test_loop_from_chunk() {
        let chunk = Chunk::new(
            0,
            Some("FORM".to_string()),
            "RIdx".to_string(),
            &vec![1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4],
        );
        let sloop = Loop::from(chunk);
        assert_eq!(sloop.entries().len(), 2);
        assert_eq!(sloop.entries()[0].number(), 0x01010101);
        assert_eq!(sloop.entries()[0].repeats(), 0x02020202);
        assert_eq!(sloop.entries()[1].number(), 0x03030303);
        assert_eq!(sloop.entries()[1].repeats(), 0x04040404);
    }
}
