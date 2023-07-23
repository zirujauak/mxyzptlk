use std::{collections::HashMap, fmt};

use ridx::RIdx;

use crate::{
    error::{ErrorCode, RuntimeError},
    iff::blorb::aiff::AIFF,
};

use self::{ifhd::IFhd, oggv::OGGV, sloop::Loop};

use super::IFF;

pub mod aiff;
pub mod ifhd;
pub mod oggv;
pub mod ridx;
pub mod sloop;

pub struct Blorb {
    ridx: Option<RIdx>,
    ifhd: Option<IFhd>,
    oggv: HashMap<usize, OGGV>,
    aiff: HashMap<usize, AIFF>,
    sloop: Option<Loop>,
}

impl fmt::Display for Blorb {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Blorb:")?;
        if let Some(ifhd) = &self.ifhd {
            writeln!(f, "{}", ifhd)?;
        } else {
            writeln!(f, "** No IFhd chunk")?;
        }
        if let Some(ridx) = &self.ridx {
            writeln!(f, "{}", ridx)?;
        }
        writeln!(f, "Sound resources:")?;
        for k in self.oggv.keys() {
            if let Some(s) = self.oggv.get(k) {
                writeln!(f, "\t{}", s)?;
            }
        }
        for k in self.aiff.keys() {
            if let Some(s) = self.aiff.get(k) {
                writeln!(f, "\t{}", s)?;
            }
        }
        if let Some(sloop) = &self.sloop {
            write!(f, "{}", sloop)
        } else {
            write!(f, "No sound loop data")
        }
    }
}

impl TryFrom<Vec<u8>> for Blorb {
    type Error = RuntimeError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        let iff = IFF::from(&value);

        if iff.form != "FORM" || iff.sub_form != "IFRS" {
            error!(
                target: "app::blorb",
                "Resource file form and/or sub-form are incorrect: {}/{}",
                iff.form, iff.sub_form
            );
            return Err(RuntimeError::new(
                ErrorCode::Blorb,
                format!(
                    "Invalid blorb file: Form '{}', sub-form '{}'",
                    iff.form, iff.sub_form
                ),
            ));
        }

        let mut ridx = None;
        let mut ifhd = None;
        let mut sloop = None;
        let mut oggv: HashMap<usize, OGGV> = HashMap::new();
        let mut aiff: HashMap<usize, AIFF> = HashMap::new();
        for chunk in iff.chunks {
            match chunk.id.as_str() {
                "RIdx" => ridx = Some(RIdx::from(chunk)),
                "IFhd" => ifhd = Some(IFhd::from(chunk)),
                "Loop" => sloop = Some(Loop::from(chunk)),
                "OGGV" => {
                    oggv.insert(chunk.offset, OGGV::from(chunk));
                }
                "AIFF" => {
                    aiff.insert(chunk.offset, AIFF::from(chunk));
                }
                _ => warn!(target: "app::blorb", "Ignoring chunk id {}", chunk.id),
            }
        }

        let blorb = Blorb::new(ridx, ifhd, oggv, aiff, sloop);
        debug!(target: "app::blorb", "{}", blorb);
        Ok(blorb)
    }
}

impl Blorb {
    pub fn new(
        ridx: Option<RIdx>,
        ifhd: Option<IFhd>,
        oggv: HashMap<usize, OGGV>,
        aiff: HashMap<usize, AIFF>,
        sloop: Option<Loop>,
    ) -> Blorb {
        Blorb {
            ridx,
            ifhd,
            oggv,
            aiff,
            sloop,
        }
    }

    pub fn ridx(&self) -> Option<&RIdx> {
        self.ridx.as_ref()
    }

    pub fn ifhd(&self) -> Option<&IFhd> {
        self.ifhd.as_ref()
    }

    pub fn oggv(&self) -> &HashMap<usize, OGGV> {
        &self.oggv
    }

    pub fn aiff(&self) -> &HashMap<usize, AIFF> {
        &self.aiff
    }

