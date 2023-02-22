use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::interpreter::Interpreter;

use super::object;
use super::state::State;
use super::text;

#[derive(Debug)]
pub enum OperandType {
    LargeConstant,
    SmallConstant,
    Variable,
}

#[derive(Debug)]
pub struct Operand {
    operand_type: OperandType,
    operand_value: u16,
}

pub struct Branch {
    condition: bool,
    address: usize,
}

pub struct Instruction {
    address: usize,
    opcode: Opcode,
    operands: Vec<Operand>,
    store: Option<u8>,
    branch: Option<Branch>,
    next_address: usize,
}

#[derive(Debug)]
pub enum OpcodeForm {
    Short,
    Long,
    Variable,
    Extended,
}

#[derive(Debug)]
pub enum OperandCount {
    _0OP,
    _1OP,
    _2OP,
    _VAR,
}
pub struct Opcode {
    opcode: u8,
    form: OpcodeForm,
    instruction: u8,
    opcount: OperandCount,
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "${:05x}: ${:02x}", self.address, self.opcode.opcode)?;
        for o in &self.operands {
            match o.operand_type {
                OperandType::SmallConstant => write!(f, " #{:02x}", o.operand_value as u8)?,
                OperandType::LargeConstant => write!(f, " #{:04x}", o.operand_value)?,
                OperandType::Variable => {
                    if o.operand_value == 0 {
                        write!(f, " (SP)+")?
                    } else if o.operand_value < 16 {
                        write!(f, " L{:02x}", o.operand_value - 1)?
                    } else {
                        write!(f, " G{:02x}", o.operand_value - 16)?
                    }
                }
            }
        }

        match self.store {
            Some(s) => {
                write!(f, " -> ")?;
                if s == 0 {
                    write!(f, "-(SP)")?
                } else if s < 16 {
                    write!(f, "L{:02x}", s - 1)?
                } else {
                    write!(f, "G{:02x}", s - 16)?
                }
            }
            None => {}
        }

        match &self.branch {
            Some(b) => write!(
                f,
                " [{}] ${:05x}",
                b.condition.to_string().to_uppercase(),
                b.address
            )?,
            None => {}
        }

        write!(f, "")
    }
}

impl Instruction {
    fn operand_type(b: u8, n: u8) -> Option<OperandType> {
        match (b >> (6 - (n * 2))) & 3 {
            0 => Some(OperandType::LargeConstant),
            1 => Some(OperandType::SmallConstant),
            2 => Some(OperandType::Variable),
            _ => None,
        }
    }

    fn operands(state: &State, mut address: usize, opcode: &Opcode) -> (usize, Vec<Operand>) {
        let mut operand_types = Vec::new();
        let mut operands = Vec::new();
        match opcode.form {
            OpcodeForm::Short => match (opcode.opcode >> 4) & 3 {
                0 => operand_types.push(OperandType::LargeConstant),
                1 => operand_types.push(OperandType::SmallConstant),
                2 => operand_types.push(OperandType::Variable),
                _ => {}
            },
            OpcodeForm::Long => {
                if opcode.opcode & 0x40 == 0 {
                    operand_types.push(OperandType::SmallConstant)
                } else {
                    operand_types.push(OperandType::Variable)
                }
                if opcode.opcode & 0x20 == 0 {
                    operand_types.push(OperandType::SmallConstant)
                } else {
                    operand_types.push(OperandType::Variable)
                }
            }
            OpcodeForm::Variable => {
                if opcode.opcode == 0xEC || opcode.opcode == 0xFA {
                    let b1 = state.byte_value(address);
                    address = address + 1;
                    for i in 0..4 {
                        match Self::operand_type(b1, i) {
                            Some(t) => operand_types.push(t),
                            None => break,
                        }
                    }

                    let b2 = state.byte_value(address);
                    address = address + 1;
                    for i in 0..4 {
                        match Self::operand_type(b2, i) {
                            Some(t) => operand_types.push(t),
                            None => break,
                        }
                    }
                } else {
                    let b = state.byte_value(address);
                    address = address + 1;
                    for i in 0..4 {
                        match Self::operand_type(b, i) {
                            Some(t) => operand_types.push(t),
                            None => break,
                        }
                    }
                }
            }
            OpcodeForm::Extended => {
                let b = state.byte_value(address);
                address = address + 1;
                for i in 0..4 {
                    match Self::operand_type(b, i) {
                        Some(t) => operand_types.push(t),
                        None => break,
                    }
                }
            }
        }

        for t in operand_types {
            match t {
                OperandType::LargeConstant => {
                    let v = state.word_value(address);
                    address = address + 2;
                    operands.push(Operand {
                        operand_type: t,
                        operand_value: v,
                    })
                }
                OperandType::SmallConstant | OperandType::Variable => {
                    let v = state.byte_value(address);
                    address = address + 1;
                    operands.push(Operand {
                        operand_type: t,
                        operand_value: v as u16,
                    })
                }
            }
        }

        (address, operands)
    }

