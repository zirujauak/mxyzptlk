use crate::error::*;
use crate::iff::quetzal::stks::{StackFrame, Stks};
use crate::instruction::StoreResult;

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

impl From<&StackFrame> for Frame {
    fn from(value: &StackFrame) -> Self {
        let result = if value.flags() & 0x10 == 0x00 {
            Some(StoreResult::new(0, value.result_variable()))
        } else {
            None
        };
        Frame::new(
            0,
            0,
            value.local_variables(),
            value.flags() & 0xF,
            value.stack(),
            result,
            value.return_address() as usize,
        )
    }
}

impl From<&Stks> for Vec<Frame> {
    fn from(value: &Stks) -> Self {
        let mut v = Vec::new();
        for sf in value.stks() {
            v.push(Frame::from(sf))
        }
        v
    }
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

    pub fn pop(&mut self) -> Result<u16, RuntimeError> {
        if let Some(v) = self.stack.pop() {
            info!(target: "app::stack", "Pop {:04x} [{}]", v, self.stack.len());
            Ok(v)
        } else {
            Err(RuntimeError::new(
                ErrorCode::StackUnderflow,
                format!("Poppped an empty stack"),
            ))
        }
    }

    pub fn peek(&self) -> Result<u16, RuntimeError> {
        if let Some(v) = self.stack.last() {
            Ok(*v)
        } else {
            Err(RuntimeError::new(
                ErrorCode::StackUnderflow,
                format!("Peeked an empty stack"),
            ))
        }
    }

    pub fn push(&mut self, value: u16) {
        self.stack.push(value);
        info!(target: "app::stack", "Push {:04x} [{}]", value, self.stack.len());
    }

    pub fn result(&self) -> Option<&StoreResult> {
        self.result.as_ref()
    }

    pub fn return_address(&self) -> usize {
        self.return_address
    }

    pub fn local_variable(&mut self, variable: u8) -> Result<u16, RuntimeError> {
        if variable == 0 {
            self.pop()
        } else if variable <= self.local_variables.len() as u8 {
            Ok(self.local_variables[variable as usize - 1])
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

    pub fn peek_local_variable(&self, variable: u8) -> Result<u16, RuntimeError> {
        if variable == 0 {
            self.peek()
        } else if variable <= self.local_variables().len() as u8 {
            Ok(self.local_variables[variable as usize - 1])
        } else {
            Err(RuntimeError::new(
                ErrorCode::InvalidLocalVariable,
                format!(
                    "Peek for local variable {} out of range ({})",
                    variable,
                    self.local_variables.len()
                ),
            ))
        }
    }
    pub fn set_local_variable(&mut self, variable: u8, value: u16) -> Result<(), RuntimeError> {
        if variable == 0 {
            self.push(value);
            Ok(())
        } else if variable <= self.local_variables.len() as u8 {
            self.local_variables[variable as usize - 1] = value;
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

    pub fn set_local_variable_indirect(
        &mut self,
        variable: u8,
        value: u16,
    ) -> Result<(), RuntimeError> {
        if variable == 0 {
            self.pop()?;
            self.push(value);
            Ok(())
        } else if variable <= self.local_variables().len() as u8 {
            self.local_variables[variable as usize - 1] = value;
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
        address: usize,
        initial_pc: usize,
        arguments: &Vec<u16>,
        local_variables: Vec<u16>,
        result: Option<StoreResult>,
        return_address: usize,
    ) -> Result<Frame, RuntimeError> {
        let mut local_variables = local_variables.clone();

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
            return_address,
        ))
    }

    pub fn call_input_interrupt(
        address: usize,
        initial_pc: usize,
        local_variables: Vec<u16>,
        return_address: usize,
    ) -> Result<Frame, RuntimeError> {
        let mut f = Frame::new(
            address,
            initial_pc,
            &local_variables,
            0,
            &Vec::new(),
            None,
            return_address,
        );
        f.input_interrupt = true;
        Ok(f)
    }
}
