use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use super::object;
use super::state::State;
use super::{text, util};

#[derive(Debug)]
pub enum OperandType {
    LargeConstant,
    SmallConstant,
    Variable,
}

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

    fn operands(
        memory_map: &Vec<u8>,
        mut address: usize,
        opcode: &Opcode,
    ) -> (usize, Vec<Operand>) {
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
                    let b1 = memory_map[address];
                    address = address + 1;
                    for i in 0..4 {
                        match Self::operand_type(b1, i) {
                            Some(t) => operand_types.push(t),
                            None => break,
                        }
                    }

                    let b2 = memory_map[address];
                    address = address + 1;
                    for i in 0..4 {
                        match Self::operand_type(b2, i) {
                            Some(t) => operand_types.push(t),
                            None => break,
                        }
                    }
                } else if opcode.opcode & 0x20 == 0 {
                    let b = memory_map[address];
                    address = address + 1;
                    operand_types.push(Self::operand_type(b, 0).unwrap());
                    operand_types.push(Self::operand_type(b, 1).unwrap());
                } else {
                    let b = memory_map[address];
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
                let b = memory_map[address];
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
                    let v = util::word_value(memory_map, address);
                    address = address + 2;
                    operands.push(Operand {
                        operand_type: t,
                        operand_value: v,
                    })
                }
                OperandType::SmallConstant | OperandType::Variable => {
                    let v = util::byte_value(memory_map, address);
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

    fn opcode(memory_map: &Vec<u8>, mut address: usize) -> (usize, Opcode) {
        let mut opcode = memory_map[address];
        address = address + 1;
        if opcode == 0xBE {
            opcode = memory_map[address];
            address = address + 1;
        }

        let form = if opcode == 0xBE {
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

    fn store(
        memory_map: &Vec<u8>,
        version: u8,
        address: usize,
        opcode: &Opcode,
    ) -> (usize, Option<u8>) {
        match opcode.form {
            OpcodeForm::Extended => match opcode.instruction {
                0x00 | 0x01 | 0x02 | 0x03 | 0x04 | 0x09 | 0x0A | 0x0C | 0x13 | 0x1D => {
                    (address + 1, Some(util::byte_value(memory_map, address)))
                }
                _ => (address, None),
            },
            _ => match opcode.opcount {
                OperandCount::_0OP => match version {
                    4 => match opcode.instruction {
                        0x05 | 0x06 => (address + 1, Some(util::byte_value(memory_map, address))),
                        _ => (address, None),
                    },
                    _ => (address, None),
                },
                OperandCount::_1OP => match opcode.instruction {
                    0x01 | 0x02 | 0x03 | 0x04 | 0x0E => {
                        (address + 1, Some(util::byte_value(memory_map, address)))
                    }
                    0x08 => {
                        if version >= 4 {
                            (address + 1, Some(util::byte_value(memory_map, address)))
                        } else {
                            (address, None)
                        }
                    }
                    0x0F => {
                        if version <= 4 {
                            (address + 1, Some(util::byte_value(memory_map, address)))
                        } else {
                            (address, None)
                        }
                    }
                    _ => (address, None),
                },
                OperandCount::_2OP => match opcode.instruction {
                    0x08 | 0x09 | 0x0F | 0x10 | 0x11 | 0x12 | 0x13 | 0x14 | 0x15 | 0x16 | 0x17
                    | 0x18 | 0x19 => (address + 1, Some(util::byte_value(memory_map, address))),
                    _ => (address, None),
                },
                OperandCount::_VAR => match opcode.instruction {
                    0x00 | 0x07 | 0x0C | 0x16 | 0x17 | 0x18 => {
                        (address + 1, Some(util::byte_value(memory_map, address)))
                    }
                    0x04 => {
                        if version >= 5 {
                            (address + 1, Some(util::byte_value(memory_map, address)))
                        } else {
                            (address, None)
                        }
                    }
                    0x09 => {
                        if version == 6 {
                            (address + 1, Some(util::byte_value(memory_map, address)))
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

    fn decode_branch(memory_map: &Vec<u8>, address: usize) -> (usize, Option<Branch>) {
        let b = util::byte_value(memory_map, address);
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
                    (((b & 0x3f) as u16) << 8) | util::byte_value(memory_map, address + 1) as u16;
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

    fn branch(
        memory_map: &Vec<u8>,
        version: u8,
        address: usize,
        opcode: &Opcode,
    ) -> (usize, Option<Branch>) {
        match opcode.form {
            OpcodeForm::Extended => match opcode.instruction {
                0x06 | 0x18 | 0x1b => Self::decode_branch(memory_map, address),
                _ => (address, None),
            },
            _ => match opcode.opcount {
                OperandCount::_0OP => match opcode.instruction {
                    0x0d | 0x0f => Self::decode_branch(memory_map, address),
                    0x05 | 0x06 => {
                        if version < 4 {
                            Self::decode_branch(memory_map, address)
                        } else {
                            (address, None)
                        }
                    }
                    _ => (address, None),
                },
                OperandCount::_1OP => match opcode.instruction {
                    0x00 | 0x01 | 0x02 => Self::decode_branch(memory_map, address),
                    _ => (address, None),
                },
                OperandCount::_2OP => match opcode.instruction {
                    0x01 | 0x02 | 0x03 | 0x04 | 0x05 | 0x06 | 0x07 | 0x0a => {
                        Self::decode_branch(memory_map, address)
                    }
                    _ => (address, None),
                },
                OperandCount::_VAR => (address, None),
            },
        }
    }

    pub fn from_address(state: &State, address: usize) -> Instruction {
        let (next_address, opcode) = Self::opcode(&state.memory_map(), address);
        let (next_address, operands) = Self::operands(&state.memory_map(), next_address, &opcode);
        let (next_address, store) =
            Self::store(&state.memory_map(), state.version, next_address, &opcode);
        let (next_address, branch) =
            Self::branch(&state.memory_map(), state.version, next_address, &opcode);
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
                0x0 => rtrue(state, &self),
                0x1 => rfalse(state, &self),
                0x2 => print_literal(state, &self),
                0xB => new_line(state, &self),
                _ => 0,
            },
            OperandCount::_1OP => match self.opcode.instruction {
                0x1 => get_sibling(state, &self),
                0x2 => get_child(state, &self),
                0x3 => get_parent(state, &self),
                0x5 => inc(state, &self),
                0x6 => dec(state, &self),
                0xA => print_obj(state, &self),
                0xC => jump(state, &self),
                0xD => print_paddr(state, &self),
                0xF => call_1n(state, &self),
                _ => 0,
            },
            OperandCount::_2OP => match self.opcode.instruction {
                0x01 => je(state, &self),
                0x02 => jl(state, &self),
                0x09 => and(state, &self),
                0x0A => test_attr(state, &self),
                0x0B => set_attr(state, &self),
                0x0C => clear_attr(state, &self),
                0x0D => store(state, &self),
                0x0F => loadw(state, &self),
                0x10 => loadb(state, &self),
                0x15 => sub(state, &self),
                0x11 => get_prop(state, &self),
                _ => 0,
            },
            OperandCount::_VAR => match self.opcode.instruction {
                0x00 => call(state, &self),
                0x01 => storew(state, &self),
                0x02 => storeb(state, &self),
                0x03 => put_prop(state, &self),
                0x04 => read(state, &self),
                0x05 => print_char(state, &self),
                0x06 => print_num(state, &self),
                0x07 => random(state, &self),
                _ => 0,
            },
        }
    }
}

fn operand_value(state: &mut State, operand: &Operand) -> u16 {
    match operand.operand_type {
        OperandType::SmallConstant | OperandType::LargeConstant => operand.operand_value,
        OperandType::Variable => state.variable(operand.operand_value as u8),
    }
}

fn branch(condition: bool, instruction: &Instruction) -> usize {
    if condition == instruction.branch.as_ref().unwrap().condition {
        instruction.branch.as_ref().unwrap().address
    } else {
        instruction.next_address
    }
}

// 0OP
fn rtrue(state: &mut State, _instruction: &Instruction) -> usize {
    trace!("RTRUE");
    state.return_fn(1 as u16)
}

fn rfalse(state: &mut State, _instruction: &Instruction) -> usize {
    trace!("RFALSE");
    state.return_fn(0 as u16)
}

fn print_literal(state: &State, instruction: &Instruction) -> usize {
    let mut ztext = Vec::new();

    let mut word = 0;
    while word & 0x8000 == 0 {
        word = util::word_value(
            &state.memory_map(),
            instruction.next_address + (ztext.len() * 2),
        );
        ztext.push(word);
    }

    let text = text::from_vec(&state.memory_map(), state.version, &ztext);
    print!("{}", text);
    trace!("PRINT \"{}\"", text);

    instruction.next_address + ztext.len() * 2
}

fn new_line(_state: &State, instruction: &Instruction) -> usize {
    println!("");
    trace!("NEW_LINE");
    instruction.next_address
}

// 1OP
fn get_sibling(state: &mut State, instruction: &Instruction) -> usize {
    let object = operand_value(state, &instruction.operands[0]) as usize;
    trace!("GET_SIBLING #{:04x}", object);

    let sibling = object::sibling(&state.memory_map(), state.version, object) as u16;
    state.set_variable(instruction.store.unwrap(), sibling);
    let condition = sibling != 0;
    branch(condition, instruction)
}

fn get_child(state: &mut State, instruction: &Instruction) -> usize {
    let object = operand_value(state, &instruction.operands[0]) as usize;
    trace!("GET_CHILD #{:04x}", object);

    let child = object::child(&state.memory_map(), state.version, object) as u16;
    state.set_variable(instruction.store.unwrap(), child);
    let condition = child != 0;
    branch(condition, instruction)
}

fn get_parent(state: &mut State, instruction: &Instruction) -> usize {
    let object = operand_value(state, &instruction.operands[0]) as usize;
    trace!("GET_PARENT #{:04x}", object);

    let parent = object::parent(&state.memory_map(), state.version, object) as u16;
    state.set_variable(instruction.store.unwrap(), parent);
    instruction.next_address
}

fn inc(state: &mut State, instruction: &Instruction) -> usize {
    let arg = operand_value(state, &instruction.operands[0]) as u8;
    trace!("INC #{:02x}", arg);
    let val = state.variable(arg);
    let new_val = val as i16 + 1;
    state.set_variable(arg, new_val as u16);
    instruction.next_address
}

fn dec(state: &mut State, instruction: &Instruction) -> usize {
    let arg = operand_value(state, &instruction.operands[0]) as u8;
    trace!("DEC #{:02x}", arg);
    let val = state.variable(arg);
    let new_val = val as i16 - 1;
    state.set_variable(arg, new_val as u16);
    instruction.next_address
}

fn print_obj(state: &mut State, instruction: &Instruction) -> usize {
    let object = operand_value(state, &instruction.operands[0]) as usize;
    let ztext = object::short_name(&state.memory_map(), state.version, object);

    let text = text::from_vec(&state.memory_map(), state.version, &ztext);
    print!("{}", text);
    trace!("PRINT_OBJ #{:04x} \"{}\'", object, text);
    instruction.next_address
}

fn jump(state: &mut State, instruction: &Instruction) -> usize {
    let offset = operand_value(state, &instruction.operands[0]) as i16;
    let address = (instruction.next_address as isize + offset as isize - 2) as usize;
    trace!("JUMP ${:05x}", address);
    address
}

fn print_paddr(state: &mut State, instruction: &Instruction) -> usize {
    let addr = operand_value(state, &instruction.operands[0]);
    let address = state.packed_string_address(addr);
    trace!("PRINT_PADDR ${:05x}", addr);

    let text = text::as_text(&state.memory_map(), state.version, address);
    print!("{}", text);
    instruction.next_address
}

fn call_1n(state: &mut State, instruction: &Instruction) -> usize {
    let addr = operand_value(state, &instruction.operands[0]);
    let address = state.packed_routine_address(addr);

    trace!("CALL_1N ${:05x}", address);
    state.call(address, instruction.next_address, &Vec::new(), None)
}

// 2OP
fn je(state: &mut State, instruction: &Instruction) -> usize {
    let a = operand_value(state, &instruction.operands[0]) as i16;
    let b = operand_value(state, &instruction.operands[1]) as i16;

    let condition = a == b;
    trace!(
        "JE #{:04x} #{:04x} [{}]",
        a,
        b,
        instruction
            .branch
            .as_ref()
            .unwrap()
            .condition
            .to_string()
            .to_uppercase()
    );
    branch(condition, instruction)
}

fn jl(state: &mut State, instruction: &Instruction) -> usize {
    let a = operand_value(state, &instruction.operands[0]) as i16;
    let b = operand_value(state, &instruction.operands[1]) as i16;

    let condition = a < b;
    trace!(
        "JL #{:04x} #{:04x} [{}]",
        a,
        b,
        instruction
            .branch
            .as_ref()
            .unwrap()
            .condition
            .to_string()
            .to_uppercase()
    );

    branch(condition, instruction)
}

fn and(state: &mut State, instruction: &Instruction) -> usize {
    let a = operand_value(state, &instruction.operands[0]) as u16;
    let b = operand_value(state, &instruction.operands[1]) as u16;

    trace!("AND #{:04x} #{:04x}", a, b);
    let v = a & b;
    state.set_variable(instruction.store.unwrap(), v);
    instruction.next_address
}

fn loadw(state: &mut State, instruction: &Instruction) -> usize {
    let addr = operand_value(state, &instruction.operands[0]) as usize;
    let index = operand_value(state, &instruction.operands[1]) as usize;

    let address = addr + (index * 2);
    let value = util::word_value(&state.memory_map(), address);
    trace!(
        "LOADW ${:05x} -> {:02x}",
        address,
        instruction.store.unwrap()
    );
    state.set_variable(instruction.store.unwrap(), value);
    instruction.next_address
}

fn loadb(state: &mut State, instruction: &Instruction) -> usize {
    let addr = operand_value(state, &instruction.operands[0]) as usize;
    let index = operand_value(state, &instruction.operands[1]) as usize;

    let address = addr + index;
    let value = util::byte_value(&state.memory_map(), address) as u16;
    trace!(
        "LOADB ${:05x} -> {:02}",
        address,
        instruction.store.unwrap()
    );
    state.set_variable(instruction.store.unwrap(), value);
    instruction.next_address
}

fn store(state: &mut State, instruction: &Instruction) -> usize {
    let var = operand_value(state, &instruction.operands[0]) as u8;
    let value = operand_value(state, &instruction.operands[1]) as u16;

    trace!("STORE #{:04x} -> #{:02x}", value, var);
    state.set_variable(var, value);
    instruction.next_address
}

fn test_attr(state: &mut State, instruction: &Instruction) -> usize {
    let object = operand_value(state, &instruction.operands[0]) as usize;
    let attribute = operand_value(state, &instruction.operands[1]) as u8;

    trace!("TEST_ATTR #{:04x} #{:02}", object, attribute);
    let version = state.version;
    let condition = object::attribute(state.memory_map_mut(), version, object, attribute);
    branch(condition, instruction)
}

fn set_attr(state: &mut State, instruction: &Instruction) -> usize {
    let object = operand_value(state, &instruction.operands[0]) as usize;
    let attribute = operand_value(state, &instruction.operands[1]) as u8;

    trace!("SET_ATTR #{:04x} #{:02}", object, attribute);
    let version = state.version;
    object::set_attribute(state.memory_map_mut(), version, object, attribute);
    instruction.next_address
}

fn clear_attr(state: &mut State, instruction: &Instruction) -> usize {
    let object = operand_value(state, &instruction.operands[0]) as usize;
    let attribute = operand_value(state, &instruction.operands[1]) as u8;

    trace!("CLEAR_ATTR #{:04x} #{:02x}", object, attribute);
    let version = state.version;
    object::clear_attribute(state.memory_map_mut(), version, object, attribute);
    instruction.next_address
}

fn get_prop(state: &mut State, instruction: &Instruction) -> usize {
    let object = operand_value(state, &instruction.operands[0]) as usize;
    let property = operand_value(state, &instruction.operands[1]) as u8;

    trace!("GET_PROP #{:04x} ${:02x}", object, property);

    let value = object::property(&state.memory_map(), state.version, object, property);
    state.set_variable(instruction.store.unwrap(), value);
    instruction.next_address
}

fn sub(state: &mut State, instruction: &Instruction) -> usize {
    let a = operand_value(state, &instruction.operands[0]) as i16;
    let b = operand_value(state, &instruction.operands[1]) as i16;

    trace!("SUB #{:04x} #{:04x}", a, b);
    let value = (a - b) as u16;
    state.set_variable(instruction.store.unwrap(), value);
    instruction.next_address
}

// VAR
fn call(state: &mut State, instruction: &Instruction) -> usize {
    let addr = operand_value(state, &instruction.operands[0]);
    let address = state.packed_routine_address(addr);

    let mut arguments = Vec::new();
    for i in 1..instruction.operands.len() {
        arguments.push(operand_value(state, &instruction.operands[i]))
    }
    trace!("CALL ${:05x} with {} arg(s)", address, arguments.len());
    state.call(
        address,
        instruction.next_address,
        &arguments,
        instruction.store,
    )
}

fn storew(state: &mut State, instruction: &Instruction) -> usize {
    let address = operand_value(state, &instruction.operands[0]) as usize;
    let index = operand_value(state, &instruction.operands[1]) as usize;
    let value = operand_value(state, &instruction.operands[2]) as u16;

    trace!("STOREW #{:04x} to ${:05x}", value, address + (index * 2));
    util::set_word(state.memory_map_mut(), address + (index * 2), value);
    instruction.next_address
}

fn storeb(state: &mut State, instruction: &Instruction) -> usize {
    let address = operand_value(state, &instruction.operands[0]) as usize;
    let index = operand_value(state, &instruction.operands[1]) as usize;
    let value = operand_value(state, &instruction.operands[2]) as u8;

    trace!("STOREB #{:02x} to ${:05x}", value, address + index);
    util::set_byte(state.memory_map_mut(), address + index, value);
    instruction.next_address
}

fn put_prop(state: &mut State, instruction: &Instruction) -> usize {
    let object = operand_value(state, &instruction.operands[0]) as usize;
    let prop = operand_value(state, &instruction.operands[1]) as u8;
    let value = operand_value(state, &instruction.operands[2]) as u16;

    trace!("PUT_PROP #{:04x} #{:02x} #{:04x}", object, prop, value);
    let version = state.version;
    object::set_property(state.memory_map_mut(), version, object, prop, value);
    instruction.next_address
}

pub fn read(state: &mut State, instruction: &Instruction) -> usize {
    let text = operand_value(state, &instruction.operands[0]) as u16;
    let parse = operand_value(state, &instruction.operands[1]) as u16;

    match state.version {
        1 | 2 | 3 => {
            trace!("SREAD #{:04x} #{:04x}", text, parse);
        }
        4 | 5 | 6 | 7 | 8 => {
            let time = operand_value(state, &instruction.operands[2]) as u16;
            let routine = operand_value(state, &instruction.operands[3]) as u16;
            trace!(
                "{}READ #{:04x} #{:04x} #{:04x} #{:04x}",
                if state.version == 4 { "S" } else { "A" },
                text,
                parse,
                time,
                routine
            );
        }
        _ => {}
    }

    panic!("read not implemented")
}

fn print_char(state: &mut State, instruction: &Instruction) -> usize {
    let c = operand_value(state, &instruction.operands[0]) as u8;

    trace!("PRINT_CHAR {}", c as char);
    print!("{}", c as char);
    instruction.next_address
}

fn print_num(state: &mut State, instruction: &Instruction) -> usize {
    let n = operand_value(state, &instruction.operands[0]);
    trace!("PRINT_NUM {}", n);
    print!("{}", n);
    instruction.next_address
}

fn random(state: &mut State, instruction: &Instruction) -> usize {
    let range = operand_value(state, &instruction.operands[0]) as i16;
    trace!("RANDOM #{}", range);

    if range < 0 {
        state.seed(range as u64);
        state.set_variable(instruction.store.unwrap(), 0);
    }
    if range == 0 {
        state.seed(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Error geting time")
                .as_millis() as u64,
        );
        state.set_variable(instruction.store.unwrap(), 0);
    } else {
        state.set_variable(instruction.store.unwrap(), state.random(range as u16));
    }

    instruction.next_address
}
