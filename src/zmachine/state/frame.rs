use crate::instruction::StoreResult;
use crate::quetzal::{Stk, Stks};
use crate::{error::*, fatal_error};

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

impl From<&Stk> for Frame {
    fn from(value: &Stk) -> Self {
        let result = if value.flags() & 0x10 == 0x00 {
            Some(StoreResult::new(0, value.result_variable()))
        } else {
            None
        };
        Frame::new(
            0,
            0,
            value.variables(),
            value.arguments(),
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
        local_variables: &[u16],
        argument_count: u8,
        stack: &[u16],
        result: Option<StoreResult>,
        return_address: usize,
    ) -> Frame {
        Frame {
            address,
            pc,
            local_variables: local_variables.to_vec(),
            argument_count,
            stack: stack.to_vec(),
            result,
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
            debug!(target: "app::state", "Pop {:04x} [{}]", v, self.stack.len());
            Ok(v)
        } else {
            fatal_error!(ErrorCode::StackUnderflow, "Popped an empty stack")
        }
    }

    pub fn peek(&self) -> Result<u16, RuntimeError> {
        if let Some(v) = self.stack.last() {
            Ok(*v)
        } else {
            fatal_error!(ErrorCode::StackUnderflow, "Peeked an empty stack")
        }
    }

    pub fn push(&mut self, value: u16) {
        self.stack.push(value);
        debug!(target: "app::state", "Push {:04x} [{}]", value, self.stack.len());
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
            fatal_error!(
                ErrorCode::InvalidLocalVariable,
                "Read from invalid local variable {} out of range: {}",
                variable,
                self.local_variables.len()
            )
        }
    }

