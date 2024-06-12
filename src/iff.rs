use core::fmt;
use std::{
    convert::TryFrom,
    fs::File,
    io::{self, Read},
};

/// An IFF "Chunk"
///
/// "Group" Chunks (`id` = "FORM", "LIST", "CAT ") include a `sub_id` value and
/// child `chunks`.  For these chunks, the `data` value should be an empty
/// vector.
///
/// Other chunks will have an empty `sub_id` and `chunks` vector and the
/// `data` vector will contain the chunk data.
///
/// The `length` field is the size of the chunk data.  For group chunks,
/// this length includes the 4-bytes `sub_id` value.
#[derive(Clone, Debug, PartialEq)]
pub struct Chunk {
    offset: u32,
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
    /// Create a new chunk
    ///
    /// Arguments:
    /// * `id`: IFF Id of the chunk
    /// * `data`: The chunk data.  Data will be padded with a 0 if needed to ensure
    /// the vector is an even number of bytes
    pub fn new_chunk(offset: u32, id: &str, data: Vec<u8>) -> Chunk {
        let length = data.len() as u32;
        // Pad data, if needed
        let data = if length % 2 == 1 {
            let mut data = data;
            data.push(0);
            data
        } else {
            data
        };

        Chunk {
            offset,
            id: id_to_vec(id),
            length,
            sub_id: Vec::new(),
            chunks: Vec::new(),
            data,
        }
    }

    pub fn new_form(offset: u32, sub_id: &str, chunks: Vec<Chunk>) -> Chunk {
        let length = chunks.iter().fold(4, |l, c| l + 8 + c.length);
        Chunk {
            offset,
            id: id_to_vec("FORM"),
            length,
            sub_id: id_to_vec(sub_id),
            chunks,
            data: Vec::new(),
        }
    }

    pub fn offset(&self) -> u32 {
        self.offset
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

    /// Searches for a child chunk matching an id and sub id from a list.
    ///
    /// The search ends when the first match is found.
    ///
    /// Arguments:
    /// * `ids`: A list of id+sub_id values to search for.
    ///
    /// Returns an Option containing the first matched chunk, or None.
    pub fn find_first_chunk(&self, ids: Vec<(&str, &str)>) -> Option<&Chunk> {
        for (id, sub_id) in ids {
            if let Some(c) = self.find_chunk(id, sub_id) {
                return Some(c);
            }
        }

        None
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
/// use iff::vec_as_unsigned;
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
/// use iff::unsigned_as_vec;
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
                "{}/{}, {:06x} bytes with {} chunks @ {:06x}",
                self.id(),
                self.sub_id(),
                self.length,
                self.chunks.len(),
                self.offset()
            )
        } else {
            write!(
                f,
                "{}, {:06x} with {} bytes of data @ {:06x}",
                self.id(),
                self.length,
                self.data.len(),
                self.offset()
            )
        }
    }
}

impl From<&Chunk> for Vec<u8> {
    fn from(value: &Chunk) -> Self {
        let mut data = Vec::new();
        data.extend(&value.id);
        data.extend(unsigned_as_vec(value.length() as usize, 4));
        if value.sub_id() == "" {
            data.extend(value.data());
        } else {
            data.extend(&value.sub_id);
            for c in value.chunks() {
                data.extend(Vec::from(c))
            }
        }
        if data.len() % 2 == 1 {
            data.push(0);
        }

        data
    }
}

impl From<&Vec<u8>> for Chunk {
    fn from(value: &Vec<u8>) -> Self {
        Chunk::from((0, value))
    }
}

