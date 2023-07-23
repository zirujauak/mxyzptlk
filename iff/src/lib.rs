use core::fmt;
use std::{
    convert::TryFrom,
    fs::File,
    io::{self, Read},
};

#[derive(Clone, Debug, PartialEq)]
pub struct Chunk {
    id: Vec<u8>,
    length: u32,
    sub_id: Vec<u8>,
    chunks: Vec<Chunk>,
    data: Vec<u8>,
}

/// Translates an IFF id string to a vector of bytes
///
/// Pads the id to ensure it is at least 4 characters long, then returns
/// a byte vector containing the first 4 characters.
fn id_to_vec(id: &str) -> Vec<u8> {
    let mut id = String::from(id);
    id.push_str("    ");
    id.as_bytes()[0..4].to_vec()
}

impl Chunk {
    pub fn new_chunk(id: &str, data: Vec<u8>) -> Chunk {
        Chunk {
            id: id_to_vec(id),
            length: data.len() as u32,
            sub_id: Vec::new(),
            chunks: Vec::new(),
            data,
        }
    }

    pub fn new_form(sub_id: &str, chunks: Vec<Chunk>) -> Chunk {
        let length = chunks.iter().fold(4, |l, c| l + 8 + c.length);
        Chunk {
            id: id_to_vec("FORM"),
            length,
            sub_id: id_to_vec(sub_id),
            chunks,
            data: Vec::new(),
        }
    }

    pub fn id(&self) -> String {
        self.id.iter().map(|x| *x as char).collect::<String>()
    }

    pub fn length(&self) -> u32 {
        self.length
    }

    pub fn sub_id(&self) -> String {
        self.sub_id.iter().map(|x| *x as char).collect::<String>()
    }

    pub fn chunks(&self) -> &Vec<Chunk> {
        &self.chunks
    }

    pub fn data(&self) -> &Vec<u8> {
        &self.data
    }

    /// Finds the (first) direct child chunk with the matching id and sub id.
    ///
    /// Arguments:
    /// * `id`: IFF id of the chunk to find
    /// * `sub_id`: IFF sub id when id is "FORM", otherwise use an empty string ""
    ///
    /// Returns an Option containing the first matched chunk, or None.
    pub fn find_chunk(&self, id: &str, sub_id: &str) -> Option<&Chunk> {
        let chunks: Vec<&Chunk> = self
            .chunks
            .iter()
            .filter(|x| x.id() == id && x.sub_id() == sub_id)
            .collect();
        if chunks.is_empty() {
            None
        } else {
            Some(chunks[0])
        }
    }

    /// Finds all direct child chunk with the matching id and sub id.
    ///
    /// Arguments:
    /// * id: IFF id of the chunk to find
    /// * sub_id: IFF sub id when id is "FORM", otherwise use an empty string ""
    ///
    /// Returns a vector of references to the matched Chunks.
    pub fn find_chunks(&self, id: &str, sub_id: &str) -> Vec<&Chunk> {
        let mut chunks = Vec::new();
        // Filter the chunks array by id and sub id
        chunks.extend(
            self.chunks
                .iter()
                .filter(|x| x.id() == id && x.sub_id() == sub_id),
        );
        chunks
    }
}

/// Tranform a vector of bytes in big-ending order to a usize
///
/// Arguments:
/// * v: A vector of bytes
///
/// # Examples
/// ```
/// use ifflib::vec_as_unsigned;
/// assert_eq!(vec_as_unsigned(&[0x12, 0x34, 0x56]), 0x123456)
/// ```
pub fn vec_as_unsigned(v: &[u8]) -> usize {
    let mut u: usize = 0;
    for (i, b) in v.iter().enumerate() {
        u |= (*b as usize) << ((v.len() - 1 - i) * 8);
    }

    u
}

/// Transforms a usize to a vector of bytes in big-endian order.
///
/// Arguments:
/// * value: The usize value
/// * length: The length of the result
///
/// # Examples
/// ```
/// use ifflib::unsigned_as_vec;
/// assert_eq!(unsigned_as_vec(0x123456, 3), vec![0x12, 0x34, 0x56]);
/// ```
pub fn unsigned_as_vec(value: usize, length: usize) -> Vec<u8> {
    let mut v = Vec::new();
    for i in (0..length).rev() {
        v.push(((value >> (8 * i)) & 0xFF) as u8);
    }
    v
}

