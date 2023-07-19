use std::fmt;

use super::super::*;

#[derive(Clone, Debug)]
pub struct StackFrame {
    return_address: u32,
    flags: u8,
    result_variable: u8,
    arguments: u8,
    local_variables: Vec<u16>,
    stack: Vec<u16>,
}

impl PartialEq for StackFrame {
    fn eq(&self, other: &Self) -> bool {
        self.return_address == other.return_address
            && self.flags == other.flags
            && self.result_variable == other.result_variable
            && self.arguments == other.arguments
            && self.local_variables == other.local_variables
            && self.stack == other.stack
    }
}

impl fmt::Display for StackFrame {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Stks:")?;
        writeln!(f, "\tReturn address: ${:06x}", self.return_address)?;
        writeln!(f, "\tFlags: {:02x}", self.flags)?;
        writeln!(f, "\tResult Variable: {:?}", self.result_variable)?;
        writeln!(f, "\tArguments: {}", self.arguments)?;
        write!(f, "\tLocal Variables:")?;
        for i in 0..self.local_variables.len() {
            write!(f, " {:04x}", self.local_variables[i])?;
        }
        writeln!(f)?;
        write!(f, "\tStack:")?;
        for i in 0..self.stack.len() {
            write!(f, " {:04x}", self.stack[i])?;
        }
        write!(f, "")
    }
}

impl From<&StackFrame> for Vec<u8> {
    fn from(value: &StackFrame) -> Vec<u8> {
        let mut data = Vec::new();
        data.append(&mut usize_as_vec(value.return_address() as usize, 3));
        data.push(value.flags());
        data.push(value.result_variable());
        data.push(value.arguments());
        data.append(&mut usize_as_vec(value.stack().len(), 2));
        for i in 0..value.local_variables().len() {
            data.append(&mut usize_as_vec(value.local_variables()[i] as usize, 2));
        }
        for i in 0..value.stack().len() {
            data.append(&mut usize_as_vec(value.stack()[i] as usize, 2));
        }

        data
    }
}

impl StackFrame {
    pub fn new(
        return_address: u32,
        flags: u8,
        result_variable: u8,
        arguments: u8,
        local_variables: &[u16],
        stack: &[u16],
    ) -> StackFrame {
        StackFrame {
            return_address,
            flags,
            result_variable,
            arguments,
            local_variables: local_variables.to_vec(),
            stack: stack.to_vec(),
        }
    }

    pub fn return_address(&self) -> u32 {
        self.return_address
    }

    pub fn flags(&self) -> u8 {
        self.flags
    }

    pub fn result_variable(&self) -> u8 {
        self.result_variable
    }

    pub fn arguments(&self) -> u8 {
        self.arguments
    }

    pub fn local_variables(&self) -> &Vec<u16> {
        &self.local_variables
    }

    pub fn stack(&self) -> &Vec<u16> {
        &self.stack
    }
}

pub struct Stks {
    stks: Vec<StackFrame>,
}

impl fmt::Display for Stks {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for frame in &self.stks {
            writeln!(f, "{}", frame)?;
        }
        write!(f, "")
    }
}

impl From<Vec<u8>> for Stks {
    fn from(value: Vec<u8>) -> Stks {
        let mut position = 0;
        let mut stks = Vec::new();
        while value.len() - position > 1 {
            let return_address = vec_as_usize(value[position..position + 3].to_vec(), 3) as u32;
            let flags = value[position + 3];
            let result_variable = value[position + 4];
            let arguments = value[position + 5];

            position += 6;
            let stack_size = vec_as_usize(value[position..position + 2].to_vec(), 2) as u16;
            position += 2;

            let mut local_variables = Vec::new();
            for i in 0..flags as usize & 0xF {
                let offset = position + (i * 2);
                local_variables.push(vec_as_usize(value[offset..offset + 2].to_vec(), 2) as u16);
            }
            position += local_variables.len() * 2;

            let mut stack = Vec::new();
            for i in 0..stack_size as usize {
                let offset = position + (i * 2);
                stack.push(vec_as_usize(value[offset..offset + 2].to_vec(), 2) as u16)
            }
            position += stack_size as usize * 2;

            stks.push(StackFrame {
                return_address,
                flags,
                result_variable,
                arguments,
                local_variables,
                stack,
            });
        }
        Stks::new(stks)
    }
}

impl From<Chunk> for Stks {
    fn from(value: Chunk) -> Stks {
        let mut position = 0;
        let mut stks = Vec::new();
        while value.data.len() - position > 1 {
            let return_address = vec_to_u32(&value.data, position, 3);
            let flags = value.data[position + 3];
            let result_variable = value.data[position + 4];
            let arguments = value.data[position + 5];

            position += 6;
            let stack_size = vec_to_u32(&value.data, position, 2) as u16;
            position += 2;

            let mut local_variables = Vec::new();
            for i in 0..flags as usize & 0xF {
                let offset = position + (i * 2);
                local_variables.push(vec_to_u32(&value.data, offset, 2) as u16);
            }
            position += local_variables.len() * 2;

            let mut stack = Vec::new();
            for i in 0..stack_size as usize {
                let offset = position + (i * 2);
                stack.push(vec_to_u32(&value.data, offset, 2) as u16);
            }
            position += stack_size as usize * 2;

            stks.push(StackFrame::new(
                return_address,
                flags,
                result_variable,
                arguments,
                &local_variables,
                &stack,
            ));
        }
        Stks::new(stks)
    }
}

