use std::fmt;

use crate::{error::RuntimeError, zmachine::InterpreterRequest};

pub mod decoder;
pub mod processor;

#[derive(Debug, Eq, PartialEq)]
/// [Opcode forms](https://inform-fiction.org/zmachine/standards/z1point1/sect04.html#three)
pub enum OpcodeForm {
    Short,
    Long,
    Var,
    Ext,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// [Operand types](https://inform-fiction.org/zmachine/standards/z1point1/sect04.html#two)
pub enum OperandType {
    LargeConstant,
    SmallConstant,
    Variable,
}

#[derive(Debug, Eq, PartialEq)]
/// [Operands](https://inform-fiction.org/zmachine/standards/z1point1/sect04.html#five)
pub struct Operand {
    /// The [OperandType]
    operand_type: OperandType,
    /// Operand value
    value: u16,
}

impl fmt::Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.operand_type {
            OperandType::LargeConstant => write!(f, "#{:04x}", self.value),
            OperandType::SmallConstant => write!(f, "#{:02x}", self.value as u8),
            OperandType::Variable => {
                if self.value == 0 {
                    write!(f, "(SP)+")
                } else if self.value < 16 {
                    write!(f, "L{:02x}", self.value - 1)
                } else {
                    write!(f, "G{:02x}", self.value - 16)
                }
            }
        }
    }
}

