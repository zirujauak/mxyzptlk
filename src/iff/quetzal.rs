use crate::state::State;

use self::{ifhd::IFhd, cmem::CMem, umem::UMem, stks::Stks};

use super::*;

pub mod ifhd;
pub mod cmem;
pub mod umem;
pub mod stks;

pub struct Quetzal {
    pub ifhd: IFhd,
    pub umem: Option<UMem>,
    pub cmem: Option<CMem>,
    pub stks: Stks,
}

impl Quetzal {
    pub fn from_state(state: &State, instruction_address: usize) -> Quetzal {
        info!(target: "app::quetzal", "Building quetzal from state");
        let ifhd = IFhd::from_state(state, instruction_address);
        info!(target: "app::quetzal", "{}", ifhd);
        let cmem = CMem::from_state(state);
        info!(target: "app::quetzal", "{}", cmem);
        let stks = Stks::from_state(state);
        info!(target: "app::quetzal", "{}", stks);
        Quetzal { ifhd, umem: None, cmem: Some(cmem), stks }
    }

    pub fn from_vec(data: &Vec<u8>) -> Option<Quetzal> {
        let iff = IFF::from_vec(&data);

        if iff.form != "FORM" || iff.sub_form != "IFZS" {
            error!("Save file form and/or sub-form are incorrect: {}/{}", iff.form, iff.sub_form);
            return None;
        }

        let mut ifhd = None;
        let mut umem = None;
        let mut cmem = None;
        let mut stks = None;
        for chunk in iff.chunks {
            match chunk.id.as_str() {
                "IFhd" => ifhd = Some(IFhd::from_chunk(chunk)),
                "CMem" => cmem = Some(CMem::from_chunk(chunk)),
                "UMem" => umem = Some(UMem::from_chunk(chunk)),
                "Stks" => stks = Some(Stks::from_chunk(chunk)),
                _ => trace!("Ignoring chunk id {}", chunk.id)
            }
        }

        match ifhd {
            Some(_) => match stks {
                Some(_) => match cmem {
                    Some(_) => (),
                    None => match umem {
                        Some(_) => (),
                        None => {
                            error!("Save file is missing CMem and UMem chunk");
                            return None;
                        }
                    }
                },
                None => {
                    error!("Save file is missing Stks chunk");
                    return None;
                }
            },
            None => {
                error!("Save file is missing IFhd chunk");
                return None;
            }
        }

        Some(Quetzal {
            ifhd: ifhd.unwrap(),
            umem,
            cmem,
            stks: stks.unwrap(),
        })
    }

    pub fn to_vec(&self) -> Vec<u8> {
        // IFF FORM
        let mut form = id_as_vec("FORM");

        let mut ifzs = id_as_vec("IFZS");
        ifzs.append(&mut self.ifhd.to_chunk());
        match &self.umem {
            Some(u) => ifzs.append(&mut u.to_chunk()),
            None => ()
        };
        match &self.cmem {
            Some(c) => ifzs.append(&mut c.to_chunk()),
            None => ()
        };
        ifzs.append(&mut self.stks.to_chunk());

        form.append(&mut usize_as_vec(ifzs.len(), 4));
        form.append(&mut ifzs);
        if form.len() % 2 == 1 {
            form.push(0);
        }

        form
    }
}
