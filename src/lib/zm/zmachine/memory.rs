//! ZMachine [memory map](https://inform-fiction.org/zmachine/standards/z1point1/sect01.html)
use std::{fmt, fs::File, io::Read};

use crate::{error::*, fatal_error};

/// Memory map
pub struct Memory {
    /// Memory map bytes
    map: Vec<u8>,
    /// Byte address of the start of static memory
    static_mark: usize,
    /// File length from the header
    file_length: usize,
    /// Pristine copy of the dynamic memory region
    dynamic: Vec<u8>,
}

impl fmt::Debug for Memory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Memory: {} bytes", self.map.len())
    }
}

/// Assembly a word from high- and low-byte balues
///
/// # Arguments
/// * `hb` - high byte value
/// * `lb` - low byte value
///
/// # Returns
/// Word value
pub fn word_value(hb: u8, lb: u8) -> u16 {
    (((hb as u16) << 8) & 0xFF00) + ((lb as u16) & 0xFF)
}

/// Break a word value down into high- and low-byte values
///
/// # Arguments
/// * `w` - Word value
///
/// # Returns
/// Tuple containing (high-byte, low-byte)
fn byte_values(w: u16) -> (u8, u8) {
    let hb = (w >> 8) as u8;
    let lb = w as u8;
    (hb, lb)
}

impl TryFrom<&mut File> for Memory {
    type Error = RuntimeError;

    fn try_from(value: &mut File) -> Result<Self, Self::Error> {
        let mut d = Vec::new();
        match value.read_to_end(&mut d) {
            Ok(_) => Ok(Memory::new(d)),
            Err(e) => fatal_error!(ErrorCode::InvalidFile, "Error reading file: {}", e),
        }
    }
}

impl Memory {
    /// Constructor
    ///
    /// # Arguments
    /// * `map` - Vector of memory bytes
    pub fn new(map: Vec<u8>) -> Memory {
        let version = map[0];
        let static_mark = word_value(map[0x0e], map[0x0f]) as usize;
        let file_length = word_value(map[0x1a], map[0x1b]) as usize
            * match version {
                3 => 2,
                4 | 5 => 4,
                _ => 8,
            };

        let dynamic = map[0..static_mark].to_vec();
        Memory {
            static_mark,
            file_length,
            map,
            dynamic,
        }
    }

    /// Get the start of the [static](https://inform-fiction.org/zmachine/standards/z1point1/sect01.html#one) memory region
    ///
    /// # Returns
    /// Byte address of the start of the static memory region
    pub fn static_mark(&self) -> usize {
        self.static_mark
    }

    /// Copy a slice of the memory map
    ///
    /// # Arguments
    /// * `start` - address of the start of the slice
    /// * `length` - length of the slice
    ///
    /// # Returns
    /// Vector containing a copy of the requested slice of memory
    pub fn slice(&self, start: usize, length: usize) -> Vec<u8> {
        let end = usize::min(start + length, self.map.len());
        self.map[start..end].to_vec()
    }

    /// Calculate the checksum of the memory map.
    ///
    /// The pristine copy of dynamic memory is used for this calculation.
    ///
    /// # Returns
    /// [Result] with the checksum value or a [RuntimeError]
    pub fn checksum(&self) -> Result<u16, RuntimeError> {
        let mut checksum = 0;
        for i in 0x40..self.dynamic.len() {
            checksum = u16::overflowing_add(checksum, self.dynamic[i] as u16).0;
        }

        for i in self.dynamic.len()..self.file_length {
            checksum = u16::overflowing_add(checksum, self.map[i] as u16).0;
        }
        Ok(checksum)
    }

    /// Read a byte from the memory map.
    ///
    /// # Arguments
    /// * `address` - Address to read from
    ///
    /// # Returns
    /// [Result] with the byte value at the requested `address` or a [RuntimeError]
    pub fn read_byte(&self, address: usize) -> Result<u8, RuntimeError> {
        if address < self.map.len() {
            Ok(self.map[address])
        } else {
            fatal_error!(
                ErrorCode::InvalidAddress,
                "Byte address {:#06x} beyond end of memory ({:#06x})",
                address,
                self.map.len() - 1
            )
        }
    }

