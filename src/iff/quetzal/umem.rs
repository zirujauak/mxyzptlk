use std::fmt;

use super::super::*;

pub struct UMem {
    data: Vec<u8>,
}

impl fmt::Display for UMem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "uncompressed data size: {}", self.data.len())
    }
}

impl From<Vec<u8>> for UMem {
    fn from(value: Vec<u8>) -> UMem {
        UMem::new(&value)
    }
}

impl From<Chunk> for UMem {
    fn from(value: Chunk) -> UMem {
        UMem::new(value.data())
    }
}

impl From<&UMem> for Vec<u8> {
    fn from(value: &UMem) -> Vec<u8> {
        chunk("UMem", value.data())
    }
}

impl UMem {
    pub fn new(data: &[u8]) -> UMem {
        UMem {
            data: data.to_vec(),
        }
    }

    pub fn data(&self) -> &Vec<u8> {
        &self.data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let c = UMem::new(&[1, 2, 3, 4]);
        assert_eq!(c.data(), &[1, 2, 3, 4]);
    }

    #[test]
    fn test_from_vec_u8() {
        let v = vec![1, 2, 3, 4];
        let c = UMem::from(v);
        assert_eq!(c.data(), &[1, 2, 3, 4]);
    }

    #[test]
    fn test_from_chunk() {
        let chunk = Chunk::new(
            0,
            Some("FORM".to_string()),
            "CMem".to_string(),
            &vec![1, 2, 3, 4],
        );
        let c = UMem::from(chunk);
        assert_eq!(c.data(), &[1, 2, 3, 4])
    }
}