impl Operand {
    /// Constructor
    ///
    /// # Arguments
    /// * `operand_type` - [OperandType]
    /// * `value` - Operand value
    pub fn new(operand_type: OperandType, value: u16) -> Operand {
        Operand {
            operand_type,
            value,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
/// Branching information
pub struct Branch {
    /// Address of the (first) branch descriptor byte
    address: usize,
    /// Branch-on condition
    condition: bool,
    /// Address of the branch destination
    branch_address: usize,
}

impl fmt::Display for Branch {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[{}] ", self.condition)?;
        match self.branch_address {
            0 => write!(f, "RFALSE"),
            1 => write!(f, "RTRUE"),
            _ => write!(f, "${:05x}", self.branch_address),
        }
    }
}

impl Branch {
    /// Constructor
    ///
    /// # Arguments
    /// * `address` - address of the (first) branch descriptor byte
    /// * `condition` - branch-on condition
    /// * `branch_address` - branch destination address
    pub fn new(address: usize, condition: bool, branch_address: usize) -> Branch {
        Branch {
            address,
            condition,
            branch_address,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Where the result of an instruction will be stored
pub struct StoreResult {
    /// Address of the store result descriptor byte
    address: usize,
    /// Variable to store to
    variable: u8,
}

impl fmt::Display for StoreResult {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.variable == 0 {
            write!(f, "-(SP)")
        } else if self.variable < 16 {
            write!(f, "L{:02x}", self.variable - 1)
        } else {
            write!(f, "G{:02x}", self.variable - 16)
        }
    }
}

impl StoreResult {
    /// Constructor
    ///
    /// # Arguments
    /// * `address` - Address of the store descriptor byte
    /// * `variable` - Variable to store to
    pub fn new(address: usize, variable: u8) -> StoreResult {
        StoreResult { address, variable }
    }

    pub fn variable(&self) -> u8 {
        self.variable
    }
}

#[derive(Debug, Eq, PartialEq)]
/// [Operand count](https://inform-fiction.org/zmachine/standards/z1point1/sect04.html#five)
pub enum OperandCount {
    _0OP,
    _1OP,
    _2OP,
    _VAR,
}

#[derive(Debug)]
/// Opcode
pub struct Opcode {
    version: u8,
    opcode: u8,
    form: OpcodeForm,
    instruction: u8,
    operand_count: OperandCount,
}

impl fmt::Display for Opcode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self.form {
                OpcodeForm::Ext => match self.instruction {
                    0x00 => "SAVE",
                    0x01 => "RESTORE",
                    0x02 => "LOG_SHIFT",
                    0x03 => "ART_SHIFT",
                    0x04 => "SET_FONT",
                    0x05 => "DRAW_PICTURE",
                    0x06 => "PICTURE_DATA",
                    0x07 => "ERASE_PICTURE",
                    0x08 => "SET_MARGINS",
                    0x09 => "SAVE_UNDO",
                    0x0A => "RESTORE_UNDO",
                    0x0B => "PRINT_UNICODE",
                    0x0C => "CHECK_UNICODE",
                    0x0D => "SET_TRUE_COLOUR",
                    0x10 => "MOVE_WINDOW",
                    0x11 => "WINDOW_SIZE",
                    0x12 => "WINDOW_STYLE",
                    0x13 => "GET_WIND_PROP",
                    0x14 => "SCROLL_WINDOW",
                    0x15 => "POP_STACK",
                    0x16 => "READ_MOUSE",
                    0x17 => "MOUSE_WINDOW",
                    0x18 => "PUSH_STACK",
                    0x19 => "PUT_WIND_PROP",
                    0x1A => "PRINT_FORM",
                    0x1B => "MAKE_MENU",
                    0x1C => "PICTURE_TABLE",
                    0x1D => "BUFFER_SCREEN",
                    _ => "UNKNOWN!",
                },
                _ => match self.operand_count {
                    OperandCount::_0OP => match self.instruction {
                        0x0 => "RTRUE",
                        0x1 => "RFALSE",
                        0x2 => "PRINT",
                        0x3 => "PRINT_RET",
                        0x4 => "NOP",
                        0x5 => "SAVE",
                        0x6 => "RESTORE",
                        0x7 => "RESTART",
                        0x8 => "RET_POPPED",
                        0x9 => {
                            if self.version < 5 {
                                "POP"
                            } else {
                                "CATCH"
                            }
                        }
                        0xA => "QUIT",
                        0xB => "NEW_LINE",
                        0xC => "SHOW_STATUS",
                        0xD => "VERIFY",
                        0xF => "PIRACY",
                        _ => "UNKNOWN!",
                    },
                    OperandCount::_1OP => match self.instruction {
                        0x0 => "JZ",
                        0x1 => "GET_SIBLING",
                        0x2 => "GET_CHILD",
                        0x3 => "GET_PARENT",
                        0x4 => "GET_PROP_LEN",
                        0x5 => "INC",
                        0x6 => "DEC",
                        0x7 => "PRINT_ADDR",
                        0x8 => "CALL_1S",
                        0x9 => "REMOVE_OBJ",
                        0xA => "PRINT_OBJ",
                        0xB => "RET",
                        0xC => "JUMP",
                        0xD => "PRINT_PADDR",
                        0xE => "LOAD",
                        0xF => {
                            if self.version < 5 {
                                "NOT"
                            } else {
                                "CALL_1N"
                            }
                        }
                        _ => "UNKNOWN!",
                    },
                    OperandCount::_2OP => match self.instruction {
                        0x01 => "JE",
                        0x02 => "JL",
                        0x03 => "JG",
                        0x04 => "DEC_CHK",
                        0x05 => "INC_CHK",
                        0x06 => "JIN",
                        0x07 => "TEST",
                        0x08 => "OR",
                        0x09 => "AND",
                        0x0A => "TEST_ATTR",
                        0x0B => "SET_ATTR",
                        0x0C => "CLEAR_ATTR",
                        0x0D => "STORE",
                        0x0E => "INSERT_OBJ",
                        0x0F => "LOADW",
                        0x10 => "LOADB",
                        0x11 => "GET_PROP",
                        0x12 => "GET_PROP_ADDR",
                        0x13 => "GET_NEXT_PROP",
                        0x14 => "ADD",
                        0x15 => "SUB",
                        0x16 => "MUL",
                        0x17 => "DIV",
                        0x18 => "MOD",
                        0x19 => "CALL_2S",
                        0x1A => "CALL_2N",
                        0x1B => "SET_COLOUR",
                        0x1C => "THROW",
                        _ => "UNKNOWN!",
                    },
                    OperandCount::_VAR => match self.instruction {
                        0x00 => {
                            if self.version < 4 {
                                "CALL"
                            } else {
                                "CALL_VS"
                            }
                        }
                        0x01 => "STOREW",
                        0x02 => "STOREB",
                        0x03 => "PUT_PROP",
                        0x04 => {
                            if self.version < 5 {
                                "SREAD"
                            } else {
                                "AREAD"
                            }
                        }
                        0x05 => "PRINT_CHAR",
                        0x06 => "PRINT_NUM",
                        0x07 => "RANDOM",
                        0x08 => "PUSH",
                        0x09 => "PULL",
                        0x0A => "SPLIT_WINDOW",
                        0x0B => "SET_WINDOW",
                        0x0C => "CALL_VS2",
                        0x0D => "ERASE_WINDOW",
                        0x0E => "ERASE_LINE",
                        0x0F => "SET_CURSOR",
                        0x10 => "GET_CURSOR",
                        0x11 => "SET_TEXT_STYLE",
                        0x12 => "BUFFER_MODE",
                        0x13 => "OUTPUT_STREAM",
                        0x14 => "INPUT_STREAM",
                        0x15 => "SOUND_EFFECT",
                        0x16 => "READ_CHAR",
                        0x17 => "SCAN_TABLE",
                        0x18 => "NOT",
                        0x19 => "CALL_VN",
                        0x1A => "CALL_VN2",
                        0x1B => "TOKENISE",
                        0x1C => "ENCODE_TEXT",
                        0x1D => "COPY_TABLE",
                        0x1E => "PRINT_TABLE",
                        0x1F => "CHECK_ARG_COUNT",
                        _ => "UNKNOWN!",
                    },
                },
            }
        )
    }
}

impl Opcode {
    pub fn new(
        version: u8,
        opcode: u8,
        instruction: u8,
        form: OpcodeForm,
        operand_count: OperandCount,
    ) -> Opcode {
        Opcode {
            version,
            opcode,
            instruction,
            form,
            operand_count,
        }
    }
}

#[derive(Debug)]
/// [Instruction](https://inform-fiction.org/zmachine/standards/z1point1/sect04.html#one)
pub struct Instruction {
    /// Vector of bytes that (may) belong to the instruction
    bytes: Vec<u8>,
    /// Address of the instruction in memory
    address: usize,
    /// Instruction [Opcode]
    opcode: Opcode,
    /// Vector of [Operand] values
    operands: Vec<Operand>,
    /// [Option] containing the [StoreResult] if the instruction stores a result
    store: Option<StoreResult>,
    /// [Option] containing the [Branch] information if the instruction branches
    branch: Option<Branch>,
    /// Address of the instruction immediately following this one in memory
    next_address: usize,
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "${:05x}: ", self.address)?;
        for b in &self.bytes {
            write!(f, "{:02x} ", b)?;
        }

