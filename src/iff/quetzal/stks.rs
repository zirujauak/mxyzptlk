use crate::state::State;

use super::super::*;

pub struct StackFrame {
    pub return_address: u32,
    pub flags: u8,
    pub result_variable: u8,
    pub arguments: u8,
    pub stack_size: u16,
    pub local_variables: Vec<u16>,
    pub stack: Vec<u16>,
}

pub struct Stks {
    pub stks: Vec<StackFrame>,
}

impl Stks {
    pub fn from_state(state: &State) -> Stks {
        let mut stks = Vec::new();
        for f in state.frame_stack().frames() {
            let flags = match f.result() {
                Some(_) => 0x00,
                None => 0x10,
            } | f.local_variables().len();
            let mut arguments = 0;
            for _ in 0..f.argument_count() {
                arguments = (arguments << 1) + 1;
            }

            stks.push(StackFrame {
                return_address: f.return_address() as u32,
                flags: flags as u8,
                result_variable: match f.result() {
                    Some(v) => v.variable(),
                    None => 0,
                },
                arguments,
                stack_size: f.stack().len() as u16,
                local_variables: f.local_variables().clone(),
                stack: f.stack().clone(),
            });
        }

        Stks { stks }
    }

    pub fn from_vec(chunk: Vec<u8>) -> Stks {
        let mut position = 0;
        let mut stks = Vec::new();
        while chunk.len() - position > 1 {
            let return_address = vec_as_usize(chunk[position..position + 3].to_vec(), 3) as u32;
            let flags = chunk[position + 3];
            let result_variable = chunk[position + 4];
            let arguments = chunk[position + 5];

            position = position + 6;
            let stack_size = vec_as_usize(chunk[position..position + 2].to_vec(), 2) as u16;
            position = position + 2;

            let mut local_variables = Vec::new();
            for i in 0..flags as usize & 0xF {
                let offset = position + (i * 2);
                local_variables.push(vec_as_usize(chunk[offset..offset + 2].to_vec(), 2) as u16);
            }
            position = position + (local_variables.len() * 2);

            let mut stack = Vec::new();
            for i in 0..stack_size as usize {
                let offset = position + (i * 2);
                stack.push(vec_as_usize(chunk[offset..offset + 2].to_vec(), 2) as u16)
            }
            position = position + (stack_size as usize * 2);

            stks.push(StackFrame {
                return_address,
                flags,
                result_variable,
                arguments,
                local_variables,
                stack_size,
                stack,
            });
        }
        Stks { stks }
    }

    pub fn from_chunk(chunk: Chunk) -> Stks {
        let mut position = 0;
        let mut stks = Vec::new();
        while chunk.data.len() - position > 1 {
            let return_address = vec_to_u32(&chunk.data, position, 3) as u32;
            let flags = chunk.data[position + 3];
            let result_variable = chunk.data[position + 4];
            let arguments = chunk.data[position + 5];

            position = position + 6;
            let stack_size = vec_to_u32(&chunk.data, position, 2) as u16;
            position = position + 2;

            let mut local_variables = Vec::new();
            for i in 0..flags as usize & 0xF {
                let offset = position + (i * 2);
                local_variables.push(vec_to_u32(&chunk.data, offset, 2) as u16);
            }
            position = position + (local_variables.len() * 2);

            let mut stack = Vec::new();
            for i in 0..stack_size as usize {
                let offset = position + (i * 2);
                stack.push(vec_to_u32(&chunk.data, offset, 2) as u16);
            }
            position = position + (stack_size as usize * 2);

            let lv = local_variables.len();
            let st = stack.len();

            stks.push(StackFrame {
                return_address,
                flags,
                result_variable,
                arguments,
                local_variables,
                stack_size,
                stack,
            });
        }
        Stks { stks }
    }

    pub fn to_chunk(&self) -> Vec<u8> {
        let mut data = Vec::new();
        for stk in &self.stks {
            data.append(&mut usize_as_vec(stk.return_address as usize, 3));
            data.push(stk.flags);
            data.push(stk.result_variable);
            data.push(stk.arguments);
            data.append(&mut usize_as_vec(stk.stack_size as usize, 2));
            for i in 0..stk.local_variables.len() {
                data.append(&mut usize_as_vec(stk.local_variables[i] as usize, 2));
            }
            for i in 0..stk.stack.len() {
                data.append(&mut usize_as_vec(stk.stack[i] as usize, 2));
            }
        }
        chunk("Stks", &mut data)
    }
}
