use std::fmt;

pub mod decoder;
pub mod processor;

#[derive(Debug, Eq, PartialEq)]
pub enum OpcodeForm {
    Short,
    Long,
    Var,
    Ext,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperandType {
    LargeConstant,
    SmallConstant,
    Variable,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Operand {
    operand_type: OperandType,
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
    fn new(operand_type: OperandType, value: u16) -> Operand {
        Operand {
            operand_type,
            value,
        }
    }

    fn operand_type(&self) -> OperandType {
        self.operand_type
    }

    fn value(&self) -> u16 {
        self.value
    }
}

#[derive(Clone, Copy, Debug)]
pub struct StoreResult {
    address: usize,
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
    pub fn new(address: usize, variable: u8) -> StoreResult {
        StoreResult { address, variable }
    }

    pub fn address(&self) -> usize {
        self.address
    }

    pub fn variable(&self) -> u8 {
        self.variable
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Branch {
    address: usize,
    condition: bool,
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
    fn new(address: usize, condition: bool, branch_address: usize) -> Branch {
        Branch {
            address,
            condition,
            branch_address,
        }
    }

    fn condition(&self) -> bool {
        self.condition
    }

    fn branch_address(&self) -> usize {
        self.branch_address
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum OperandCount {
    _0OP,
    _1OP,
    _2OP,
    _VAR,
}

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
                OpcodeForm::Ext => match self.instruction() {
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
                _ => match self.operand_count() {
                    OperandCount::_0OP => match self.instruction() {
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
                            if self.version() < 5 {
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
                    OperandCount::_1OP => match self.instruction() {
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
                    OperandCount::_2OP => match self.instruction() {
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
                    OperandCount::_VAR => match self.instruction() {
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

    pub fn version(&self) -> u8 {
        self.version
    }

    pub fn opcode(&self) -> u8 {
        self.opcode
    }
    pub fn form(&self) -> &OpcodeForm {
        &self.form
    }

    pub fn instruction(&self) -> u8 {
        self.instruction
    }

    pub fn operand_count(&self) -> &OperandCount {
        &self.operand_count
    }
}

pub struct Instruction {
    address: usize,
    opcode: Opcode,
    operands: Vec<Operand>,
    store: Option<StoreResult>,
    branch: Option<Branch>,
    next_address: usize,
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "${:05x} ", self.address())?;
        match self.opcode().form() {
            OpcodeForm::Ext => write!(f, "be {:02x}", self.opcode().opcode())?,
            _ => write!(f, "{:02x}", self.opcode().opcode())?,
        }
        write!(f, " {}", self.opcode())?;

        for o in self.operands() {
            write!(f, " {}", o)?;
        }

        if let Some(s) = self.store() {
            write!(f, " -> {}", s)?
        }

        if let Some(b) = self.branch() {
            write!(f, " {}", b)?
        }

        write!(f, "")
    }
}

impl Instruction {
    fn new(
        address: usize,
        opcode: Opcode,
        operands: Vec<Operand>,
        store: Option<StoreResult>,
        branch: Option<Branch>,
        next_address: usize,
    ) -> Instruction {
        Instruction {
            address,
            opcode,
            operands,
            store,
            branch,
            next_address,
        }
    }

    pub fn address(&self) -> usize {
        self.address
    }

    fn opcode(&self) -> &Opcode {
        &self.opcode
    }

    fn operands(&self) -> &Vec<Operand> {
        &self.operands
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