    fn opcode(state: &State, mut address: usize) -> (usize, Opcode) {
        let mut opcode = state.byte_value(address);
        let extended = opcode == 0xBE;
        address = address + 1;
        if opcode == 0xBE {
            opcode = state.byte_value(address);
            address = address + 1;
        }

        let form = if extended {
            OpcodeForm::Extended
        } else {
            match (opcode >> 6) & 0x3 {
                3 => OpcodeForm::Variable,
                2 => OpcodeForm::Short,
                _ => OpcodeForm::Long,
            }
        };

        let instruction = match form {
            OpcodeForm::Variable | OpcodeForm::Long => opcode & 0x1F,
            OpcodeForm::Short => opcode & 0x0F,
            OpcodeForm::Extended => opcode,
        };

        let opcount = match form {
            OpcodeForm::Short => {
                if opcode & 0x30 == 0x30 {
                    OperandCount::_0OP
                } else {
                    OperandCount::_1OP
                }
            }
            OpcodeForm::Long => OperandCount::_2OP,
            OpcodeForm::Variable => {
                if opcode & 0x20 == 0x20 {
                    OperandCount::_VAR
                } else {
                    OperandCount::_2OP
                }
            }
            OpcodeForm::Extended => OperandCount::_VAR,
        };

        (
            address,
            Opcode {
                opcode,
                instruction,
                form,
                opcount,
            },
        )
    }

    fn decode_store(state: &State, address: usize, opcode: &Opcode) -> (usize, Option<u8>) {
        match opcode.form {
            OpcodeForm::Extended => match opcode.instruction {
                0x00 | 0x01 | 0x02 | 0x03 | 0x04 | 0x09 | 0x0A | 0x0C | 0x13 | 0x1D => {
                    (address + 1, Some(state.byte_value(address)))
                }
                _ => (address, None),
            },
            _ => match opcode.opcount {
                OperandCount::_0OP => match state.version {
                    4 => match opcode.instruction {
                        0x05 | 0x06 => (address + 1, Some(state.byte_value(address))),
                        _ => (address, None),
                    },
                    _ => (address, None),
                },
                OperandCount::_1OP => match opcode.instruction {
                    0x01 | 0x02 | 0x03 | 0x04 | 0x0E => {
                        (address + 1, Some(state.byte_value(address)))
                    }
                    0x08 => {
                        if state.version >= 4 {
                            (address + 1, Some(state.byte_value(address)))
                        } else {
                            (address, None)
                        }
                    }
                    0x0F => {
                        if state.version <= 4 {
                            (address + 1, Some(state.byte_value(address)))
                        } else {
                            (address, None)
                        }
                    }
                    _ => (address, None),
                },
                OperandCount::_2OP => match opcode.instruction {
                    0x08 | 0x09 | 0x0F | 0x10 | 0x11 | 0x12 | 0x13 | 0x14 | 0x15 | 0x16 | 0x17
                    | 0x18 | 0x19 => (address + 1, Some(state.byte_value(address))),
                    _ => (address, None),
                },
                OperandCount::_VAR => match opcode.instruction {
                    0x00 | 0x07 | 0x0C | 0x16 | 0x17 | 0x18 => {
                        (address + 1, Some(state.byte_value(address)))
                    }
                    0x04 => {
                        if state.version >= 5 {
                            (address + 1, Some(state.byte_value(address)))
                        } else {
                            (address, None)
                        }
                    }
                    0x09 => {
                        if state.version == 6 {
                            (address + 1, Some(state.byte_value(address)))
                        } else {
                            (address, None)
                        }
                    }
                    _ => (address, None),
                },
            },
        }
    }