        write!(f, " {}", self.opcode)?;

        for o in &self.operands {
            write!(f, " {}", o)?;
        }

        if let Some(s) = self.store {
            write!(f, " -> {}", s)?
        }

        if let Some(b) = &self.branch {
            write!(f, " {}", b)?
        }

        write!(f, "")
    }
}

impl Instruction {
    pub fn new(
        bytes: &[u8],
        address: usize,
        opcode: Opcode,
        operands: Vec<Operand>,
        store: Option<StoreResult>,
        branch: Option<Branch>,
        next_address: usize,
    ) -> Instruction {
        Instruction {
            bytes: bytes.to_vec(),
            address,
            opcode,
            operands,
            store,
            branch,
            next_address,
        }
    }

    pub fn store(&self) -> Option<&StoreResult> {
        self.store.as_ref()
    }

    fn branch(&self) -> Option<&Branch> {
        self.branch.as_ref()
    }

    pub fn next_address(&self) -> usize {
        self.next_address
    }
}

#[derive(Debug)]
/// Address of the next instruction to execute
pub enum NextAddress {
    /// Simple address
    Address(usize),
    /// Returning from a READ_CHAR interrupt routine
    ReadCharInterrupt(usize, u16),
    /// Returning from a READ interrupt routine
    ReadInterrupt(usize, u16, bool),
    /// QUITting
    Quit,
}

#[derive(Debug)]
/// Instruction result
pub struct InstructionResult {
    /// [NextAddress] to execute
    next_address: NextAddress,
    /// [Option] with any [InterpreterRequest] necessary to complete the instruction execution
    interpreter_request: Option<InterpreterRequest>,
}

