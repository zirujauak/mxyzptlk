pub mod frame;

use crate::error::*;
use crate::state::header;
use crate::state::header::*;
use crate::state::instruction::StoreResult;
use crate::state::memory::Memory;
use frame::Frame;

pub struct FrameStack {
    frames: Vec<Frame>,
}

impl FrameStack {
    pub fn new(address: usize) -> FrameStack {
        let frame = Frame::new(address, address, &Vec::new(), 0, &Vec::new(), None, 0);
        let mut frames = Vec::new();
        frames.push(frame);

        FrameStack { frames }
    }

    pub fn frames(&self) -> &Vec<Frame> {
        &self.frames
    }

    pub fn frames_mut(&mut self) -> &mut Vec<Frame> {
        &mut self.frames
    }
    
    pub fn pop_frame(&mut self) -> Result<Frame, RuntimeError> {
        if let Some(f) = self.frames.pop() {
            Ok(f)
        } else {
            Err(RuntimeError::new(
                ErrorCode::FrameUnderflow,
                format!("Frame stack is empty"),
            ))
        }
    }

    pub fn current_frame(&self) -> Result<&Frame, RuntimeError> {
        if let Some(f) = self.frames.last() {
            Ok(f)
        } else {
            Err(RuntimeError::new(
                ErrorCode::FrameUnderflow,
                format!("Frame stack is empty"),
            ))
        }
    }

    pub fn current_frame_mut(&mut self) -> Result<&mut Frame, RuntimeError> {
        if let Some(f) = self.frames.last_mut() {
            Ok(f)
        } else {
            Err(RuntimeError::new(
                ErrorCode::FrameUnderflow,
                format!("Frame stack is empty"),
            ))
        }
    }

    fn global_variable_address(
        &self,
        memory: &Memory,
        variable: usize,
    ) -> Result<usize, RuntimeError> {
        let table = header::field_word(memory, HeaderField::GlobalTable)? as usize;
        let index = (variable - 16) * 2;
        Ok(table + index)
    }

    pub fn variable(&mut self, memory: &Memory, variable: usize) -> Result<u16, RuntimeError> {
        if variable < 16 {
            self.current_frame_mut()?.variable(variable)
        } else {
            memory.read_word(self.global_variable_address(memory, variable)?)
        }
    }

    pub fn set_variable(
        &mut self,
        memory: &mut Memory,
        variable: usize,
        value: u16,
    ) -> Result<(), RuntimeError> {
        if variable < 16 {
            self.current_frame_mut()?.set_variable(variable, value)
        } else {
            memory.write_word(self.global_variable_address(memory, variable)?, value)
        }
    }

    pub fn pc(&self) -> Result<usize, RuntimeError> {
        Ok(self.current_frame()?.pc())
    }

    pub fn call_routine(
        &mut self,
        memory: &mut Memory,
        address: usize,
        arguments: &Vec<u16>,
        result: Option<StoreResult>,
        return_address: usize,
    ) -> Result<(), RuntimeError> {
        let frame = Frame::call_routine(memory, address, arguments, result, return_address)?;
        info!(target: "app::frame", "Call to ${:06x} => {:?}", address, result);
        self.frames.push(frame);
        Ok(())
    }

    pub fn input_interrupt(
        &mut self,
        memory: &mut Memory,
        address: usize,
        return_address: usize,
    ) -> Result<(), RuntimeError> {
        let mut frame = Frame::call_routine(memory, address, &vec![], None, return_address)?;
        frame.set_input_interrupt(true);
        info!(target: "app::frame", "Input interrupt ${:06x}", address);
        self.frames.push(frame);
        Ok(())
    }

    pub fn sound_interrupt(&mut self, memory: &mut Memory, address: usize, return_address: usize) -> Result<(), RuntimeError> {
        let mut frame = Frame::call_routine(memory, address, &vec![], None, return_address)?;
        info!(target: "app::frame", "Sound interrupt ${:06x}", address);
        self.frames.push(frame);
        Ok(())
    }
    
    pub fn return_routine(&mut self, _memory: &mut Memory, _result: u16) -> Result<Option<StoreResult>, RuntimeError> {
        let f = self.pop_frame()?;
        let n = self.current_frame_mut()?;
        n.set_pc(f.return_address());
        info!(target: "app::frame", "Return to ${:06x} -> {:?}", f.return_address(), f.result());
        if let Some(r) = f.result() {
            Ok(Some(r.clone()))
        } else {
            Ok(None)
        }
    }
}
