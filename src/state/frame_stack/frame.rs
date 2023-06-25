use crate::error::*;
use crate::state::header;
use crate::state::header::*;
use crate::state::instruction::StoreResult;
use crate::state::memory::*;

#[derive(Debug)]
pub struct Frame {
    address: usize,
    pc: usize,
    local_variables: Vec<u16>,
    argument_count: u8,
    stack: Vec<u16>,
    result: Option<StoreResult>,
    return_address: usize,
    input_interrupt: bool,
    sound_interrupt: bool,
}

impl Frame {
    pub fn new(
        address: usize,
        pc: usize,
        local_variables: &Vec<u16>,
        argument_count: u8,
        stack: &Vec<u16>,
        result: Option<StoreResult>,
        return_address: usize,
    ) -> Frame {
        Frame {
            address,
            pc,
            local_variables: local_variables.clone(),
            argument_count,
            stack: stack.clone(),
            result: result.clone(),
            return_address,
            input_interrupt: false,
            sound_interrupt: false,
        }
    }

    pub fn address(&self) -> usize {
        self.address
    }

    pub fn pc(&self) -> usize {
        self.pc
    }

    pub fn set_pc(&mut self, pc: usize) {
        self.pc = pc;
    }

    pub fn local_variables(&self) -> &Vec<u16> {
        &self.local_variables
    }

    pub fn local_variables_mut(&mut self) -> &mut Vec<u16> {
        &mut self.local_variables
    }

    pub fn argument_count(&self) -> u8 {
        self.argument_count
    }

    pub fn stack(&self) -> &Vec<u16> {
        &self.stack
    }

    pub fn input_interrupt(&self) -> bool {
        self.input_interrupt
    }

    pub fn set_input_interrupt(&mut self, v: bool) {
        self.input_interrupt = v;
    }

    pub fn sound_interrupt(&self) -> bool {
        self.sound_interrupt
    }

    pub fn set_sound_interrupt(&mut self, v: bool) {
        self.sound_interrupt = v;
    }
    
    pub fn pop(&mut self) -> Result<u16,RuntimeError> {
        if let Some(v) = self.stack.pop() {
            Ok(v)
        } else {
            Err(RuntimeError::new(ErrorCode::StackUnderflow, format!("Poppped an empty stack")))
        }
    }

    pub fn peek(&self) -> Result<u16,RuntimeError> {
        if let Some(v) = self.stack.last() {
            Ok(*v)
        } else {
            Err(RuntimeError::new(ErrorCode::StackUnderflow, format!("Peeked an empty stack")))
        }
    }

    pub fn push(&mut self, value: u16)  {
        self.stack.push(value);
    }

    pub fn result(&self) -> Option<&StoreResult> {
        self.result.as_ref()
    }

    pub fn return_address(&self) -> usize {
        self.return_address
    }

    pub fn variable(&mut self, variable: usize) -> Result<u16, RuntimeError> {
        if variable == 0 {
            if let Some(v) = self.stack.pop() {
                Ok(v)
            } else {
                Err(RuntimeError::new(
                    ErrorCode::StackUnderflow,
                    format!("Popped an empty stack"),
                ))
            }
        } else if variable <= self.local_variables.len() {
            Ok(self.local_variables[variable - 1])
        } else {
            Err(RuntimeError::new(
                ErrorCode::InvalidLocalVariable,
                format!(
                    "Read for local variable {} out of range ({})",
                    variable,
                    self.local_variables.len()
                ),
            ))
        }
    }

    pub fn set_variable(&mut self, variable: usize, value: u16) -> Result<(), RuntimeError> {
        if variable == 0 {
            self.stack.push(value);
            Ok(())
        } else if variable <= self.local_variables.len() {
            self.local_variables[variable - 1] = value;
            Ok(())
        } else {
            Err(RuntimeError::new(
                ErrorCode::InvalidLocalVariable,
                format!(
                    "Write to local variable {} out of range ({})",
                    variable,
                    self.local_variables.len()
                ),
            ))
        }
    }

    pub fn call_routine(
        memory: &Memory,
        address: usize,
        arguments: &Vec<u16>,
        result: Option<StoreResult>,
        return_address: usize,
    ) -> Result<Frame, RuntimeError> {
        let version = header::field_byte(memory, HeaderField::Version)?;
        let var_count = memory.read_byte(address)?;
        let initial_pc = if version < 5 {
            address + 1 + (var_count as usize * 2)
        } else {
            address + 1
        };

        let mut local_variables = if version < 5 {
            let mut v = Vec::new();
            for i in 0..var_count as usize {
                let addr = address + 1 + (2 * i);
                v.push(memory.read_word(addr)?);
            }
            v
        } else {
            vec![0 as u16; var_count as usize]
        };

        for i in 0..arguments.len() {
            if local_variables.len() > i {
                local_variables[i] = arguments[i]
            }
        }

        Ok(Frame::new(
            address,
            initial_pc,
            &local_variables,
            arguments.len() as u8,
            &Vec::new(),
            result,
            return_address
        ))
    }
}
