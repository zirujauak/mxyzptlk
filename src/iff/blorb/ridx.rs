use std::fmt;

use super::super::*;

#[derive(Clone, Debug)]
pub struct Index {
    usage: String,
    number: u32,
    start: u32,
}

impl fmt::Display for Index {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Index: {} [{}] @ {:08x}",
            self.number, self.usage, self.start
        )
    }
}

impl PartialEq for Index {
    fn eq(&self, other: &Self) -> bool {
        self.usage == other.usage && self.number == other.number && self.start == other.start
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
        let n = vec_to_u32(value.data(), 0, 4);
        let mut entries = Vec::new();

        for i in 0..n as usize {
            let s = 4 + (12 * i);
            let index = Index::from(value.data()[s..s + 12].to_vec());
            entries.push(index);
        }

        RIdx::new(&entries)
    }
}

impl RIdx {
    pub fn new(entries: &[Index]) -> RIdx {
        RIdx {
            entries: entries.to_vec(),
        }
    }

    pub fn entries(&self) -> &Vec<Index> {
        &self.entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_new() {
        let index = Index::new("USAGE".to_string(), 1, 2);
        assert_eq!(index.usage(), "USAGE");
        assert_eq!(index.number(), 1);
        assert_eq!(index.start(), 2);
    }

    #[test]
    fn test_index_partial_eq() {
        let i1 = Index::new("Snd ".to_string(), 1, 2);
        let i2 = Index::new("Snd ".to_string(), 1, 2);
        let i3 = Index::new("XXXX".to_string(), 1, 2);
        let i4 = Index::new("Snd ".to_string(), 3, 2);
        let i5 = Index::new("Snd ".to_string(), 1, 3);
        assert_eq!(i1, i2);
        assert_eq!(i2, i1);
        assert_ne!(i1, i3);
        assert_ne!(i1, i4);
        assert_ne!(i1, i5);
    }

    #[test]
    fn test_index_from_vec_u8() {
        let v = vec![0x53, 0x6E, 0x64, 0x20, 0, 0, 0, 1, 1, 2, 3, 4];
        let index = Index::from(v);
        assert_eq!(index.usage(), "Snd ");
        assert_eq!(index.number(), 1);
        assert_eq!(index.start(), 0x01020304);
    }

    #[test]
    fn test_ridx_new() {
        let i1 = Index::new("1".to_string(), 0, 1);
        let i2 = Index::new("2".to_string(), 2, 3);
        let e = vec![i1, i2];
        let ridx = RIdx::new(&e);
        assert_eq!(ridx.entries(), &e);
    }

    #[test]
    fn test_ridx_from_chunk() {
        let chunk = Chunk::new(
            0,
            Some("FORM".to_string()),
            "RIdx".to_string(),
            &vec![
                0, 0, 0, 2, 0x53, 0x6E, 0x64, 0x20, 0, 0, 0, 2, 0, 0, 0, 3, 0x53, 0x6E, 0x64, 0x20,
                0, 0, 0, 4, 0, 0, 0, 5,
            ],
        );
        let ridx = RIdx::from(chunk);
        assert_eq!(ridx.entries().len(), 2);
        assert_eq!(ridx.entries()[0].usage(), "Snd ");
        assert_eq!(ridx.entries()[0].number(), 2);
        assert_eq!(ridx.entries()[0].start(), 3);
        assert_eq!(ridx.entries()[1].usage(), "Snd ");
        assert_eq!(ridx.entries()[1].number(), 4);
        assert_eq!(ridx.entries()[1].start(), 5);
    }
}
