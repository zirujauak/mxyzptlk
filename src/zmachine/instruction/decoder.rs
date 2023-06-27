use crate::error::*;
use crate::zmachine::instruction::*;
use crate::zmachine::state::{memory, State};

fn operand_type(type_byte: u8, operand_index: u8) -> Option<OperandType> {
    // Types are packed in the byte: 00112233
    // To get type 1 (index 0), shift left 6 bits
    // To get type 2 (index 1), shift left 4 bits
    // ... to get type n, shift left 6 - (n * 2) bits
    let t = (type_byte >> (6 - (operand_index * 2))) & 3;
    match t {
        0 => Some(OperandType::LargeConstant),
        1 => Some(OperandType::SmallConstant),
        2 => Some(OperandType::Variable),
        _ => None,
    }
}

fn long_operand_type(opcode: u8, index: u8) -> OperandType {
    if opcode >> (6 - index) & 1 == 1 {
        OperandType::Variable
    } else {
        OperandType::SmallConstant
    }
}

fn operand_types(
    bytes: &Vec<u8>,
    opcode: &Opcode,
    mut offset: usize,
) -> Result<(usize, Vec<OperandType>), RuntimeError> {
    let mut types = Vec::new();
    match opcode.form() {
        OpcodeForm::Short => {
            if let Some(t) = operand_type(opcode.opcode(), 1) {
                types.push(t);
            }
        }
        OpcodeForm::Long => {
            types.push(long_operand_type(opcode.opcode(), 0));
            types.push(long_operand_type(opcode.opcode(), 1));
        }
        OpcodeForm::Var | OpcodeForm::Ext => {
            let b = bytes[offset];
            offset = offset + 1;
            for i in 0..4 {
                match operand_type(b, i) {
                    Some(t) => types.push(t),
                    None => break,
                }
            }
            // 2VAR opcodes have another byte of operand types
            if opcode.opcode() == 0xEC || opcode.opcode() == 0xFA {
                let b = bytes[offset];
                offset = offset + 1;
                for i in 0..4 {
                    match operand_type(b, i) {
                        Some(t) => types.push(t),
                        None => break,
                    }
                }
            }
        }
    }

    Ok((offset, types))
}

fn operands(
    bytes: &Vec<u8>,
    operand_types: &Vec<OperandType>,
    mut offset: usize,
) -> Result<(usize, Vec<Operand>), RuntimeError> {
    let mut operands = Vec::new();

    for optype in operand_types {
        match optype {
            OperandType::LargeConstant => {
                operands.push(Operand::new(
                    *optype,
                    memory::word_value(bytes[offset], bytes[offset + 1]),
                ));
                offset = offset + 2;
            }
            OperandType::SmallConstant | OperandType::Variable => {
                operands.push(Operand::new(*optype, bytes[offset] as u16));
                offset = offset + 1;
            }
        }
    }

    Ok((offset, operands))
}

