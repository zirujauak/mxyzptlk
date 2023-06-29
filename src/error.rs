use std::fmt;

#[derive(Debug)]
pub enum ErrorCode {
    Blorb,
    FrameUnderflow,
    IFF,
    IllegalAccess,
    Instruction,
    InvalidAddress,
    InvalidColor,
    InvalidLocalVariable,
    InvalidWindow,
    ObjectTreeState,
    PropertySize,
    Restore,
    Save,
    StackUnderflow,
    System,
    UnimplementedInstruction,
    UnsupportedVersion,
}

pub struct RuntimeError {
    code: ErrorCode,
    message: String,
}

impl RuntimeError {
    pub fn new(code: ErrorCode, message: String) -> RuntimeError {
        RuntimeError { code, message }
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({:?}) {}", self.code, self.message)
    }
}

impl fmt::Debug for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "RuntimeError {{ code: {:?}, message: {} }}",
            self.code, self.message
        )
    }
}