impl InstructionResult {
    /// Constructor for a simple result with no intepreter request
    ///
    /// # Arguments
    /// * `next_address` - [NextAddress] to execute
    pub fn new(next_address: NextAddress) -> Result<InstructionResult, RuntimeError> {
        Ok(InstructionResult {
            next_address,
            interpreter_request: None,
        })
    }

    /// The [NextAddress] to execute
    pub fn next_address(&self) -> &NextAddress {
        &self.next_address
    }

    /// [Option] with any [InterpreterRequest] required by the instruction
    pub fn interpreter_request(&self) -> Option<&InterpreterRequest> {
        self.interpreter_request.as_ref()
    }

    /// Constructor for a request to have the interpreter display a message and continue
    ///
    /// # Arguments:
    /// * `next_address` - [NextAddress] to execute
    /// * `message` - message string
    pub fn message(
        next_address: NextAddress,
        message: &str,
    ) -> Result<InstructionResult, RuntimeError> {
        Ok(InstructionResult {
            next_address,
            interpreter_request: InterpreterRequest::message(message),
        })
    }

    /// Constructor for a request to update the buffer mode
    ///
    /// # Arguments:
    /// * `next_address` - [NextAddress] to execute
    /// * `mode` - buffer mode
    pub fn buffer_mode(
        next_address: NextAddress,
        mode: u16,
    ) -> Result<InstructionResult, RuntimeError> {
        Ok(InstructionResult {
            next_address,
            interpreter_request: InterpreterRequest::buffer_mode(mode),
        })
    }

    /// Constructor for a request to erase from the cursor position to the end of the line.
    ///
    /// # Arguments:
    /// * `next_address` - [NextAddress] to execute
    pub fn erase_line(next_address: NextAddress) -> Result<InstructionResult, RuntimeError> {
        Ok(InstructionResult {
            next_address,
            interpreter_request: InterpreterRequest::erase_line(),
        })
    }

    /// Constructor for a request to erase a window
    ///
    /// # Arguments:
    /// * `next_address` - [NextAddress] to execute
    /// * `window` - window to erase
    pub fn erase_window(
        next_address: NextAddress,
        window: i16,
    ) -> Result<InstructionResult, RuntimeError> {
        Ok(InstructionResult {
            next_address,
            interpreter_request: InterpreterRequest::erase_window(window),
        })
    }

    /// Constructor for a request to get the current cursor position
    ///
    /// # Arguments:
    /// * `next_address` - [NextAddress] to execute
    pub fn get_cursor(next_address: NextAddress) -> Result<InstructionResult, RuntimeError> {
        Ok(InstructionResult {
            next_address,
            interpreter_request: InterpreterRequest::get_cursor(),
        })
    }

    /// Constructor for a request to enable or disable an input stream
    ///
    /// # Arguments:
    /// * `next_address` - [NextAddress] to execute
    /// * `stream` - input stream
    pub fn input_stream(
        next_address: NextAddress,
        stream: i16,
    ) -> Result<InstructionResult, RuntimeError> {
        Ok(InstructionResult {
            next_address,
            interpreter_request: InterpreterRequest::input_stream(stream),
        })
    }

    /// Constructor for a request to print a new line
    ///
    /// # Arguments:
    /// * `next_address` - [NextAddress] to execute
    pub fn new_line(next_address: NextAddress) -> Result<InstructionResult, RuntimeError> {
        Ok(InstructionResult {
            next_address,
            interpreter_request: InterpreterRequest::new_line(),
        })
    }

    /// Constructor for a request to enable or disable an output stream
    ///
    /// # Arguments:
    /// * `next_address` - [NextAddress] to execute
    /// * `stream` - output stream
    pub fn output_stream(
        next_address: NextAddress,
        stream: i16,
    ) -> Result<InstructionResult, RuntimeError> {
        Ok(InstructionResult {
            next_address,
            interpreter_request: InterpreterRequest::output_stream(stream),
        })
    }

