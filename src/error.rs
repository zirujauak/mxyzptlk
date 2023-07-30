use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    BlorbMissingChunk,
    BlorbLoopEntrySize,
    BlorbRIdxEntrySize,
    ConfigError,
    DivideByZero,
    FileError,
    FileExists,
    FrameUnderflow,
    IFFInvalidChunkId,
    IFhdChunkLength,
    IllegalMemoryAccess,
    Interpreter,
    InvalidAbbreviation,
    InvalidAddress,
    InvalidColor,
    InvalidFile,
    InvalidFilename,
    InvalidInput,
    InvalidInstruction,
    InvalidLocalVariable,
    InvalidObjectAttribute,
    InvalidObjectTree,
    InvalidObjectProperty,
    InvalidObjectPropertySize,
    InvalidOutputStream,
    InvalidShift,
    InvalidSoundEffect,
    InvalidWindow,
    NoFrame,
    NoReadInterrupt,
    NoSoundInterrupt,
    Quetzal,
    ReadNothing,
    ReadNoTerminator,
    Restore,
    ReturnNoCaller,
    Save,
    Stream3Table,
    SoundConversion,
    SoundPlayback,
    StackUnderflow,
    Transcript,
    UndoNoState,
    UnimplementedInstruction,
    UnsupportedVersion,
}

pub struct RuntimeError {
    recoverable: bool,
    code: ErrorCode,
    message: String,
}

impl RuntimeError {
    pub fn recoverable(code: ErrorCode, message: String) -> RuntimeError {
        RuntimeError {
            recoverable: true,
            code,
            message,
        }
    }

    pub fn fatal(code: ErrorCode, message: String) -> RuntimeError {
        RuntimeError {
            recoverable: false,
            code,
            message,
        }
    }

    pub fn code(&self) -> ErrorCode {
        self.code
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn is_recoverable(&self) -> bool {
        self.recoverable
    }
}

#[macro_export]
macro_rules! fatal_error {
    ($code:expr, $($arg:tt)*) => {
        Err(RuntimeError::fatal($code, format!($($arg)*)))
    };
}

#[macro_export]
macro_rules! recoverable_error {
    ($code:expr, $($arg:tt)*) => {
        Err(RuntimeError::recoverable($code, format!($($arg)*)))
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} error - [{:?}]: {}",
            if self.recoverable {
                "Recoverable"
            } else {
                "Fatal"
            },
            self.code,
            self.message
        )
    }
}

impl fmt::Debug for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} error - [{:?}]: {}",
            if self.recoverable {
                "Recoverable"
            } else {
                "Fatal"
            },
            self.code,
            self.message
        )
    }
}