impl fmt::Display for Chunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.id() == "FORM" {
            write!(
                f,
                "{}/{}, {:06x} bytes with {} chunks",
                self.id(),
                self.sub_id(),
                self.length,
                self.chunks.len()
            )
        } else {
            write!(
                f,
                "{}, {:06x} with {} bytes of data",
                self.id(),
                self.length,
                self.data.len()
            )
        }
    }
}

impl From<Chunk> for Vec<u8> {
    fn from(value: Chunk) -> Self {
        let mut data = Vec::new();
        data.extend(&value.id);
        data.extend(unsigned_as_vec(value.length() as usize, 4));
        if value.sub_id() == "" {
            data.extend(value.data());
        } else {
            data.extend(&value.sub_id);
            for c in value.chunks() {
                data.extend(Vec::from(c.clone()))
            }
        }
        if data.len() % 2 == 1 {
            data.push(0);
        }

        data
    }
}
impl From<Vec<u8>> for Chunk {
    fn from(value: Vec<u8>) -> Self {
        let id = value[0..4].to_vec();
        let length = vec_as_unsigned(&value[4..8]) as u32;
        if id == [b'F', b'O', b'R', b'M'] {
            let sub_id = value[8..12].to_vec();
            let mut chunks = Vec::new();
            let mut offset = 12;
            while offset < length as usize {
                let chunk = Chunk::from(value[offset..].to_vec());
                offset += 8 + chunk.length() as usize;
                if offset % 2 == 1 {
                    offset += 1;
                }
                chunks.push(chunk);
            }

            Chunk {
                id,
                length,
                sub_id,
                chunks,
                data: Vec::new(),
            }
        } else {
            let data = value[8..8 + length as usize].to_vec();
            Chunk {
                id,
                length,
                sub_id: Vec::new(),
                chunks: Vec::new(),
                data,
            }
        }
    }
}

impl TryFrom<&mut File> for Chunk {
    type Error = io::Error;