impl From<(usize, &Vec<u8>)> for Chunk {
    fn from((start, value): (usize, &Vec<u8>)) -> Self {
        let id = value[start..start + 4].to_vec();
        let length = vec_as_unsigned(&value[start + 4..start + 8]) as u32;
        if id == [b'F', b'O', b'R', b'M'] {
            let sub_id = value[start + 8..start + 12].to_vec();
            let mut chunks = Vec::new();
            let mut offset = start + 12;
            while offset < start + length as usize {
                let chunk = Chunk::from((offset, value));
                offset += 8 + chunk.length() as usize;
                if offset % 2 == 1 {
                    offset += 1;
                }
                chunks.push(chunk);
            }

            Chunk {
                offset: start as u32,
                id,
                length,
                sub_id,
                chunks,
                data: Vec::new(),
            }
        } else {
            let data = value[start + 8..start + 8 + length as usize].to_vec();
            Chunk {
                offset: start as u32,
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
        if data.len() >= 12 {
            Ok(Chunk::from(&data))
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("File doesn't contain enough data: {}", data.len()),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, io::Write, path::Path};

    use super::*;

    #[test]
    fn test_id_to_vec() {
        assert_eq!(super::id_to_vec("ABCD"), vec![b'A', b'B', b'C', b'D']);
        assert_eq!(super::id_to_vec("A"), vec![b'A', b' ', b' ', b' ']);
        assert_eq!(super::id_to_vec("ABCDE"), vec![b'A', b'B', b'C', b'D']);
    }

    #[test]
    fn test_new_chunk() {
        let chunk = Chunk::new_chunk(0, "Test", vec![1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(chunk.offset, 0);
        assert_eq!(chunk.id, vec![b'T', b'e', b's', b't']);
        assert_eq!(chunk.id(), "Test");
        assert_eq!(chunk.length(), 7);
        assert!(chunk.sub_id.is_empty());
        assert_eq!(chunk.sub_id(), "");
        assert!(chunk.chunks().is_empty());
        assert_eq!(chunk.data(), &vec![1, 2, 3, 4, 5, 6, 7, 0])
    }

    #[test]
    fn test_new_form() {
        let c1 = Chunk::new_chunk(12, "Test", vec![1, 2, 3, 4, 5, 6, 7, 8]);
        let c2 = Chunk::new_chunk(28, "Foo", vec![4, 3, 2, 1]);
        let chunk = Chunk::new_form(0, "FTst", vec![c1.clone(), c2.clone()]);
        assert_eq!(chunk.offset, 0);
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
        let c1 = Chunk::new_chunk(12, "Test", vec![1, 2, 3, 4, 5, 6, 7, 8]);
        let c2 = Chunk::new_chunk(28, "Foo", vec![4, 3, 2, 1]);
        let chunk = Chunk::new_form(0, "FTst", vec![c1.clone(), c2]);
        let found = chunk.find_chunk("Test", "");
        assert!(found.is_some());
        assert_eq!(found.unwrap(), &c1);
    }

    #[test]
    fn test_find_chunk_multiple() {
        let c1 = Chunk::new_chunk(12, "Test", vec![4, 3, 2, 1]);
        let c2 = Chunk::new_chunk(24, "Test", vec![1, 2, 3, 4, 5, 6, 7, 8]);
        let chunk = Chunk::new_form(0, "FTst", vec![c1.clone(), c2]);
        let found = chunk.find_chunk("Test", "");
        assert!(found.is_some());
        assert_eq!(found.unwrap(), &c1);
    }

    #[test]
    fn test_find_chunk_none() {
        let c1 = Chunk::new_chunk(12, "Test", vec![4, 3, 2, 1]);
        let c2 = Chunk::new_chunk(24, "Test", vec![1, 2, 3, 4, 5, 6, 7, 8]);
        let chunk = Chunk::new_form(0, "FTst", vec![c1, c2]);
        let found = chunk.find_chunk("None", "");
        assert!(found.is_none());
    }

    #[test]
    fn test_find_chunk_with_sub_id() {
        let c1 = Chunk::new_chunk(12, "Test", vec![1, 2, 3, 4, 5, 6, 7, 8]);
        let c2 = Chunk::new_chunk(30, "Foo", vec![4, 3, 2, 1]);
        let c3 = Chunk::new_form(28, "SbId", vec![c2.clone()]);
        let chunk = Chunk::new_form(0, "FTst", vec![c1, c3]);
        let found = chunk.find_chunk("FORM", "SbId");
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.offset, 28);
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
    fn test_find_first_chunk() {
        let c1 = Chunk::new_chunk(12, "Test", vec![1, 2, 3, 4, 5, 6, 7, 8]);
        let c2 = Chunk::new_chunk(28, "Foo", vec![4, 3, 2, 1]);
        let chunk = Chunk::new_form(0, "FTst", vec![c1, c2.clone()]);
        let found = chunk.find_first_chunk(vec![("Nope", ""), ("Foo ", ""), ("Test", "")]);
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found, &c2);
    }

    #[test]
    fn test_find_first_chunk_none() {
        let c1 = Chunk::new_chunk(12, "Test", vec![1, 2, 3, 4, 5, 6, 7, 8]);
        let c2 = Chunk::new_chunk(28, "Foo", vec![4, 3, 2, 1]);
        let chunk = Chunk::new_form(0, "FTst", vec![c1, c2]);
        let found = chunk.find_first_chunk(vec![("Nope", ""), ("Foop", ""), ("Tast", "")]);
        assert!(found.is_none());
    }

    #[test]
    fn test_find_chunks() {
        let c1 = Chunk::new_chunk(12, "Test", vec![4, 3, 2, 1]);
        let c2 = Chunk::new_chunk(24, "Otro", vec![5, 6, 7, 8, 9, 10, 11, 12]);
        let c3 = Chunk::new_chunk(40, "Test", vec![1, 2, 3, 4, 5, 6, 7, 8]);
        let chunk = Chunk::new_form(0, "FTst", vec![c1.clone(), c2, c3.clone()]);
        let found = chunk.find_chunks("Test", "");
        assert_eq!(found.len(), 2);
        assert_eq!(found[0], &c1);
        assert_eq!(found[1], &c3);
    }

    #[test]
    fn test_find_chunks_none() {
        let c1 = Chunk::new_chunk(12, "Test", vec![4, 3, 2, 1]);
        let c2 = Chunk::new_chunk(24, "Otro", vec![5, 6, 7, 8, 9, 10, 11, 12]);
        let c3 = Chunk::new_chunk(40, "Test", vec![1, 2, 3, 4, 5, 6, 7, 8]);
        let chunk = Chunk::new_form(0, "FTst", vec![c1, c2, c3]);
        let found = chunk.find_chunks("Nope", "");
        assert!(found.is_empty());
    }

    #[test]
    fn test_find_chunks_with_sub_id() {
        let c1 = Chunk::new_chunk(24, "Test", vec![4, 3, 2, 1]);
        let c2 = Chunk::new_chunk(36, "Otro", vec![5, 6, 7, 8, 9, 10, 11, 12]);
        let c3 = Chunk::new_chunk(64, "Test", vec![1, 2, 3, 4, 5, 6, 7, 8]);
        let c4 = Chunk::new_form(12, "Sb1", vec![c1, c2]);
        let c5 = Chunk::new_form(52, "Sb1", vec![c3]);
        let c6 = Chunk::new_form(80, "Sub2", vec![]);
        let chunk = Chunk::new_form(0, "FTst", vec![c4.clone(), c5.clone(), c6]);
        let found = chunk.find_chunks("FORM", "Sb1 ");
        assert_eq!(found.len(), 2);
        assert_eq!(found[0], &c4);
        assert_eq!(found[1], &c5);
    }

    #[test]
    fn test_from_vec_u8() {
        let v = vec![
            b'F', b'O', b'R', b'M', 0x00, 0x00, 0x00, 0x34, b'S', b'U', b'B', b' ', b'C', b'h',
            b'n', b'k', 0x00, 0x00, 0x00, 0x04, 0x01, 0x02, 0x03, 0x04, b'F', b'O', b'R', b'M',
            0x00, 0x00, 0x00, 0x1C, b'C', b'l', b'C', b'k', b'B', b'n', b'z', b' ', 0x00, 0x00,
            0x00, 0x04, 0x05, 0x06, 0x07, 0x08, b'C', b'h', b'n', b'k', 0x00, 0x00, 0x00, 0x04,
            0x09, 0x0a, 0x0b, 0x0c,
        ];
        let chunk = Chunk::from(&v);
        assert_eq!(chunk.id(), "FORM");
        assert_eq!(chunk.length(), 0x34);
        assert_eq!(chunk.sub_id(), "SUB ");
        assert_eq!(
            chunk.chunks(),
            &vec![
                Chunk::new_chunk(0x0c, "Chnk", vec![0x01, 0x02, 0x03, 0x04]),
                Chunk::new_form(
                    0x18,
                    "ClCk",
                    vec![
                        Chunk::new_chunk(0x24, "Bnz ", vec![0x05, 0x06, 0x07, 0x08]),
                        Chunk::new_chunk(0x30, "Chnk", vec![0x09, 0x0a, 0x0b, 0x0c])
                    ]
                )
            ]
        )
    }

    #[test]
    fn test_from_file() {
        let v = vec![
            b'F', b'O', b'R', b'M', 0x00, 0x00, 0x00, 0x34, b'S', b'U', b'B', b' ', b'C', b'h',
            b'n', b'k', 0x00, 0x00, 0x00, 0x04, 0x01, 0x02, 0x03, 0x04, b'F', b'O', b'R', b'M',
            0x00, 0x00, 0x00, 0x1C, b'C', b'l', b'C', b'k', b'B', b'n', b'z', b' ', 0x00, 0x00,
            0x00, 0x04, 0x05, 0x06, 0x07, 0x08, b'C', b'h', b'n', b'k', 0x00, 0x00, 0x00, 0x04,
            0x09, 0x0a, 0x0b, 0x0c,
        ];
        let f = fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open("test.iff");
        assert!(f.is_ok());
        let mut file = f.unwrap();
        assert!(file.write_all(&v).is_ok());
        assert!(file.flush().is_ok());
        assert!(Path::new("test.iff").exists());
        let f = fs::OpenOptions::new().read(true).open("test.iff");
        assert!(f.is_ok());
        let mut file = f.unwrap();
        let chunk = Chunk::try_from(&mut file);
        assert!(fs::remove_file("test.iff").is_ok());
        assert!(chunk.is_ok());
        let chunk = chunk.unwrap();
        assert_eq!(chunk.id(), "FORM");
        assert_eq!(chunk.length(), 0x34);
        assert_eq!(chunk.sub_id(), "SUB ");
        assert_eq!(
            chunk.chunks(),
            &vec![
                Chunk::new_chunk(0x0c, "Chnk", vec![0x01, 0x02, 0x03, 0x04]),
                Chunk::new_form(
                    0x18,
                    "ClCk",
                    vec![
                        Chunk::new_chunk(0x24, "Bnz ", vec![0x05, 0x06, 0x07, 0x08]),
                        Chunk::new_chunk(0x30, "Chnk", vec![0x09, 0x0a, 0x0b, 0x0c])
                    ]
                )
            ]
        )
    }

    #[test]
    fn test_from_file_error() {
        let f = fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open("test-2.iff");
        assert!(f.is_ok());
        let mut file = f.unwrap();
        assert!(file.write_all(&[]).is_ok());
        assert!(file.flush().is_ok());
        assert!(Path::new("test-2.iff").exists());
        let f = fs::OpenOptions::new().read(true).open("test-2.iff");
        assert!(f.is_ok());
        let mut file = f.unwrap();
        let chunk = Chunk::try_from(&mut file);
        assert!(fs::remove_file("test-2.iff").is_ok());
        assert!(chunk.is_err());
    }
}
