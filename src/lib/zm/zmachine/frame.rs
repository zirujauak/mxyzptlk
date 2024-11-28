//! ZMachine [stack frame](https://inform-fiction.org/zmachine/standards/z1point1/sect06.html#five)
use crate::instruction::StoreResult;
use crate::quetzal::{Stk, Stks};
use crate::{error::*, fatal_error};

#[derive(Debug)]
/// Stack frame
pub struct Frame {
    /// Address of the routine this frame is executing
    address: usize,
    /// The address of the executing or most-recently executed instruction
    pc: usize,
    /// The address of the next instruction to execute
    next_pc: usize,
    /// Local variable storage
    local_variables: Vec<u16>,
    /// Number of arguments to the routine
    argument_count: u8,
    /// Stack
    stack: Vec<u16>,
    /// [Option] with the [StoreResult] location for this frame or [None]
    result: Option<StoreResult>,
    /// The address to return to when this frame returns
    return_address: usize,
    /// Is this frame a READ interrupt routine?
    read_interrupt: bool,
    /// Is this frame a READ_CHAR interrupt routine?
    read_char_interrupt: bool,
    /// Does existing player input need to be redrawn?
    redraw_input: bool,
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
    /// Constructor
    ///
    /// # Arguments
    /// * `address` - address of the routine header this frame will execute
    /// * `pc` - address of the first instruction to execute
    /// * `local_variables` - local variable storage
    /// * `argument_count` - number of arguments passed to the routine
    /// * `stack` - stack
    /// * `result` - [Option] with [StoreResult] location or [None]
    /// * `return_address` - Address to resume execution at when frame returns
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
            next_pc: pc,
            local_variables: local_variables.to_vec(),
            argument_count,
            stack: stack.to_vec(),
            result,
            return_address,
            read_interrupt: false,
            read_char_interrupt: false,
            redraw_input: false,
        }
    }

    /// Get the address of the currently executing instruction
    ///
    /// # Returns
    /// Address of the currently executing instruction
    pub fn pc(&self) -> usize {
        self.pc
    }

    /// Set the address of the currently executing instruction
    ///
    /// # Arguments
    /// * `pc` - address of the currently executing instruction
    pub fn set_pc(&mut self, pc: usize) {
        self.pc = pc;
    }

    /// Get the address of the next instruction to execute
    ///
    /// # Returns
    /// Address of the next instruction to execute
    pub fn next_pc(&self) -> usize {
        self.next_pc
    }

    /// Set the address of the next insturction to execute
    ///
    /// # Arguments
    /// * `next_pc` - Address of the next instruction to execute
    pub fn set_next_pc(&mut self, next_pc: usize) {
        self.next_pc = next_pc;
    }

    /// Get a reference to local variable storage
    ///
    /// # Returns
    /// Reference to local variable storage
    pub fn local_variables(&self) -> &Vec<u16> {
        &self.local_variables
    }

    /// Get the number of arguments passed to the frame's routine
    ///
    /// # Returns
    /// The count of arguments passed to the frame's routine
    pub fn argument_count(&self) -> u8 {
        self.argument_count
    }

    /// Get a reference to the stack
    ///
    /// # Returns
    /// Reference to the stack
    pub fn stack(&self) -> &Vec<u16> {
        &self.stack
    }

    /// Is the current frame a READ interrupt routine?
    ///
    /// # Returns
    /// `true` if the frame is a READ interupt routine, `false` if not
    pub fn read_interrupt(&self) -> bool {
        self.read_interrupt
    }

    /// Mark this frame as a READ interrupt routine
    pub fn set_read_interrupt(&mut self) {
        self.read_interrupt = true;
    }

    /// Is the current frame a READ_CHAR interrupt routine?
    ///
    /// # Returns
    /// `true` if the frame is a READ_CHAR interupt routine, `false` if not
    pub fn read_char_interrupt(&self) -> bool {
        self.read_char_interrupt
    }

    /// mark this frame as a READ_CHAR interrupt routine
    pub fn set_read_char_interrupt(&mut self) {
        self.read_char_interrupt = true;
    }

    /// Should any input to READ be redrawn?
    ///
    /// # Returns
    /// `true` if input should be redrawn, `false` if not
    pub fn redraw_input(&self) -> bool {
        self.redraw_input
    }

    /// Mark that input should be redrawn
    pub fn set_redraw_input(&mut self) {
        self.redraw_input = true;
    }

    /// Pops the value from the top of the stack
    ///
    /// # Returns
    /// [Result] containing the value from the top of the stack or [RuntimeError]
    pub fn pop(&mut self) -> Result<u16, RuntimeError> {
        if let Some(v) = self.stack.pop() {
            debug!(target: "app::state", "Pop {:04x} [{}]", v, self.stack.len());
            Ok(v)
        } else {
            fatal_error!(ErrorCode::StackUnderflow, "Popped an empty stack")
        }
    }

    /// Peeks at the value on the top of the stack without removing it
    ///
    /// # Returns
    /// [Result] containing the value from the top of the stack or [RuntimeError]
    pub fn peek(&self) -> Result<u16, RuntimeError> {
        if let Some(v) = self.stack.last() {
            Ok(*v)
        } else {
            fatal_error!(ErrorCode::StackUnderflow, "Peeked an empty stack")
        }
    }

    /// Pushes a value onto the stack
    ///
    /// # Arguments
    /// * `value` - Value to push
    pub fn push(&mut self, value: u16) {
        self.stack.push(value);
        debug!(target: "app::state", "Push {:04x} [{}]", value, self.stack.len());
    }

    /// Gets the store location for the routine
    ///
    /// # Returns
    /// [Option] with a reference to the [StoreResult] or [None]
    pub fn result(&self) -> Option<&StoreResult> {
        self.result.as_ref()
    }

    /// Gets the return address for the routine
    ///
    /// # Returns
    /// Address to resume execution at when the routine returns
    pub fn return_address(&self) -> usize {
        self.return_address
    }

    /// Gets the value of a local variable.
    ///
    /// If local variable 0 is read, the value is popped from the stack.
    ///
    /// # Arguments
    /// * `variable` - Local variable number, which should be 0 (stack) or from 1 to the number of local variables
    ///
    /// # Returns
    /// [Result] with the local variable value or a [RuntimeError]
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

    /// Peeks at a local variable without removing any values from the stack.
    ///
    /// If local variable 0 is read, the value is peeked from the stack.
    ///
    /// # Arguments
    /// * `variable` - Local variable number, which should be 0 (stack) or from 1 to the number of local variables
    pub fn peek_local_variable(&self, variable: u8) -> Result<u16, RuntimeError> {
        if variable == 0 {
            self.peek()
        } else if variable <= self.local_variables.len() as u8 {
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

    /// Set a local variable
    ///
    /// If local variable 0 is set, the value is pushed onto the stack
    ///
    /// # Arguments
    /// * `variable` - Local variable number, which should be 0 (stack) or from 1 to the number of local variables
    /// * `value` - Value to set
    ///
    /// # Returns
    /// Empty [Result] or a [RuntimeError]
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

    /// Set a local variable indirectly.
    ///
    /// If local variable 0 is set, the value will replace the value currently at the top of the stack.
    ///
    /// # Arguments
    /// * `variable` - Local variable number, which should be 0 (stack) or from 1 to the number of local variables
    /// * `value` - Value to set
    ///
    /// # Returns
    /// Empty [Result] or a [RuntimeError]
    pub fn set_local_variable_indirect(
        &mut self,
        variable: u8,
        value: u16,
    ) -> Result<(), RuntimeError> {
        if variable == 0 {
            self.pop()?;
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

    /// Create a new frame for a routine call
    ///
    /// # Arguments
    /// * `address` - Address of the routine header
    /// * `initial_pc` - Address of the instruction to begin execution for the routine
    /// * `arguments` - Arguments to the routine call
    /// * `local_variables` - Local variable storage pre-loaded with any arguments or default local variable values
    /// * `result` - [Option] with [StoreResult] location or [None]
    /// * `return_address` - Address to resume execution when the routine returns
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
        assert_eq!(frame.address, 0);
        assert_eq!(frame.pc, 0);
        assert_eq!(frame.local_variables, &[0x5678, 0x9abc, 0xf0ad]);
        assert_eq!(frame.argument_count, 0x3);
        assert_eq!(frame.stack, &[0x1111, 0x2222, 0x3333, 0x4444]);
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
        assert_eq!(frame.address, 0);
        assert_eq!(frame.pc, 0);
        assert_eq!(frame.local_variables, &[0x5678, 0x9abc, 0xf0ad]);
        assert_eq!(frame.argument_count, 0x3);
        assert_eq!(frame.stack, &[0x1111, 0x2222, 0x3333, 0x4444]);
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
        assert_eq!(frames[0].address, 0);
        assert_eq!(frames[0].pc, 0);
        assert_eq!(frames[0].local_variables, &[0x5678, 0x9abc, 0xdef0]);
        assert_eq!(frames[0].argument_count, 0x1);
        assert_eq!(frames[0].stack, &[0x1111, 0x2222]);
        assert!(frames[0].result().is_none());
        assert_eq!(frames[0].return_address(), 0x1234);
        assert_eq!(frames[1].address, 0);
        assert_eq!(frames[1].pc, 0);
        assert_eq!(frames[1].local_variables, &[0x8765, 0xcba9]);
        assert_eq!(frames[1].argument_count, 0x2);
        assert!(frames[1].stack.is_empty());
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
        assert_eq!(frame.address, 0x1234);
        assert_eq!(frame.pc, 0x5678);
        assert_eq!(frame.local_variables, &[0x1122, 0x3344, 0x5566, 0x7788]);
        assert_eq!(frame.argument_count, 3);
        assert_eq!(frame.stack, &[0x1111, 0x2222]);
        assert!(frame.result().is_none());
        assert_eq!(frame.return_address(), 0x9876);
        assert!(!frame.read_interrupt);
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
        assert_eq!(frame.address, 0x1234);
        assert_eq!(frame.pc, 0x5678);
        assert_eq!(frame.local_variables, &[0x1122, 0x3344, 0x5566, 0x7788]);
        assert_eq!(frame.argument_count, 3);
        assert_eq!(frame.stack, &[0x1111, 0x2222]);
        assert_some_eq!(frame.result(), &StoreResult::new(0x4321, 0x80));
        assert_eq!(frame.return_address(), 0x9876);
        assert!(!frame.read_interrupt);
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
        assert_eq!(frame.stack.len(), 2);
        frame.push(0x3456);
        assert_eq!(frame.stack.len(), 3);
        frame.push(0x789a);
        assert_eq!(frame.stack.len(), 4);
        assert_ok_eq!(frame.pop(), 0x789a);
        assert_eq!(frame.stack.len(), 3);
        assert_ok_eq!(frame.pop(), 0x3456);
        assert_eq!(frame.stack.len(), 2);
        assert_ok_eq!(frame.peek(), 0x2222);
        assert_eq!(frame.stack.len(), 2);
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
        assert_eq!(frame.stack.len(), 2);
        assert_ok_eq!(frame.local_variable(0), 0x2222);
        assert_eq!(frame.stack.len(), 1);
        assert_ok_eq!(frame.local_variable(0), 0x1111);
        assert_eq!(frame.stack.len(), 0);
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
        assert_eq!(frame.stack.len(), 2);
        assert_ok_eq!(frame.peek_local_variable(0), 0x2222);
        assert_eq!(frame.stack.len(), 2);
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
        assert_eq!(frame.stack.len(), 2);
        assert!(frame.set_local_variable(0, 0x3333).is_ok());
        assert_eq!(frame.stack.len(), 3);
        assert_ok_eq!(frame.local_variable(0), 0x3333);
        assert_eq!(frame.stack.len(), 2);
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
        assert_eq!(frame.stack.len(), 2);
        assert!(frame.set_local_variable_indirect(0, 0x3333).is_ok());
        assert_eq!(frame.stack.len(), 2);
        assert_ok_eq!(frame.local_variable(0), 0x3333);
        assert_eq!(frame.stack.len(), 1);
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
        assert_eq!(frame.address, 0x1234);
        assert_eq!(frame.pc, 0x1235);
        assert_eq!(frame.local_variables, &[0x1122, 0x3344, 0x5544, 0x3322]);
        assert_eq!(frame.argument_count, 2);
        assert!(frame.result().is_none());
        assert_eq!(frame.return_address(), 0x4321);
        assert!(frame.stack.is_empty());
        assert!(!frame.read_interrupt);
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
        assert_eq!(frame.address, 0x1234);
        assert_eq!(frame.pc, 0x1235);
        assert_eq!(frame.local_variables, &[0x1122, 0x3344, 0x5544, 0x3322]);
        assert_eq!(frame.argument_count, 2);
        assert_some_eq!(frame.result(), &StoreResult::new(0x1001, 0x80));
        assert_eq!(frame.return_address(), 0x4321);
        assert!(frame.stack.is_empty());
        assert!(!frame.read_interrupt);
    }
}