    fn try_from(value: &mut File) -> Result<Self, Self::Error> {
        let mut data = Vec::new();
        value.read_to_end(&mut data)?;
        Ok(Chunk::from(data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_to_vec() {
        assert_eq!(super::id_to_vec("ABCD"), vec![b'A', b'B', b'C', b'D']);
        assert_eq!(super::id_to_vec("A"), vec![b'A', b' ', b' ', b' ']);
        assert_eq!(super::id_to_vec("ABCDE"), vec![b'A', b'B', b'C', b'D']);
    }

    #[test]
    fn test_new_chunk() {
        let chunk = Chunk::new_chunk("Test", vec![1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(chunk.id, vec![b'T', b'e', b's', b't']);
        assert_eq!(chunk.id(), "Test");
        assert_eq!(chunk.length(), 8);
        assert!(chunk.sub_id.is_empty());
        assert_eq!(chunk.sub_id(), "");
        assert!(chunk.chunks().is_empty());
        assert_eq!(chunk.data(), &vec![1, 2, 3, 4, 5, 6, 7, 8])
    }

    #[test]
    fn test_new_form() {
        let c1 = Chunk::new_chunk("Test", vec![1, 2, 3, 4, 5, 6, 7, 8]);
        let c2 = Chunk::new_chunk("Foo", vec![4, 3, 2, 1]);
        let chunk = Chunk::new_form("FTst", vec![c1.clone(), c2.clone()]);
        assert_eq!(chunk.id, vec![b'F', b'O', b'R', b'M']);
        assert_eq!(chunk.id(), "FORM");
        // Length = 4 (sub id) + chunk 1 (8 + 8 data) + chunk 2 (8 + 4 data)
        assert_eq!(chunk.length(), 32);
        assert_eq!(chunk.sub_id, vec![b'F', b'T', b's', b't']);
        assert_eq!(chunk.sub_id(), "FTst");
        assert_eq!(chunk.chunks().len(), 2);
        assert_eq!(chunk.chunks()[0], c1);
        assert_eq!(chunk.chunks()[1], c2);
        assert!(chunk.data().is_empty())
    }

    #[test]
    fn test_find_chunk() {
        let c1 = Chunk::new_chunk("Test", vec![1, 2, 3, 4, 5, 6, 7, 8]);
        let c2 = Chunk::new_chunk("Foo", vec![4, 3, 2, 1]);
        let chunk = Chunk::new_form("FTst", vec![c1, c2]);
        let found = chunk.find_chunk("Test", "");
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.id, vec![b'T', b'e', b's', b't']);
        assert_eq!(found.id(), "Test");
        assert_eq!(found.length(), 8);
        assert_eq!(found.sub_id, vec![]);
        assert_eq!(found.sub_id(), "");
        assert!(found.chunks().is_empty());
        assert_eq!(found.data(), &vec![1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn test_find_chunk_multiple() {
        let c1 = Chunk::new_chunk("Test", vec![4, 3, 2, 1]);
        let c2 = Chunk::new_chunk("Test", vec![1, 2, 3, 4, 5, 6, 7, 8]);
        let chunk = Chunk::new_form("FTst", vec![c1, c2]);
        let found = chunk.find_chunk("Test", "");
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.id, vec![b'T', b'e', b's', b't']);
        assert_eq!(found.id(), "Test");
        assert_eq!(found.length(), 4);
        assert_eq!(found.sub_id, vec![]);
        assert_eq!(found.sub_id(), "");
        assert!(found.chunks().is_empty());
        assert_eq!(found.data(), &vec![4, 3, 2, 1]);
    }

    #[test]
    fn test_find_chunk_none() {
        let c1 = Chunk::new_chunk("Test", vec![4, 3, 2, 1]);
        let c2 = Chunk::new_chunk("Test", vec![1, 2, 3, 4, 5, 6, 7, 8]);
        let chunk = Chunk::new_form("FTst", vec![c1, c2]);
        let found = chunk.find_chunk("None", "");
        assert!(found.is_none());
    }

    #[test]
    fn test_find_chunk_with_sub_id() {
        let c1 = Chunk::new_chunk("Test", vec![1, 2, 3, 4, 5, 6, 7, 8]);
        let c2 = Chunk::new_chunk("Foo", vec![4, 3, 2, 1]);
        let c3 = Chunk::new_form("SbId", vec![c2.clone()]);
        let chunk = Chunk::new_form("FTst", vec![c1, c3]);
        let found = chunk.find_chunk("FORM", "SbId");
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.id, vec![b'F', b'O', b'R', b'M']);
        assert_eq!(found.id(), "FORM");
        // Length = 4 (sub id) + chunk 2 (8 + 4 data)
        assert_eq!(found.length(), 16);
        assert_eq!(found.sub_id, vec![b'S', b'b', b'I', b'd']);
        assert_eq!(found.sub_id(), "SbId");
        assert_eq!(found.chunks().len(), 1);
        assert_eq!(found.chunks()[0], c2);
        assert!(found.data().is_empty())
    }

    #[test]
    fn test_find_chunks() {
        let c1 = Chunk::new_chunk("Test", vec![4, 3, 2, 1]);
        let c2 = Chunk::new_chunk("Otro", vec![5, 6, 7, 8, 9, 10, 11, 12]);
        let c3 = Chunk::new_chunk("Test", vec![1, 2, 3, 4, 5, 6, 7, 8]);
        let chunk = Chunk::new_form("FTst", vec![c1.clone(), c2, c3.clone()]);
        let found = chunk.find_chunks("Test", "");
        assert_eq!(found.len(), 2);
        assert_eq!(found[0], &c1);
        assert_eq!(found[1], &c3);
    }

    #[test]
    fn test_find_chunks_none() {
        let c1 = Chunk::new_chunk("Test", vec![4, 3, 2, 1]);
        let c2 = Chunk::new_chunk("Otro", vec![5, 6, 7, 8, 9, 10, 11, 12]);
        let c3 = Chunk::new_chunk("Test", vec![1, 2, 3, 4, 5, 6, 7, 8]);
        let chunk = Chunk::new_form("FTst", vec![c1, c2, c3]);
        let found = chunk.find_chunks("Nope", "");
        assert!(found.is_empty());
    }

    #[test]
    fn test_find_chunks_with_sub_id() {
        let c1 = Chunk::new_chunk("Test", vec![4, 3, 2, 1]);
        let c2 = Chunk::new_chunk("Otro", vec![5, 6, 7, 8, 9, 10, 11, 12]);
        let c3 = Chunk::new_chunk("Test", vec![1, 2, 3, 4, 5, 6, 7, 8]);
        let c4 = Chunk::new_form("Sb1", vec![c1, c2]);
        let c5 = Chunk::new_form("Sb1", vec![c3]);
        let c6 = Chunk::new_form("Sub2", vec![]);
        let chunk = Chunk::new_form("FTst", vec![c4.clone(), c5.clone(), c6]);
        let found = chunk.find_chunks("FORM", "Sb1 ");
        assert_eq!(found.len(), 2);
        assert_eq!(found[0], &c4);
        assert_eq!(found[1], &c5);
    }
}
