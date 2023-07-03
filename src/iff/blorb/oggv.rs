use std::fmt;

use super::super::*;

pub struct OGGV {
    data: Vec<u8>,
}

impl fmt::Display for OGGV {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "OGG Vorbis data size: {}", self.data.len())
    }
}

impl From<Chunk> for OGGV {
    fn from(value: Chunk) -> OGGV {
        OGGV::new(value.data())
    }
}

impl OGGV {
    pub fn new(data: &[u8]) -> OGGV {
        OGGV {
            data: data.to_vec(),
        }
    }

    pub fn data(&self) -> &Vec<u8> {
        &self.data
    }
}

#[cfg(test)]
mod tests {
    use crate::iff::Chunk;

    use super::OGGV;

    #[test]
    fn test_new() {
        let oggv = OGGV::new(&[0, 1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(oggv.data(), &vec![0, 1, 2, 3, 4, 5, 6, 7]);
    }

    #[test]
    fn test_from_chunk() {
        let chunk = Chunk::new(
            0,
            Some("FORM".to_string()),
            "OGGV".to_string(),
            &vec![0, 1, 2, 3, 4, 5, 6, 7],
        );
        let oggv = OGGV::from(chunk);
        assert_eq!(oggv.data(), &vec![0, 1, 2, 3, 4, 5, 6, 7]);
    }
}