fn result_variable(
    address: usize,
    bytes: &Vec<u8>,
    opcode: &Opcode,
    version: u8,
    offset: usize,
) -> Result<(usize, Option<StoreResult>), RuntimeError> {
    match opcode.form() {
        OpcodeForm::Ext => match opcode.opcode() {
            0x00 | 0x01 | 0x02 | 0x03 | 0x04 | 0x09 | 0x0a => {
                Ok((offset + 1, Some(StoreResult::new(address, bytes[offset]))))
            }
            _ => Ok((offset, None)),
        },
        _ => match opcode.opcode() {
            // Always store, regardless of version
            0x08 | 0x28 | 0x48 | 0x68 | 0xc8 | 0x09 | 0x29 | 0x49 | 0x69 | 0xc9 | 0x0F | 0x2F
            | 0x4F | 0x6F | 0xcf | 0x10 | 0x30 | 0x50 | 0x70 | 0xd0 | 0x11 | 0x31 | 0x51 | 0x71
            | 0xd1 | 0x12 | 0x32 | 0x52 | 0x72 | 0xd2 | 0x13 | 0x33 | 0x53 | 0x73 | 0xd3 | 0x14
            | 0x34 | 0x54 | 0x74 | 0xd4 | 0x15 | 0x35 | 0x55 | 0x75 | 0xd5 | 0x16 | 0x36 | 0x56
            | 0x76 | 0xd6 | 0x17 | 0x37 | 0x57 | 0x77 | 0xd7 | 0x18 | 0x38 | 0x58 | 0x78 | 0xd8
            | 0x19 | 0x39 | 0x59 | 0x79 | 0xd9 | 0x81 | 0x91 | 0xa1 | 0x82 | 0x92 | 0xa2 | 0x83
            | 0x93 | 0xa3 | 0x84 | 0x94 | 0xa4 | 0x88 | 0x98 | 0xa8 | 0x8e | 0x9e | 0xae | 0xe0
            | 0xe7 | 0xeC | 0xf6 | 0xf7 | 0xf8 => {
                Ok((offset + 1, Some(StoreResult::new(address, bytes[offset]))))
            }
            // Version < 5
            0xbf => {
                if version < 5 {
                    return Ok((offset + 1, Some(StoreResult::new(address, bytes[offset]))));
                } else {
                    return Ok((offset, None));
                }
            }
            // Version 4
            0xb5 | 0xb6 => {
                if version == 4 {
                    return Ok((offset + 1, Some(StoreResult::new(address, bytes[offset]))));
                } else {
                    return Ok((offset, None));
                }
            }
            // Version > 4
            0xb9 | 0xe4 => {
                if version > 4 {
                    return Ok((offset + 1, Some(StoreResult::new(address, bytes[offset]))));
                } else {
                    return Ok((offset, None));
                }
            }
            _ => Ok((offset, None)),
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

fn branch_condition(
    address: usize,
    bytes: &Vec<u8>,
    offset: usize,
) -> Result<(usize, Option<Branch>), RuntimeError> {
    let b = bytes[offset];
    let condition = b & 0x80 == 0x80;
    match b & 0x40 {
        0x40 => {
            let b_offset = b & 0x3f;
            Ok((
                offset + 1,
                Some(Branch::new(
                    address + offset,
                    condition,
                    branch_address(address - 1, b_offset as i16),
                )),
            ))
        }
        _ => {
            let mut b_offset = ((b as u16 & 0x3f) << 8) | (bytes[offset + 1] as u16) & 0xFF;
            if b_offset & 0x2000 == 0x2000 {
                b_offset = b_offset | 0xC000;
            }
            Ok((
                offset + 2,
                Some(Branch::new(
                    address + offset,
                    condition,
                    branch_address(address, b_offset as i16),
                )),
            ))
        }
    }
}

fn branch(
    address: usize,
    bytes: &Vec<u8>,
    version: u8,
    opcode: &Opcode,
    offset: usize,
) -> Result<(usize, Option<Branch>), RuntimeError> {
    match opcode.form {
        OpcodeForm::Ext => match opcode.instruction() {
            0x06 | 0x18 | 0x1b => branch_condition(address, bytes, offset),
            _ => Ok((offset, None)),
        },
        _ => match opcode.operand_count() {
            OperandCount::_0OP => match opcode.instruction() {
                0x0d | 0x0f => branch_condition(address, bytes, offset),
                0x05 | 0x06 => {
                    if version < 4 {
                        branch_condition(address, bytes, offset)
                    } else {
                        Ok((offset, None))
                    }
                }
                _ => Ok((offset, None)),
            },
            OperandCount::_1OP => match opcode.instruction() {
                0x00 | 0x01 | 0x02 => branch_condition(address, bytes, offset),
                _ => Ok((offset, None)),
            },
            OperandCount::_2OP => match opcode.instruction() {
                0x01 | 0x02 | 0x03 | 0x04 | 0x05 | 0x06 | 0x07 | 0x0a => {
                    branch_condition(address, bytes, offset)
                }
                _ => Ok((offset, None)),
            },
            OperandCount::_VAR => match opcode.instruction() {
                0x17 | 0x1F => branch_condition(address, bytes, offset),
                _ => Ok((offset, None)),
            },
        },
    }
}

fn opcode(
    bytes: &Vec<u8>,
    version: u8,
    mut offset: usize,
) -> Result<(usize, Opcode), RuntimeError> {
    let mut opcode = bytes[offset];
    let extended = opcode == 0xBE;
    offset = offset + 1;
    if extended {
        opcode = bytes[offset];
        offset = offset + 1;
    }

    let form = if extended {
        OpcodeForm::Ext
    } else {
        match (opcode >> 6) & 0x3 {
            3 => OpcodeForm::Var,
            2 => OpcodeForm::Short,
            _ => OpcodeForm::Long,
        }
    };

    let instruction = match form {
        OpcodeForm::Var | OpcodeForm::Long => opcode & 0x1F,
        OpcodeForm::Short => opcode & 0xF,
        OpcodeForm::Ext => opcode,
    };

    let operand_count = match form {
        OpcodeForm::Short => {
            if opcode & 0x30 == 0x30 {
                OperandCount::_0OP
            } else {
                OperandCount::_1OP
            }
        }
        OpcodeForm::Long => OperandCount::_2OP,
        OpcodeForm::Var => {
            if opcode & 0x20 == 0x20 {
                OperandCount::_VAR
            } else {
                OperandCount::_2OP
            }
        }
        OpcodeForm::Ext => OperandCount::_VAR,
    };

    Ok((
        offset,
        Opcode::new(version, opcode, instruction, form, operand_count),
    ))
}

pub fn decode_instruction(state: &State, address: usize) -> Result<Instruction, RuntimeError> {
    let version = state.version();
    let bytes = state.instruction(address);
    let (offset, opcode) = opcode(&bytes, version, 0)?;

    let (offset, operand_types) = operand_types(&bytes, &opcode, offset)?;
    let (offset, operands) = operands(&bytes, &operand_types, offset)?;
    let (offset, store) = result_variable(address + offset, &bytes, &opcode, version, offset)?;
    let (offset, branch) = branch(address + offset, &bytes, version, &opcode, offset)?;

    Ok(Instruction::new(
        address,
        opcode,
        operands,
        store,
        branch,
        address + offset,
    ))
}