    fn branch_address(address: usize, offset: i16) -> usize {
        match offset {
            0 => 0,
            1 => 1,
            _ => ((address as isize) + (offset as i16) as isize) as usize,
        }
    }

    fn decode_branch(state: &State, address: usize) -> (usize, Option<Branch>) {
        let b = state.byte_value(address);
        let condition = b & 0x80 == 0x80;
        match b & 0x40 {
            0x40 => {
                let offset = b & 0x3f;
                (
                    address + 1,
                    Some(Branch {
                        condition,
                        address: Self::branch_address(address + 1 - 2, offset as i16),
                    }),
                )
            }
            _ => {
                let mut offset =
                    ((b as u16 & 0x3f) << 8) | (state.byte_value(address + 1) as u16) & 0xFF;
                if offset & 0x2000 == 0x2000 {
                    offset = offset | 0xC000;
                }
                (
                    address + 2,
                    Some(Branch {
                        condition,
                        address: Self::branch_address(address + 2 - 2, offset as i16),
                    }),
                )
            }
        }
    }

    fn branch(state: &State, address: usize, opcode: &Opcode) -> (usize, Option<Branch>) {
        match opcode.form {
            OpcodeForm::Extended => match opcode.instruction {
                0x06 | 0x18 | 0x1b => Self::decode_branch(state, address),
                _ => (address, None),
            },
            _ => match opcode.opcount {
                OperandCount::_0OP => match opcode.instruction {
                    0x0d | 0x0f => Self::decode_branch(state, address),
                    0x05 | 0x06 => {
                        if state.version < 4 {
                            Self::decode_branch(state, address)
                        } else {
                            (address, None)
                        }
                    }
                    _ => (address, None),
                },
                OperandCount::_1OP => match opcode.instruction {
                    0x00 | 0x01 | 0x02 => Self::decode_branch(state, address),
                    _ => (address, None),
                },
                OperandCount::_2OP => match opcode.instruction {
                    0x01 | 0x02 | 0x03 | 0x04 | 0x05 | 0x06 | 0x07 | 0x0a => {
                        Self::decode_branch(state, address)
                    }
                    _ => (address, None),
                },
                OperandCount::_VAR => (address, None),
            },
        }
    }

    pub fn from_address(state: &State, address: usize) -> Instruction {
        let (next_address, opcode) = Self::opcode(state, address);
        let (next_address, operands) = Self::operands(state, next_address, &opcode);
        let (next_address, store) = Self::decode_store(state, next_address, &opcode);
        let (next_address, branch) = Self::branch(state, next_address, &opcode);
        Instruction {
            address,
            opcode,
            operands,
            store,
            branch,
            next_address,
        }
    }

    pub fn execute(&mut self, state: &mut State) -> usize {
        match self.opcode.opcount {
            OperandCount::_0OP => match self.opcode.instruction {
                0x0 => self.rtrue(state),
                0x1 => self.rfalse(state),
                0x2 => self.print_literal(state),
                0xB => self.new_line(state),
                _ => 0,
            },
            OperandCount::_1OP => match self.opcode.instruction {
                0x1 => self.get_sibling(state),
                0x2 => self.get_child(state),
                0x3 => self.get_parent(state),
                0x5 => self.inc(state),
                0x6 => self.dec(state),
                0x8 => self.call_1s(state),
                0xA => self.print_obj(state),
                0xC => self.jump(state),
                0xD => self.print_paddr(state),
                0xF => self.call_1n(state),
                _ => 0,
            },
            OperandCount::_2OP => match self.opcode.instruction {
                0x01 => self.je(state),
                0x02 => self.jl(state),
                0x08 => self.or(state),
                0x09 => self.and(state),
                0x0A => self.test_attr(state),
                0x0B => self.set_attr(state),
                0x0C => self.clear_attr(state),
                0x0D => self.store(state),
                0x0E => self.insert_obj(state),
                0x0F => self.loadw(state),
                0x10 => self.loadb(state),
                0x11 => self.get_prop(state),
                0x14 => self.add(state),
                0x15 => self.sub(state),
                0x17 => self.div(state),
                0x18 => self.modulus(state),
                _ => 0,
            },
            OperandCount::_VAR => match self.opcode.instruction {
                0x00 => self.call(state),
                0x01 => self.storew(state),
                0x02 => self.storeb(state),
                0x03 => self.put_prop(state),
                0x04 => self.read(state),
                0x05 => self.print_char(state),
                0x06 => self.print_num(state),
                0x07 => self.random(state),
                0x0A => self.split_window(state),
                0x0B => self.set_window(state),
                0x0D => self.erase_window(state),
                0x0F => self.set_cursor(state),
                0x11 => self.set_text_style(state),
                0x12 => self.buffer_mode(state),
                0x13 => self.output_stream(state),
                0x16 => self.read_char(state),
                _ => 0,
            },
        }
    }