    pub fn peek_local_variable(&self, variable: u8) -> Result<u16, RuntimeError> {
        if variable == 0 {
            self.peek()
        } else if variable <= self.local_variables().len() as u8 {
            Ok(self.local_variables[variable as usize - 1])
        } else {
            fatal_error!(
                ErrorCode::InvalidLocalVariable,
                "Peek from local variable {} out of range: {}",
                variable,
                self.local_variables.len()
            )
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
            fatal_error!(
                ErrorCode::InvalidLocalVariable,
                "Write to local variable {} out of range: {}",
                variable,
                self.local_variables.len()
            )
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
            fatal_error!(
                ErrorCode::InvalidLocalVariable,
                "Write to local variable {} out of range: {}",
                variable,
                self.local_variables.len()
            )
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
        let mut lv = local_variables;

        for i in 0..arguments.len() {
            if lv.len() > i {
                lv[i] = arguments[i]
            }
        }

        Ok(Frame::new(
            address,
            initial_pc,
            &lv,
            arguments.len() as u8,
            &Vec::new(),
            result,
            return_address,
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::{assert_ok, assert_ok_eq, assert_some_eq};

    use super::*;

    #[test]
    fn test_from_stackframe() {
        let sf = Stk::new(
            0x1234,
            0x0F,
            0x80,
            3,
            &[0x5678, 0x9abc, 0xf0ad],
            &[0x1111, 0x2222, 0x3333, 0x4444],
        );

        let frame = Frame::from(&sf);
        assert_eq!(frame.address(), 0);
        assert_eq!(frame.pc(), 0);
        assert_eq!(frame.local_variables(), &[0x5678, 0x9abc, 0xf0ad]);
        assert_eq!(frame.argument_count(), 0x3);
        assert_eq!(frame.stack(), &[0x1111, 0x2222, 0x3333, 0x4444]);
        assert_eq!(frame.result(), Some(&StoreResult::new(0, 0x80)));
        assert_eq!(frame.return_address(), 0x1234);
    }

    #[test]
    fn test_from_stackframe_no_result() {
        let sf = Stk::new(
            0x1234,
            0x1F,
            0x80,
            3,
            &[0x5678, 0x9abc, 0xf0ad],
            &[0x1111, 0x2222, 0x3333, 0x4444],
        );

        let frame = Frame::from(&sf);
        assert_eq!(frame.address(), 0);
        assert_eq!(frame.pc(), 0);
        assert_eq!(frame.local_variables(), &[0x5678, 0x9abc, 0xf0ad]);
        assert_eq!(frame.argument_count(), 0x3);
        assert_eq!(frame.stack(), &[0x1111, 0x2222, 0x3333, 0x4444]);
        assert!(frame.result().is_none());
        assert_eq!(frame.return_address(), 0x1234);
    }

    #[test]
    fn test_vec_from_stks() {
        let stks = Stks::new(vec![
            Stk::new(
                0x1234,
                0x13,
                0x80,
                1,
                &[0x5678, 0x9abc, 0xdef0],
                &[0x1111, 0x2222],
            ),
            Stk::new(0x4321, 0x02, 0x80, 2, &[0x8765, 0xcba9], &[]),
        ]);
        let frames: Vec<Frame> = Vec::from(&stks);
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].address(), 0);
        assert_eq!(frames[0].pc(), 0);
        assert_eq!(frames[0].local_variables(), &[0x5678, 0x9abc, 0xdef0]);
        assert_eq!(frames[0].argument_count(), 0x1);
        assert_eq!(frames[0].stack(), &[0x1111, 0x2222]);
        assert!(frames[0].result().is_none());
        assert_eq!(frames[0].return_address(), 0x1234);
        assert_eq!(frames[1].address(), 0);
        assert_eq!(frames[1].pc(), 0);
        assert_eq!(frames[1].local_variables(), &[0x8765, 0xcba9]);
        assert_eq!(frames[1].argument_count(), 0x2);
        assert!(frames[1].stack().is_empty());
        assert_some_eq!(frames[1].result(), &StoreResult::new(0, 0x80));
        assert_eq!(frames[1].return_address(), 0x4321);
    }

    #[test]
    fn test_constructor() {
        let frame = Frame::new(
            0x1234,
            0x5678,
            &[0x1122, 0x3344, 0x5566, 0x7788],
            3,
            &[0x1111, 0x2222],
            None,
            0x9876,
        );
        assert_eq!(frame.address(), 0x1234);
        assert_eq!(frame.pc(), 0x5678);
        assert_eq!(frame.local_variables(), &[0x1122, 0x3344, 0x5566, 0x7788]);
        assert_eq!(frame.argument_count(), 3);
        assert_eq!(frame.stack(), &[0x1111, 0x2222]);
        assert!(frame.result().is_none());
        assert_eq!(frame.return_address(), 0x9876);
        assert!(!frame.input_interrupt());
        assert!(!frame.sound_interrupt());
    }

    #[test]
    fn test_constructor_result() {
        let frame = Frame::new(
            0x1234,
            0x5678,
            &[0x1122, 0x3344, 0x5566, 0x7788],
            3,
            &[0x1111, 0x2222],
            Some(StoreResult::new(0x4321, 0x80)),
            0x9876,
        );
        assert_eq!(frame.address(), 0x1234);
        assert_eq!(frame.pc(), 0x5678);
        assert_eq!(frame.local_variables(), &[0x1122, 0x3344, 0x5566, 0x7788]);
        assert_eq!(frame.argument_count(), 3);
        assert_eq!(frame.stack(), &[0x1111, 0x2222]);
        assert_some_eq!(frame.result(), &StoreResult::new(0x4321, 0x80));
        assert_eq!(frame.return_address(), 0x9876);
        assert!(!frame.input_interrupt());
        assert!(!frame.sound_interrupt());
    }

    #[test]
    fn test_pop() {
        let mut frame = Frame::new(
            0x1234,
            0x5678,
            &[0x1122, 0x3344, 0x5566, 0x7788],
            3,
            &[0x1111, 0x2222],
            Some(StoreResult::new(0x4321, 0x80)),
            0x9876,
        );
        assert_ok_eq!(frame.pop(), 0x2222);
        assert_ok_eq!(frame.pop(), 0x1111);
        assert!(frame.pop().is_err());
    }

    #[test]
    fn test_peek() {
        let mut frame = Frame::new(
            0x1234,
            0x5678,
            &[0x1122, 0x3344, 0x5566, 0x7788],
            3,
            &[0x1111, 0x2222],
            Some(StoreResult::new(0x4321, 0x80)),
            0x9876,
        );
        assert_ok_eq!(frame.peek(), 0x2222);
        assert_ok_eq!(frame.pop(), 0x2222);
        assert_ok_eq!(frame.peek(), 0x1111);
        assert_ok_eq!(frame.pop(), 0x1111);
        assert!(frame.peek().is_err());
    }

    #[test]
    fn test_push() {
        let mut frame = Frame::new(
            0x1234,
            0x5678,
            &[0x1122, 0x3344, 0x5566, 0x7788],
            3,
            &[0x1111, 0x2222],
            Some(StoreResult::new(0x4321, 0x80)),
            0x9876,
        );
        assert_eq!(frame.stack().len(), 2);
        frame.push(0x3456);
        assert_eq!(frame.stack().len(), 3);
        frame.push(0x789a);
        assert_eq!(frame.stack().len(), 4);
        assert_ok_eq!(frame.pop(), 0x789a);
        assert_eq!(frame.stack().len(), 3);
        assert_ok_eq!(frame.pop(), 0x3456);
        assert_eq!(frame.stack().len(), 2);
        assert_ok_eq!(frame.peek(), 0x2222);
        assert_eq!(frame.stack().len(), 2);
    }

    #[test]
    fn test_local_variable() {
        let mut frame = Frame::new(
            0x1234,
            0x5678,
            &[0x1122, 0x3344, 0x5566, 0x7788],
            3,
            &[0x1111, 0x2222],
            Some(StoreResult::new(0x4321, 0x80)),
            0x9876,
        );
        assert_ok_eq!(frame.local_variable(1), 0x1122);
        assert_ok_eq!(frame.local_variable(2), 0x3344);
        assert_ok_eq!(frame.local_variable(3), 0x5566);
        assert_ok_eq!(frame.local_variable(4), 0x7788);
        assert!(frame.local_variable(5).is_err());
        assert_eq!(frame.stack().len(), 2);
        assert_ok_eq!(frame.local_variable(0), 0x2222);
        assert_eq!(frame.stack().len(), 1);
        assert_ok_eq!(frame.local_variable(0), 0x1111);
        assert_eq!(frame.stack().len(), 0);
        assert!(frame.local_variable(0).is_err());
    }

    #[test]
    fn test_peek_local_variable() {
        let frame = Frame::new(
            0x1234,
            0x5678,
            &[0x1122, 0x3344, 0x5566, 0x7788],
            3,
            &[0x1111, 0x2222],
            Some(StoreResult::new(0x4321, 0x80)),
            0x9876,
        );
        assert_ok_eq!(frame.peek_local_variable(1), 0x1122);
        assert_ok_eq!(frame.peek_local_variable(2), 0x3344);
        assert_ok_eq!(frame.peek_local_variable(3), 0x5566);
        assert_ok_eq!(frame.peek_local_variable(4), 0x7788);
        assert!(frame.peek_local_variable(5).is_err());
        assert_eq!(frame.stack().len(), 2);
        assert_ok_eq!(frame.peek_local_variable(0), 0x2222);
        assert_eq!(frame.stack().len(), 2);
        assert_ok_eq!(frame.peek_local_variable(0), 0x2222);
    }

    #[test]
    fn test_set_local_variable() {
        let mut frame = Frame::new(
            0x1234,
            0x5678,
            &[0x1122, 0x3344, 0x5566, 0x7788],
            3,
            &[0x1111, 0x2222],
            Some(StoreResult::new(0x4321, 0x80)),
            0x9876,
        );
        assert_ok_eq!(frame.local_variable(1), 0x1122);
        assert_ok_eq!(frame.local_variable(2), 0x3344);
        assert_ok_eq!(frame.local_variable(3), 0x5566);
        assert_ok_eq!(frame.local_variable(4), 0x7788);
        assert!(frame.set_local_variable(2, 0).is_ok());
        assert_ok_eq!(frame.local_variable(1), 0x1122);
        assert_ok_eq!(frame.local_variable(2), 0);
        assert_ok_eq!(frame.local_variable(3), 0x5566);
        assert_ok_eq!(frame.local_variable(4), 0x7788);
        assert!(frame.set_local_variable(5, 0).is_err());
        assert_eq!(frame.stack().len(), 2);
        assert!(frame.set_local_variable(0, 0x3333).is_ok());
        assert_eq!(frame.stack().len(), 3);
        assert_ok_eq!(frame.local_variable(0), 0x3333);
        assert_eq!(frame.stack().len(), 2);
    }

    #[test]
    fn test_set_local_variable_indirect() {
        let mut frame = Frame::new(
            0x1234,
            0x5678,
            &[0x1122, 0x3344, 0x5566, 0x7788],
            3,
            &[0x1111, 0x2222],
            Some(StoreResult::new(0x4321, 0x80)),
            0x9876,
        );
        assert_ok_eq!(frame.local_variable(1), 0x1122);
        assert_ok_eq!(frame.local_variable(2), 0x3344);
        assert_ok_eq!(frame.local_variable(3), 0x5566);
        assert_ok_eq!(frame.local_variable(4), 0x7788);
        assert!(frame.set_local_variable_indirect(2, 0).is_ok());
        assert_ok_eq!(frame.local_variable(1), 0x1122);
        assert_ok_eq!(frame.local_variable(2), 0);
        assert_ok_eq!(frame.local_variable(3), 0x5566);
        assert_ok_eq!(frame.local_variable(4), 0x7788);
        assert!(frame.set_local_variable_indirect(5, 0).is_err());
        assert_eq!(frame.stack().len(), 2);
        assert!(frame.set_local_variable_indirect(0, 0x3333).is_ok());
        assert_eq!(frame.stack().len(), 2);
        assert_ok_eq!(frame.local_variable(0), 0x3333);
        assert_eq!(frame.stack().len(), 1);
        assert_ok_eq!(frame.local_variable(0), 0x1111);
    }

    #[test]
    fn test_call_routine() {
        let frame = assert_ok!(Frame::call_routine(
            0x1234,
            0x1235,
            &vec![0x1122, 0x3344],
            vec![0x9988, 0x7766, 0x5544, 0x3322],
            None,
            0x4321,
        ));
        assert_eq!(frame.address(), 0x1234);
        assert_eq!(frame.pc(), 0x1235);
        assert_eq!(frame.local_variables(), &[0x1122, 0x3344, 0x5544, 0x3322]);
        assert_eq!(frame.argument_count(), 2);
        assert!(frame.result().is_none());
        assert_eq!(frame.return_address(), 0x4321);
        assert!(frame.stack().is_empty());
        assert!(!frame.input_interrupt());
        assert!(!frame.sound_interrupt());
    }

    #[test]
    fn test_call_routine_result() {
        let frame = assert_ok!(Frame::call_routine(
            0x1234,
            0x1235,
            &vec![0x1122, 0x3344],
            vec![0x9988, 0x7766, 0x5544, 0x3322],
            Some(StoreResult::new(0x1001, 0x80)),
            0x4321,
        ));
        assert_eq!(frame.address(), 0x1234);
        assert_eq!(frame.pc(), 0x1235);
        assert_eq!(frame.local_variables(), &[0x1122, 0x3344, 0x5544, 0x3322]);
        assert_eq!(frame.argument_count(), 2);
        assert_some_eq!(frame.result(), &StoreResult::new(0x1001, 0x80));
        assert_eq!(frame.return_address(), 0x4321);
        assert!(frame.stack().is_empty());
        assert!(!frame.input_interrupt());
        assert!(!frame.sound_interrupt());
    }
}
