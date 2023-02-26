use std::time::{SystemTime, UNIX_EPOCH};
use std::{fmt, process};

use crate::interpreter::Interpreter;

use super::state::State;
use super::text;
use super::{header, object};

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
    store_byte_address: usize,
    branch: Option<Branch>,
    branch_byte_address: usize,
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
                OperandCount::_VAR => match opcode.instruction {
                    0x17 | 0x1F => {
                        Self::decode_branch(state, address)
                    },
                    _ => (address, None),
                }
            },
        }
    }

    pub fn from_address(state: &State, address: usize) -> Instruction {
        let (next_address, opcode) = Self::opcode(state, address);
        let (store_byte_address, operands) = Self::operands(state, next_address, &opcode);
        let (branch_byte_address, store) = Self::decode_store(state, store_byte_address, &opcode);
        let (next_address, branch) = Self::branch(state, branch_byte_address, &opcode);
        Instruction {
            address,
            opcode,
            operands,
            store,
            store_byte_address,
            branch,
            branch_byte_address,
            next_address,
        }
    }

    pub fn execute(&mut self, state: &mut State) -> usize {
        match self.opcode.form {
            OpcodeForm::Extended => match self.opcode.instruction {
                0x02 => self.log_shift(state),
                0x03 => self.art_shift(state),
                0x09 => self.save_undo(state),
                _ => 0,
            },
            _ => match self.opcode.opcount {
                OperandCount::_0OP => match self.opcode.instruction {
                    0x0 => self.rtrue(state),
                    0x1 => self.rfalse(state),
                    0x2 => self.print_literal(state),
                    0x3 => self.print_ret(state),
                    0x5 => self.save(state),
                    0x6 => self.restore(state),
                    0x8 => self.ret_popped(state),
                    0xA => self.quit(state),
                    0xB => self.new_line(state),
                    0xC => self.show_status(state),
                    0xD => self.verify(state),
                    0xF => self.piracy(state),
                    _ => 0,
                },
                OperandCount::_1OP => match self.opcode.instruction {
                    0x0 => self.jz(state),
                    0x1 => self.get_sibling(state),
                    0x2 => self.get_child(state),
                    0x3 => self.get_parent(state),
                    0x4 => self.get_prop_len(state),
                    0x5 => self.inc(state),
                    0x6 => self.dec(state),
                    0x7 => self.print_addr(state),
                    0x8 => self.call_1s(state),
                    0x9 => self.remove_obj(state),
                    0xA => self.print_obj(state),
                    0xB => self.ret(state),
                    0xC => self.jump(state),
                    0xD => self.print_paddr(state),
                    0xE => self.load(state),
                    0xF => if state.version < 5 {
                        self.not(state)
                    } else {
                        self.call_1n(state)
                    },
                    _ => 0,
                },
                OperandCount::_2OP => match self.opcode.instruction {
                    0x01 => self.je(state),
                    0x02 => self.jl(state),
                    0x03 => self.jg(state),
                    0x04 => self.dec_chk(state),
                    0x05 => self.inc_chk(state),
                    0x06 => self.jin(state),
                    0x07 => self.test(state),
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
                    0x12 => self.get_prop_addr(state),
                    0x13 => self.get_next_prop(state),
                    0x14 => self.add(state),
                    0x15 => self.sub(state),
                    0x16 => self.mul(state),
                    0x17 => self.div(state),
                    0x18 => self.modulus(state),
                    0x19 => self.call_2s(state),
                    0x1A => self.call_2n(state),
                    0x1B => self.set_colour(state),
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
                    0x08 => self.push(state),
                    0x09 => self.pull(state),
                    0x0A => self.split_window(state),
                    0x0B => self.set_window(state),
                    0x0C => self.call_vs2(state),
                    0x0D => self.erase_window(state),
                    0x0F => self.set_cursor(state),
                    0x11 => self.set_text_style(state),
                    0x12 => self.buffer_mode(state),
                    0x13 => self.output_stream(state),
                    0x16 => self.read_char(state),
                    0x18 => self.not(state),
                    0x19 => self.call_vn(state),
                    0x1A => self.call_vn2(state),
                    0x1F => self.check_arg_count(state),
                    _ => 0,
                },
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

    fn execute_branch(&self, state: &mut State, condition: bool) -> usize {
        if condition == self.branch.as_ref().unwrap().condition {
            match self.branch.as_ref().unwrap().address {
                0 => state.return_fn(0),
                1 => state.return_fn(1),
                _ => self.branch.as_ref().unwrap().address,
            }
        } else {
            self.next_address
        }
    }

    fn store_result(&self, state: &mut State, value: u16) {
        match self.store {
            Some(v) => {
                state.set_variable(v, value);
            },
            None => {}
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
            Some(b) => match b.address {
                0 => format!(" [{}] => RFALSE", b.condition.to_string().to_uppercase()),
                1 => format!(" [{}] => RTRUE", b.condition.to_string().to_uppercase()),
                _ => format!(
                    " [{}] => ${:05x}",
                    b.condition.to_string().to_uppercase(),
                    b.address
                ),
            },
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
        match (&self.opcode.opcount, self.opcode.instruction) {
            (OperandCount::_0OP, 0x2) | (OperandCount::_0OP, 0x3) => trace!(
                "${:05x}: {} \"{}\"",
                self.address,
                self.name(state),
                text::as_text(state, self.address + 1)
            ),
            (OperandCount::_1OP, 0xD) => {
                let a = match self.operands[0].operand_type {
                    OperandType::SmallConstant | OperandType::LargeConstant => {
                        self.operands[0].operand_value
                    }
                    OperandType::Variable => {
                        state.peek_variable(self.operands[0].operand_value as u8)
                    }
                };
                trace!(
                    "${:05x}: {} {} \"{}\"",
                    self.address,
                    self.name(state),
                    self.format_operand(state, 0),
                    text::as_text(state, state.packed_string_address(a))
                )
            }
            _ => trace!(
                "${:05x}: {} {}{}{}",
                self.address,
                self.name(state),
                self.format_operands(state),
                self.format_branch(),
                self.format_store()
            ),
        }
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

    fn print_ret(&self, state: &mut State) -> usize {
        let mut ztext = Vec::new();

        let mut word = 0;
        while word & 0x8000 == 0 {
            word = state.word_value(self.next_address + (ztext.len() * 2));
            ztext.push(word);
        }

        let text = text::from_vec(state, &ztext);
        state.print(text);
        state.new_line();
        state.return_fn(1)
    }

    fn save(&self, state: &mut State) -> usize {
        if state.version < 4 {
            let data = state.prepare_save(self.branch_byte_address);
            state.save(&String::new(), &data);
            // TODO: branch condition should depend on whether save succeeded or not
            self.execute_branch(state, true)
        } else {
            self.next_address
        }
    }

    fn restore(&mut self, state: &mut State) -> usize {
        let instruction_address = state.prepare_restore();
        if state.version < 4 {
            let (_address, branch) = Self::decode_branch(state, instruction_address);
            self.branch = branch;
            self.execute_branch(state, true)
        } else {
            self.next_address
        }
    }
    fn ret_popped(&self, state: &mut State) -> usize {
        let v = state.variable(0);
        state.return_fn(v)
    }

    fn quit(&self, _state: &mut State) -> usize {
        pancurses::reset_shell_mode();
        pancurses::curs_set(1);
        process::exit(0);
    }

    fn new_line(&self, state: &mut State) -> usize {
        state.interpreter.new_line();
        self.next_address
    }

    fn show_status(&self, state: &mut State) -> usize {
        if state.version < 4 {
            let loc_obj = state.variable(16) as usize;
            let location = text::from_vec(state, &object::short_name(state, loc_obj));
            let stat_1 = state.variable(17) as i16;
            let stat_2 = state.variable(18);
            let status =
                if state.version == 3 && header::flag(state, header::Flag::StatusLineType) == 1 {
                    format!("{:02}:{:02}", stat_1, stat_2)
                } else {
                    format!("Score: {:>3}  Turn: {:>4}", stat_1, stat_2)
                };

            trace!("{} / {}", location, status);
            state.show_status(&location, &status);
        }
        self.next_address
    }

    fn verify(&self, state: &mut State) -> usize {
        let expected = header::checksum(state);
        let checksum = state.checksum();

        trace!("verify: {:#04x} -- {:#04x}", expected, checksum);
        self.execute_branch(state, expected == checksum)
    }

    fn piracy(&self, state: &mut State) -> usize {
        self.execute_branch(state, true)
    }
    
    // Utility fn
    fn call_fn(
        &self,
        state: &mut State,
        address: usize,
        return_addr: usize,
        arguments: &Vec<u16>,
        result: Option<u8>,
    ) -> usize {
        if address == 0 || address == 1 {
            match result {
                Some(v) => state.set_variable(v, address as u16),
                None => {}
            }
            return_addr
        } else {
            state.call(address, return_addr, arguments, result)
        }
    }

    // 1OP
    fn jz(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        self.execute_branch(state, operands[0] == 0)
    }

    fn get_sibling(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);
        let sibling = object::sibling(state, operands[0] as usize) as u16;
        let condition = sibling != 0;

        self.store_result(state, sibling);
        self.execute_branch(state, condition)
    }

    fn get_child(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);
        let child = object::child(state, operands[0] as usize) as u16;
        let condition = child != 0;

        self.store_result(state, child);
        self.execute_branch(state, condition)
    }

    fn get_parent(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);
        let parent = object::parent(state, operands[0] as usize) as u16;

        self.store_result(state, parent);
        self.next_address
    }

    fn get_prop_len(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        let len = object::property_length(state, operands[0] as usize);
        self.store_result(state, len as u16);
        self.next_address
    }

    fn inc(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);
        let val = state.peek_variable(operands[0] as u8) as i16;
        let new_val = i16::overflowing_add(val, 1);
    
        state.set_variable_indirect(operands[0] as u8, new_val.0 as u16);
        self.next_address
    }

    fn dec(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);
        let val = state.peek_variable(operands[0] as u8) as i16;
        let new_val = i16::overflowing_sub(val, 1);

        state.set_variable_indirect(operands[0] as u8, new_val.0 as u16);
        self.next_address
    }

    fn print_addr(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);
        let address = operands[0] as usize;

        let text = text::as_text(state, address);
        state.print(text);
        self.next_address
    }

    fn call_1s(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);
        let address = state.packed_routine_address(operands[0]);

        self.call_fn(state, address, self.next_address, &Vec::new(), self.store)
    }

    fn remove_obj(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        let object = operands[0] as usize;
        let parent = object::parent(state, object);

        if parent != 0 {
            let parent_child = object::child(state, parent);

            if parent_child == object {
                // object is direct child of parent
                // Set child of parent to the object's sibling
                object::set_child(state, parent, object::sibling(state, object));
            } else {
                // scan the parent child list for the sibling prior to the object
                let mut sibling = parent_child;
                while sibling != 0 && object::sibling(state, sibling) != object {
                    sibling = object::sibling(state, sibling);
                }

                if sibling == 0 {
                    panic!("Inconsistent object tree state!")
                }

                // Set the previous sibling's sibling to the object's sibling
                object::set_sibling(state, sibling, object::sibling(state, object));
            }
        }
        // Set parent and sibling of object to 0
        object::set_parent(state, object, 0);
        object::set_sibling(state, object, 0);

        self.next_address
    }

    fn print_obj(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);
        let ztext = object::short_name(state, operands[0] as usize);

        let text = text::from_vec(state, &ztext);
        state.print(text);
        self.next_address
    }

    fn ret(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        state.return_fn(operands[0])
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

    fn load(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        let value = state.peek_variable(operands[0] as u8);
        self.store_result(state, value);
        self.next_address
    }

    fn not(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);
        
        let value = !operands[0];
        self.store_result(state, value);
        self.next_address
    }

    fn call_1n(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);
        let address = state.packed_routine_address(operands[0]);

        self.call_fn(state, address, self.next_address, &Vec::new(), None)
    }

    // 2OP
    fn je(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        let mut condition = false;
        for i in 1..operands.len() {
            condition = condition || (operands[0] as i16 == operands[i] as i16);
        }

        self.execute_branch(state, condition)
    }

    fn jl(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        self.execute_branch(state, (operands[0] as i16) < (operands[1] as i16))
    }

    fn jg(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        self.execute_branch(state, (operands[0] as i16) > (operands[1] as i16))
    }

    fn dec_chk(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        let val = state.peek_variable(operands[0] as u8) as i16;
        let new_val = i16::overflowing_sub(val, 1);
        state.set_variable_indirect(operands[0] as u8, new_val.0 as u16);

        self.execute_branch(state, new_val.0 < operands[1] as i16)
    }

    fn inc_chk(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        let val = state.peek_variable(operands[0] as u8) as i16;
        let new_val = i16::overflowing_add(val, 1);
        state.set_variable_indirect(operands[0] as u8, new_val.0 as u16);

        self.execute_branch(state, new_val.0 > operands[1] as i16)
    }

    fn jin(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        self.execute_branch(
            state,
            object::parent(state, operands[0] as usize) == operands[1] as usize,
        )
    }

    fn test(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        self.execute_branch(state, operands[0] & operands[1] == operands[1])
    }
    fn or(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        let mut v = operands[0];
        for i in 1..operands.len() {
            v = v | operands[i]
        }

        self.store_result(state, v);
        self.next_address
    }

    fn and(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        let mut v = operands[0];
        for i in 1..operands.len() {
            v = v & operands[i];
        }

        self.store_result(state, v);
        self.next_address
    }

    fn loadw(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        let address = operands[0] as usize + (operands[1] as usize * 2);
        let value = state.word_value(address);

        self.store_result(state, value);
        self.next_address
    }

    fn loadb(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        let address = operands[0] as usize + operands[1] as usize;
        let value = state.byte_value(address) as u16;

        self.store_result(state, value);
        self.next_address
    }

    fn store(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        state.set_variable_indirect(operands[0] as u8, operands[1]);
        self.next_address
    }

    fn insert_obj(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        let object = operands[0] as usize;
        let new_parent = operands[1] as usize;
        trace!("Insert {} into {}", object, new_parent);

        // Step 1: remove object from its current parent
        let old_parent = object::parent(state, object);
        trace!("Old parent {}", old_parent);

        // If the old parent is not "nothing"
        if old_parent != 0 {
            let old_parent_child = object::child(state, old_parent);
            trace!("Old parent child {}", old_parent_child);

            // If the old_parent's child is this object
            if old_parent_child == object {
                trace!(
                    "Set {} child to {}",
                    old_parent,
                    object::sibling(state, object)
                );
                // Simply set the old parent's child to the object's sibling
                object::set_child(state, old_parent, object::sibling(state, object));
            } else {
                // Else need to traverse the child list until we find
                // the entry whose next sibiling is the object
                let mut sibling = old_parent_child;
                while sibling != 0 && object::sibling(state, sibling) != object {
                    sibling = object::sibling(state, sibling);
                }

                trace!("Object previous sibling {}", sibling);
                if sibling == 0 {
                    panic!("Inconsistent object tree state!")
                }

                trace!(
                    "Set previous sibling {} sibling to {}",
                    sibling,
                    object::sibling(state, object)
                );
                object::set_sibling(state, sibling, object::sibling(state, object));
            }
        }

        // Step 2: Set object's sibling to the new_parent's child
        trace!(
            "Set object {} sibling to {}",
            object,
            object::child(state, new_parent)
        );
        object::set_sibling(state, object, object::child(state, new_parent));

        // Step 3: Set new_parent's child to the object
        trace!("Set object {} child to {}", new_parent, object);
        object::set_child(state, new_parent, object);

        // Step 4: Set the object's parent to new_parent
        trace!("Set object {} parent to {}", object, new_parent);
        object::set_parent(state, object, new_parent);

        self.next_address
    }

    fn test_attr(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        let condition = object::attribute(state, operands[0] as usize, operands[1] as u8);

        self.execute_branch(state, condition)
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
        self.store_result(state, value);
        self.next_address
    }

    fn get_prop_addr(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        let value = object::property_data_addr(state, operands[0] as usize, operands[1] as u8);
        self.store_result(state, value as u16);
        self.next_address
    }

    fn get_next_prop(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        let prop = object::next_property(state, operands[0] as usize, operands[1] as u8);
        self.store_result(state, prop as u16);
        self.next_address
    }

    fn add(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        let mut value = operands[0] as i16;
        for i in 1..operands.len() {
            value = i16::overflowing_add(value, operands[i] as i16).0;
        }

        self.store_result(state, value as u16);
        self.next_address
    }

    fn sub(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        let mut value = operands[0] as i16;
        for i in 1..operands.len() {
            value = i16::overflowing_sub(value, operands[i] as i16).0
        }

        self.store_result(state, value as u16);
        self.next_address
    }

    fn mul(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        let mut value = operands[0] as i16;
        for i in 1..operands.len() {
            value = i16::overflowing_mul(value, operands[i] as i16).0;
        }

        self.store_result(state, value as u16);
        self.next_address
    }

    fn div(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        let mut value = operands[0] as i16;
        for i in 1..operands.len() {
            value = i16::overflowing_div(value, operands[i] as i16).0;
        }

        self.store_result(state, value as u16);
        self.next_address
    }

    fn modulus(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        let mut value = operands[0] as i16;
        for i in 1..operands.len() {
            value = i16::overflowing_rem(value, operands[i] as i16).0;
        }

        self.store_result(state, value as u16);
        self.next_address
    }

    pub fn call_2s(&self, state: &mut State) -> usize {
        let operands: Vec<u16> = self.operand_values(state);

        let address = state.packed_routine_address(operands[0]);
        let arguments = vec![operands[1]];

        self.call_fn(state, address, self.next_address, &arguments, self.store)
    }

    pub fn call_2n(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        let address = state.packed_routine_address(operands[0]);
        let arguments = vec![operands[1]];

        self.call_fn(state, address, self.next_address, &arguments, None)
    }

    pub fn set_colour(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        state.set_colour(operands[0], operands[1]);
        self.next_address
    }

    // VAR
    // aka call_vs
    fn call(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        let address = state.packed_routine_address(operands[0]);
        let mut arguments = Vec::new();
        for i in 1..operands.len() {
            arguments.push(operands[i]);
        }

        self.call_fn(state, address, self.next_address, &arguments, self.store)
    }

    fn storew(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        state.set_word(
            operands[0] as usize + (operands[1] as usize * 2),
            operands[2],
        );
        self.next_address
    }

    fn storeb(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        state.set_byte(
            operands[0] as usize + operands[1] as usize,
            operands[2] as u8,
        );
        self.next_address
    }

    fn put_prop(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        object::set_property(state, operands[0] as usize, operands[1] as u8, operands[2]);
        self.next_address
    }

    pub fn read(&self, state: &mut State) -> usize {
        if state.version < 4 {
            trace!("SHOW_STATUS before READ");
            self.show_status(state);
        }

        let operands = self.operand_values(state);

        let text = operands[0] as usize;
        let mut existing_input = Vec::new();

        // If text was printed in an interrupt, let the interpreter know
        let redraw = state.print_in_interrupt;

        if state.read_interrupt() {
            trace!("Return from interrupt: {}", state.read_interrupt_result());
            state.set_read_interrupt(false);
            if state.read_interrupt_result() == 1 {
                // Clear the text buffer
                let b = state.byte_value(text) as usize;
                for i in 0..b {
                    state.set_byte(text + i, 0);
                }
                // Return terminator 0
                self.store_result(state, 0);
                return self.next_address;
            } 
        }
        if state.version > 4 {
            // Read text buffer into existing input
            trace!("Recovering {} bytes from text buffer", state.byte_value(text + 1));
            let s = state.byte_value(text + 1) as usize;
            for i in 0..s {
                existing_input.push(state.byte_value(text + 2 + i) as char);
            }
        }

        let parse = if operands.len() > 1 { 
            operands[1] as usize 
        } else { 
            0
        };

        let len = if state.version < 5 {
            state.byte_value(text) - 1
        } else {
            state.byte_value(text)
        };

        trace!("Read up to {} characters", len);

        state.clear_read_input();
        let time = if self.operands.len() > 2 {
            operands[2] / 10
        } else {
            0
        };
        let routine = if self.operands.len() > 2 {
            operands[3] 
        } else {
            0
        };

        let (input, interrupt) = state.read(len, time, &existing_input, redraw);
        state.print_in_interrupt = false;
        if interrupt {
            trace!("READ interrupt: {} bytes in input buffer", input.len());
            state.set_read_interrupt(true);
            state.set_byte(text + 1, input.len() as u8);
            for i in 0..input.len() {
                state.set_byte(text + 2 + i, input[i] as u8);
            }
            return state.call_read_interrupt(routine, self.address);
        }

        trace!("Read <= \"{:?}\"", &input);

        // Store input to the text buffer
        let terminator = input.last().unwrap();
        if state.version < 5 {
            for i in 0..input.len()-1 {
                state.set_byte(text + 1 + i, input[i].to_ascii_lowercase() as u8);
            }
            state.set_byte(text + 1 + input.len(), 0);
        } else {
            state.set_byte(text + 1, input.len() as u8 - 1);
            for i in 0..input.len()-1 {
                state.set_byte(text + 2 + i, input[i].to_ascii_lowercase() as u8);
            }
        }

        // Lexical analysis
        if parse > 0 || state.version < 5 {
            let separators = text::separators(state);
            let mut word = Vec::new();
            let mut word_start: usize = 0;
            let mut word_count: usize = 0;
            let max_words = state.byte_value(parse) as usize;

            for i in 0..input.len() {
                if word_count > max_words {
                    break;
                }

                if separators.contains(&input[i]) {
                    if word.len() > 0 {
                        let entry = text::from_dictionary(state, &word);
                        state.set_word(parse + 2 + (4 * word_count), entry as u16);
                        state.set_byte(parse + 4 + (4 * word_count), word.len() as u8);
                        state.set_byte(parse + 5 + (4 * word_count), word_start as u8 + 1);
                        word_count = word_count + 1;
                        trace!("{:?} => ${:05x}", word, entry);
                    }
                    let entry = text::from_dictionary(state, &vec![input[i]]);
                    state.set_word(parse + 2 + (4 * word_count), entry as u16);
                    state.set_byte(parse + 4 + (4 * word_count), 1);
                    state.set_byte(parse + 5 + (4 * word_count), i as u8 + 1);
                    word_count = word_count + 1;
                    trace!("{} => ${:05x}", input[i], entry);

                    word.clear();
                    word_start = i + 1;
                } else if input[i] == ' ' {
                    if word.len() > 0 {
                        let entry = text::from_dictionary(state, &word);
                        state.set_word(parse + 2 + (4 * word_count), entry as u16);
                        state.set_byte(parse + 4 + (4 * word_count), word.len() as u8);
                        state.set_byte(parse + 5 + (4 * word_count), word_start as u8 + 1);
                        word_count = word_count + 1;
                        trace!("{:?} => ${:05x}", word, entry)
                    }
                    word.clear();
                    word_start = i + 1;
                } else {
                    word.push(input[i].to_ascii_lowercase())
                }
            }

            if word.len() > 0 && word_count < max_words {
                let entry = text::from_dictionary(state, &word);
                state.set_word(parse + 2 + (4 * word_count), entry as u16);
                state.set_byte(parse + 4 + (4 * word_count), word.len() as u8);
                state.set_byte(parse + 5 + (4 * word_count), word_start as u8 + 1);
                word_count = word_count + 1;
                trace!("{:?} => ${:05x}", word, entry)
            }

            state.set_byte(parse + 1, word_count as u8);
            trace!("Parsed {} words", word_count);
        }

        if state.version > 4 {
            self.store_result(state, *terminator as u16);
        }

        self.next_address
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
            if range.abs() < 1000 {
                trace!("RNG predictable 1..{}", range.abs());
                state.random_predictable = true;
                state.random_predictable_range = range.abs() as u16;
                state.random_predictable_next = 1;
            } else {
                trace!("Re-seeding RNG: {:#04x}", range);
                state.random_predictable = false;
                state.seed(range as u64 & 0xFFFF);
            }
            0
        } else if range == 0 {
            trace!("Re-seeding RNG with current time");
            state.random_predictable = false;
            state.seed(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("Error geting time")
                    .as_millis() as u64,
            );
            0
        } else {
            if state.random_predictable {
                let v = state.random_predictable_next;
                let next = v + 1;
                if next > state.random_predictable_range {
                    state.random_predictable_next = 1;
                } else {
                    state.random_predictable_next = v
                }
                v.max(range as u16)
            } else {
                state.random(range as u16)
            }
        };

        self.store_result(state, v);
        self.next_address
    }

    fn push(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        state.current_frame_mut().push(operands[0]);
        self.next_address
    }

    fn pull(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);
        let value = state.variable(0);

        if operands[0] == 0 {
            state.current_frame_mut().pop();
        }

        state.set_variable(operands[0] as u8, value);
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

    fn call_vs2(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        let address = state.packed_routine_address(operands[0]);
        let mut arguments = Vec::new();
        for i in 1..operands.len() {
            arguments.push(operands[i]);
        }

        self.call_fn(state, address, self.next_address, &arguments, self.store)
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

        // If state.read_char_interrupt == true
        //   If state.read_char_interrupt_result == true
        //      Set state.read_char_interrupt = false
        //      Set result variable = 0
        //      Return self.next_address
        //   Else
        //      Set state_read_char_interrupt = false
        //      Continue
        if state.read_char_interrupt() {
            state.set_read_char_interrupt(false);
            if state.read_char_interrupt_result() == 1 {
                self.store_result(state, 0);
                return self.next_address;
            }
        }

        if self.operands.len() == 3 && operands[1] > 0 && operands[2] > 0 {
            let time = operands[1];
            let routine = operands[2];
            let c = state.read_char(time / 10) as u16;
            if c == 0 {
                return state.call_read_char_interrupt(routine, self.address);
                // Set state.read_char_interrupt = true
                // Call routine, returning to address of this instruction
            }
            self.store_result(state, c);
        } else {
            let c = state.read_char(0) as u16;
            self.store_result(state, c);
        }

        self.next_address
    }

    fn call_vn(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        let address = state.packed_routine_address(operands[0]);
        let mut arguments = Vec::new();
        for i in 1..operands.len() {
            arguments.push(operands[i]);
        }

        self.call_fn(state, address, self.next_address, &arguments, None)
    }

    fn call_vn2(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        let address = state.packed_routine_address(operands[0]);
        let mut arguments = Vec::new();
        for i in 1..operands.len() {
            arguments.push(operands[i]);
        }

        self.call_fn(state, address, self.next_address, &arguments, None)
    }

    fn check_arg_count(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);
        
        self.execute_branch(state, state.current_frame().argument_count >= operands[0] as u8)
    }

    // EXT
    fn log_shift(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        let value = operands[0];
        let places = operands[1] as i16;

        let new_value = if places < 0 && places > -16 {
            u16::overflowing_shr(value, places.abs() as u32).0
            // value >> places
        } else if places > 0 && places < 16 {
            u16::overflowing_shl(value, places as u32).0
            // value << places
        } else if places == 0 {
            value 
        } else {
            error!("LOG_SHIFT places {} is out of range [-15,15]", places);
            value
        };

        self.store_result(state, new_value as u16);
        self.next_address
    }

    fn art_shift(&self, state: &mut State) -> usize {
        let operands = self.operand_values(state);

        let value = operands[0] as i16;
        let places = operands[1] as i16;

        let new_value = if places < 0 && places > -16 {
            i16::overflowing_shr(value, places.abs() as u32).0
            // value >> places
        } else if places > 0 && places < 16 {
            i16::overflowing_shl(value, places as u32).0
            // value << places
        } else if places == 0 {
            value 
        } else {
            error!("ART_SHIFT places {} is out of range [-15,15]", places);
            value
        };

        self.store_result(state, new_value as u16);
        self.next_address
    }

    fn save_undo(&self, state: &mut State) -> usize {
        self.store_result(state, 0xFFFF as u16);
        self.next_address
    }
}