    pub fn sloop(&self) -> Option<&Loop> {
        self.sloop.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use crate::{assert_ok, assert_some};

    use super::*;
    use ridx::Index;
    use sloop::Entry;

    #[test]
    fn test_new() {
        let ridx = RIdx::new(&[Index::new("Snd ".to_string(), 1, 2)]);
        let ifhd = IFhd::new(1, &[1, 2, 3, 4, 5, 6], 0x1122, 0x654321);
        let mut oggv = HashMap::new();
        oggv.insert(1, OGGV::new(&[1, 2, 3, 4]));
        let mut aiff = HashMap::new();
        aiff.insert(2, AIFF::new(&[5, 6, 7, 8]));
        let sloop = Loop::new(&[Entry::new(10, 11)]);
        let blorb = Blorb::new(Some(ridx), Some(ifhd), oggv, aiff, Some(sloop));
        let ridx = assert_some!(blorb.ridx());
        assert_eq!(ridx.entries().len(), 1);
        assert_eq!(ridx.entries()[0].usage(), "Snd ");
        assert_eq!(ridx.entries()[0].number(), 1);
        assert_eq!(ridx.entries()[0].start(), 2);
        assert_eq!(blorb.oggv().len(), 1);
        let oggv = assert_some!(blorb.oggv().get(&1));
        assert_eq!(oggv.data(), &[1, 2, 3, 4]);
        assert_eq!(blorb.aiff().len(), 1);
        let aiff = assert_some!(blorb.aiff.get(&2));
        assert_eq!(aiff.data(), &[5, 6, 7, 8]);
        let ifhd = assert_some!(blorb.ifhd());
        assert_eq!(ifhd.release_number(), 1);
        assert_eq!(ifhd.serial_number(), &[1, 2, 3, 4, 5, 6]);
        assert_eq!(ifhd.checksum(), 0x1122);
        assert_eq!(ifhd.pc(), 0x654321);
        let sloop = assert_some!(blorb.sloop());
        assert_eq!(sloop.entries().len(), 1);
        assert_eq!(sloop.entries()[0].number(), 10);
        assert_eq!(sloop.entries()[0].repeats(), 11);
    }

    #[test]
    fn test_try_from_vec_u8() {
        let v = vec![
            /* 0000 */ b'F', b'O', b'R', b'M', /* 0004 */ 0x00, 0x00, 0x00, 0x4e,
            /* 0008 */ b'I', b'F', b'R', b'S', /* 000c */ b'R', b'I', b'd', b'x',
            /* 0010 */ 0x00, 0x00, 0x00, 0x1c, /* 0014 */ 0x00, 0x00, 0x00, 0x02,
            /* 0018 */ b'S', b'n', b'd', b' ', /* 001c */ 0x01, 0x00, 0x00, 0x01,
            /* 0020 */ 0x00, 0x00, 0x00, 0x46, /* 0024 */ b'S', b'n', b'd', b' ',
            /* 0028 */ 0x01, 0x00, 0x00, 0x02, /* 002c */ 0x00, 0x00, 0x00, 0x5a,
            /* 0x30 */ b'I', b'F', b'h', b'd', /* 0x34 */ 0x00, 0x00, 0x00, 0x0d,
            /* 0x38 */ 0x11, 0x22, b'1', b'2', /* 0x3C */ b'3', b'4', b'5', b'6',
            /* 0x40 */ 0x33, 0x44, 0x55, 0x66, /* 0x44 */ 0x77, 0x00, b'O', b'G',
            /* 0x48 */ b'G', b'V', 0x00, 0x00, /* 0x4c */ 0x00, 0x0c, 0x10, 0x20,
            /* 0x50 */ 0x30, 0x40, b'O', b'g', /* 0x54 */ b'g', b's', 0x11, 0x22,
            /* 0x58 */ 0x33, 0x44, b'F', b'O', /* 0x5c */ b'R', b'M', 0x00, 0x00,
            /* 0x60 */ 0x00, 0x0c, b'A', b'I', /* 0x64 */ b'F', b'F', b'C', b'O',
            /* 0x68 */ b'M', b'M', 0x00, 0x00, /* 0x6c */ 0x00, 0x03, b'L', b'o',
            /* 0x70 */ b'o', b'p', 0x00, 0x00, /* 0x74 */ 0x00, 0x10, 0x00, 0x00,
            /* 0x78 */ 0x00, 0x01, 0x00, 0x00, /* 0x7c */ 0x00, 0x10, 0x00, 0x00,
            /* 0x80 */ 0x00, 0x02, 0x00, 0x00, /* 0x84 */ 0x00, 0x20,
        ];
        let blorb = assert_ok!(Blorb::try_from(v));
        let ridx = assert_some!(blorb.ridx());
        assert_eq!(ridx.entries().len(), 2);
        assert_eq!(ridx.entries()[0].usage(), "Snd ");
        assert_eq!(ridx.entries()[0].number(), 0x01000001);
        assert_eq!(ridx.entries()[0].start(), 0x46);
        assert_eq!(ridx.entries()[1].usage(), "Snd ");
        assert_eq!(ridx.entries()[1].number(), 0x01000002);
        assert_eq!(ridx.entries()[1].start(), 0x5a);
        assert_eq!(blorb.oggv().len(), 1);
        let oggv = assert_some!(blorb.oggv().get(&0x46));
        assert_eq!(
            oggv.data(),
            &[0x10, 0x20, 0x30, 0x40, b'O', b'g', b'g', b's', 0x11, 0x22, 0x33, 0x44]
        );
        assert_eq!(blorb.aiff().len(), 1);
        let aiff = assert_some!(blorb.aiff.get(&0x5a));
        assert_eq!(
            aiff.data(),
            &[b'A', b'I', b'F', b'F', b'C', b'O', b'M', b'M', 0, 0, 0, 3]
        );
        let ifhd = assert_some!(blorb.ifhd());
        assert_eq!(ifhd.release_number(), 0x1122);
        assert_eq!(ifhd.serial_number(), &[b'1', b'2', b'3', b'4', b'5', b'6']);
        assert_eq!(ifhd.checksum(), 0x3344);
        assert_eq!(ifhd.pc(), 0x556677);
        let sloop = assert_some!(blorb.sloop());
        assert_eq!(sloop.entries().len(), 2);
        assert_eq!(sloop.entries()[0].number(), 1);
        assert_eq!(sloop.entries()[0].repeats(), 0x10);
        assert_eq!(sloop.entries()[1].number(), 2);
        assert_eq!(sloop.entries()[1].repeats(), 0x20);
    }

    #[test]
    fn test_try_from_vec_u8_error() {
        let v = vec![
            /* 0000 */ b'F', b'O', b'R', b'M', /* 0004 */ 0x00, 0x00, 0x00, 0x4e,
            /* 0008 */ b'I', b'F', b'Z', b'S',
        ];
        let blorb = Blorb::try_from(v);
        assert!(blorb.is_err());

        let v = vec![
            /* 0000 */ b'F', b'R', b'O', b'B', /* 0004 */ 0x00, 0x00, 0x00, 0x4e,
            /* 0008 */ b'I', b'F', b'R', b'S',
        ];
        assert!(Blorb::try_from(v).is_err());
    }
}

// pub fn rebuild_blorb(name: String) {
//     let input = File::open(format!("{}.blorb", name));
//     let samples = vec![
//         "204", "33538", "72232", "78784", "131958", "170182", "196056", "252702", "303234",
//         "317288", "331342", "345386", "364840", "413102", "463076",
//     ];
//     let mut sample_iter = samples.iter();
//     let mut sample_index = 0;
//     match input {
//         Ok(mut f) => {
//             let mut buffer = Vec::new();
//             match f.read_to_end(&mut buffer) {
//                 Ok(_) => {
//                     let iff = IFF::from_vec(&buffer);

//                     let mut new_iff = Vec::new();

//                     // Form
//                     new_iff.append(&mut super::id_as_vec("FORM"));

//                     // Placeholder for length
//                     new_iff.push(0);
//                     new_iff.push(0);
//                     new_iff.push(0);
//                     new_iff.push(0);

//                     // Subform
//                     new_iff.append(&mut super::id_as_vec("IFRS"));

//                     let mut ridx_offset = 0;
//                     for i in iff.chunks {
//                         match i.id.as_str() {
//                             "RIdx" => {
//                                 ridx_offset = new_iff.len();
//                                 new_iff.append(&mut i.to_vec());
//                             }
//                             "AIFF" => {
//                                 let mut aiff = Vec::new();
//                                 let sample_file =
//                                     format!("sample-{}.ogg", sample_iter.next().unwrap());
//                                 File::open(sample_file)
//                                     .unwrap()
//                                     .read_to_end(&mut aiff)
//                                     .unwrap();
//                                 let l = super::u32_to_vec(new_iff.len() as u32, 4);
//                                 new_iff[ridx_offset + 20 + (sample_index * 12)] = l[0];
//                                 new_iff[ridx_offset + 21 + (sample_index * 12)] = l[1];
//                                 new_iff[ridx_offset + 22 + (sample_index * 12)] = l[2];
//                                 new_iff[ridx_offset + 23 + (sample_index * 12)] = l[3];
//                                 sample_index = sample_index + 1;
//                                 new_iff.append(&mut super::id_as_vec("OGGV"));
//                                 new_iff.append(&mut super::u32_to_vec(aiff.len() as u32, 4));
//                                 new_iff.append(&mut aiff);
//                                 if new_iff.len() % 2 == 1 {
//                                     new_iff.push(0);
//                                 }
//                             }
//                             _ => {
//                                 new_iff.append(&mut i.to_vec());
//                             }
//                         }
//                     }

//                     let mut file = fs::OpenOptions::new()
//                         .create(true)
//                         .truncate(true)
//                         .write(true)
//                         .open(format!("{}-new.blorb", name))
//                         .unwrap();

//                     file.write_all(&new_iff).unwrap();
//                     file.flush().unwrap();
//                 }
//                 Err(_) => {}
//             }
//         }
//         Err(_) => (),
//     }
// }