    /// Read a word from the memory map.
    ///
    /// # Arguments
    /// * `address` - Address to read from
    ///
    /// # Returns
    /// [Result] with the word value at the requested `address` or a [RuntimeError]
    pub fn read_word(&self, address: usize) -> Result<u16, RuntimeError> {
        if address < self.map.len() - 1 {
            Ok(word_value(self.map[address], self.map[address + 1]))
        } else {
            fatal_error!(
                ErrorCode::InvalidAddress,
                "Word address {:#06x} beyond end of memory ({:#06x})",
                address,
                self.map.len() - 1
            )
        }
    }

    /// Write a byte to the memory map.
    ///
    /// # Arguments
    /// * `address` - Address to write to
    /// * `value` - Byte value to write
    ///
    /// # Returns
    /// Empty [Result] or a [RuntimeError]
    pub fn write_byte(&mut self, address: usize, value: u8) -> Result<(), RuntimeError> {
        if address < self.map.len() {
            debug!(target: "app::state", "Write {:#02x} to ${:04x}", value, address);
            self.map[address] = value;
            Ok(())
        } else {
            fatal_error!(
                ErrorCode::InvalidAddress,
                "Byte address {:#06x} beyond end of memory ({:#06x})",
                address,
                self.map.len() - 1
            )
        }
    }

    /// Write a word to the memory map.
    ///
    /// # Arguments
    /// * `address` - Address to write to
    /// * `value` - Word value to write
    ///
    /// # Returns
    /// Empty [Result] or a [RuntimeError]    
    pub fn write_word(&mut self, address: usize, value: u16) -> Result<(), RuntimeError> {
        if address < self.map.len() - 2 {
            debug!(target: "app::state", "Write {:#04x} to ${:04x}", value, address);
            let (hb, lb) = byte_values(value);
            self.map[address] = hb;
            self.map[address + 1] = lb;
            Ok(())
        } else {
            fatal_error!(
                ErrorCode::InvalidAddress,
                "Word address {:#06x} beyond end of memory ({:#06x})",
                address,
                self.map.len() - 1
            )
        }
    }

    /// Perform basic [RLE compression](http://inform-fiction.org/zmachine/standards/quetzal/index.html#three) of the current state of dynamic memory
    ///
    /// # Returns
    /// Vector containing the compressed contents of the dynamic memory region
    pub fn compress(&self) -> Vec<u8> {
        let mut cdata: Vec<u8> = Vec::new();
        let mut run_length = 0;
        let dynamic_len = self.dynamic.len();
        for i in 0..dynamic_len {
            let b = self.map[i] ^ self.dynamic[i];
            if b == 0 {
                if run_length == 255 {
                    cdata.push(0);
                    cdata.push(run_length);
                    run_length = 0;
                } else {
                    run_length += 1;
                }
            } else {
                if run_length > 0 {
                    cdata.push(0);
                    cdata.push(run_length - 1);
                    run_length = 0;
                }
                cdata.push(b);
            }
        }

        if run_length > 0 {
            cdata.push(0);
            cdata.push(run_length - 1);
        }

        cdata
    }

    /// Reset dynamic memory back to the initial state
    pub fn reset(&mut self) {
        self.map[..][..self.dynamic.len()].copy_from_slice(&self.dynamic)
    }

    /// Replace dynamic memory, presumably from a saved game state
    ///
    /// # Arguments
    /// * `data` - Dynamic memory region to restore
    ///
    /// # Returns
    /// Empty [Result] or a [RuntimeError]
    pub fn restore(&mut self, data: &Vec<u8>) -> Result<(), RuntimeError> {
        if data.len() != self.dynamic.len() {
            fatal_error!(
                ErrorCode::Restore,
                "Restore dynamic memory size doesn't match: {:04x} != {:04x}",
                self.dynamic.len(),
                data.len()
            )
        } else {
            self.map[..][..data.len()].copy_from_slice(data);
            Ok(())
        }
    }

    /// Decompress a compressed dynamic memory region
    ///
    /// # Arguments
    /// * `cdata` - Compressed data, as per the [compress](#method.compress) function
    ///
    /// # returns
    /// Vector containing uncompressed data
    fn decompress(&self, cdata: &[u8]) -> Vec<u8> {
        let mut data = Vec::new();
        let mut iter = cdata.iter();
        let mut done = false;

        while !done {
            let b = iter.next();
            match b {
                Some(b) => {
                    let i = data.len();
                    if *b == 0 {
                        let l = *iter.next().expect("Incomplete CMem 0 run") as usize;
                        for j in 0..l + 1 {
                            data.push(self.dynamic[i + j]);
                        }
                    } else {
                        data.push(b ^ self.dynamic[i])
                    }
                }
                None => done = true,
            }
        }

        data
    }

