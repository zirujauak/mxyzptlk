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

#[derive(Debug, PartialEq)]
pub enum ErrorType {
    Recoverable,
    Fatal,
}
pub struct RuntimeError {
    error_type: ErrorType,
    code: ErrorCode,
    message: String,
}

impl RuntimeError {
    pub fn recoverable(code: ErrorCode, message: String) -> RuntimeError {
        RuntimeError {
            error_type: ErrorType::Recoverable,
            code,
            message,
        }
    }

    pub fn fatal(code: ErrorCode, message: String) -> RuntimeError {
        RuntimeError {
            error_type: ErrorType::Fatal,
            code,
            message,
        }
    }

    pub fn is_recoverable(&self) -> bool {
        self.error_type == ErrorType::Recoverable
    }

    pub fn is_fatal(&self) -> bool {
        self.error_type == ErrorType::Fatal
    }
}

#[macro_export]
macro_rules! fatal_error {
    ($code:expr, $($arg:tt)*) => {
        Err(RuntimeError::fatal($code, format!($($arg)*)))
    };
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:?} [{:?}] {}",
            self.error_type, self.code, self.message
        )
    }
}

impl fmt::Debug for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:?} [{:?}] {}",
            self.error_type, self.code, self.message
        )
    }
}
