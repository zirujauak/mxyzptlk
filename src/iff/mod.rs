use std::fmt;

pub mod blorb;
pub mod quetzal;

fn usize_as_vec(d: usize, bytes: usize) -> Vec<u8> {
    let mut data = Vec::new();
    for i in (0..bytes).rev() {
        data.push(((d >> (8 * i)) & 0xFF) as u8);
    }
    data
}

fn vec_as_usize(v: Vec<u8>, bytes: usize) -> usize {
    let mut u: usize = 0;
    for (i, b) in v.iter().enumerate() {
        // Stop after the specified number of bytes
        if i < bytes {
            u |= (*b as usize) << ((bytes - 1 - i) * 8);
        }
    }

    u
}

fn vec_to_u32(v: &[u8], offset: usize, bytes: usize) -> u32 {
    let mut u: u32 = 0;
    for (i, b) in v[offset..offset + bytes].iter().enumerate() {
        u |= (*b as u32) << ((bytes - i - 1) * 8);
    }
    u
}

fn u32_to_vec(d: u32, bytes: usize) -> Vec<u8> {
    let mut v = Vec::new();
    for i in (0..bytes).rev() {
        v.push(((d >> (8 * i)) & 0xFF) as u8);
    }
    v
}

fn vec_to_id(v: &[u8], offset: usize) -> String {
    let mut id = String::new();
    for i in 0..4 {
        id.push(v[offset + i] as char);
    }

    id
}

fn id_as_vec(id: &str) -> Vec<u8> {
    id.as_bytes()[0..4].to_vec()
}

fn chunk(id: &str, data: &[u8]) -> Vec<u8> {
    let mut chunk = id_as_vec(id);
    let data_length = data.len();
    chunk.append(&mut usize_as_vec(data.len(), 4));
    chunk.append(&mut data.to_vec());
    if data_length % 2 == 1 {
        // Padding byte, not included in chunk length
        chunk.push(0);
    }

    chunk
}

pub struct Chunk {
    offset: usize,
    form: Option<String>,
    id: String,
    length: u32,
    data: Vec<u8>,
}

impl fmt::Display for Chunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.form.is_some() {
            write!(
                f,
                "FORM: {} {:06x} {}",
                self.id,
                self.length(),
                self.data().len()
            )
        } else {
            write!(
                f,
                "Chunk: {} {:06x} {}",
                self.id,
                self.length(),
                self.data().len()
            )
        }
    }
}

impl From<(&Vec<u8>, usize)> for Chunk {
    fn from((value, offset): (&Vec<u8>, usize)) -> Self {
        let mut form = None;
        let mut id = vec_to_id(value, offset);
        if id == "FORM" {
            form = Some(id);
            id = vec_to_id(value, offset + 8);
        }

        let length = vec_to_u32(value, offset + 4, 4);
        let data = value[offset + 8..offset + 8 + length as usize].to_vec();

        Chunk::new(offset, form, id, &data)
    }
}

impl From<Chunk> for Vec<u8> {
    fn from(value: Chunk) -> Self {
        let mut v = Vec::new();

        // Chunk ID
        match value.form {
            Some(f) => {
                for b in f.as_bytes() {
                    v.push(*b);
                }
                v.append(&mut u32_to_vec(value.data.len() as u32 + 4, 4));
                for b in value.id.as_bytes() {
                    v.push(*b)
                }
            }
            None => {
                for b in value.id.as_bytes() {
                    v.push(*b);
                }

                v.append(&mut u32_to_vec(value.length, 4));
            }
        }

        // Data
        v.append(&mut value.data.clone());
        if value.data.len() % 2 == 1 {
            v.push(0);
        }

        v
    }
}