    /// Restore the dynamic region from a compressed memory slice
    ///
    /// # Arguments
    /// * `cdata` - Compressed data
    ///
    /// # Returns
    /// Empty [Result] or a [RuntimeError]
    pub fn restore_compressed(&mut self, cdata: &[u8]) -> Result<(), RuntimeError> {
        let data = self.decompress(cdata);
        self.restore(&data)
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, io::Write, path::Path};

    use crate::{assert_ok, assert_ok_eq};

    use super::*;

    #[test]
    fn test_word_value() {
        for i in 0..=0xFFFF {
            let bytes = (i as u32).to_be_bytes();
            assert_eq!(word_value(bytes[2], bytes[3]), i as u16);
        }
    }

    #[test]
    fn test_byte_values() {
        for i in 0..=0xFFFF {
            let bytes = (i as u32).to_be_bytes();
            assert_eq!(byte_values(i), (bytes[2], bytes[3]));
        }
    }

    #[test]
    fn test_from_file() {
        let mut map = vec![0; 0x800];
        map[0] = 5;
        map[0xE] = 0x4;
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        let mut file = assert_ok!(fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open("test-code.z5"));
        assert!(file.write_all(&map).is_ok());
        assert!(file.flush().is_ok());
        assert!(Path::new("test-code.z5").exists());
        let read_file = fs::OpenOptions::new().read(true).open("test-code.z5");
        let mut rf = assert_ok!(read_file);
        let m = assert_ok!(Memory::try_from(&mut rf));
        assert!(fs::remove_file("test-code.z5").is_ok());
        assert_ok_eq!(m.read_byte(0), 5);
        assert_ok_eq!(m.read_word(0xE), 0x400);
        for i in 1..0x40 {
            if i != 0x0E && i != 0x0F {
                assert_ok_eq!(m.read_byte(i), 0);
            }
        }
        for i in 0x40..0x800 {
            assert_ok_eq!(m.read_byte(i), i as u8);
        }

        assert_eq!(m.dynamic.len(), 0x400);
        for i in 0..0x400 {
            assert_ok_eq!(m.read_byte(i), m.dynamic[i]);
        }
    }

    #[test]
    fn test_new() {
        let mut map = vec![0; 0x800];
        map[0] = 5;
        map[0xE] = 0x4;
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        let m = Memory::new(map);
        assert_ok_eq!(m.read_byte(0), 5);
        assert_ok_eq!(m.read_word(0xE), 0x400);
        for i in 1..0x40 {
            if i != 0x0E && i != 0x0F {
                assert_ok_eq!(m.read_byte(i), 0);
            }
        }
        for i in 0x40..0x800 {
            assert_ok_eq!(m.read_byte(i), i as u8);
        }

        assert_eq!(m.dynamic.len(), 0x400);
        for i in 0..0x400 {
            assert_ok_eq!(m.read_byte(i), m.dynamic[i]);
        }
    }

    // #[test]
    // fn test_size() {
    //     let mut map = vec![0; 0x800];
    //     map[0] = 5;
    //     map[0xE] = 0x4;
    //     for (i, b) in (0x40..0x800).enumerate() {
    //         map[i + 0x40] = b as u8;
    //     }
    //     let m = Memory::new(map);
    //     assert_eq!(m.size(), 0x800);
    // }

    #[test]
    fn test_slice() {
        let mut map = vec![0; 0x800];
        map[0] = 5;
        map[0xE] = 0x4;
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        let m = Memory::new(map);
        let s = m.slice(0x400, 0x10);
        assert_eq!(s.len(), 0x10);
        for (i, b) in (0..0x10).enumerate() {
            assert_eq!(s[i], b);
        }
    }

    #[test]
    fn test_checksum_v3() {
        let mut map = vec![0; 0x800];
        map[0] = 3;
        map[0xE] = 0x4;
        map[0x1A] = 0x4;
        map[0x1B] = 0;
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        let m = Memory::new(map);
        assert_ok_eq!(m.checksum(), 0xf420);
    }

    #[test]
    fn test_checksum_v4() {
        let mut map = vec![0; 0x800];
        map[0] = 4;
        map[0xE] = 0x4;
        map[0x1A] = 0x2;
        map[0x1B] = 0;
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        let m = Memory::new(map);
        assert_ok_eq!(m.checksum(), 0xf420);
    }