    fn operand_value(&self, state: &mut State, operand: &Operand) -> u16 {
        match operand.operand_type {
            OperandType::SmallConstant | OperandType::LargeConstant => operand.operand_value,
            OperandType::Variable => state.variable(operand.operand_value as u8),
        }
    }

    fn operand_values(&self, state: &mut State) -> Vec<u16> {
        let mut v = Vec::new();
        for o in &self.operands {
            v.push(self.operand_value(state, &o))
        }

        v
    }

    fn execute_branch(&self, condition: bool) -> usize {
        if condition == self.branch.as_ref().unwrap().condition {
            self.branch.as_ref().unwrap().address
        } else {
            self.next_address
        }
    }

    fn format_variable(&self, var: u8) -> String {
        if var == 0 {
            "(SP+)".to_string()
        } else if var < 16 {
            format!("L{:02x}", var - 1)
        } else {
            format!("G{:02x}", var - 16)
        }
    }

    fn format_operand(&self, state: &State, index: usize) -> String {
        match self.operands[index].operand_type {
            OperandType::SmallConstant => {
                format!("#{:02x}", self.operands[index].operand_value)
            }
            OperandType::LargeConstant => {
                format!("#{:04x}", self.operands[index].operand_value)
            }
            OperandType::Variable => {
                let mut var_string = self.format_variable(self.operands[index].operand_value as u8);
                var_string.push_str(&format!(
                    " [{:04x}]",
                    state.peek_variable(self.operands[index].operand_value as u8)
                ));
                var_string
            }
        }
    }

    fn format_operands(&self, state: &State) -> String {
        let mut f = String::new();
        for i in 0..self.operands.len() {
            f.push_str(&self.format_operand(state, i));
            if i < self.operands.len() - 1 {
                f.push(',')
            }
        }

        f
    }

    fn format_branch(&self) -> String {
        match &self.branch {
            Some(b) => {
                format!(
                    " [{}] => ${:05x}",
                    b.condition.to_string().to_uppercase(),
                    b.address
                )
            }
            None => String::new(),
        }
    }

    fn format_store(&self) -> String {
        match &self.store {
            Some(s) => format!(" -> {}", self.format_variable(*s)),
            None => String::new(),
        }
    }

