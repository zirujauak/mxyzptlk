use std::{collections::HashMap, fs::File};

use iff::Chunk;

use crate::{
    error::{ErrorCode, RuntimeError},
    fatal_error,
};

#[derive(Clone, Debug)]
pub struct IFhd {
    release_number: u16,
    serial_number: Vec<u8>,
    checksum: u16,
    pc: u32,
}

impl IFhd {
    pub fn new(release_number: u16, serial_number: &[u8], checksum: u16, pc: u32) -> IFhd {
        IFhd {
            release_number,
            serial_number: serial_number.to_vec(),
            checksum,
            pc,
        }
    }

    pub fn release_number(&self) -> u16 {
        self.release_number
    }

    pub fn serial_number(&self) -> &Vec<u8> {
        &self.serial_number
    }

    pub fn checksum(&self) -> u16 {
        self.checksum
    }

    pub fn pc(&self) -> u32 {
        self.pc
    }
}

impl PartialEq for IFhd {
    fn eq(&self, other: &Self) -> bool {
        // Check everything but the PC, which will vary
        self.release_number == other.release_number
            && self.serial_number == other.serial_number
            && self.checksum == other.checksum
    }
}

impl TryFrom<&Chunk> for IFhd {
    type Error = RuntimeError;

    fn try_from(value: &Chunk) -> Result<Self, Self::Error> {
        if value.id() != "IFhd" {
            fatal_error!(
                ErrorCode::System,
                "Chunk ID is not 'IFhd': '{}'",
                value.id(),
            )
        } else if value.length() < 13 {
            fatal_error!(
                ErrorCode::System,
                "Chunk data should be (at least) 13 bytes: {}",
                value.length()
            )
        } else {
            let data = value.data();
            let release_number = iff::vec_as_unsigned(&data[0..2]) as u16;
            let serial_number = data[2..8].to_vec();
            let checksum = iff::vec_as_unsigned(&data[8..10]) as u16;
            let pc = iff::vec_as_unsigned(&data[10..13]) as u32;

            Ok(IFhd {
                release_number,
                serial_number,
                checksum,
                pc,
            })
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Index {
    usage: String,
    number: u32,
    start: u32,
}

impl Index {
    pub fn new(usage: String, number: u32, start: u32) -> Index {
        Index {
            usage,
            number,
            start,
        }
    }

    pub fn usage(&self) -> &String {
        &self.usage
    }

    pub fn number(&self) -> u32 {
        self.number
    }

    pub fn start(&self) -> u32 {
        self.start
    }
}

impl TryFrom<&[u8]> for Index {
    type Error = RuntimeError;
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() != 12 {
            fatal_error!(
                ErrorCode::System,
                "Index entry should be 12 bytes: {}",
                value.len()
            )
        } else {
            let usage = value[0..4].iter().map(|x| *x as char).collect::<String>();
            let number = iff::vec_as_unsigned(&value[4..8]) as u32;
            let start = iff::vec_as_unsigned(&value[8..12]) as u32;
            Ok(Index::new(usage, number, start))
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RIdx {
    indices: Vec<Index>,
}

impl RIdx {
    pub fn new(indices: Vec<Index>) -> RIdx {
        RIdx { indices }
    }

    pub fn indices(&self) -> &Vec<Index> {
        &self.indices
    }
}

impl TryFrom<&Chunk> for RIdx {
    type Error = RuntimeError;

    fn try_from(value: &Chunk) -> Result<Self, Self::Error> {
        if value.id() != "RIdx" {
            fatal_error!(ErrorCode::System, "Chunk is not 'RIdx': '{}'", value.id())
        } else {
            let data = value.data();
            let count = iff::vec_as_unsigned(&data[0..4]);
            if data.len() != 4 + (count * 12) {
                fatal_error!(
                    ErrorCode::System,
                    "Chunk data size should be {} for {} entries: {}",
                    4 + (count * 12),
                    count,
                    value.length()
                )
            } else {
                let mut indices = Vec::new();
                for i in 0..count {
                    let offset = 4 + (i * 12);
                    indices.push(Index::try_from(&data[offset..offset + 12])?)
                }

                Ok(RIdx::new(indices))
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Entry {
    number: u32,
    repeats: u32,
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

impl TryFrom<&[u8]> for Entry {
    type Error = RuntimeError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() != 8 {
            fatal_error!(
                ErrorCode::System,
                "Entry data should be 8 bytes: {}",
                value.len()
            )
        } else {
            let number = iff::vec_as_unsigned(&value[0..4]) as u32;
            let repeats = iff::vec_as_unsigned(&value[4..8]) as u32;
            Ok(Entry::new(number, repeats))
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Loop {
    entries: Vec<Entry>,
}

impl Loop {
    pub fn new(entries: Vec<Entry>) -> Loop {
        Loop { entries }
    }

    pub fn entries(&self) -> &Vec<Entry> {
        &self.entries
    }
}

impl TryFrom<&Chunk> for Loop {
    type Error = RuntimeError;

    fn try_from(value: &Chunk) -> Result<Self, Self::Error> {
        if value.id() != "Loop" {
            fatal_error!(
                ErrorCode::System,
                "Chunk id is not 'Loop': '{}'",
                value.id()
            )
        } else if value.length() % 8 != 0 {
            fatal_error!(
                ErrorCode::System,
                "Chunk data length should be a multiple of 8: '{}'",
                value.length()
            )
        } else {
            let data = value.data();
            let mut offset = 0;
            let mut entries = Vec::new();
            while data.len() > offset {
                entries.push(Entry::try_from(&data[offset..offset + 8])?);
                offset += 8;
            }

            Ok(Loop::new(entries))
        }
    }
}

#[derive(Debug)]
pub struct Blorb {
    ridx: RIdx,
    ifhd: Option<IFhd>,
    sounds: HashMap<u32, Chunk>,
    loops: Option<Loop>,
    exec: Option<Vec<u8>>,
}

impl Blorb {
    pub fn new(
        ridx: RIdx,
        ifhd: Option<IFhd>,
        sounds: HashMap<u32, Chunk>,
        loops: Option<Loop>,
        exec: Option<Vec<u8>>,
    ) -> Blorb {
        Blorb {
            ridx,
            ifhd,
            sounds,
            loops,
            exec,
        }
    }

    pub fn ridx(&self) -> &RIdx {
        &self.ridx
    }

    pub fn ifhd(&self) -> Option<&IFhd> {
        self.ifhd.as_ref()
    }

    pub fn sounds(&self) -> &HashMap<u32, Chunk> {
        &self.sounds
    }

    pub fn loops(&self) -> Option<&Loop> {
        self.loops.as_ref()
    }

    pub fn exec(&self) -> Option<&Vec<u8>> {
        self.exec.as_ref()
    }
}

impl TryFrom<&Chunk> for Blorb {
    type Error = RuntimeError;

    fn try_from(value: &Chunk) -> Result<Self, Self::Error> {
        if value.id() != "FORM" || value.sub_id() != "IFRS" {
            fatal_error!(
                ErrorCode::System,
                "Expected 'FORM'/'IFRS': '{}'/'{}'",
                value.id(),
                value.sub_id()
            )
        } else {
            let ifhd_chunk = value.find_chunk("IFhd", "");
            let ifhd = match ifhd_chunk {
                Some(i) => match IFhd::try_from(i) {
                    Ok(i) => Some(i),
                    Err(e) => {
                        error!(target: "app::sound", "Error reading IFhd chunk: {}", e);
                        warn!(target: "app::sound", "Unable to extract IFhd chunk, unabled to verify relation to game");
                        None
                    }
                },
                None => {
                    warn!(target: "app::sound", "No IFhd chunk found, unable to verify relation to game");
                    None
                }
            };

            let ridx_chunk = value.find_chunk("RIdx", "");
            if ridx_chunk.is_none() {
                return fatal_error!(ErrorCode::System, "No RIdx chunk");
            }
            let ridx = RIdx::try_from(ridx_chunk.unwrap())?;

            let loop_chunk = value.find_chunk("Loop", "");
            let loops = match loop_chunk {
                Some(l) => Some(Loop::try_from(l)?),
                None => None,
            };
            let oggv_chunks = value.find_chunks("OGGV", "");
            let aiff_chunks = value.find_chunks("FORM", "AIFF");

            // Look for an index with usage 'Exec'
            let execs: Vec<&Index> = ridx
                .indices()
                .iter()
                .filter(|x| x.usage() == "Exec" && x.number() == 0)
                .collect();
            let exec = if execs.len() == 1 {
                if execs[0].number() != 0 {
                    warn!("Exec index should have number '0': {}", execs[0].number());
                    None
                } else {
                    match value.find_chunk("ZCOD", "") {
                        Some(e) => {
                            if e.offset() == execs[0].start() {
                                Some(e.data().clone())
                            } else {
                                warn!(target: "app::trace", "'Exec' resources should start at {:06x}, but the ZCOD chunk starts at {:06}, therefore ignoring it", execs[0].start, e.offset());
                                None
                            }
                        }
                        None => {
                            warn!(target: "app::trace", "'Exec' resource index exists, but no ZCOD chunk found");
                            None
                        }
                    }
                }
            } else {
                None
            };

            let mut sounds = HashMap::new();
            for c in oggv_chunks {
                sounds.insert(c.offset(), c.clone());
            }
            for c in aiff_chunks {
                sounds.insert(c.offset(), c.clone());
            }

            Ok(Blorb {
                ifhd,
                ridx,
                sounds,
                loops,
                exec,
            })
        }
    }
}

impl TryFrom<Vec<u8>> for Blorb {
    type Error = RuntimeError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        let chunk = Chunk::from(&value);
        Blorb::try_from(&chunk)
    }
}

impl TryFrom<&mut File> for Blorb {
    type Error = RuntimeError;

    fn try_from(value: &mut File) -> Result<Self, Self::Error> {
        match Chunk::try_from(value) {
            Ok(c) => Blorb::try_from(&c),
            Err(e) => fatal_error!(ErrorCode::System, "Error opening file: {}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, io::Write, path::Path};

    use crate::{assert_ok, assert_some_eq};

    use super::*;

    #[test]
    fn test_ifhd_constructor() {
        let ifhd = IFhd::new(
            0x1234,
            &[0x32, 0x33, 0x30, 0x37, 0x32, 0x32],
            0x5678,
            0x9abcde,
        );

        assert_eq!(ifhd.release_number(), 0x1234);
        assert_eq!(ifhd.serial_number(), &[0x32, 0x33, 0x30, 0x37, 0x32, 0x32]);
        assert_eq!(ifhd.checksum(), 0x5678);
        assert_eq!(ifhd.pc(), 0x9abcde);
    }

    #[test]
    fn test_ifhd_partial_eq() {
        let i1 = IFhd::new(
            0x1234,
            &[0x32, 0x33, 0x30, 0x37, 0x32, 0x32],
            0x5678,
            0x9abcde,
        );
        let i2 = IFhd::new(
            0x1234,
            &[0x32, 0x33, 0x30, 0x37, 0x32, 0x32],
            0x5678,
            0x9abcdf,
        );
        let i3 = IFhd::new(
            0x1235,
            &[0x32, 0x33, 0x30, 0x37, 0x32, 0x32],
            0x5678,
            0x9abcde,
        );
        let i4 = IFhd::new(
            0x1234,
            &[0x32, 0x33, 0x30, 0x37, 0x32, 0x33],
            0x5678,
            0x9abcde,
        );
        let i5 = IFhd::new(
            0x1234,
            &[0x32, 0x33, 0x30, 0x37, 0x32, 0x32],
            0x5679,
            0x9abcde,
        );
        assert_eq!(i1, i1);
        assert_eq!(i1, i2);
        assert_eq!(i2, i1);
        assert_ne!(i1, i3);
        assert_ne!(i3, i1);
        assert_ne!(i1, i4);
        assert_ne!(i1, i5);
    }

    #[test]
    fn test_ifhd_try_from_chunk() {
        let chunk = Chunk::new_chunk(
            0,
            "IFhd",
            vec![
                0x12, 0x34, 0x32, 0x33, 0x30, 0x37, 0x32, 0x32, 0x56, 0x78, 0x9a, 0xbc, 0xde,
            ],
        );

        let ifhd = assert_ok!(IFhd::try_from(&chunk));
        assert_eq!(ifhd.release_number(), 0x1234);
        assert_eq!(ifhd.serial_number(), &[0x32, 0x33, 0x30, 0x37, 0x32, 0x32]);
        assert_eq!(ifhd.checksum(), 0x5678);
        assert_eq!(ifhd.pc(), 0x9abcde);
    }

    #[test]
    fn test_ifhd_try_from_chunk_wrong_id() {
        let chunk = Chunk::new_chunk(0, "RIdx", vec![]);

        assert!(IFhd::try_from(&chunk).is_err());
    }

    #[test]
    fn test_ifhd_try_from_chunk_bad_data() {
        let chunk = Chunk::new_chunk(
            0,
            "IFhd",
            vec![
                0x12, 0x34, 0x32, 0x33, 0x30, 0x37, 0x32, 0x32, 0x56, 0x78, 0x9a, 0xbc,
            ],
        );

        assert!(IFhd::try_from(&chunk).is_err());
    }

    #[test]
    fn test_index_constructor() {
        let index = Index::new("USAGE".to_string(), 1, 2);
        assert_eq!(index.usage(), "USAGE");
        assert_eq!(index.number(), 1);
        assert_eq!(index.start(), 2);
    }

    #[test]
    fn test_index_try_from_array() {
        let v = [
            b'S', b'n', b'd', b' ', 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0,
        ];
        let index = assert_ok!(Index::try_from(v.as_slice()));
        assert_eq!(index.usage(), "Snd ");
        assert_eq!(index.number(), 0x12345678);
        assert_eq!(index.start(), 0x9abcdef0);
    }

    #[test]
    fn test_index_try_from_array_bad_data() {
        let v = [
            b'S', b'n', b'd', b' ', 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0xff,
        ];
        assert!(Index::try_from(v.as_slice()).is_err());
        assert!(Index::try_from(&v[0..11]).is_err());
    }

    #[test]
    fn test_ridx_constructor() {
        let i1 = Index::new("USE1".to_string(), 1, 2);
        let i2 = Index::new("USE1".to_string(), 3, 4);
        let ridx = RIdx::new(vec![i1.clone(), i2.clone()]);
        assert_eq!(ridx.indices(), &vec![i1, i2]);
    }

    #[test]
    fn test_ridx_try_from_chunk() {
        let chunk = Chunk::new_chunk(
            0,
            "RIdx",
            vec![
                0x00, 0x00, 0x00, 0x02, b'S', b'n', b'd', b' ', 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc,
                0xde, 0xf0, b'S', b'n', b'd', b' ', 0x21, 0x43, 0x56, 0x87, 0xa9, 0xcb, 0xed, 0x0f,
            ],
        );
        let ridx = assert_ok!(RIdx::try_from(&chunk));
        assert_eq!(
            ridx.indices(),
            &vec![
                Index::new("Snd ".to_string(), 0x12345678, 0x9abcdef0),
                Index::new("Snd ".to_string(), 0x21435687, 0xa9cbed0f)
            ]
        )
    }

    #[test]
    fn test_ridx_try_from_chunk_wrong_id() {
        let chunk = Chunk::new_chunk(0, "Loop", vec![]);
        assert!(RIdx::try_from(&chunk).is_err());
    }

    #[test]
    fn test_ridx_try_from_chunk_bad_data() {
        let chunk = Chunk::new_chunk(
            0,
            "Loop",
            vec![
                0x00, 0x00, 0x00, 0x03, b'S', b'n', b'd', b' ', 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc,
                0xde, 0xf0, b'S', b'n', b'd', b' ', 0x21, 0x43, 0x56, 0x87, 0xa9, 0xcb, 0xed, 0x0f,
            ],
        );
        assert!(RIdx::try_from(&chunk).is_err());
    }

    #[test]
    fn test_entry_constructor() {
        let entry = Entry::new(1, 2);
        assert_eq!(entry.number(), 1);
        assert_eq!(entry.repeats(), 2);
    }

    #[test]
    fn test_entry_try_from_array() {
        let v = [0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0];
        let entry = assert_ok!(Entry::try_from(&v[0..8]));
        assert_eq!(entry.number(), 0x12345678);
        assert_eq!(entry.repeats(), 0x9abcdef0);
    }

    #[test]
    fn test_loop_constructor() {
        let e1 = Entry::new(0x12345678, 0x9abcdef0);
        let e2 = Entry::new(0x21436587, 0xa9cbed0f);
        let l = Loop::new(vec![e1, e2]);
        assert_eq!(l.entries(), &vec![e1, e2]);
    }

    #[test]
    fn test_loop_try_from_chunk() {
        let chunk = Chunk::new_chunk(
            0,
            "Loop",
            vec![
                0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x21, 0x43, 0x65, 0x87, 0xa9, 0xcb,
                0xed, 0x0f,
            ],
        );
        let l = assert_ok!(Loop::try_from(&chunk));
        assert_eq!(
            l.entries(),
            &vec![
                Entry::new(0x12345678, 0x9abcdef0),
                Entry::new(0x21436587, 0xa9cbed0f)
            ]
        );
    }

    #[test]
    fn test_loop_try_from_chunk_wrong_id() {
        let chunk = Chunk::new_chunk(0, "RIdx", vec![]);
        assert!(Loop::try_from(&chunk).is_err());
    }

    #[test]
    fn test_loop_try_from_chunk_bad_data() {
        let chunk = Chunk::new_chunk(
            0,
            "Loop",
            vec![
                0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x21, 0x43, 0x65, 0x87, 0xa9, 0xcb,
                0xed, 0x0f, 0xff,
            ],
        );
        assert!(Loop::try_from(&chunk).is_err());
    }

    #[test]
    fn test_blorb_constructor() {
        let ridx = RIdx::new(vec![
            Index::new("Snd ".to_string(), 1, 2),
            Index::new("Snd ".to_string(), 3, 4),
        ]);
        let ifhd = IFhd::new(
            0x1234,
            &[0x32, 0x33, 0x30, 0x37, 0x32, 0x32],
            0x5678,
            0x9abcde,
        );
        let mut sounds = HashMap::new();
        sounds.insert(0x100, Chunk::new_chunk(0x100, "OGGV", vec![1, 2, 3, 4]));
        sounds.insert(0x200, Chunk::new_chunk(0x200, "OGGV", vec![5, 6, 7]));
        let l = Loop::new(vec![Entry::new(5, 6), Entry::new(7, 8)]);
        let exec = vec![0x11, 0x22, 0x33, 0x44];
        let blorb = Blorb::new(
            ridx.clone(),
            Some(ifhd.clone()),
            sounds.clone(),
            Some(l.clone()),
            Some(exec.clone()),
        );
        assert_eq!(blorb.ridx(), &ridx);
        assert_some_eq!(blorb.ifhd(), &ifhd);
        assert_eq!(blorb.sounds(), &sounds);
        assert_some_eq!(blorb.loops(), &l);
        assert_some_eq!(blorb.exec(), &exec);
    }

    #[test]
    fn test_blorb_try_from_chunk() {
        let ifhd = Chunk::new_chunk(
            0x0C,
            "IFhd",
            vec![
                0x12, 0x34, 0x32, 0x33, 0x30, 0x37, 0x32, 0x32, 0x56, 0x78, 0x9a, 0xbc, 0xde,
            ],
        );
        let ridx = Chunk::new_chunk(
            0x22,
            "RIdx",
            vec![
                0x00, 0x00, 0x00, 0x03, b'S', b'n', b'd', b' ', 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc,
                0xde, 0xf0, b'S', b'n', b'd', b' ', 0x21, 0x43, 0x65, 0x87, 0xa9, 0xcb, 0xed, 0x0f,
                b'E', b'x', b'e', b'c', 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x88,
            ],
        );
        let l = Chunk::new_chunk(
            0x46,
            "Loop",
            vec![
                0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00,
                0x00, 0x00,
            ],
        );
        let exec = Chunk::new_chunk(0x88, "ZCOD", vec![0x11, 0x22, 0x33, 0x44]);
        let oggv = Chunk::new_chunk(0x70, "OGGV", vec![1, 2, 3, 4]);
        let aiff = Chunk::new_form(0x7C, "AIFF", vec![]);
        let iff = Chunk::new_form(
            0,
            "IFRS",
            vec![ifhd, ridx, l, oggv.clone(), aiff.clone(), exec],
        );
        let blorb = assert_ok!(Blorb::try_from(&iff));
        assert_some_eq!(
            blorb.ifhd(),
            &IFhd::new(
                0x1234,
                &[0x32, 0x33, 0x30, 0x37, 0x32, 0x32],
                0x5678,
                0x9abcde
            )
        );
        assert_eq!(
            blorb.ridx(),
            &RIdx::new(vec![
                Index::new("Snd ".to_string(), 0x12345678, 0x9abcdef0),
                Index::new("Snd ".to_string(), 0x21436587, 0xa9cbed0f),
                Index::new("Exec".to_string(), 0, 0x88),
            ])
        );
        assert_some_eq!(
            blorb.loops(),
            &Loop::new(vec![Entry::new(1, 2), Entry::new(2, 0)])
        );
        assert_eq!(blorb.sounds().len(), 2);
        assert_some_eq!(blorb.sounds().get(&0x70), &oggv);
        assert_some_eq!(blorb.sounds().get(&0x7C), &aiff);
        assert_some_eq!(blorb.exec(), &vec![0x11, 0x22, 0x33, 0x44]);
    }

    #[test]
    fn test_blorb_try_from_chunk_no_exec() {
        let ifhd = Chunk::new_chunk(
            0x0C,
            "IFhd",
            vec![
                0x12, 0x34, 0x32, 0x33, 0x30, 0x37, 0x32, 0x32, 0x56, 0x78, 0x9a, 0xbc, 0xde,
            ],
        );
        let ridx = Chunk::new_chunk(
            0x22,
            "RIdx",
            vec![
                0x00, 0x00, 0x00, 0x02, b'S', b'n', b'd', b' ', 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc,
                0xde, 0xf0, b'S', b'n', b'd', b' ', 0x21, 0x43, 0x65, 0x87, 0xa9, 0xcb, 0xed, 0x0f,
            ],
        );
        let l = Chunk::new_chunk(
            0x46,
            "Loop",
            vec![
                0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00,
                0x00, 0x00,
            ],
        );
        let oggv = Chunk::new_chunk(0x70, "OGGV", vec![1, 2, 3, 4]);
        let aiff = Chunk::new_form(0x7C, "AIFF", vec![]);
        let iff = Chunk::new_form(0, "IFRS", vec![ifhd, ridx, l, oggv.clone(), aiff.clone()]);
        let blorb = assert_ok!(Blorb::try_from(&iff));
        assert_some_eq!(
            blorb.ifhd(),
            &IFhd::new(
                0x1234,
                &[0x32, 0x33, 0x30, 0x37, 0x32, 0x32],
                0x5678,
                0x9abcde
            )
        );
        assert_eq!(
            blorb.ridx(),
            &RIdx::new(vec![
                Index::new("Snd ".to_string(), 0x12345678, 0x9abcdef0),
                Index::new("Snd ".to_string(), 0x21436587, 0xa9cbed0f),
            ])
        );
        assert_some_eq!(
            blorb.loops(),
            &Loop::new(vec![Entry::new(1, 2), Entry::new(2, 0)])
        );
        assert_eq!(blorb.sounds().len(), 2);
        assert_some_eq!(blorb.sounds().get(&0x70), &oggv);
        assert_some_eq!(blorb.sounds().get(&0x7C), &aiff);
        assert!(blorb.exec().is_none());
    }

    #[test]
    fn test_blorb_try_from_wrong_zcode_offset() {
        let ifhd = Chunk::new_chunk(
            0x0C,
            "IFhd",
            vec![
                0x12, 0x34, 0x32, 0x33, 0x30, 0x37, 0x32, 0x32, 0x56, 0x78, 0x9a, 0xbc, 0xde,
            ],
        );
        let ridx = Chunk::new_chunk(
            0x22,
            "RIdx",
            vec![
                0x00, 0x00, 0x00, 0x03, b'S', b'n', b'd', b' ', 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc,
                0xde, 0xf0, b'S', b'n', b'd', b' ', 0x21, 0x43, 0x65, 0x87, 0xa9, 0xcb, 0xed, 0x0f,
                b'E', b'x', b'e', b'c', 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x86,
            ],
        );
        let l = Chunk::new_chunk(
            0x46,
            "Loop",
            vec![
                0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00,
                0x00, 0x00,
            ],
        );
        let exec = Chunk::new_chunk(0x88, "ZCOD", vec![0x11, 0x22, 0x33, 0x44]);
        let oggv = Chunk::new_chunk(0x70, "OGGV", vec![1, 2, 3, 4]);
        let aiff = Chunk::new_form(0x7C, "AIFF", vec![]);
        let iff = Chunk::new_form(
            0,
            "IFRS",
            vec![ifhd, ridx, l, oggv.clone(), aiff.clone(), exec],
        );
        let blorb = assert_ok!(Blorb::try_from(&iff));
        assert_some_eq!(
            blorb.ifhd(),
            &IFhd::new(
                0x1234,
                &[0x32, 0x33, 0x30, 0x37, 0x32, 0x32],
                0x5678,
                0x9abcde
            )
        );
        assert_eq!(
            blorb.ridx(),
            &RIdx::new(vec![
                Index::new("Snd ".to_string(), 0x12345678, 0x9abcdef0),
                Index::new("Snd ".to_string(), 0x21436587, 0xa9cbed0f),
                Index::new("Exec".to_string(), 0, 0x86),
            ])
        );
        assert_some_eq!(
            blorb.loops(),
            &Loop::new(vec![Entry::new(1, 2), Entry::new(2, 0)])
        );
        assert_eq!(blorb.sounds().len(), 2);
        assert_some_eq!(blorb.sounds().get(&0x70), &oggv);
        assert_some_eq!(blorb.sounds().get(&0x7C), &aiff);
        assert!(blorb.exec().is_none());
    }

    #[test]
    fn test_blorb_try_from_wrong_exec_number() {
        let ifhd = Chunk::new_chunk(
            0x0C,
            "IFhd",
            vec![
                0x12, 0x34, 0x32, 0x33, 0x30, 0x37, 0x32, 0x32, 0x56, 0x78, 0x9a, 0xbc, 0xde,
            ],
        );
        let ridx = Chunk::new_chunk(
            0x22,
            "RIdx",
            vec![
                0x00, 0x00, 0x00, 0x03, b'S', b'n', b'd', b' ', 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc,
                0xde, 0xf0, b'S', b'n', b'd', b' ', 0x21, 0x43, 0x65, 0x87, 0xa9, 0xcb, 0xed, 0x0f,
                b'E', b'x', b'e', b'c', 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x88,
            ],
        );
        let l = Chunk::new_chunk(
            0x46,
            "Loop",
            vec![
                0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00,
                0x00, 0x00,
            ],
        );
        let exec = Chunk::new_chunk(0x88, "ZCOD", vec![0x11, 0x22, 0x33, 0x44]);
        let oggv = Chunk::new_chunk(0x70, "OGGV", vec![1, 2, 3, 4]);
        let aiff = Chunk::new_form(0x7C, "AIFF", vec![]);
        let iff = Chunk::new_form(
            0,
            "IFRS",
            vec![ifhd, ridx, l, oggv.clone(), aiff.clone(), exec],
        );
        let blorb = assert_ok!(Blorb::try_from(&iff));
        assert_some_eq!(
            blorb.ifhd(),
            &IFhd::new(
                0x1234,
                &[0x32, 0x33, 0x30, 0x37, 0x32, 0x32],
                0x5678,
                0x9abcde
            )
        );
        assert_eq!(
            blorb.ridx(),
            &RIdx::new(vec![
                Index::new("Snd ".to_string(), 0x12345678, 0x9abcdef0),
                Index::new("Snd ".to_string(), 0x21436587, 0xa9cbed0f),
                Index::new("Exec".to_string(), 1, 0x88),
            ])
        );
        assert_some_eq!(
            blorb.loops(),
            &Loop::new(vec![Entry::new(1, 2), Entry::new(2, 0)])
        );
        assert_eq!(blorb.sounds().len(), 2);
        assert_some_eq!(blorb.sounds().get(&0x70), &oggv);
        assert_some_eq!(blorb.sounds().get(&0x7C), &aiff);
        assert!(blorb.exec().is_none());
    }

    #[test]
    fn test_blorb_try_from_multiple_exec() {
        let ifhd = Chunk::new_chunk(
            0x0C,
            "IFhd",
            vec![
                0x12, 0x34, 0x32, 0x33, 0x30, 0x37, 0x32, 0x32, 0x56, 0x78, 0x9a, 0xbc, 0xde,
            ],
        );
        let ridx = Chunk::new_chunk(
            0x22,
            "RIdx",
            vec![
                0x00, 0x00, 0x00, 0x04, b'S', b'n', b'd', b' ', 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc,
                0xde, 0xf0, b'S', b'n', b'd', b' ', 0x21, 0x43, 0x65, 0x87, 0xa9, 0xcb, 0xed, 0x0f,
                b'E', b'x', b'e', b'c', 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x88, b'E', b'x',
                b'e', b'c', 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x98,
            ],
        );
        let l = Chunk::new_chunk(
            0x46,
            "Loop",
            vec![
                0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00,
                0x00, 0x00,
            ],
        );
        let exec = Chunk::new_chunk(0x88, "ZCOD", vec![0x11, 0x22, 0x33, 0x44]);
        let oggv = Chunk::new_chunk(0x70, "OGGV", vec![1, 2, 3, 4]);
        let aiff = Chunk::new_form(0x7C, "AIFF", vec![]);
        let iff = Chunk::new_form(
            0,
            "IFRS",
            vec![ifhd, ridx, l, oggv.clone(), aiff.clone(), exec],
        );
        let blorb = assert_ok!(Blorb::try_from(&iff));
        assert_some_eq!(
            blorb.ifhd(),
            &IFhd::new(
                0x1234,
                &[0x32, 0x33, 0x30, 0x37, 0x32, 0x32],
                0x5678,
                0x9abcde
            )
        );
        assert_eq!(
            blorb.ridx(),
            &RIdx::new(vec![
                Index::new("Snd ".to_string(), 0x12345678, 0x9abcdef0),
                Index::new("Snd ".to_string(), 0x21436587, 0xa9cbed0f),
                Index::new("Exec".to_string(), 0, 0x88),
                Index::new("Exec".to_string(), 0, 0x98),
            ])
        );
        assert_some_eq!(
            blorb.loops(),
            &Loop::new(vec![Entry::new(1, 2), Entry::new(2, 0)])
        );
        assert_eq!(blorb.sounds().len(), 2);
        assert_some_eq!(blorb.sounds().get(&0x70), &oggv);
        assert_some_eq!(blorb.sounds().get(&0x7C), &aiff);
        assert!(blorb.exec().is_none());
    }

    #[test]
    fn test_blorb_try_from_chunk_no_ifhd() {
        let ridx = Chunk::new_chunk(
            0x22,
            "RIdx",
            vec![
                0x00, 0x00, 0x00, 0x03, b'S', b'n', b'd', b' ', 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc,
                0xde, 0xf0, b'S', b'n', b'd', b' ', 0x21, 0x43, 0x65, 0x87, 0xa9, 0xcb, 0xed, 0x0f,
                b'E', b'x', b'e', b'c', 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x58,
            ],
        );
        let l = Chunk::new_chunk(
            0x46,
            "Loop",
            vec![
                0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00,
                0x00, 0x00,
            ],
        );
        let exec = Chunk::new_chunk(0x58, "ZCOD", vec![0x11, 0x22, 0x33, 0x44]);
        let oggv = Chunk::new_chunk(0x64, "OGGV", vec![1, 2, 3, 4]);
        let aiff = Chunk::new_form(0x70, "AIFF", vec![]);
        let iff = Chunk::new_form(0, "IFRS", vec![ridx, l, oggv.clone(), aiff.clone(), exec]);
        let blorb = assert_ok!(Blorb::try_from(&iff));
        assert!(blorb.ifhd().is_none());
        assert_eq!(
            blorb.ridx(),
            &RIdx::new(vec![
                Index::new("Snd ".to_string(), 0x12345678, 0x9abcdef0),
                Index::new("Snd ".to_string(), 0x21436587, 0xa9cbed0f),
                Index::new("Exec".to_string(), 0, 0x58),
            ])
        );
        assert_some_eq!(
            blorb.loops(),
            &Loop::new(vec![Entry::new(1, 2), Entry::new(2, 0)])
        );
        assert_eq!(blorb.sounds().len(), 2);
        assert_some_eq!(blorb.sounds().get(&0x64), &oggv);
        assert_some_eq!(blorb.sounds().get(&0x70), &aiff);
        assert_some_eq!(blorb.exec(), &vec![0x11, 0x22, 0x33, 0x44]);
    }

    #[test]
    fn test_blorb_try_from_chunk_wrong_sub_id() {
        let iff = Chunk::new_form(0, "IFZS", vec![]);
        assert!(Blorb::try_from(&iff).is_err());
    }

    #[test]
    fn test_blorb_try_from_chunk_wrong_id() {
        let iff = Chunk::new_chunk(0, "IFRS", vec![]);
        assert!(Blorb::try_from(&iff).is_err());
    }

    #[test]
    fn test_blorb_try_vec_u8() {
        let data = vec![
            b'F', b'O', b'R', b'M', 0x00, 0x00, 0x00, 0x92, b'I', b'F', b'R', b'S', b'I', b'F',
            b'h', b'd', 0x00, 0x00, 0x00, 0x0d, 0x12, 0x34, 0x32, 0x33, 0x30, 0x37, 0x32, 0x32,
            0x56, 0x78, 0x9a, 0xbc, 0xde, 0x00, b'R', b'I', b'd', b'x', 0x00, 0x00, 0x00, 0x28,
            0x00, 0x00, 0x00, 0x03, b'S', b'n', b'd', b' ', 0x01, 0x02, 0x03, 0x04, 0x00, 0x00,
            0x00, 0x52, b'S', b'n', b'd', b' ', 0x05, 0x06, 0x07, 0x08, 0x00, 0x00, 0x00, 0x5E,
            b'E', b'x', b'e', b'c', 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x8E, b'O', b'G',
            b'G', b'V', 0x00, 0x00, 0x00, 0x04, 0x0a, 0x0b, 0x0c, 0x0d, b'F', b'O', b'R', b'M',
            0x00, 0x00, 0x00, 0x10, b'A', b'I', b'F', b'F', b'C', b'O', b'M', b'M', 0x00, 0x00,
            0x00, 0x04, 0x0f, 0x10, 0x11, 0x12, b'L', b'o', b'o', b'p', 0x00, 0x00, 0x00, 0x10,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00,
            0x00, 0x00, b'Z', b'C', b'O', b'D', 0x00, 0x00, 0x00, 0x04, 0x13, 0x14, 0x15, 0x16,
        ];
        let b = Blorb::try_from(data);
        let blorb = assert_ok!(b);
        assert_some_eq!(
            blorb.ifhd(),
            &IFhd::new(
                0x1234,
                &[0x32, 0x33, 0x30, 0x37, 0x32, 0x32],
                0x5678,
                0x9abcde
            )
        );
        assert_eq!(
            blorb.ridx(),
            &RIdx::new(vec![
                Index::new("Snd ".to_string(), 0x01020304, 0x52),
                Index::new("Snd ".to_string(), 0x05060708, 0x5E),
                Index::new("Exec".to_string(), 0, 0x8E),
            ])
        );
        assert_some_eq!(
            blorb.loops(),
            &Loop::new(vec![Entry::new(1, 2), Entry::new(2, 0)])
        );
        assert_eq!(blorb.sounds().len(), 2);
        assert_some_eq!(
            blorb.sounds().get(&0x52),
            &Chunk::new_chunk(0x52, "OGGV", vec![0x0a, 0x0b, 0x0c, 0x0d])
        );
        assert_some_eq!(
            blorb.sounds().get(&0x5E),
            &Chunk::new_form(
                0x5E,
                "AIFF",
                vec![Chunk::new_chunk(0x6A, "COMM", vec![0x0f, 0x10, 0x11, 0x12])]
            )
        );
        assert_some_eq!(blorb.exec(), &vec![0x13, 0x14, 0x15, 0x16]);
    }

    #[test]
    fn test_blorb_try_from_file() {
        let data = vec![
            b'F', b'O', b'R', b'M', 0x00, 0x00, 0x00, 0x92, b'I', b'F', b'R', b'S', b'I', b'F',
            b'h', b'd', 0x00, 0x00, 0x00, 0x0d, 0x12, 0x34, 0x32, 0x33, 0x30, 0x37, 0x32, 0x32,
            0x56, 0x78, 0x9a, 0xbc, 0xde, 0x00, b'R', b'I', b'd', b'x', 0x00, 0x00, 0x00, 0x28,
            0x00, 0x00, 0x00, 0x03, b'S', b'n', b'd', b' ', 0x01, 0x02, 0x03, 0x04, 0x00, 0x00,
            0x00, 0x52, b'S', b'n', b'd', b' ', 0x05, 0x06, 0x07, 0x08, 0x00, 0x00, 0x00, 0x5E,
            b'E', b'x', b'e', b'c', 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x8E, b'O', b'G',
            b'G', b'V', 0x00, 0x00, 0x00, 0x04, 0x0a, 0x0b, 0x0c, 0x0d, b'F', b'O', b'R', b'M',
            0x00, 0x00, 0x00, 0x10, b'A', b'I', b'F', b'F', b'C', b'O', b'M', b'M', 0x00, 0x00,
            0x00, 0x04, 0x0f, 0x10, 0x11, 0x12, b'L', b'o', b'o', b'p', 0x00, 0x00, 0x00, 0x10,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00,
            0x00, 0x00, b'Z', b'C', b'O', b'D', 0x00, 0x00, 0x00, 0x04, 0x13, 0x14, 0x15, 0x16,
        ];
        let mut file = assert_ok!(fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open("test.blb"));
        assert!(file.write_all(&data).is_ok());
        assert!(file.flush().is_ok());
        assert!(Path::new("test.blb").exists());
        let mut file = assert_ok!(fs::OpenOptions::new().read(true).open("test.blb"));
        let b = Blorb::try_from(&mut file);
        assert_ok!(fs::remove_file("test.blb"));
        let blorb = assert_ok!(b);
        assert_some_eq!(
            blorb.ifhd(),
            &IFhd::new(
                0x1234,
                &[0x32, 0x33, 0x30, 0x37, 0x32, 0x32],
                0x5678,
                0x9abcde
            )
        );
        assert_eq!(
            blorb.ridx(),
            &RIdx::new(vec![
                Index::new("Snd ".to_string(), 0x01020304, 0x52),
                Index::new("Snd ".to_string(), 0x05060708, 0x5E),
                Index::new("Exec".to_string(), 0, 0x8E),
            ])
        );
        assert_some_eq!(
            blorb.loops(),
            &Loop::new(vec![Entry::new(1, 2), Entry::new(2, 0)])
        );
        assert_eq!(blorb.sounds().len(), 2);
        assert_some_eq!(
            blorb.sounds().get(&0x52),
            &Chunk::new_chunk(0x52, "OGGV", vec![0x0a, 0x0b, 0x0c, 0x0d])
        );
        assert_some_eq!(
            blorb.sounds().get(&0x5E),
            &Chunk::new_form(
                0x5E,
                "AIFF",
                vec![Chunk::new_chunk(0x6A, "COMM", vec![0x0f, 0x10, 0x11, 0x12])]
            )
        );
        assert_some_eq!(blorb.exec(), &vec![0x13, 0x14, 0x15, 0x16]);
    }

    #[test]
    fn test_blorb_try_from_file_error() {
        let mut file = assert_ok!(fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open("test.blorb"));
        assert!(file.write_all(&[]).is_ok());
        assert!(file.flush().is_ok());
        assert!(Path::new("test.blorb").exists());
        let mut file = assert_ok!(fs::OpenOptions::new().read(true).open("test.blorb"));
        let b = Blorb::try_from(&mut file);
        assert!(fs::remove_file("test.blorb").is_ok());
        assert!(b.is_err());
    }
}
