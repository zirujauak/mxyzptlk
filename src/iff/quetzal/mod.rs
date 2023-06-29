use crate::error::{ErrorCode, RuntimeError};

use self::{cmem::CMem, ifhd::IFhd, stks::Stks, umem::UMem};

use super::*;

pub mod cmem;
pub mod ifhd;
pub mod stks;
mod umem;

pub struct Quetzal {
    ifhd: IFhd,
    umem: Option<UMem>,
    cmem: Option<CMem>,
    stks: Stks,
}

impl TryFrom<Vec<u8>> for Quetzal {
    type Error = RuntimeError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        if !IFF::form_from_vec(&value)?.eq("FORM") {
            error!(target: "app::quetzal", "Not an IFF file");
            return Err(RuntimeError::new(
                ErrorCode::IFF,
                "Not an IFF file".to_string(),
            ));
        }

        let iff = IFF::from_vec(&value);
        if iff.form != "FORM" || iff.sub_form != "IFZS" {
            error!(target: "app::quetzal", "Expected sub form 'IFZS': '{}'", iff.sub_form);
            return Err(RuntimeError::new(
                ErrorCode::Restore,
                "Not a Quetzal save file".to_string(),
            ));
        }

        let mut ifhd = None;
        let mut umem = None;
        let mut cmem = None;
        let mut stks = None;
        for chunk in iff.chunks {
            match chunk.id.as_str() {
                "IFhd" => ifhd = Some(IFhd::from(chunk)),
                "CMem" => cmem = Some(CMem::from(chunk)),
                "UMem" => umem = Some(UMem::from(chunk)),
                "Stks" => stks = Some(Stks::from(chunk)),
                _ => debug!(target: "app::quetzal", "Ignoring chunk with id '{}'", chunk.id),
            }
        }

        if let None = ifhd {
            return Err(RuntimeError::new(
                ErrorCode::Restore,
                "Quetzal is missing IFhd chunk".to_string(),
            ));
        }

        if let None = stks {
            return Err(RuntimeError::new(
                ErrorCode::Restore,
                "Quetzal is missing Stks chunk".to_string(),
            ));
        }

        if let None = cmem {
            if let None = umem {
                return Err(RuntimeError::new(
                    ErrorCode::Restore,
                    "Quetzal is missing memory (CMem or UMem) chunk".to_string(),
                ));
            }
        }

        Ok(Quetzal::new(ifhd.unwrap(), umem, cmem, stks.unwrap()))
    }
}

impl From<Quetzal> for Vec<u8> {
    fn from(value: Quetzal) -> Self {
        let mut form = id_as_vec("FORM");

        let mut ifzs = id_as_vec("IFZS");
        ifzs.append(&mut Vec::from(value.ifhd()));
        // ifzs.append(&mut value.ifhd().to_chunk());
        match value.umem() {
            Some(u) => ifzs.append(&mut Vec::from(u)),
            None => (),
        };
        match value.cmem() {
            Some(c) => ifzs.append(&mut Vec::from(c)),
            None => (),
        };
        ifzs.append(&mut Vec::from(value.stks()));

        form.append(&mut usize_as_vec(ifzs.len(), 4));
        form.append(&mut ifzs);
        if form.len() % 2 == 1 {
            form.push(0);
        }

        form
    }
}

impl Quetzal {
    pub fn new(ifhd: IFhd, umem: Option<UMem>, cmem: Option<CMem>, stks: Stks) -> Quetzal {
        Quetzal {
            ifhd,
            umem: umem,
            cmem: cmem,
            stks,
        }
    }

    pub fn ifhd(&self) -> &IFhd {
        &self.ifhd
    }

    pub fn umem(&self) -> Option<&UMem> {
        self.umem.as_ref()
    }

    pub fn cmem(&self) -> Option<&CMem> {
        self.cmem.as_ref()
    }

    pub fn stks(&self) -> &Stks {
        &self.stks
    }
}