    #[test]
    fn test_checksum_v5() {
        let mut map = vec![0; 0x800];
        map[0] = 5;
        map[0xE] = 0x4;
        map[0x1A] = 0x2;
        map[0x1B] = 0;
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        let m = Memory::new(map);
        assert_ok_eq!(m.checksum(), 0xf420);
    }

    #[test]
    fn test_checksum_v8() {
        let mut map = vec![0; 0x800];
        map[0] = 8;
        map[0xE] = 0x4;
        map[0x1A] = 0x1;
        map[0x1B] = 0;
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        let m = Memory::new(map);
        assert_ok_eq!(m.checksum(), 0xf420);
    }

    #[test]
    fn test_read_byte() {
        let mut map = vec![0; 0x800];
        map[0] = 8;
        map[0xE] = 0x4;
        map[0x1A] = 0x1;
        map[0x1B] = 0;
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        let m = Memory::new(map);
        for i in 0x40..0x800 {
            assert_ok_eq!(m.read_byte(i), i as u8);
        }

        assert!(m.read_byte(0x800).is_err());
    }

    #[test]
    fn test_read_word() {
        let mut map = vec![0; 0x800];
        map[0] = 8;
        map[0xE] = 0x4;
        map[0x1A] = 0x1;
        map[0x1B] = 0;
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        let m = Memory::new(map);
        for i in 0x40..0x7FF {
            let w = word_value(i as u8, u8::overflowing_add(i as u8, 1).0);
            assert_ok_eq!(m.read_word(i), w);
        }

        assert!(m.read_word(0x7FF).is_err());
    }

    #[test]
    fn test_write_byte() {
        let mut map = vec![0; 0x800];
        map[0] = 8;
        map[0xE] = 0x4;
        map[0x1A] = 0x1;
        map[0x1B] = 0;
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        let mut m = Memory::new(map);
        for i in 0x40..0x80 {
            assert!(m.write_byte(i, i as u8 + 1).is_ok());
        }
        assert_ok_eq!(m.read_byte(0x39), 0);
        for i in 0x40..0x80 {
            assert_ok_eq!(m.read_byte(i), i as u8 + 1);
        }
        assert_ok_eq!(m.read_byte(0x81), 0x81);

        assert!(m.write_byte(0x800, 0).is_err());
    }

    #[test]
    fn test_write_word() {
        let mut map = vec![0; 0x800];
        map[0] = 8;
        map[0xE] = 0x4;
        map[0x1A] = 0x1;
        map[0x1B] = 0;
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        let mut m = Memory::new(map);
        for i in 0x20..0x40 {
            assert!(m.write_word(i * 2, i as u16 * 0x10).is_ok());
        }
        assert_ok_eq!(m.read_word(0x38), 0);
        for i in 0x20..0x40 {
            assert_ok_eq!(m.read_word(i * 2), i as u16 * 0x10);
        }
        assert_ok_eq!(m.read_word(0x81), 0x8182);

        assert!(m.write_word(0x7FF, 0).is_err());
    }

    #[test]
    fn test_compress() {
        let mut map = vec![0; 0x800];
        map[0] = 8;
        map[0xE] = 0x4;
        map[0x1A] = 0x1;
        map[0x1B] = 0;
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        let mut m = Memory::new(map);
        // Change dynamic memory just a bit
        assert!(m.write_byte(0x200, 0xFC).is_ok());
        assert!(m.write_byte(0x280, 0x10).is_ok());
        assert!(m.write_byte(0x300, 0xFD).is_ok());
        // 0x0000 - 0x0100 is unchanged: 0x00, 0xFF
        // 0x0100 - 0x01FF is unchanged: 0x00, 0xFE
        // 0x0201 is changed: 0xFC
        // 0x0202 - 0x027F is unchanged: 0x00, 0x7E
        // 0x0280 is changed: 0x10 ^ 0x80 = 0x90
        // 0x0281 - 0x02FF is unchanged: 0x00, 0x7E
        // 0x0301 is changed: 0xFD
        // 0x0302 - 0x03FF is unchanged: 0x00, 0xFE
        assert_eq!(
            m.compress(),
            vec![0x00, 0xFF, 0x00, 0xFF, 0xFC, 0x00, 0x7E, 0x90, 0x00, 0x7E, 0xFD, 0x00, 0xFE]
        );
    }