    fn name(&self, state: &State) -> &str {
        match self.opcode.form {
            OpcodeForm::Extended => match self.opcode.instruction {
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
            _ => match self.opcode.opcount {
                OperandCount::_0OP => match self.opcode.instruction {
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
                        if state.version < 5 {
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
                OperandCount::_1OP => match self.opcode.instruction {
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
                        if state.version < 5 {
                            "NOT"
                        } else {
                            "CALL_1N"
                        }
                    }
                    _ => "UNKNOWN!",
                },
                OperandCount::_2OP => match self.opcode.instruction {
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
                OperandCount::_VAR => match self.opcode.instruction {
                    0x00 => {
                        if state.version < 4 {
                            "CALL"
                        } else {
                            "CALL_VS"
                        }
                    }
                    0x01 => "STOREW",
                    0x02 => "STOREB",
                    0x03 => "PUT_PROP",
                    0x04 => {
                        if state.version < 5 {
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
    }

    pub fn trace_instruction(&self, state: &State) {
        trace!(
            "${:05x}: {} {}{}{}",
            self.address,
            self.name(state),
            self.format_operands(state),
            self.format_branch(),
            self.format_store()
        );
    }

    // 0OP
    fn rtrue(&self, state: &mut State) -> usize {
        state.return_fn(1 as u16)
    }

    fn rfalse(&self, state: &mut State) -> usize {
        state.return_fn(0 as u16)
    }

    fn print_literal(&self, state: &mut State) -> usize {
        let mut ztext = Vec::new();

        let mut word = 0;
        while word & 0x8000 == 0 {
            word = state.word_value(self.next_address + (ztext.len() * 2));
            ztext.push(word);
        }

        let text = text::from_vec(state, &ztext);
        state.print(text);

        self.next_address + ztext.len() * 2
    }

    fn new_line(&self, state: &mut State) -> usize {
        state.interpreter.new_line();
        self.next_address
    }

    // 1OP
    fn get_sibling(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);
        let sibling = object::sibling(state, operands[0] as usize) as u16;
        let condition = sibling != 0;

        state.set_variable(self.store.unwrap(), sibling);
        self.execute_branch(condition)
    }

    fn get_child(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);
        let child = object::child(state, operands[0] as usize) as u16;
        let condition = child != 0;

        state.set_variable(self.store.unwrap(), child);
        self.execute_branch(condition)
    }

    fn get_parent(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);
        let parent = object::parent(state, operands[0] as usize) as u16;

        state.set_variable(self.store.unwrap(), parent);
        self.next_address
    }

    fn inc(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);
        let val = state.variable(operands[0] as u8);
        let new_val = val as i16 + 1;

        state.set_variable(operands[0] as u8, new_val as u16);
        self.next_address
    }

    fn dec(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);
        let val = state.variable(operands[0] as u8);
        let new_val = val as i16 - 1;

        state.set_variable(operands[0] as u8, new_val as u16);
        self.next_address
    }

    fn call_1s(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);
        let address = state.packed_routine_address(operands[0]);

        state.call(address, self.next_address, &Vec::new(), self.store)
    }

    fn print_obj(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);
        let ztext = object::short_name(state, operands[0] as usize);

        let text = text::from_vec(state, &ztext);
        state.print(text);
        self.next_address
    }

    fn jump(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        let address = (self.next_address as isize) + (operands[0] as i16) as isize - 2;
        address as usize
    }

    fn print_paddr(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);
        let address = state.packed_string_address(operands[0]);

        let text = text::as_text(state, address);
        state.print(text);
        self.next_address
    }

    fn call_1n(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);
        let address = state.packed_routine_address(operands[0]);

