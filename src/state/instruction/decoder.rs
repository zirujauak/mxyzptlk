use crate::error::*;
use crate::state::header;
use crate::state::header::*;
use crate::state::instruction::*;
use crate::state::memory::*;

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
    memory: &Memory,
    opcode: &Opcode,
    mut address: usize,
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
            let b = memory.read_byte(address)?;
            address = address + 1;
            for i in 0..4 {
                match operand_type(b, i) {
                    Some(t) => types.push(t),
                    None => break,
                }
            }
            // 2VAR opcodes have another byte of operand types
            if opcode.opcode() == 0xEC || opcode.opcode() == 0xFA {
                let b = memory.read_byte(address)?;
                address = address + 1;
                for i in 0..4 {
                    match operand_type(b, i) {
                        Some(t) => types.push(t),
                        None => break,
                    }
                }
            }
        }
    }

    Ok((address, types))
}

fn operands(
    memory: &Memory,
    operand_types: &Vec<OperandType>,
    mut address: usize,
) -> Result<(usize, Vec<Operand>), RuntimeError> {
    let mut operands = Vec::new();

    for optype in operand_types {
        match optype {
            OperandType::LargeConstant => {
                operands.push(Operand::new(*optype, memory.read_word(address)?));
                address = address + 2;
            }
            OperandType::SmallConstant | OperandType::Variable => {
                operands.push(Operand::new(*optype, memory.read_byte(address)? as u16));
                address = address + 1;
            }
        }
    }

    Ok((address, operands))
}

fn result_variable(
    memory: &Memory,
    opcode: &Opcode,
    version: u8,
    address: usize,
) -> Result<(usize, Option<StoreResult>), RuntimeError> {
    match opcode.form() {
        OpcodeForm::Ext => match opcode.opcode() {
            0x00 | 0x01 | 0x02 | 0x03 | 0x04 | 0x09 | 0x0a => Ok((
                address + 1,
                Some(StoreResult::new(address, memory.read_byte(address)?)),
            )),
            _ => Ok((address, None)),
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
            | 0xe7 | 0xeC | 0xf6 | 0xf7 | 0xf8 => Ok((
                address + 1,
                Some(StoreResult::new(address, memory.read_byte(address)?)),
            )),
            // Version < 5
            0xbf => {
                if version < 5 {
                    return Ok((
                        address + 1,
                        Some(StoreResult::new(address, memory.read_byte(address)?)),
                    ));
                } else {
                    return Ok((address, None));
                }
            }
            // Version 4
            0xb5 | 0xb6 => {
                if version == 4 {
                    return Ok((
                        address + 1,
                        Some(StoreResult::new(address, memory.read_byte(address)?)),
                    ));
                } else {
                    return Ok((address, None));
                }
            }
            // Version > 4
            0xb9 | 0xe4 => {
                if version > 4 {
                    return Ok((
                        address + 1,
                        Some(StoreResult::new(address, memory.read_byte(address)?)),
                    ));
                } else {
                    return Ok((address, None));
                }
            }
            _ => Ok((address, None)),
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
    memory: &Memory,
    address: usize,
) -> Result<(usize, Option<Branch>), RuntimeError> {
    let b = memory.read_byte(address)?;
    let condition = b & 0x80 == 0x80;
    match b & 0x40 {
        0x40 => {
            let offset = b & 0x3f;
            Ok((
                address + 1,
                Some(Branch::new(
                    address,
                    condition,
                    branch_address(address + 1 - 2, offset as i16),
                )),
            ))
        }
        _ => {
            let mut offset =
                ((b as u16 & 0x3f) << 8) | (memory.read_byte(address + 1)? as u16) & 0xFF;
            if offset & 0x2000 == 0x2000 {
                offset = offset | 0xC000;
            }
            Ok((
                address + 2,
                Some(Branch::new(
                    address,
                    condition,
                    branch_address(address + 2 - 2, offset as i16),
                )),
            ))
        }
    }
}

fn branch(
    memory: &Memory,
    opcode: &Opcode,
    address: usize,
) -> Result<(usize, Option<Branch>), RuntimeError> {
    let version = header::field_byte(memory, HeaderField::Version)?;
    match opcode.form {
        OpcodeForm::Ext => match opcode.instruction() {
            0x06 | 0x18 | 0x1b => branch_condition(memory, address),
            _ => Ok((address, None)),
        },
        _ => match opcode.operand_count() {
            OperandCount::_0OP => match opcode.instruction() {
                0x0d | 0x0f => branch_condition(memory, address),
                0x05 | 0x06 => {
                    if version < 4 {
                        branch_condition(memory, address)
                    } else {
                        Ok((address, None))
                    }
                }
                _ => Ok((address, None)),
            },
            OperandCount::_1OP => match opcode.instruction() {
                0x00 | 0x01 | 0x02 => branch_condition(memory, address),
                _ => Ok((address, None)),
            },
            OperandCount::_2OP => match opcode.instruction() {
                0x01 | 0x02 | 0x03 | 0x04 | 0x05 | 0x06 | 0x07 | 0x0a => {
                    branch_condition(memory, address)
                }
                _ => Ok((address, None)),
            },
            OperandCount::_VAR => match opcode.instruction() {
                0x17 | 0x1F => branch_condition(memory, address),
                _ => Ok((address, None)),
            },
        },
    }
}

fn opcode(memory: &Memory, mut address: usize) -> Result<(usize, Opcode), RuntimeError> {
    let mut opcode = memory.read_byte(address)?;
    let extended = opcode == 0xBE;
    address = address + 1;
    if extended {
        opcode = memory.read_byte(address)?;
        address = address + 1;
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
        address,
        Opcode::new(
            header::field_byte(memory, HeaderField::Version)?,
            opcode,
            instruction,
            form,
            operand_count,
        ),
    ))
}

pub fn decode_instruction(memory: &Memory, address: usize) -> Result<Instruction, RuntimeError> {
    let version = header::field_byte(memory, HeaderField::Version)?;
    let (offset, opcode) = opcode(memory, address)?;

    let (offset, operand_types) = operand_types(memory, &opcode, offset)?;
    let (offset, operands) = operands(memory, &operand_types, offset)?;
    let (offset, store) = result_variable(memory, &opcode, version, offset)?;
    let (offset, branch) = branch(memory, &opcode, offset)?;

    let mut bytes = Vec::new();
    for i in address..offset {
        bytes.push(memory.read_byte(i)?)
    }

    Ok(Instruction::new(
        address, opcode, operands, store, branch, offset,
    ))
}
