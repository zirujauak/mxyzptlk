pub mod frame;

use crate::error::*;
use crate::iff::quetzal::stks::Stks;
use crate::state::instruction::StoreResult;
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

    pub fn from_stks(stks: &Stks) -> Result<FrameStack, RuntimeError> {
        let mut fs = FrameStack::new(0);
        for frame in stks.stks() {
            let result = if frame.flags() & 0x10 == 0x00 {
                Some(StoreResult::new(0, frame.result_variable()))
            } else {
                None
            };
            let f = Frame::new(
                0,
                0,
                frame.local_variables(),
                frame.flags() & 0xF,
                frame.stack(),
                result,
                frame.return_address() as usize,
            );

            //debug!(target: "app::quetzal", "Frame: {}", f);
            fs.frames_mut().push(f);
        }

        Ok(fs)
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

    pub fn local_variable(&mut self, variable: u8) -> Result<u16, RuntimeError> {
        if variable < 16 {
            self.current_frame_mut()?.variable(variable)
        } else {
            Err(RuntimeError::new(ErrorCode::InvalidLocalVariable, format!("Variable {} is not a frame local variable", variable)))
        }
    }

    pub fn peek_local_variable(&mut self, variable: u8) -> Result<u16, RuntimeError> {
        if variable < 16 {
            self.current_frame()?.peek_variable(variable)
        } else {
            Err(RuntimeError::new(ErrorCode::InvalidLocalVariable, format!("Variable {} is not a frame local variable", variable)))
        }
    }

    pub fn set_local_variable(
        &mut self,
        variable: u8,
        value: u16,
    ) -> Result<(), RuntimeError> {
        if variable < 16 {
            self.current_frame_mut()?.set_variable(variable, value)
        } else {
            Err(RuntimeError::new(ErrorCode::InvalidLocalVariable, format!("Variable {} is not a frame local variable", variable)))
        }
    }

    pub fn set_local_variable_indirect(
        &mut self,
        variable: u8,
        value: u16,
    ) -> Result<(), RuntimeError> {
        if variable < 16 {
            self.current_frame_mut()?.set_variable_indirect(variable, value)
        } else {
            Err(RuntimeError::new(ErrorCode::InvalidLocalVariable, format!("Variable {} is not a frame local variable", variable)))
        }
    }

    pub fn pc(&self) -> Result<usize, RuntimeError> {
        Ok(self.current_frame()?.pc())
    }

    pub fn call_routine(
        &mut self,
        address: usize,
        initial_pc: usize,
        arguments: &Vec<u16>,
        local_variables: Vec<u16>,
        result: Option<StoreResult>,
        return_address: usize,
    ) -> Result<(), RuntimeError> {
        let frame = Frame::call_routine(
            address,
            initial_pc,
            arguments,
            local_variables,
            result,
            return_address,
        )?;
        info!(target: "app::frame", "Call to ${:06x} => {:?}", address, result);
        self.frames.push(frame);
        Ok(())
    }

    pub fn input_interrupt(
        &mut self,
        address: usize,
        initial_pc: usize,
        local_variables: Vec<u16>,
        return_address: usize,
    ) -> Result<(), RuntimeError> {
        let mut frame = Frame::call_routine(
            address,
            initial_pc,
            &vec![],
            local_variables,
            None,
            return_address,
        )?;
        frame.set_input_interrupt(true);
        info!(target: "app::frame", "Input interrupt ${:06x}", address);
        self.frames.push(frame);
        Ok(())
    }

    pub fn sound_interrupt(
        &mut self,
        address: usize,
        initial_pc: usize,
        local_variables: Vec<u16>,
        return_address: usize,
    ) -> Result<(), RuntimeError> {
        let frame = Frame::call_routine(
            address,
            initial_pc,
            &vec![],
            local_variables,
            None,
            return_address,
        )?;
        info!(target: "app::frame", "Sound interrupt ${:06x}", address);
        self.frames.push(frame);
        Ok(())
    }

    pub fn return_routine(
        &mut self,
    ) -> Result<Option<StoreResult>, RuntimeError> {
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