    #[test]
    fn test_reset() {
        let mut map = vec![0; 0x800];
        map[0] = 8;
        map[0xE] = 0x4;
        map[0x1A] = 0x1;
        map[0x1B] = 0;
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        let mut m = Memory::new(map);
        for i in 0x40..0x400 {
            assert!(m.write_byte(i, 0).is_ok());
        }
        for i in 0x40..0x400 {
            assert_ok_eq!(m.read_byte(i), 0)
        }
        m.reset();
        for i in 0x40..0x400 {
            assert_ok_eq!(m.read_byte(i), i as u8)
        }
    }

    #[test]
    fn test_restore() {
        let mut map = vec![0; 0x800];
        map[0] = 8;
        map[0xE] = 0x4;
        map[0x1A] = 0x1;
        map[0x1B] = 0;
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        let mut restore = vec![0; 0x400];
        for (i, b) in map[0..0x40].iter().enumerate() {
            restore[i] = *b;
        }
        for (i, b) in (0x40..0x400).enumerate() {
            restore[i + 0x40] = !(b as u8);
        }
        let mut m = Memory::new(map.clone());
        for i in 0x40..0x400 {
            assert_ok_eq!(m.read_byte(i), i as u8)
        }
        assert!(m.restore(&restore).is_ok());
        for (i, _) in (0..0x40).enumerate() {
            assert_ok_eq!(m.read_byte(i), map[i]);
        }
        for i in 0x40..0x400 {
            assert_ok_eq!(m.read_byte(i), !(i as u8));
        }
    }

    #[test]
    fn test_decompress() {
        let mut map = vec![0; 0x800];
        map[0] = 8;
        map[0xE] = 0x4;
        map[0x1A] = 0x1;
        map[0x1B] = 0;
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        let m = Memory::new(map.clone());
        let dc = m.decompress(&[
            0x00, 0xFF, 0x00, 0xFF, 0xFC, 0x00, 0x7E, 0x90, 0x00, 0x7E, 0xFD, 0x00, 0xFE,
        ]);
        assert_eq!(dc[0..0x200], map[0..0x200]);
        assert_eq!(dc[0x200], 0xFC);
        assert_eq!(dc[0x201..0x280], map[0x201..0x280]);
        assert_eq!(dc[0x280], 0x10);
        assert_eq!(dc[0x281..0x300], map[0x281..0x300]);
        assert_eq!(dc[0x300], 0xFD);
        assert_eq!(dc[0x301..], map[0x301..0x400]);
    }

    #[test]
    fn test_restore_compressed() {
        let mut map = vec![0; 0x800];
        map[0] = 8;
        map[0xE] = 0x4;
        map[0x1A] = 0x1;
        map[0x1B] = 0;
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        let mut m = Memory::new(map.clone());
        assert!(m
            .restore_compressed(&[
                0x00, 0xFF, 0x00, 0xFF, 0xFC, 0x00, 0x7E, 0x90, 0x00, 0x7E, 0xFD, 0x00, 0xFE,
            ])
            .is_ok());
        for (i, _) in (0..0x200).enumerate() {
            assert_ok_eq!(
                m.read_byte(i),
                map[i],
                "{:04x}: {:?}/{}",
                i,
                m.read_byte(i),
                map[i]
            );
        }
        for (i, _) in (0x201..0x280).enumerate() {
            let offset = i + 0x201;
            assert_ok_eq!(
                m.read_byte(offset),
                map[offset],
                "{:04x}: {:?}/{}",
                offset,
                m.read_byte(offset),
                map[offset]
            );
        }
        for (i, _) in (0x281..0x300).enumerate() {
            let offset = i + 0x281;
            assert_ok_eq!(
                m.read_byte(offset),
                map[offset],
                "{:04x}: {:?}/{}",
                offset,
                m.read_byte(offset),
                map[offset]
            );
        }
        for (i, _) in (0x301..0x800).enumerate() {
            let offset = i + 0x301;
            assert_ok_eq!(
                m.read_byte(offset),
                map[offset],
                "{:04x}: {:?}/{}",
                offset,
                m.read_byte(offset),
                map[offset]
            );
        }
        assert_ok_eq!(m.read_byte(0x200), 0xFC);
        assert_ok_eq!(m.read_byte(0x280), 0x10);
        assert_ok_eq!(m.read_byte(0x300), 0xFD);
    }
}
