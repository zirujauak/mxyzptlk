use std::fmt;

use super::super::*;

pub struct StackFrame {
    return_address: u32,
    flags: u8,
    result_variable: u8,
    arguments: u8,
    local_variables: Vec<u16>,
    stack: Vec<u16>,
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
        writeln!(f, "")?;
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
        local_variables: &Vec<u16>,
        stack: &Vec<u16>,
    ) -> StackFrame {
        StackFrame {
            return_address,
            flags,
            result_variable,
            arguments,
            local_variables: local_variables.clone(),
            stack: stack.clone(),
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

            position = position + 6;
            let stack_size = vec_as_usize(value[position..position + 2].to_vec(), 2) as u16;
            position = position + 2;

            let mut local_variables = Vec::new();
            for i in 0..flags as usize & 0xF {
                let offset = position + (i * 2);
                local_variables.push(vec_as_usize(value[offset..offset + 2].to_vec(), 2) as u16);
            }
            position = position + (local_variables.len() * 2);

            let mut stack = Vec::new();
            for i in 0..stack_size as usize {
                let offset = position + (i * 2);
                stack.push(vec_as_usize(value[offset..offset + 2].to_vec(), 2) as u16)
            }
            position = position + (stack_size as usize * 2);

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
            let return_address = vec_to_u32(&value.data, position, 3) as u32;
            let flags = value.data[position + 3];
            let result_variable = value.data[position + 4];
            let arguments = value.data[position + 5];

            position = position + 6;
            let stack_size = vec_to_u32(&value.data, position, 2) as u16;
            position = position + 2;

            let mut local_variables = Vec::new();
            for i in 0..flags as usize & 0xF {
                let offset = position + (i * 2);
                local_variables.push(vec_to_u32(&value.data, offset, 2) as u16);
            }
            position = position + (local_variables.len() * 2);

            let mut stack = Vec::new();
            for i in 0..stack_size as usize {
                let offset = position + (i * 2);
                stack.push(vec_to_u32(&value.data, offset, 2) as u16);
            }
            position = position + (stack_size as usize * 2);

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
        chunk("Stks", &mut data)
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
