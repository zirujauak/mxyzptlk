use std::{collections::HashMap, fmt};

use ridx::RIdx;

use crate::{
    error::{ErrorCode, RuntimeError},
    iff::blorb::aiff::AIFF,
};

use self::{oggv::OGGV, sloop::Loop};

use super::{quetzal::ifhd::IFhd, IFF};

pub mod aiff;
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