    /// Constructor for a request to print text
    ///
    /// # Arguments:
    /// * `next_address` - [NextAddress] to execute
    /// * `text` - text to print
    /// * `transcript` - if true, text should also be recorded to the transcript
    pub fn print(
        next_address: NextAddress,
        text: Vec<u16>,
        transcript: bool,
    ) -> Result<InstructionResult, RuntimeError> {
        Ok(InstructionResult {
            next_address,
            interpreter_request: InterpreterRequest::print(text, transcript),
        })
    }

    /// Constructor for a request to print text, followed by a new line, and then
    /// return true from the current routine
    ///
    /// # Arguments:
    /// * `next_address` - [NextAddress] to execute
    /// * `text` - text to print
    /// * `transcript` - if true, text should also be recorded to the transcript
    pub fn print_ret(
        next_address: NextAddress,
        text: Vec<u16>,
        transcript: bool,
    ) -> Result<InstructionResult, RuntimeError> {
        Ok(InstructionResult {
            next_address,
            interpreter_request: InterpreterRequest::print_ret(text, transcript),
        })
    }

    /// Constructor for a request to print a table
    ///
    /// # Arguments:
    /// * `next_address` - [NextAddress] to execute
    /// * `table` - table data
    /// * `width` - row width
    /// * `height` - table height
    /// * `skip` - number of bytes to skip between lines
    /// * `transcript` - if true, text should also be recorded to the transcript
    pub fn print_table(
        next_address: NextAddress,
        table: Vec<u16>,
        width: u16,
        height: u16,
        skip: u16,
        transcript: bool,
    ) -> Result<InstructionResult, RuntimeError> {
        Ok(InstructionResult {
            next_address,
            interpreter_request: InterpreterRequest::print_table(
                table, width, height, skip, transcript,
            ),
        })
    }

    /// Constructor for a request to quit
    ///
    /// # Arguments:
    /// * `next_address` - [NextAddress] to execute
    pub fn quit() -> Result<InstructionResult, RuntimeError> {
        Ok(InstructionResult {
            next_address: NextAddress::Quit,
            interpreter_request: None,
        })
    }

    /// Constructor for a request to read input
    ///
    /// # Arguments:
    /// * `next_address` - [NextAddress] to execute
    /// * `length` - maximum number of characters, including terminator
    /// * `terminators` - vector of input terminators
    /// * `timeout` - read timeout
    /// * `input` - existing input
    /// * `redraw` - if true, existing input should be printed to the screen
    pub fn read(
        next_address: NextAddress,
        length: u8,
        terminators: Vec<u16>,
        timeout: u16,
        input: Vec<u16>,
    ) -> Result<InstructionResult, RuntimeError> {
        Ok(InstructionResult {
            next_address,
            interpreter_request: InterpreterRequest::read(length, terminators, timeout, input),
        })
    }

    /// Constructor for a request to read a single character
    ///
    /// # Arguments:
    /// * `next_address` - [NextAddress] to execute
    /// * `timeout` - read timeout
    pub fn read_char(
        next_address: NextAddress,
        timeout: u16,
    ) -> Result<InstructionResult, RuntimeError> {
        Ok(InstructionResult {
            next_address,
            interpreter_request: InterpreterRequest::read_char(timeout),
        })
    }

    /// Constructor for a request to restart
    ///
    /// # Arguments:
    /// * `next_address` - [NextAddress] to execute
    pub fn restart(next_address: NextAddress) -> Result<InstructionResult, RuntimeError> {
        Ok(InstructionResult {
            next_address,
            interpreter_request: InterpreterRequest::restart(),
        })
    }

    /// Constructor for a request to restore from a saved game
    ///
    /// # Arguments:
    /// * `next_address` - [NextAddress] to execute
    /// * `name` - zcode base filename
    pub fn restore(
        next_address: NextAddress,
        name: &str,
    ) -> Result<InstructionResult, RuntimeError> {
        Ok(InstructionResult {
            next_address,
            interpreter_request: InterpreterRequest::restore(name),
        })
    }

    /// Constructor for a request to save the game
    ///
    /// # Arguments:
    /// * `next_address` - [NextAddress] to execute
    /// * `name` - zcode base filename
    /// * `data` - Byte vector of save date
    pub fn save(
        next_address: NextAddress,
        name: &str,
        data: Vec<u8>,
    ) -> Result<InstructionResult, RuntimeError> {
        Ok(InstructionResult {
            next_address,
            interpreter_request: InterpreterRequest::save(name, data),
        })
    }