impl Chunk {
    pub fn new(offset: usize, form: Option<String>, id: String, data: &Vec<u8>) -> Chunk {
        Chunk {
            offset,
            form,
            id,
            length: data.len() as u32,
            data: data.clone(),
        }
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn form(&self) -> Option<&String> {
        self.form.as_ref()
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn length(&self) -> u32 {
        self.length
    }

    pub fn data(&self) -> &Vec<u8> {
        &self.data
    }
}

pub struct IFF {
    form: String,
    sub_form: String,
    chunks: Vec<Chunk>,
}

impl From<&Vec<u8>> for IFF {
    fn from(value: &Vec<u8>) -> Self {
        let form = vec_to_id(value, 0);
        let sub_form = vec_to_id(value, 8);
        let mut chunks = Vec::new();

        let mut offset = 12;
        let len = value.len();
        while offset < len - 1 {
            let chunk = Chunk::from((value, offset));
            let l = chunk.data.len();
            chunks.push(chunk);
            offset = offset + 8 + l;
            if l % 2 == 1 {
                offset += 1;
            }
        }

        IFF::new(form, sub_form, chunks)
    }
}

impl IFF {
    pub fn new(form: String, sub_form: String, chunks: Vec<Chunk>) -> IFF {
        IFF {
            form,
            sub_form,
            chunks,
        }
    }

    pub fn form(&self) -> &String {
        &self.form
    }

    pub fn sub_form(&self) -> &String {
        &self.sub_form
    }

    pub fn chunks(&self) -> &Vec<Chunk> {
        &self.chunks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usize_as_vec() {
        // usize_as_vec ignores anything above the specified byte count
        assert_eq!(usize_as_vec(0xAAFF, 1), &[0xFF]);
        assert_eq!(usize_as_vec(0xAA1234, 2), &[0x12, 0x34]);
        assert_eq!(usize_as_vec(0xAA123456, 3), &[0x12, 0x34, 0x56]);
        assert_eq!(usize_as_vec(0xAA12345678, 4), &[0x12, 0x34, 0x56, 0x78]);
    }

    #[test]
    fn test_vec_as_usize() {
        assert_eq!(vec_as_usize(vec![0xFF, 0xAA], 1), 0xFF);
        assert_eq!(vec_as_usize(vec![0x12, 0x34, 0xAA], 2), 0x1234);
        assert_eq!(vec_as_usize(vec![0x12, 0x34, 0x56, 0xAA], 3), 0x123456);
        assert_eq!(
            vec_as_usize(vec![0x12, 0x34, 0x56, 0x78, 0xAA], 4),
            0x12345678
        );
    }

    #[test]
    fn test_vec_to_u32() {
        assert_eq!(
            vec_to_u32(&[0xAA, 0x12, 0x34, 0x56, 0x78, 0xAA], 1, 4),
            0x12345678
        );
    }

    #[test]
    fn test_vec_to_id() {
        assert_eq!(vec_to_id(&[0xAA, b'F', b'O', b'R', b'M', 0xAA], 1), "FORM");
    }

    #[test]
    fn test_id_as_vec() {
        assert_eq!(id_as_vec("FORM"), &[b'F', b'O', b'R', b'M']);
    }

    #[test]
    fn test_chunk() {
        assert_eq!(
            chunk("FORM", &[1, 2, 3, 4, 5, 6, 7, 8]),
            &[
                b'F', b'O', b'R', b'M', 0x00, 0x00, 0x00, 0x08, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06,
                0x07, 0x08
            ]
        );
    }

    #[test]
    fn test_chunk_new() {
        let chunk = Chunk::new(
            0x1234,
            Some("FORM".to_string()),
            "ID  ".to_string(),
            &vec![0x1, 0x2, 0x3, 0x4],
        );
        assert_eq!(chunk.offset(), 0x1234);
        assert!(chunk.form().is_some());
        assert_eq!(chunk.form().unwrap(), "FORM");
        assert_eq!(chunk.id(), "ID  ");
        assert_eq!(chunk.length(), 4);
        assert_eq!(chunk.data(), &[1, 2, 3, 4]);
    }

    #[test]
    fn test_chunk_from_vec_u8() {
        let chunk = Chunk::from((
            &vec![
                0xAA, b'I', b'D', b' ', b' ', 0x00, 0x00, 0x00, 0x04, 0x01, 0x02, 0x03, 0x04,
            ],
            1,
        ));
        assert!(chunk.form().is_none());
        assert_eq!(chunk.id(), "ID  ");
        assert_eq!(chunk.length(), 4);
        assert_eq!(chunk.data(), &[1, 2, 3, 4]);

        let chunk = Chunk::from((
            &vec![
                0xAA, b'F', b'O', b'R', b'M', 0x00, 0x00, 0x00, 0x08, b'I', b'D', b' ', b' ', 0x01,
                0x02, 0x03, 0x04,
            ],
            1,
        ));
        assert!(chunk.form().is_some());
        assert_eq!(chunk.form().unwrap(), "FORM");
        assert_eq!(chunk.id(), "ID  ");
        assert_eq!(chunk.length(), 8);
        assert_eq!(chunk.data(), &[b'I', b'D', b' ', b' ', 1, 2, 3, 4]);
    }

    #[test]
    fn test_vec_u8_from_chunk() {
        let chunk = Chunk::new(0x1234, None, "ID  ".to_string(), &vec![0x1, 0x2, 0x3, 0x4]);
        let v = Vec::from(chunk);
        assert_eq!(
            v,
            &[b'I', b'D', b' ', b' ', 0x00, 0x00, 0x00, 0x04, 0x01, 0x02, 0x03, 0x04]
        );

        let chunk = Chunk::new(
            0x1234,
            Some("FORM".to_string()),
            "ID  ".to_string(),
            &vec![0x1, 0x2, 0x3, 0x4],
        );
        let v = Vec::from(chunk);
        assert_eq!(
            v,
            &[
                b'F', b'O', b'R', b'M', 0x00, 0x00, 0x00, 0x08, b'I', b'D', b' ', b' ', 0x01, 0x02,
                0x03, 0x04
            ]
        );
    }

    #[test]
    fn test_vec_u8_from_chunk_padding() {
        let chunk = Chunk::new(0x1234, None, "ID  ".to_string(), &vec![0x1, 0x2, 0x3]);
        let v = Vec::from(chunk);
        assert_eq!(
            v,
            &[b'I', b'D', b' ', b' ', 0x00, 0x00, 0x00, 0x03, 0x01, 0x02, 0x03, 0x00]
        );

        let chunk = Chunk::new(
            0x1234,
            Some("FORM".to_string()),
            "ID  ".to_string(),
            &vec![0x1, 0x2, 0x3, 0x4],
        );
        let v = Vec::from(chunk);
        assert_eq!(
            v,
            &[
                b'F', b'O', b'R', b'M', 0x00, 0x00, 0x00, 0x08, b'I', b'D', b' ', b' ', 0x01, 0x02,
                0x03, 0x04
            ]
        );
    }

    #[test]
    fn test_iff_new() {
        let c1 = Chunk::new(0x0c, None, "ID1 ".to_string(), &vec![0x1, 0x2, 0x3, 0x4]);
        let c2 = Chunk::new(0x14, None, "ID2 ".to_string(), &vec![0x5, 0x6, 0x7, 0x8]);
        let iff = IFF::new("FORM".to_string(), "TEST".to_string(), vec![c1, c2]);
        assert_eq!(iff.form(), "FORM");
        assert_eq!(iff.sub_form(), "TEST");
        assert_eq!(iff.chunks().len(), 2);
        assert!(iff.chunks()[0].form().is_none());
        assert_eq!(iff.chunks()[0].id(), "ID1 ");
        assert_eq!(iff.chunks()[0].length(), 4);
        assert_eq!(iff.chunks()[0].data(), &[1, 2, 3, 4]);
        assert!(iff.chunks()[1].form().is_none());
        assert_eq!(iff.chunks()[1].id(), "ID2 ");
        assert_eq!(iff.chunks()[1].length(), 4);
        assert_eq!(iff.chunks()[1].data(), &[5, 6, 7, 8]);
    }

    #[test]
    fn test_iff_from_vec() {
        let v = vec![
            b'F', b'O', b'R', b'M', 0x00, 0x00, 0x00, 0x20, b'T', b'E', b'S', b'T', b'C', b'k',
            b'1', b' ', 0x00, 0x00, 0x00, 0x08, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
            b'C', b'k', b'2', b' ', 0x00, 0x00, 0x00, 0x04, 0x10, 0x11, 0x12, 0x13,
        ];
        let iff = IFF::from(&v);
        assert_eq!(iff.form(), "FORM");
        assert_eq!(iff.sub_form(), "TEST");
        assert_eq!(iff.chunks().len(), 2);
        assert!(iff.chunks()[0].form().is_none());
        assert_eq!(iff.chunks()[0].id(), "Ck1 ");
        assert_eq!(iff.chunks()[0].length(), 8);
        assert_eq!(iff.chunks()[0].data(), &[1, 2, 3, 4, 5, 6, 7, 8]);
        assert!(iff.chunks()[1].form().is_none());
        assert_eq!(iff.chunks()[1].id(), "Ck2 ");
        assert_eq!(iff.chunks()[1].length(), 4);
        assert_eq!(iff.chunks()[1].data(), &[0x10, 0x11, 0x12, 0x13]);
    }
}
