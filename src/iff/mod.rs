use crate::error::{ErrorCode, RuntimeError};

pub mod blorb;
pub mod quetzal;

pub fn usize_as_vec(d: usize, bytes: usize) -> Vec<u8> {
    let mut data = Vec::new();
    for i in (0..bytes).rev() {
        data.push(((d >> (8 * i)) & 0xFF) as u8);
    }
    data
}

pub fn vec_as_usize(v: Vec<u8>, bytes: usize) -> usize {
    let mut u: usize = 0;
    for i in 0..bytes {
        u = u | ((v[i] as usize) << ((bytes - 1 - i) * 8));
    }

    u
}

pub fn vec_to_u32(v: &Vec<u8>, offset: usize, bytes: usize) -> u32 {
    let mut u: u32 = 0;
    for i in 0..bytes {
        u = u | ((v[offset + i] as u32) << ((bytes - i - 1) * 8));
    }
    u
}

pub fn u32_to_vec(d: u32, bytes: usize) -> Vec<u8> {
    let mut v = Vec::new();
    for i in (0..bytes).rev() {
        v.push(((d >> (8 * i)) & 0xFF) as u8);
    }
    v
}

pub fn vec_to_id(v: &Vec<u8>, offset: usize) -> String {
    let mut id = String::new();
    for i in 0..4 {
        id.push(v[offset + i] as char);
    }

    id
}

pub fn id_as_vec(id: &str) -> Vec<u8> {
    id.as_bytes()[0..4].to_vec()
}

pub fn chunk(id: &str, data: &mut Vec<u8>) -> Vec<u8> {
    let mut chunk = id_as_vec(id);
    let data_length = data.len();
    chunk.append(&mut usize_as_vec(data.len(), 4));
    chunk.append(data);
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

impl Chunk {
    pub fn new(offset: usize, form: Option<String>, id: String, data: &Vec<u8>) -> Chunk {
        let length = data.len() as u32 + if data.len() % 2 == 1 { 1 } else { 0 };
        Chunk {
            offset,
            form,
            id,
            length,
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

    pub fn from_vec(vec: &Vec<u8>, offset: usize) -> Chunk {
        let mut form = None;
        let mut id = vec_to_id(&vec, offset);
        if id == "FORM" {
            form = Some(id);
            id = vec_to_id(&vec, offset + 8);
        }

        let length = vec_to_u32(&vec, offset + 4, 4);
        let data = vec[offset + 8..offset + 8 + length as usize].to_vec();

        Chunk {
            offset,
            form,
            id,
            length,
            data,
        }
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut v = Vec::new();

        // Chunk ID
        match &self.form {
            Some(f) => {
                for b in f.as_bytes() {
                    v.push(*b);
                }
                v.append(&mut u32_to_vec(self.length, 4));
                for b in self.id.as_bytes() {
                    v.push(*b)
                }
            }
            None => {
                for b in self.id.as_bytes() {
                    v.push(*b);
                }

                v.append(&mut u32_to_vec(self.length, 4));
            }
        }

        // Data
        v.append(&mut self.data.clone());

        if self.data.len() % 2 == 1 {
            v.push(0);
        }

        v
    }
}

pub struct IFF {
    form: String,
    _length: u32,
    sub_form: String,
    chunks: Vec<Chunk>,
}

impl IFF {
    pub fn form_from_vec(v: &Vec<u8>) -> Result<String, RuntimeError> {
        if v.len() < 4 {
            Err(RuntimeError::new(
                ErrorCode::IFF,
                "Not an IFF file".to_string(),
            ))
        } else {
            Ok(vec_to_id(v, 0))
        }
    }

    pub fn from_vec(v: &Vec<u8>) -> IFF {
        let form = vec_to_id(v, 0);
        let length = vec_to_u32(v, 4, 4);
        let sub_form = vec_to_id(v, 8);
        let mut chunks = Vec::new();

        let mut offset = 12;
        let len = v.len();
        while offset < len - 1 {
            let chunk = Chunk::from_vec(v, offset);
            let l = chunk.data.len();
            chunks.push(chunk);
            offset = offset + 8 + l;
            if l % 2 == 1 {
                offset = offset + 1;
            }
        }

        IFF {
            form,
            _length: length,
            sub_form,
            chunks,
        }
    }
}