    /// Constructor for a request to set test colours
    ///
    /// # Arguments:
    /// * `next_address` - [NextAddress] to execute
    /// * `foreground` - foreground colour
    /// * `background` - background colour
    pub fn set_colour(
        next_address: NextAddress,
        foreground: u16,
        background: u16,
    ) -> Result<InstructionResult, RuntimeError> {
        Ok(InstructionResult {
            next_address,
            interpreter_request: InterpreterRequest::set_colour(foreground, background),
        })
    }

    /// Constructor for a request to set the cursor
    ///
    /// # Arguments:
    /// * `next_address` - [NextAddress] to execute
    /// * `row` - cursor row
    /// * `column` - cursor column    
    pub fn set_cursor(
        next_address: NextAddress,
        row: u16,
        column: u16,
    ) -> Result<InstructionResult, RuntimeError> {
        Ok(InstructionResult {
            next_address,
            interpreter_request: InterpreterRequest::set_cursor(row, column),
        })
    }

    /// Constructor for a request to set the font
    ///
    /// # Arguments:
    /// * `next_address` - [NextAddress] to execute
    /// * `font` - font number
    pub fn set_font(
        next_address: NextAddress,
        font: u16,
    ) -> Result<InstructionResult, RuntimeError> {
        Ok(InstructionResult {
            next_address,
            interpreter_request: InterpreterRequest::set_font(font),
        })
    }

    /// Constructor for a request to set the text style
    ///
    /// # Arguments:
    /// * `next_address` - [NextAddress] to execute
    /// * `style` - text style(s)
    pub fn set_text_style(
        next_address: NextAddress,
        style: u16,
    ) -> Result<InstructionResult, RuntimeError> {
        Ok(InstructionResult {
            next_address,
            interpreter_request: InterpreterRequest::set_text_style(style),
        })
    }

    /// Constructor for a request to set the active window
    ///
    /// # Arguments:
    /// * `next_address` - [NextAddress] to execute
    /// * `window` - window to activate
    pub fn set_window(
        next_address: NextAddress,
        window: u16,
    ) -> Result<InstructionResult, RuntimeError> {
        Ok(InstructionResult {
            next_address,
            interpreter_request: InterpreterRequest::set_window(window),
        })
    }

    /// Constructor for a request to draw the status line
    ///
    /// # Arguments:
    /// * `next_address` - [NextAddress] to execute
    /// * `left` - text for the left side of the status line
    /// * `right` - text for the right side of the status line
    pub fn show_status(
        next_address: NextAddress,
        left: Vec<u16>,
        right: Vec<u16>,
    ) -> Result<InstructionResult, RuntimeError> {
        Ok(InstructionResult {
            next_address,
            interpreter_request: InterpreterRequest::show_status(left, right),
        })
    }

    /// Constructor for a request to play or stop a sound effect
    ///
    /// # Arguments:
    /// * `next_address` - [NextAddress] to execute
    /// * `number` - sound effect operation
    /// * `effect` - sound effect
    /// * `volume` - playback volume
    /// * `repeats` - number of times to play the effect
    /// * `routine` - address of a routine to execute when the sound finished
    pub fn sound_effect(
        next_address: NextAddress,
        number: u16,
        effect: u16,
        volume: u8,
        repeats: u8,
        routine: usize,
    ) -> Result<InstructionResult, RuntimeError> {
        Ok(InstructionResult {
            next_address,
            interpreter_request: InterpreterRequest::sound_effect(
                number, effect, volume, repeats, routine,
            ),
        })
    }

    /// Constructor for a request to split or unsplit the screen
    ///
    /// # Arguments:
    /// * `next_address` - [NextAddress] to execute
    /// * `lines` - lines to split
    pub fn split_window(
        next_address: NextAddress,
        lines: u16,
    ) -> Result<InstructionResult, RuntimeError> {
        Ok(InstructionResult {
            next_address,
            interpreter_request: InterpreterRequest::split_window(lines),
        })
    }
}
