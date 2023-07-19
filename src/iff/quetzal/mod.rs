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
        if !vec_to_id(&value, 0).eq("FORM") {
            error!(target: "app::quetzal", "Not an IFF file");
            return Err(RuntimeError::new(
                ErrorCode::IFF,
                "Not an IFF file".to_string(),
            ));
        }

        let iff = IFF::from(&value);
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

        if ifhd.is_none() {
            return Err(RuntimeError::new(
                ErrorCode::Restore,
                "Quetzal is missing IFhd chunk".to_string(),
            ));
        }

        if stks.is_none() {
            return Err(RuntimeError::new(
                ErrorCode::Restore,
                "Quetzal is missing Stks chunk".to_string(),
            ));
        }

        if cmem.is_none() && umem.is_none() {
            return Err(RuntimeError::new(
                ErrorCode::Restore,
                "Quetzal is missing memory (CMem or UMem) chunk".to_string(),
            ));
        }

        Ok(Quetzal::new(ifhd.unwrap(), umem, cmem, stks.unwrap()))
    }
}

impl From<Quetzal> for Vec<u8> {
    fn from(value: Quetzal) -> Self {
        let mut form = id_as_vec("FORM");

        let mut ifzs = id_as_vec("IFZS");
        ifzs.append(&mut Vec::from(value.ifhd()));
        if let Some(u) = value.umem() {
            ifzs.append(&mut Vec::from(u))
        }

        if let Some(c) = value.cmem() {
            ifzs.append(&mut Vec::from(c))
        }

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
            umem,
            cmem,
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

#[cfg(test)]
mod tests {
    use crate::{assert_some, assert_ok};

    use super::{stks::StackFrame, *};

    #[test]
    fn test_new() {
        let ifhd = IFhd::new(0x1234, &[1, 2, 3, 4, 5, 6], 0x4321, 0xFEDCBA);
        let umem = UMem::new(&[1, 2, 3, 4]);
        let cmem = CMem::new(&[5, 6, 7, 8]);
        let sf = StackFrame::new(
            0x123456,
            0x13,
            0x34,
            2,
            &[0x11, 0x22, 0x33, 0x44, 0x55, 0x66],
            &[0x88, 0x99, 0xAA, 0xBB],
        );
        let stks = Stks::new(vec![sf.clone()]);
        let quetzal = Quetzal::new(ifhd.clone(), Some(umem), Some(cmem), stks);
        assert_eq!(quetzal.ifhd(), &ifhd);
        assert_eq!(assert_some!(quetzal.umem()).data(), &[1, 2, 3, 4]);
        assert_eq!(assert_some!(quetzal.cmem()).data(), &[5, 6, 7, 8]);
        assert_eq!(quetzal.stks().stks().len(), 1);
        assert_eq!(quetzal.stks().stks()[0], sf);
    }

    #[test]
    fn test_try_from_vec_u8() {
        let v = vec![
            b'F', b'O', b'R', b'M', 0x00, 0x00, 0x00, 0x50, b'I', b'F', b'Z', b'S', b'I', b'F',
            b'h', b'd', 0x00, 0x00, 0x00, 0x0d, 0x12, 0x34, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06,
            0x56, 0x78, 0x11, 0x22, 0x33, 0x00, b'C', b'M', b'e', b'm', 0x00, 0x00, 0x00, 0x08,
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, b'S', b't', b'k', b's', 0x00, 0x00,
            0x00, 0x1E, 0x12, 0x34, 0x56, 0x14, 0xFE, 0x04, 0x00, 0x03, 0x00, 0x01, 0x00, 0x02,
            0x00, 0x03, 0x00, 0x04, 0x00, 0x11, 0x00, 0x22, 0x00, 0x33, 0x65, 0x43, 0x21, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ];
        let quetzal = assert_ok!(Quetzal::try_from(v));
        assert_eq!(quetzal.ifhd().release_number(), 0x1234);
        assert_eq!(quetzal.ifhd().serial_number(), &[1, 2, 3, 4, 5, 6]);
        assert_eq!(quetzal.ifhd().checksum(), 0x5678);
        assert_eq!(quetzal.ifhd().pc(), 0x112233);
        assert_eq!(assert_some!(quetzal.cmem()).data(), &[1, 2, 3, 4, 5, 6, 7, 8]);
        assert!(quetzal.umem().is_none());
        assert_eq!(quetzal.stks().stks().len(), 2);
        assert_eq!(quetzal.stks().stks()[0].return_address(), 0x123456);
        assert_eq!(quetzal.stks().stks()[0].flags(), 0x14);
        assert_eq!(quetzal.stks().stks()[0].result_variable(), 0xFE);
        assert_eq!(quetzal.stks().stks()[0].arguments(), 4);
        assert_eq!(
            quetzal.stks().stks()[0].local_variables(),
            &[0x01, 0x02, 0x03, 0x04]
        );
        assert_eq!(quetzal.stks().stks()[0].stack(), &[0x11, 0x22, 0x33]);
        assert_eq!(quetzal.stks().stks()[1].return_address(), 0x654321);
        assert_eq!(quetzal.stks().stks()[1].flags(), 0);
        assert_eq!(quetzal.stks().stks()[1].result_variable(), 0);
        assert_eq!(quetzal.stks().stks()[1].arguments(), 0);
        assert!(quetzal.stks().stks()[1].local_variables().is_empty());
        assert!(quetzal.stks().stks()[1].stack().is_empty());
    }

    #[test]
    fn test_try_from_vec_u8_error() {
        let v = vec![
            b'F', b'O', b'R', b'M', 0x00, 0x00, 0x00, 0x50, b'I', b'F', b'R', b'S',
        ];
        assert!(Quetzal::try_from(v).is_err());

        let v = vec![
            b'F', b'R', b'O', b'B', 0x00, 0x00, 0x00, 0x50, b'I', b'F', b'Z', b'S',
        ];
        assert!(Quetzal::try_from(v).is_err());
    }

    #[test]
    fn test_vec_u8_from_quetzal() {
        let ifhd = IFhd::new(0x1234, &[1, 2, 3, 4, 5, 6], 0x4321, 0xFEDCBA);
        let cmem = CMem::new(&[5, 6, 7, 8]);
        let sf = StackFrame::new(
            0x123456,
            0x13,
            0x34,
            2,
            &[0x11, 0x22, 0x33, 0x44, 0x55, 0x66],
            &[0x88, 0x99, 0xAA, 0xBB],
        );
        let stks = Stks::new(vec![sf]);
        let quetzal = Quetzal::new(ifhd, None, Some(cmem), stks);
        assert_eq!(
            Vec::from(quetzal),
            &[
                b'F', b'O', b'R', b'M', 0x00, 0x00, 0x00, 0x4a, b'I', b'F', b'Z', b'S', b'I', b'F',
                b'h', b'd', 0x00, 0x00, 0x00, 0x0d, 0x12, 0x34, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06,
                0x43, 0x21, 0xFE, 0xDC, 0xBA, 0x00, b'C', b'M', b'e', b'm', 0x00, 0x00, 0x00, 0x04,
                0x05, 0x06, 0x07, 0x08, b'S', b't', b'k', b's', 0x00, 0x00, 0x00, 0x1C, 0x12, 0x34,
                0x56, 0x13, 0x34, 0x02, 0x00, 0x04, 0x00, 0x11, 0x00, 0x22, 0x00, 0x33, 0x00, 0x44,
                0x00, 0x55, 0x00, 0x66, 0x00, 0x88, 0x00, 0x99, 0x00, 0xAA, 0x00, 0xBB
            ]
        )
    }
}