        state.call(address, self.next_address, &Vec::new(), None)
    }

    // 2OP
    fn je(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        let mut condition = false;
        for i in 1..operands.len() {
            condition = condition || (operands[0] as i16 == operands[i] as i16);
        }

        self.execute_branch(condition)
    }

    fn jl(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        let mut condition = false;
        for i in 1..operands.len() {
            condition = condition || (operands[0] as i16 == operands[i] as i16);
        }

        self.execute_branch(condition)
    }

    fn or(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        let mut v = operands[0];
        for i in 1..operands.len() {
            v = v | operands[i]
        }

        state.set_variable(self.store.unwrap(), v);
        self.next_address
    }

    fn and(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        let mut v = operands[0];
        for i in 1..operands.len() {
            v = v & operands[i];
        }

        state.set_variable(self.store.unwrap(), v);
        self.next_address
    }

    fn loadw(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        let address = operands[0] as usize + (operands[1] as usize * 2);
        let value = state.word_value(address);

        state.set_variable(self.store.unwrap(), value);
        self.next_address
    }

    fn loadb(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        let address = operands[0] as usize + operands[1] as usize;
        let value = state.byte_value(address) as u16;

        state.set_variable(self.store.unwrap(), value);
        self.next_address
    }

    fn store(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        state.set_variable(operands[0] as u8, operands[1]);
        self.next_address
    }

    fn insert_obj(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        self.next_address
    }

    fn test_attr(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        let condition = object::attribute(state, operands[0] as usize, operands[1] as u8);

        self.execute_branch(condition)
    }

    fn set_attr(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        object::set_attribute(state, operands[0] as usize, operands[1] as u8);
        self.next_address
    }

    fn clear_attr(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        object::clear_attribute(state, operands[0] as usize, operands[1] as u8);
        self.next_address
    }

    fn get_prop(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        let value = object::property(state, operands[0] as usize, operands[1] as u8);
        state.set_variable(self.store.unwrap(), value);
        self.next_address
    }

    fn add(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        let mut value = operands[0] as i16;
        for i in 1..operands.len() {
            value = value + operands[i] as i16;
        }

        state.set_variable(self.store.unwrap(), value as u16);
        self.next_address
    }

    fn sub(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        let mut value = operands[0] as i16;
        for i in 1..operands.len() {
            value = value - operands[i] as i16;
        }

        state.set_variable(self.store.unwrap(), value as u16);
        self.next_address
    }

    fn div(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        let mut value = operands[0] as i16;
        for i in 1..operands.len() {
            value = value / operands[i] as i16;
        }

        state.set_variable(self.store.unwrap(), value as u16);
        self.next_address
    }

    fn modulus(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        let mut value = operands[0] as i16;
        for i in 1..operands.len() {
            value = value % operands[i] as i16;
        }

        state.set_variable(self.store.unwrap(), value as u16);
        self.next_address
    }

    // VAR
    fn call(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        let address = state.packed_routine_address(operands[0]);
        let mut arguments = Vec::new();
        for i in 1..operands.len() {
            arguments.push(operands[i]);
        }

        state.call(address, self.next_address, &arguments, self.store)
    }

    fn storew(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        state.set_word(operands[0] as usize + (operands[1] as usize * 2), operands[2]);
        self.next_address
    }

    fn storeb(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        state.set_byte(operands[0] as usize + operands[1] as usize, operands[2] as u8);
        self.next_address
    }

    fn put_prop(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        object::set_property(state, operands[0] as usize, operands[1] as u8, operands[2]);
        self.next_address
    }

    pub fn read(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        let text = operands[0] as usize;
        let parse = operands[1] as usize;

        let len = if state.version < 5 {
            state.byte_value(text) - 1
        } else {
            state.byte_value(text)
        };

        if self.operands.len() > 2 {
            let time = operands[2] as u16;
            let routine = operands[3] as u16;
            state.read(len, time);
        } else {
            state.read(len, 0);
        }

        panic!("read not implemented")
    }

    fn print_char(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        state.print(format!("{}", (operands[0] as u8) as char));
        self.next_address
    }

    fn print_num(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        state.print(format!("{}", operands[0]));
        self.next_address
    }

    fn random(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        let range = operands[0] as i16;
        let v = if range < 0 {
            state.seed(range as u64);
            0
        } else if range == 0 {
            state.seed(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("Error geting time")
                    .as_millis() as u64,
            );
            0
        } else {
            state.random(range as u16)
        };

        state.set_variable(self.store.unwrap(), v);
        self.next_address
    }

    fn split_window(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        state.split_window(operands[0]);
        self.next_address
    }

    fn set_window(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        state.set_window(operands[0]);
        self.next_address
    }

    fn erase_window(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        state.erase_window(operands[0] as i16);
        self.next_address
    }

    fn set_cursor(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        state.set_cursor(operands[0], operands[1]);
        self.next_address
    }

    fn set_text_style(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        state.set_text_style(operands[0]);
        self.next_address
    }

    fn buffer_mode(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        state.buffer_mode(operands[0] == 1);
        self.next_address
    }

    fn output_stream(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        let stream = operands[0] as i16;
        if stream != 3 {
            state.output_stream(stream, 0);
        } else {
            let table = operands[1];
            state.output_stream(stream, table as usize);
        }

        self.next_address
    }

    fn read_char(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        let x = operands[0];
        if x != 1 {
            panic!("READ_CHAR first argument ({}) must be 1", x);
        }
        if self.operands.len() > 1 {
            let time = operands[1];
            let routine = operands[2];
            let c = state.read_char(time) as u16;
            state.set_variable(self.store.unwrap(), c);
        } else {
            let c = state.read_char(0) as u16;
            state.set_variable(self.store.unwrap(), c);
        }

        self.next_address
    }
}
