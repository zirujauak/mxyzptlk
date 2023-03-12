use std::{
    fs::{self, File},
    io::{Read, Write}, collections::HashMap,
};

use ridx::RIdx;

use self::{sloop::Loop, oggv::OGGV};

use super::{IFF, quetzal::ifhd::IFhd};

pub mod ridx;
pub mod oggv;
pub mod sloop;

pub struct Blorb {
    ridx: RIdx,
    ifhd: Option<IFhd>,
    snds: HashMap<usize,OGGV>,
    sloop: Option<Loop>,
}

impl Blorb {
    pub fn from_vec(data: Vec<u8>) -> Option<Blorb> {
        let iff = IFF::from_vec(data);

        if iff.form != "FORM" || iff.sub_form != "IFRS" {
            error!(
                "Resource file form and/or sub-form are incorrect: {}/{}",
                iff.form, iff.sub_form
            );
            return None;
        }

        let mut ridx = None;
        let mut ifhd = None;
        let mut sloop = None;
        let mut snds:HashMap<usize, OGGV> = HashMap::new();

        for chunk in iff.chunks {
            match chunk.id.as_str() {
                "RIdx" => ridx = Some(RIdx::from_chunk(chunk)),
                "IFhd" => ifhd = Some(IFhd::from_chunk(chunk)),
                "Loop" => sloop = Some(Loop::from_chunk(chunk)),
                "OGGV" => {snds.insert(chunk.offset, OGGV::from_chunk(chunk));},
                _ => trace!("Ignoring chunk id {}", chunk.id),
            }
        }

        trace!("Blorb: {} resources, {} sounds, {} loops", ridx.as_ref().unwrap().entries.len(), snds.len(), sloop.as_ref().unwrap().entries.len());

        Some(Blorb {
            ridx: ridx.unwrap(),
            ifhd,
            snds,
            sloop,
        })
    }
}

// pub fn rebuild_blorb(name: String) {
//     let input = File::open(format!("{}.blorb", name));
//     let samples = vec![
//         "308", "50118", "70692", "116746", "176790", "184572", "242640", "275974", "316028",
//         "360402", "417032", "468686", "493780", "533082",
//     ];
//     let mut sample_iter = samples.iter();
//     let mut sample_index = 0;
//     match input {
//         Ok(mut f) => {
//             trace!("Rebuilding blorb {}.blorb -> {}-new.blorb", name, name);
//             let mut buffer = Vec::new();
//             match f.read_to_end(&mut buffer) {
//                 Ok(_) => {
//                     let iff = IFF::from_vec(buffer);

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
//                             },
//                             "AIFF" => {
//                                 let mut aiff = Vec::new();
//                                 let sample_file = format!("sample-{}.ogg", sample_iter.next().unwrap());
//                                 trace!("Loading {}", sample_file);
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
