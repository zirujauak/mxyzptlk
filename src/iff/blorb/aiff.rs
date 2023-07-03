use std::fmt;

use crate::iff;

use super::super::*;

pub struct AIFF {
    data: Vec<u8>,
}

impl fmt::Display for AIFF {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "AIFF data size: {}", self.data.len())
    }
}

impl From<Chunk> for AIFF {
    fn from(value: Chunk) -> AIFF {
        AIFF::new(value.data())
    }
}

impl From<&AIFF> for Vec<u8> {
    /// Reconstitutes a full AIFF file as a Vec<u8> from an AIFF struct.
    fn from(value: &AIFF) -> Vec<u8> {
        let mut v = Vec::new();
        v.append(&mut iff::id_as_vec("FORM"));
        v.append(&mut iff::u32_to_vec(value.data.len() as u32, 4));
        v.append(&mut value.data.clone());
        // Pad the buffer to an even number of bytes
        if v.len() % 2 == 1 {
            v.push(0);
        }
        v
    }
}

impl AIFF {
    /// Instantiate
    pub fn new(data: &[u8]) -> AIFF {
        AIFF {
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

    use super::AIFF;

    #[test]
    fn test_new() {
        let aiff = AIFF::new(&[0, 1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(aiff.data(), &vec![0, 1, 2, 3, 4, 5, 6, 7]);
    }

    #[test]
    fn test_from_aiff() {
        let chunk = Chunk::new(
            0,
            Some("FORM".to_string()),
            "AIFF".to_string(),
            &vec![0, 1, 2, 3, 4, 5, 6, 7],
        );
        let aiff = AIFF::from(chunk);
        assert_eq!(aiff.data(), &vec![0, 1, 2, 3, 4, 5, 6, 7]);
    }

    #[test]
    fn test_from_vec_u8() {
        let aiff = AIFF::new(&[0, 1, 2, 3, 4, 5, 6, 7]);
        let v = Vec::from(&aiff);
        assert_eq!(
            v,
            vec![0x46, 0x4F, 0x52, 0x4D, 0, 0, 0, 8, 0, 1, 2, 3, 4, 5, 6, 7]
        );
    }
}