impl From<&Stks> for Vec<u8> {
    fn from(value: &Stks) -> Vec<u8> {
        let mut data = Vec::new();
        for stk in value.stks() {
            data.append(&mut Vec::from(stk));
        }
        chunk("Stks", &data)
    }
}

impl Stks {
    pub fn new(stks: Vec<StackFrame>) -> Stks {
        Stks { stks }
    }

    pub fn stks(&self) -> &Vec<StackFrame> {
        &self.stks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stackframe_new() {
        let sf = StackFrame::new(
            0x123456,
            0x1F,
            0xFE,
            4,
            &[0x1, 0x2, 0x3, 0x4],
            &[0x11, 0x22, 0x33],
        );
        assert_eq!(sf.return_address(), 0x123456);
        assert_eq!(sf.flags(), 0x1F);
        assert_eq!(sf.result_variable(), 0xFE);
        assert_eq!(sf.arguments(), 4);
        assert_eq!(sf.local_variables(), &[1, 2, 3, 4]);
        assert_eq!(sf.stack(), &[0x11, 0x22, 0x33]);
    }

    #[test]
    fn test_stackrame_partial_eq() {
        let sf1 = StackFrame::new(
            0x123456,
            0x1F,
            0xFE,
            4,
            &[0x1, 0x2, 0x3, 0x4],
            &[0x11, 0x22, 0x33],
        );
        let sf2 = StackFrame::new(
            0x123456,
            0x1F,
            0xFE,
            4,
            &[0x1, 0x2, 0x3, 0x4],
            &[0x11, 0x22, 0x33],
        );
        let sf3 = StackFrame::new(
            0x123457,
            0x1F,
            0xFE,
            4,
            &[0x1, 0x2, 0x3, 0x4],
            &[0x11, 0x22, 0x33],
        );
        let sf4 = StackFrame::new(
            0x123456,
            0x10,
            0xFE,
            4,
            &[0x1, 0x2, 0x3, 0x4],
            &[0x11, 0x22, 0x33],
        );
        let sf5 = StackFrame::new(
            0x123456,
            0x1F,
            0xFE,
            0,
            &[0x1, 0x2, 0x3, 0x4],
            &[0x11, 0x22, 0x33],
        );
        let sf6 = StackFrame::new(
            0x123456,
            0x1F,
            0xFE,
            4,
            &[0x11, 0x2, 0x3, 0x4],
            &[0x11, 0x22, 0x33],
        );
        let sf7 = StackFrame::new(
            0x123456,
            0x1F,
            0xFE,
            4,
            &[0x1, 0x2, 0x3, 0x4],
            &[0x11, 0x22, 0x33, 0x44],
        );
        assert_eq!(sf1, sf2);
        assert_eq!(sf2, sf1);
        assert_ne!(sf1, sf3);
        assert_ne!(sf1, sf4);
        assert_ne!(sf1, sf5);
        assert_ne!(sf1, sf6);
        assert_ne!(sf1, sf7);
    }

    #[test]
    fn test_vec_u8_from_stackframe() {
        let sf = StackFrame::new(
            0x123456,
            0x1F,
            0xFE,
            4,
            &[0x1, 0x2, 0x3, 0x4],
            &[0x11, 0x22, 0x33],
        );
        assert_eq!(
            Vec::from(&sf),
            &[
                0x12, 0x34, 0x56, 0x1F, 0xFE, 0x04, 0x00, 0x03, 0x00, 0x01, 0x00, 0x02, 0x00, 0x03,
                0x00, 0x04, 0x00, 0x11, 0x00, 0x22, 0x00, 0x33
            ]
        );
    }

    #[test]
    fn test_stks_new() {
        let sf1 = StackFrame::new(
            0x123456,
            0x1F,
            0xFE,
            4,
            &[0x1, 0x2, 0x3, 0x4],
            &[0x11, 0x22, 0x33],
        );
        let sf2 = StackFrame::new(0x654321, 0, 0, 0, &[], &[]);
        let stks = Stks::new(vec![sf1.clone(), sf2.clone()]);
        assert_eq!(stks.stks()[0], sf1);
        assert_eq!(stks.stks()[1], sf2);
    }

    #[test]
    fn test_stks_from_vec_u8() {
        let v = vec![
            0x12, 0x34, 0x56, 0x14, 0xFE, 0x04, 0x00, 0x03, 0x00, 0x01, 0x00, 0x02, 0x00, 0x03,
            0x00, 0x04, 0x00, 0x11, 0x00, 0x22, 0x00, 0x33, 0x78, 0x9A, 0xBC, 0x03, 0, 0, 0x00,
            0x01, 0x00, 0x01, 0x00, 0x02, 0x00, 0x03, 0x00, 0x11, 0x00, 0xF0, 0xAD, 0x00, 0xFE,
            0x00, 0x00, 0x00,
        ];
        let stks = Stks::from(v);
        assert_eq!(stks.stks().len(), 3);
        assert_eq!(stks.stks()[0].return_address(), 0x123456);
        assert_eq!(stks.stks()[0].flags(), 0x14);
        assert_eq!(stks.stks()[0].result_variable(), 0xFE);
        assert_eq!(stks.stks()[0].arguments(), 4);
        assert_eq!(stks.stks()[0].local_variables(), &[0x01, 0x02, 0x03, 0x04]);
        assert_eq!(stks.stks()[0].stack(), &[0x11, 0x22, 0x33]);
        assert_eq!(stks.stks()[1].return_address(), 0x789ABC);
        assert_eq!(stks.stks()[1].flags(), 0x3);
        assert_eq!(stks.stks()[1].result_variable(), 0);
        assert_eq!(stks.stks()[1].arguments(), 0);
        assert_eq!(stks.stks()[1].local_variables(), &[0x01, 0x02, 0x03]);
        assert_eq!(stks.stks()[1].stack(), &[0x11]);
        assert_eq!(stks.stks()[2].return_address(), 0xF0AD);
        assert_eq!(stks.stks()[2].flags(), 0x0);
        assert_eq!(stks.stks()[2].result_variable(), 0xFE);
        assert_eq!(stks.stks()[2].arguments(), 0);
        assert!(stks.stks()[2].local_variables().is_empty());
        assert!(stks.stks()[2].stack().is_empty());
    }

    #[test]
    fn test_stks_from_chunk() {
        let chunk = Chunk::new(
            0,
            None,
            "Stks".to_string(),
            &vec![
                0x12, 0x34, 0x56, 0x14, 0xFE, 0x04, 0x00, 0x03, 0x00, 0x01, 0x00, 0x02, 0x00, 0x03,
                0x00, 0x04, 0x00, 0x11, 0x00, 0x22, 0x00, 0x33, 0x78, 0x9A, 0xBC, 0x03, 0, 0, 0x00,
                0x01, 0x00, 0x01, 0x00, 0x02, 0x00, 0x03, 0x00, 0x11, 0x00, 0xF0, 0xAD, 0x00, 0xFE,
                0x00, 0x00, 0x00,
            ],
        );
        let stks = Stks::from(chunk);
        assert_eq!(stks.stks().len(), 3);
        assert_eq!(stks.stks()[0].return_address(), 0x123456);
        assert_eq!(stks.stks()[0].flags(), 0x14);
        assert_eq!(stks.stks()[0].result_variable(), 0xFE);
        assert_eq!(stks.stks()[0].arguments(), 4);
        assert_eq!(stks.stks()[0].local_variables(), &[0x01, 0x02, 0x03, 0x04]);
        assert_eq!(stks.stks()[0].stack(), &[0x11, 0x22, 0x33]);
        assert_eq!(stks.stks()[1].return_address(), 0x789ABC);
        assert_eq!(stks.stks()[1].flags(), 0x3);
        assert_eq!(stks.stks()[1].result_variable(), 0);
        assert_eq!(stks.stks()[1].arguments(), 0);
        assert_eq!(stks.stks()[1].local_variables(), &[0x01, 0x02, 0x03]);
        assert_eq!(stks.stks()[1].stack(), &[0x11]);
        assert_eq!(stks.stks()[2].return_address(), 0xF0AD);
        assert_eq!(stks.stks()[2].flags(), 0x0);
        assert_eq!(stks.stks()[2].result_variable(), 0xFE);
        assert_eq!(stks.stks()[2].arguments(), 0);
        assert!(stks.stks()[2].local_variables().is_empty());
        assert!(stks.stks()[2].stack().is_empty());
    }

    #[test]
    fn test_vec_u8_from_stks() {
        let sf1 = StackFrame::new(
            0x123456,
            0x14,
            0xFE,
            4,
            &[0x1, 0x2, 0x3, 0x4],
            &[0x11, 0x22, 0x33],
        );
        let sf2 = StackFrame::new(0x654321, 0, 0, 0, &[], &[]);
        let stks = Stks::new(vec![sf1, sf2]);
        let v: Vec<u8> = Vec::from(&stks);
        assert_eq!(
            v,
            [
                b'S', b't', b'k', b's', 0x00, 0x00, 0x00, 0x1E, 0x12, 0x34, 0x56, 0x14, 0xFE, 4,
                0x00, 0x03, 0x00, 0x01, 0x00, 0x02, 0x00, 0x03, 0x00, 0x04, 0x00, 0x11, 0x00, 0x22,
                0x00, 0x33, 0x65, 0x43, 0x21, 0x00, 0x00, 0x00, 0x00, 0x00
            ]
        )
    }
}
