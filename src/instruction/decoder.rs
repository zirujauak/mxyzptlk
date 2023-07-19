use super::*;
use crate::{
    error::*,
    zmachine::{state::memory, ZMachine},
};

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
    bytes: &[u8],
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
            offset += 1;
            for i in 0..4 {
                match operand_type(b, i) {
                    Some(t) => types.push(t),
                    None => break,
                }
            }
            // 2VAR opcodes have another byte of operand types
            if opcode.form() == &OpcodeForm::Var
                && (opcode.opcode() == 0xEC || opcode.opcode() == 0xFA)
            {
                let b = bytes[offset];
                offset += 1;
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
    bytes: &[u8],
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
                offset += 2;
            }
            OperandType::SmallConstant | OperandType::Variable => {
                operands.push(Operand::new(*optype, bytes[offset] as u16));
                offset += 1;
            }
        }
    }

    Ok((offset, operands))
}

const STORE_INSTRUCTIONS: &[u8] = &[
    0x08, 0x28, 0x48, 0x68, 0xc8, 0x09, 0x29, 0x49, 0x69, 0xc9, 0x0F, 0x2F, 0x4F, 0x6F, 0xcf, 0x10,
    0x30, 0x50, 0x70, 0xd0, 0x11, 0x31, 0x51, 0x71, 0xd1, 0x12, 0x32, 0x52, 0x72, 0xd2, 0x13, 0x33,
    0x53, 0x73, 0xd3, 0x14, 0x34, 0x54, 0x74, 0xd4, 0x15, 0x35, 0x55, 0x75, 0xd5, 0x16, 0x36, 0x56,
    0x76, 0xd6, 0x17, 0x37, 0x57, 0x77, 0xd7, 0x18, 0x38, 0x58, 0x78, 0xd8, 0x19, 0x39, 0x59, 0x79,
    0xd9, 0x81, 0x91, 0xa1, 0x82, 0x92, 0xa2, 0x83, 0x93, 0xa3, 0x84, 0x94, 0xa4, 0x88, 0x98, 0xa8,
    0x8e, 0x9e, 0xae, 0xe0, 0xe7, 0xec, 0xf6, 0xf7, 0xf8,
];

const EXT_STORE_INSTRUCTIONS: &[u8] = &[0x00, 0x01, 0x02, 0x03, 0x04, 0x09, 0x0a];

fn is_store_instruction(opcode: &Opcode) -> bool {
    match opcode.form() {
        OpcodeForm::Ext => EXT_STORE_INSTRUCTIONS.to_vec().contains(&opcode.opcode()),
        _ => {
            let mut v = STORE_INSTRUCTIONS.to_vec();
            match opcode.version() {
                3 => {}
                4 => {
                    v.push(0xB5);
                    v.push(0xB6);
                }
                _ => {
                    v.push(0xB9);
                    v.push(0xE4);
                }
            }

            v.contains(&opcode.opcode())
        }
    }
}

fn result_variable(
    address: usize,
    bytes: &[u8],
    opcode: &Opcode,
    offset: usize,
) -> Result<(usize, Option<StoreResult>), RuntimeError> {
    if is_store_instruction(opcode) {
        Ok((offset + 1, Some(StoreResult::new(address, bytes[offset]))))
    } else {
        Ok((offset, None))
    }
}

fn branch_address(address: usize, offset: i16) -> usize {
    match offset {
        0 => 0,
        1 => 1,
        _ => ((address as isize) + offset as isize) as usize,
    }
}

fn branch_condition(
    address: usize,
    bytes: &[u8],
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
                    branch_address(address + offset - 1, b_offset as i16),
                )),
            ))
        }
        _ => {
            let mut b_offset = ((b as u16 & 0x3f) << 8) | (bytes[offset + 1] as u16) & 0xFF;
            if b_offset & 0x2000 == 0x2000 {
                b_offset |= 0xC000;
            }
            Ok((
                offset + 2,
                Some(Branch::new(
                    address + offset,
                    condition,
                    branch_address(address + offset, b_offset as i16),
                )),
            ))
        }
    }
}

fn branch(
    address: usize,
    bytes: &[u8],
    opcode: &Opcode,
    offset: usize,
) -> Result<(usize, Option<Branch>), RuntimeError> {
    match opcode.form {
        OpcodeForm::Ext => match opcode.instruction() {
            0x06 | 0x18 | 0x1b => branch_condition(address, bytes, offset),
            _ => Ok((offset, None)),
        },
        _ => match opcode.operand_count() {
            OperandCount::_0OP => match (opcode.version(), opcode.instruction()) {
                (_, 0x0d) | (_, 0x0f) => branch_condition(address, bytes, offset),
                (3, 0x05) | (3, 0x06) => branch_condition(address, bytes, offset),
                (_, _) => Ok((offset, None)),
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

fn opcode(bytes: &[u8], version: u8, offset: usize) -> Result<(usize, Opcode), RuntimeError> {
    let mut opcode = bytes[offset];
    let (offset, form) = match opcode {
        0xBE => {
            opcode = bytes[offset + 1];
            (offset + 2, OpcodeForm::Ext)
        }
        _ => (
            offset + 1,
            match (opcode >> 6) & 0x3 {
                3 => OpcodeForm::Var,
                2 => OpcodeForm::Short,
                _ => OpcodeForm::Long,
            },
        ),
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

pub fn decode_instruction(
    zmachine: &ZMachine,
    address: usize,
) -> Result<Instruction, RuntimeError> {
    let version = zmachine.version();
    let bytes = zmachine.instruction(address);
    let (offset, opcode) = opcode(&bytes, version, 0)?;
    let (offset, operand_types) = operand_types(&bytes, &opcode, offset)?;
    let (offset, operands) = operands(&bytes, &operand_types, offset)?;
    let (offset, store) = result_variable(address + offset, &bytes, &opcode, offset)?;
    let (offset, branch) = branch(address, &bytes, &opcode, offset)?;

    Ok(Instruction::new(
        address,
        opcode,
        operands,
        store,
        branch,
        address + offset,
    ))
}

#[cfg(test)]
mod tests {
    use crate::{
        assert_ok, assert_some, assert_some_eq,
        test_util::{mock_zmachine, test_map},
    };

    use super::*;

    fn mock_opcode(
        version: u8,
        opcode: u8,
        instruction: u8,
        form: OpcodeForm,
        operand_count: OperandCount,
    ) -> Opcode {
        Opcode::new(version, opcode, instruction, form, operand_count)
    }

    fn operand(operand_type: OperandType, value: u16) -> Operand {
        Operand::new(operand_type, value)
    }

    fn mock_branch(byte_address: usize, condition: bool, branch_address: usize) -> Branch {
        Branch::new(byte_address, condition, branch_address)
    }

    #[test]
    fn test_operand_type() {
        let types = 0x1B;
        assert_some_eq!(operand_type(types, 0), OperandType::LargeConstant);
        assert_some_eq!(operand_type(types, 1), OperandType::SmallConstant);
        assert_some_eq!(operand_type(types, 2), OperandType::Variable);
        assert!(operand_type(types, 3).is_none());
    }

    #[test]
    fn test_operand_types_zero_op() {
        assert!(operand_types(
            &[],
            &mock_opcode(0, 0x30, 0, OpcodeForm::Short, OperandCount::_0OP),
            0
        )
        .is_ok_and(|x| x == (0, vec![])));
    }

    #[test]
    fn test_operand_types_one_op_large_constant() {
        assert!(operand_types(
            &[],
            &mock_opcode(0, 0x00, 0, OpcodeForm::Short, OperandCount::_1OP),
            0
        )
        .is_ok_and(|x| x == (0, vec![OperandType::LargeConstant])));
    }

    #[test]
    fn test_operand_types_one_op_small_constant() {
        assert!(operand_types(
            &[],
            &mock_opcode(0, 0x10, 0, OpcodeForm::Short, OperandCount::_1OP),
            0
        )
        .is_ok_and(|x| x == (0, vec![OperandType::SmallConstant])));
    }

    #[test]
    fn test_operand_types_one_op_variable() {
        assert!(operand_types(
            &[],
            &mock_opcode(0, 0x20, 0, OpcodeForm::Short, OperandCount::_1OP),
            0
        )
        .is_ok_and(|x| x == (0, vec![OperandType::Variable])));
    }

    #[test]
    fn test_operand_types_two_op_small_small() {
        assert!(operand_types(
            &[],
            &mock_opcode(0, 0x00, 0, OpcodeForm::Long, OperandCount::_2OP),
            0
        )
        .is_ok_and(|x| x
            == (
                0,
                vec![OperandType::SmallConstant, OperandType::SmallConstant]
            )));
    }

    #[test]
    fn test_operand_types_two_op_small_variable() {
        assert!(operand_types(
            &[],
            &mock_opcode(0, 0x20, 0, OpcodeForm::Long, OperandCount::_2OP),
            0
        )
        .is_ok_and(|x| x == (0, vec![OperandType::SmallConstant, OperandType::Variable])));
    }

    #[test]
    fn test_operand_types_two_op_variable_variable() {
        assert!(operand_types(
            &[],
            &mock_opcode(0, 0x60, 0, OpcodeForm::Long, OperandCount::_2OP),
            0
        )
        .is_ok_and(|x| x == (0, vec![OperandType::Variable, OperandType::Variable])));
    }

    #[test]
    fn test_operand_types_two_op_variable_small() {
        assert!(operand_types(
            &[],
            &mock_opcode(0, 0x40, 0, OpcodeForm::Long, OperandCount::_2OP),
            0
        )
        .is_ok_and(|x| x == (0, vec![OperandType::Variable, OperandType::SmallConstant])));
    }

    #[test]
    fn test_operand_types_var_four_operands() {
        assert!(operand_types(
            &[0x18],
            &mock_opcode(0, 0, 0, OpcodeForm::Var, OperandCount::_VAR),
            0
        )
        .is_ok_and(|x| x
            == (
                1,
                vec![
                    OperandType::LargeConstant,
                    OperandType::SmallConstant,
                    OperandType::Variable,
                    OperandType::LargeConstant
                ]
            )));
    }

    #[test]
    fn test_operand_types_var_none() {
        assert!(operand_types(
            &[0x1C],
            &mock_opcode(0, 0, 0, OpcodeForm::Var, OperandCount::_VAR),
            0
        )
        .is_ok_and(|x| x
            == (
                1,
                vec![OperandType::LargeConstant, OperandType::SmallConstant]
            )));
    }

    #[test]
    fn test_operand_types_two_var_eight() {
        assert!(operand_types(
            &[0x18, 0x61],
            &mock_opcode(0, 0xEC, 0, OpcodeForm::Var, OperandCount::_VAR),
            0
        )
        .is_ok_and(|x| x
            == (
                2,
                vec![
                    OperandType::LargeConstant,
                    OperandType::SmallConstant,
                    OperandType::Variable,
                    OperandType::LargeConstant,
                    OperandType::SmallConstant,
                    OperandType::Variable,
                    OperandType::LargeConstant,
                    OperandType::SmallConstant
                ]
            )));
    }

    #[test]
    fn test_operand_types_two_var_none() {
        assert!(operand_types(
            &[0x18, 0x6f],
            &mock_opcode(0, 0xFA, 0, OpcodeForm::Var, OperandCount::_VAR),
            0
        )
        .is_ok_and(|x| x
            == (
                2,
                vec![
                    OperandType::LargeConstant,
                    OperandType::SmallConstant,
                    OperandType::Variable,
                    OperandType::LargeConstant,
                    OperandType::SmallConstant,
                    OperandType::Variable
                ]
            )));
    }

    #[test]
    fn test_operand_types_ext_four_operands() {
        assert!(operand_types(
            &[0x18],
            &mock_opcode(0, 0xBE, 0, OpcodeForm::Ext, OperandCount::_VAR),
            0
        )
        .is_ok_and(|x| x
            == (
                1,
                vec![
                    OperandType::LargeConstant,
                    OperandType::SmallConstant,
                    OperandType::Variable,
                    OperandType::LargeConstant
                ]
            )));
    }

    #[test]
    fn test_operand_types_ext_none() {
        assert!(operand_types(
            &[0x1C],
            &mock_opcode(0, 0xBE, 0, OpcodeForm::Ext, OperandCount::_VAR),
            0
        )
        .is_ok_and(|x| x
            == (
                1,
                vec![OperandType::LargeConstant, OperandType::SmallConstant]
            )));
    }

    #[test]
    fn test_operands() {
        assert!(operands(
            &[0xFF, 0x12, 0x34, 0x56, 0x78, 0xFF],
            &vec![
                OperandType::LargeConstant,
                OperandType::SmallConstant,
                OperandType::Variable
            ],
            1
        )
        .is_ok_and(|x| x
            == (
                5,
                vec![
                    operand(OperandType::LargeConstant, 0x1234),
                    operand(OperandType::SmallConstant, 0x56),
                    operand(OperandType::Variable, 0x78)
                ]
            )));
    }

    #[test]
    fn test_result_variable_v3() {
        let opcodes = [
            0x08, 0x28, 0x48, 0x68, 0xc8, 0x09, 0x29, 0x49, 0x69, 0xc9, 0x0F, 0x2F, 0x4F, 0x6F,
            0xcf, 0x10, 0x30, 0x50, 0x70, 0xd0, 0x11, 0x31, 0x51, 0x71, 0xd1, 0x12, 0x32, 0x52,
            0x72, 0xd2, 0x13, 0x33, 0x53, 0x73, 0xd3, 0x14, 0x34, 0x54, 0x74, 0xd4, 0x15, 0x35,
            0x55, 0x75, 0xd5, 0x16, 0x36, 0x56, 0x76, 0xd6, 0x17, 0x37, 0x57, 0x77, 0xd7, 0x18,
            0x38, 0x58, 0x78, 0xd8, 0x19, 0x39, 0x59, 0x79, 0xd9, 0x81, 0x91, 0xa1, 0x82, 0x92,
            0xa2, 0x83, 0x93, 0xa3, 0x84, 0x94, 0xa4, 0x88, 0x98, 0xa8, 0x8e, 0x9e, 0xae, 0xe0,
            0xe7, 0xec, 0xf6, 0xf7, 0xf8,
        ];

        for o in 0..=0xFF {
            let r = assert_ok!(result_variable(
                0x1234,
                &[0xFF, 0x56, 0xFF],
                &mock_opcode(3, o, o, OpcodeForm::Var, OperandCount::_VAR),
                1,
            ));
            if opcodes.contains(&o) {
                assert_eq!(r.0, 2);
                assert_some_eq!(r.1, StoreResult::new(0x1234, 0x56));
            } else {
                assert_eq!(r.0, 1);
                assert!(r.1.is_none());
            }
        }
    }

    #[test]
    fn test_result_variable_v4() {
        let opcodes = [
            0x08, 0x28, 0x48, 0x68, 0xc8, 0x09, 0x29, 0x49, 0x69, 0xc9, 0x0F, 0x2F, 0x4F, 0x6F,
            0xcf, 0x10, 0x30, 0x50, 0x70, 0xd0, 0x11, 0x31, 0x51, 0x71, 0xd1, 0x12, 0x32, 0x52,
            0x72, 0xd2, 0x13, 0x33, 0x53, 0x73, 0xd3, 0x14, 0x34, 0x54, 0x74, 0xd4, 0x15, 0x35,
            0x55, 0x75, 0xd5, 0x16, 0x36, 0x56, 0x76, 0xd6, 0x17, 0x37, 0x57, 0x77, 0xd7, 0x18,
            0x38, 0x58, 0x78, 0xd8, 0x19, 0x39, 0x59, 0x79, 0xd9, 0x81, 0x91, 0xa1, 0x82, 0x92,
            0xa2, 0x83, 0x93, 0xa3, 0x84, 0x94, 0xa4, 0x88, 0x98, 0xa8, 0x8e, 0x9e, 0xae, 0xe0,
            0xe7, 0xec, 0xf6, 0xf7, 0xf8, 0xb5, 0xb6,
        ];

        for o in 0..=0xFF {
            let r = assert_ok!(result_variable(
                0x1234,
                &[0xFF, 0x56, 0xFF],
                &mock_opcode(4, o, o, OpcodeForm::Var, OperandCount::_VAR),
                1,
            ));
            if opcodes.contains(&o) {
                assert_eq!(r.0, 2);
                assert_some_eq!(r.1, StoreResult::new(0x1234, 0x56));
            } else {
                assert_eq!(r.0, 1);
                assert!(r.1.is_none());
            }
        }
    }

    #[test]
    fn test_result_variable_v5() {
        let opcodes = [
            0x08, 0x28, 0x48, 0x68, 0xc8, 0x09, 0x29, 0x49, 0x69, 0xc9, 0x0F, 0x2F, 0x4F, 0x6F,
            0xcf, 0x10, 0x30, 0x50, 0x70, 0xd0, 0x11, 0x31, 0x51, 0x71, 0xd1, 0x12, 0x32, 0x52,
            0x72, 0xd2, 0x13, 0x33, 0x53, 0x73, 0xd3, 0x14, 0x34, 0x54, 0x74, 0xd4, 0x15, 0x35,
            0x55, 0x75, 0xd5, 0x16, 0x36, 0x56, 0x76, 0xd6, 0x17, 0x37, 0x57, 0x77, 0xd7, 0x18,
            0x38, 0x58, 0x78, 0xd8, 0x19, 0x39, 0x59, 0x79, 0xd9, 0x81, 0x91, 0xa1, 0x82, 0x92,
            0xa2, 0x83, 0x93, 0xa3, 0x84, 0x94, 0xa4, 0x88, 0x98, 0xa8, 0x8e, 0x9e, 0xae, 0xe0,
            0xe7, 0xec, 0xf6, 0xf7, 0xf8, 0xB9, 0xE4,
        ];

        for o in 0..=0xFF {
            let r = assert_ok!(result_variable(
                0x1234,
                &[0xFF, 0x56, 0xFF],
                &mock_opcode(5, o, o, OpcodeForm::Var, OperandCount::_VAR),
                1,
            ));
            if opcodes.contains(&o) {
                assert_eq!(r.0, 2);
                assert_some_eq!(r.1, StoreResult::new(0x1234, 0x56));
            } else {
                assert_eq!(r.0, 1);
                assert!(r.1.is_none());
            }
        }
    }

    #[test]
    fn test_result_variable_v5_ext() {
        let opcodes = [0x00, 0x01, 0x02, 0x03, 0x04, 0x09, 0x0a];

        for o in 0..=0xFF {
            let r = assert_ok!(result_variable(
                0x1234,
                &[0xFF, 0x56, 0xFF],
                &mock_opcode(5, o, o, OpcodeForm::Ext, OperandCount::_VAR),
                1,
            ));
            if opcodes.contains(&o) {
                assert_eq!(r.0, 2);
                assert_some_eq!(r.1, StoreResult::new(0x1234, 0x56));
            } else {
                assert_eq!(r.0, 1);
                assert!(r.1.is_none());
            }
        }
    }

    #[test]
    fn test_branch_address() {
        assert_eq!(branch_address(0x1234, 0), 0);
        assert_eq!(branch_address(0x1234, 1), 1);
        assert_eq!(branch_address(0x1234, 0x5678), 0x68AC);
        assert_eq!(branch_address(0x1234, -15), 0x1225);
    }

    #[test]
    fn test_branch_condition_one_byte_true() {
        let b = assert_ok!(branch_condition(0x1234, &[0xFF, 0xFE, 0xFF], 1));
        assert_eq!(b.0, 2);
        assert_some_eq!(b.1, mock_branch(0x1235, true, 0x1272));
    }

    #[test]
    fn test_branch_condition_one_byte_false() {
        let b = assert_ok!(branch_condition(0x1234, &[0xFF, 0x7E, 0xFF], 1));
        assert_eq!(b.0, 2);
        assert_some_eq!(b.1, mock_branch(0x1235, false, 0x1272));
    }

    #[test]
    fn test_branch_condition_two_byte_true() {
        let b = assert_ok!(branch_condition(0x1234, &[0xFF, 0xB2, 0x00, 0xFF], 1));
        assert_eq!(b.0, 3);
        assert_some_eq!(b.1, mock_branch(0x1235, true, 0x435));
    }

    #[test]
    fn test_branch_condition_two_byte_false() {
        let b = assert_ok!(branch_condition(0x1234, &[0xFF, 0x1F, 0xFF, 0xFF], 1));
        assert_eq!(b.0, 3);
        assert_some_eq!(b.1, mock_branch(0x1235, false, 0x3234));
    }

    #[test]
    fn test_branch_v3_zero_op() {
        let instructions = [0x05, 0x06, 0x0D, 0x0F];
        for i in 0..=0xFF {
            let b = assert_ok!(branch(
                0x1234,
                &[0xFF, 0xFE, 0xFF],
                &mock_opcode(3, i as u8, i as u8, OpcodeForm::Short, OperandCount::_0OP),
                1,
            ));
            if instructions.contains(&i) {
                assert_eq!(b.0, 2);
                assert_some_eq!(b.1, mock_branch(0x1235, true, 0x1272));
            } else {
                assert_eq!(b.0, 1);
                assert!(b.1.is_none());
            }
        }
    }

    #[test]
    fn test_branch_v4_zero_op() {
        let instructions = [0x0D, 0x0F];
        for i in 0..=0xFF {
            let b = assert_ok!(branch(
                0x1234,
                &[0xFF, 0xFE, 0xFF],
                &mock_opcode(4, i as u8, i as u8, OpcodeForm::Short, OperandCount::_0OP),
                1,
            ));
            if instructions.contains(&i) {
                assert_eq!(b.0, 2);
                assert_some_eq!(b.1, mock_branch(0x1235, true, 0x1272));
            } else {
                assert_eq!(b.0, 1);
                assert!(b.1.is_none());
            }
        }
    }

    #[test]
    fn test_branch_one_op() {
        let instructions = [0x00, 0x01, 0x02];
        for i in 0..=0xFF {
            let b = assert_ok!(branch(
                0x1234,
                &[0xFF, 0xFE, 0xFF],
                &mock_opcode(3, i as u8, i as u8, OpcodeForm::Short, OperandCount::_1OP),
                1,
            ));
            if instructions.contains(&i) {
                assert_eq!(b.0, 2);
                assert_some_eq!(b.1, mock_branch(0x1235, true, 0x1272));
            } else {
                assert_eq!(b.0, 1);
                assert!(b.1.is_none());
            }
        }
    }

    #[test]
    fn test_branch_two_op() {
        let instructions = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x0a];
        for i in 0..=0xFF {
            let b = assert_ok!(branch(
                0x1234,
                &[0xFF, 0xFE, 0xFF],
                &mock_opcode(3, i as u8, i as u8, OpcodeForm::Long, OperandCount::_2OP),
                1,
            ));
            if instructions.contains(&i) {
                assert_eq!(b.0, 2);
                assert_some_eq!(b.1, mock_branch(0x1235, true, 0x1272));
            } else {
                assert_eq!(b.0, 1);
                assert!(b.1.is_none());
            }
        }
    }

    #[test]
    fn test_branch_var() {
        let instructions = [0x17, 0x1F];
        for i in 0..=0xFF {
            let b = assert_ok!(branch(
                0x1234,
                &[0xFF, 0xFE, 0xFF],
                &mock_opcode(3, i as u8, i as u8, OpcodeForm::Var, OperandCount::_VAR),
                1,
            ));
            if instructions.contains(&i) {
                assert_eq!(b.0, 2);
                assert_some_eq!(b.1, mock_branch(0x1235, true, 0x1272));
            } else {
                assert_eq!(b.0, 1);
                assert!(b.1.is_none());
            }
        }
    }

    #[test]
    fn test_branch_ext() {
        let instructions = [0x06, 0x18, 0x1B];
        for i in 0..=0xFF {
            let b = assert_ok!(branch(
                0x1234,
                &[0xFF, 0xFE, 0xFF],
                &mock_opcode(5, i as u8, i as u8, OpcodeForm::Ext, OperandCount::_VAR),
                1,
            ));
            if instructions.contains(&i) {
                assert_eq!(b.0, 2);
                assert_some_eq!(b.1, mock_branch(0x1235, true, 0x1272));
            } else {
                assert_eq!(b.0, 1);
                assert!(b.1.is_none());
            }
        }
    }

    #[test]
    fn test_opcode_short_zero_op() {
        // 1011 = Short form, no operand
        let o = opcode(&[0xFF, 0xBF, 0xFF], 3, 1);
        let (offset, opcode) = assert_ok!(o);
        assert_eq!(offset, 2);
        assert_eq!(opcode.version(), 3);
        assert_eq!(opcode.opcode(), 0xBF);
        assert_eq!(opcode.instruction(), 0xF);
        assert_eq!(opcode.form(), &OpcodeForm::Short);
        assert_eq!(opcode.operand_count(), &OperandCount::_0OP);
    }

    #[test]
    fn test_opcode_short_one_op_large_constant() {
        // 1000 = Short form, large const operand
        let o = opcode(&[0xFF, 0x8F, 0xFF], 3, 1);
        let (offset, opcode) = assert_ok!(o);
        assert_eq!(offset, 2);
        assert_eq!(opcode.version(), 3);
        assert_eq!(opcode.opcode(), 0x8F);
        assert_eq!(opcode.instruction(), 0xF);
        assert_eq!(opcode.form(), &OpcodeForm::Short);
        assert_eq!(opcode.operand_count(), &OperandCount::_1OP);
    }

    #[test]
    fn test_opcode_short_one_op_small_constant() {
        // 1001 = Short form, small const operand
        let o = opcode(&[0xFF, 0x9F, 0xFF], 3, 1);
        let (offset, opcode) = assert_ok!(o);
        assert_eq!(offset, 2);
        assert_eq!(opcode.version(), 3);
        assert_eq!(opcode.opcode(), 0x9F);
        assert_eq!(opcode.instruction(), 0xF);
        assert_eq!(opcode.form(), &OpcodeForm::Short);
        assert_eq!(opcode.operand_count(), &OperandCount::_1OP);
    }

    #[test]
    fn test_opcode_short_one_op_var() {
        // 1010 = Short form, variable operand
        let o = opcode(&[0xFF, 0xAF, 0xFF], 3, 1);
        let (offset, opcode) = assert_ok!(o);
        assert_eq!(offset, 2);
        assert_eq!(opcode.version(), 3);
        assert_eq!(opcode.opcode(), 0xAF);
        assert_eq!(opcode.instruction(), 0xF);
        assert_eq!(opcode.form(), &OpcodeForm::Short);
        assert_eq!(opcode.operand_count(), &OperandCount::_1OP);
    }

    #[test]
    fn test_opcode_long_two_op_small_small() {
        // 0000 = Long form, small constant, small constant
        let o = opcode(&[0xFF, 0x1F, 0xFF], 3, 1);
        let (offset, opcode) = assert_ok!(o);
        assert_eq!(offset, 2);
        assert_eq!(opcode.version(), 3);
        assert_eq!(opcode.opcode(), 0x1F);
        assert_eq!(opcode.instruction(), 0x1F);
        assert_eq!(opcode.form(), &OpcodeForm::Long);
        assert_eq!(opcode.operand_count(), &OperandCount::_2OP);
    }

    #[test]
    fn test_opcode_long_two_op_small_var() {
        // 0010 = Long form, small constant, variable
        let o = opcode(&[0xFF, 0x3F, 0xFF], 3, 1);
        let (offset, opcode) = assert_ok!(o);
        assert_eq!(offset, 2);
        assert_eq!(opcode.version(), 3);
        assert_eq!(opcode.opcode(), 0x3F);
        assert_eq!(opcode.instruction(), 0x1F);
        assert_eq!(opcode.form(), &OpcodeForm::Long);
        assert_eq!(opcode.operand_count(), &OperandCount::_2OP);
    }

    #[test]
    fn test_opcode_long_two_op_var_small() {
        // 0100 = Long form, variable, small constant
        let o = opcode(&[0xFF, 0x5F, 0xFF], 3, 1);
        let (offset, opcode) = assert_ok!(o);
        assert_eq!(offset, 2);
        assert_eq!(opcode.version(), 3);
        assert_eq!(opcode.opcode(), 0x5F);
        assert_eq!(opcode.instruction(), 0x1F);
        assert_eq!(opcode.form(), &OpcodeForm::Long);
        assert_eq!(opcode.operand_count(), &OperandCount::_2OP);
    }

    #[test]
    fn test_opcode_long_two_op_var_var() {
        // 0110 = Long form, variable, variable
        let o = opcode(&[0xFF, 0x7F, 0xFF], 3, 1);
        let (offset, opcode) = assert_ok!(o);
        assert_eq!(offset, 2);
        assert_eq!(opcode.version(), 3);
        assert_eq!(opcode.opcode(), 0x7F);
        assert_eq!(opcode.instruction(), 0x1F);
        assert_eq!(opcode.form(), &OpcodeForm::Long);
        assert_eq!(opcode.operand_count(), &OperandCount::_2OP);
    }

    #[test]
    fn test_opcode_long_var_two_op() {
        // 0x1100 = Variable form, 2OP
        let o = opcode(&[0xFF, 0xDF, 0xFF], 3, 1);
        let (offset, opcode) = assert_ok!(o);
        assert_eq!(offset, 2);
        assert_eq!(opcode.version(), 3);
        assert_eq!(opcode.opcode(), 0xDF);
        assert_eq!(opcode.instruction(), 0x1F);
        assert_eq!(opcode.form(), &OpcodeForm::Var);
        assert_eq!(opcode.operand_count(), &OperandCount::_2OP);
    }

    #[test]
    fn test_opcode_var_var() {
        // 0x1110 = Variable form, VAR
        let o = opcode(&[0xFF, 0xFF, 0xFF], 3, 1);
        let (offset, opcode) = assert_ok!(o);
        assert_eq!(offset, 2);
        assert_eq!(opcode.version(), 3);
        assert_eq!(opcode.opcode(), 0xFF);
        assert_eq!(opcode.instruction(), 0x1F);
        assert_eq!(opcode.form(), &OpcodeForm::Var);
        assert_eq!(opcode.operand_count(), &OperandCount::_VAR);
    }

    #[test]
    fn test_opcode_ext() {
        let o = opcode(&[0xFF, 0xBE, 0xFF, 0xFF], 5, 1);
        let (offset, opcode) = assert_ok!(o);
        assert_eq!(offset, 3);
        assert_eq!(opcode.version(), 5);
        assert_eq!(opcode.opcode(), 0xFF);
        assert_eq!(opcode.instruction(), 0xFF);
        assert_eq!(opcode.form(), &OpcodeForm::Ext);
        assert_eq!(opcode.operand_count(), &OperandCount::_VAR);
    }

    #[test]
    fn test_decode_instruction_zero_op() {
        let mut map = test_map(3);
        // Put instruction above dynamic memory
        // PIRACY ?(label)
        map[0x600] = 0xBF;
        // Branch
        map[0x601] = 0xFE;
        let zmachine = mock_zmachine(map);

        let instruction = assert_ok!(decode_instruction(&zmachine, 0x600));
        assert_eq!(instruction.address(), 0x600);
        assert_eq!(instruction.opcode().version(), 3);
        assert_eq!(instruction.opcode().opcode(), 0xBF);
        assert_eq!(instruction.opcode().instruction(), 0xF);
        assert_eq!(instruction.opcode().form(), &OpcodeForm::Short);
        assert_eq!(instruction.operands(), &[]);
        assert!(instruction.branch().is_some());
        assert_eq!(instruction.branch().unwrap().address, 0x601);
        assert!(instruction.branch().unwrap().condition());
        assert_eq!(instruction.branch().unwrap().branch_address(), 0x63e);
        assert!(instruction.store().is_none());
        assert_eq!(instruction.next_address(), 0x602);
    }

    // Store
    #[test]
    fn test_decode_instruction_one_op_large_const() {
        let mut map = test_map(3);
        // GET_PARENT_OBJECT -> (result)
        // Put instruction above dynamic memory
        map[0x600] = 0x83;
        // Operand
        map[0x601] = 0x12;
        map[0x602] = 0x34;
        // Store var
        map[0x603] = 0x80;

        let zmachine = mock_zmachine(map);

        let instruction = assert_ok!(decode_instruction(&zmachine, 0x600));
        assert_eq!(instruction.address(), 0x600);
        assert_eq!(instruction.opcode().version(), 3);
        assert_eq!(instruction.opcode().opcode(), 0x83);
        assert_eq!(instruction.opcode().instruction(), 0x3);
        assert_eq!(instruction.opcode().form(), &OpcodeForm::Short);
        assert_eq!(
            instruction.operands(),
            &[operand(OperandType::LargeConstant, 0x1234)]
        );
        assert!(instruction.branch().is_none());
        let store = assert_some!(instruction.store());
        assert_eq!(store.address(), 0x603);
        assert_eq!(store.variable(), 0x80);
        assert_eq!(instruction.next_address(), 0x604);
    }

    // Branch + Store
    #[test]
    fn test_decode_instruction_one_op_small_const() {
        let mut map = test_map(3);
        // Put instruction above dynamic memory
        // GET_SIBLING_OBJECT -> (result) ?(label)
        map[0x600] = 0x91;
        // Operand
        map[0x601] = 0x12;
        // Store var
        map[0x602] = 0x80;
        // Branch
        map[0x603] = 0x81;
        map[0x604] = 0x00;

        let zmachine = mock_zmachine(map);

        let instruction = assert_ok!(decode_instruction(&zmachine, 0x600));
        assert_eq!(instruction.address(), 0x600);
        assert_eq!(instruction.opcode().version(), 3);
        assert_eq!(instruction.opcode().opcode(), 0x91);
        assert_eq!(instruction.opcode().instruction(), 0x1);
        assert_eq!(instruction.opcode().form(), &OpcodeForm::Short);
        assert_eq!(
            instruction.operands(),
            &[operand(OperandType::SmallConstant, 0x12)]
        );
        assert!(instruction.branch().is_some());
        assert_eq!(instruction.branch().unwrap().address(), 0x603);
        assert!(instruction.branch().unwrap().condition());
        assert_eq!(instruction.branch().unwrap().branch_address(), 0x703);
        assert!(instruction.store().is_some());
        assert_eq!(instruction.store().unwrap().address(), 0x602);
        assert_eq!(instruction.store().unwrap().variable(), 0x80);
        assert_eq!(instruction.next_address(), 0x605);
    }

    #[test]
    fn test_decode_instruction_one_op_var() {
        let mut map = test_map(3);
        // Put instruction above dynamic memory
        // REMOVE_OBJ
        map[0x600] = 0xA9;
        // Operand
        map[0x601] = 0x12;

        let zmachine = mock_zmachine(map);

        let instruction = assert_ok!(decode_instruction(&zmachine, 0x600));
        assert_eq!(instruction.address(), 0x600);
        assert_eq!(instruction.opcode().version(), 3);
        assert_eq!(instruction.opcode().opcode(), 0xA9);
        assert_eq!(instruction.opcode().instruction(), 0x9);
        assert_eq!(instruction.opcode().form(), &OpcodeForm::Short);
        assert_eq!(
            instruction.operands(),
            &[operand(OperandType::Variable, 0x12)]
        );
        assert!(instruction.branch().is_none());
        assert!(instruction.store().is_none());
        assert_eq!(instruction.next_address(), 0x602);
    }

    #[test]
    fn test_decode_instruction_two_op_small_small() {
        let mut map = test_map(3);
        // Put instruction above dynamic memory
        // JE ?(label)
        map[0x600] = 0x01;
        // Operands
        map[0x601] = 0x12;
        map[0x602] = 0x34;
        // Branch
        map[0x603] = 0x7F;

        let zmachine = mock_zmachine(map);

        let instruction = assert_ok!(decode_instruction(&zmachine, 0x600));
        assert_eq!(instruction.address(), 0x600);
        assert_eq!(instruction.opcode().version(), 3);
        assert_eq!(instruction.opcode().opcode(), 0x01);
        assert_eq!(instruction.opcode().instruction(), 0x1);
        assert_eq!(instruction.opcode().form(), &OpcodeForm::Long);
        assert_eq!(
            instruction.operands(),
            &[
                operand(OperandType::SmallConstant, 0x12),
                operand(OperandType::SmallConstant, 0x34)
            ]
        );
        assert!(instruction.branch().is_some());
        assert_eq!(instruction.branch().unwrap().address(), 0x603);
        assert!(!instruction.branch().unwrap().condition());
        assert_eq!(instruction.branch().unwrap().branch_address(), 0x641);
        assert!(instruction.store().is_none());
        assert_eq!(instruction.next_address(), 0x604);
    }

    #[test]
    fn test_decode_instruction_two_op_small_var() {
        let mut map = test_map(3);
        // Put instruction above dynamic memory
        // OR -> (result)
        map[0x600] = 0x28;
        // Operands
        map[0x601] = 0x12;
        map[0x602] = 0x34;
        // Store
        map[0x603] = 0x80;

        let zmachine = mock_zmachine(map);

        let instruction = assert_ok!(decode_instruction(&zmachine, 0x600));
        assert_eq!(instruction.address(), 0x600);
        assert_eq!(instruction.opcode().version(), 3);
        assert_eq!(instruction.opcode().opcode(), 0x28);
        assert_eq!(instruction.opcode().instruction(), 0x8);
        assert_eq!(instruction.opcode().form(), &OpcodeForm::Long);
        assert_eq!(
            instruction.operands(),
            &[
                operand(OperandType::SmallConstant, 0x12),
                operand(OperandType::Variable, 0x34)
            ]
        );
        assert!(instruction.branch().is_none());
        assert!(instruction.store().is_some());
        assert_eq!(instruction.store().unwrap().address(), 0x603);
        assert_eq!(instruction.store().unwrap().variable(), 0x80);
        assert_eq!(instruction.next_address(), 0x604);
    }

    #[test]
    fn test_decode_instruction_two_op_var_small() {
        let mut map = test_map(3);
        // Put instruction above dynamic memory
        // SET_ATTR
        map[0x600] = 0x4B;
        // Operands
        map[0x601] = 0x12;
        map[0x602] = 0x34;

        let zmachine = mock_zmachine(map);

        let instruction = assert_ok!(decode_instruction(&zmachine, 0x600));
        assert_eq!(instruction.address(), 0x600);
        assert_eq!(instruction.opcode().version(), 3);
        assert_eq!(instruction.opcode().opcode(), 0x4B);
        assert_eq!(instruction.opcode().instruction(), 0xB);
        assert_eq!(instruction.opcode().form(), &OpcodeForm::Long);
        assert_eq!(
            instruction.operands(),
            &[
                operand(OperandType::Variable, 0x12),
                operand(OperandType::SmallConstant, 0x34)
            ]
        );
        assert!(instruction.branch().is_none());
        assert!(instruction.store().is_none());
        assert_eq!(instruction.next_address(), 0x603);
    }

    #[test]
    fn test_decode_instruction_two_op_var_var() {
        let mut map = test_map(3);
        // Put instruction above dynamic memory
        // LOADW -> (result)
        map[0x600] = 0x6F;
        // Operands
        map[0x601] = 0x12;
        map[0x602] = 0x34;
        // Result
        map[0x603] = 0x80;

        let zmachine = mock_zmachine(map);

        let instruction = assert_ok!(decode_instruction(&zmachine, 0x600));
        assert_eq!(instruction.address(), 0x600);
        assert_eq!(instruction.opcode().version(), 3);
        assert_eq!(instruction.opcode().opcode(), 0x6F);
        assert_eq!(instruction.opcode().instruction(), 0xF);
        assert_eq!(instruction.opcode().form(), &OpcodeForm::Long);
        assert_eq!(
            instruction.operands(),
            &[
                operand(OperandType::Variable, 0x12),
                operand(OperandType::Variable, 0x34)
            ]
        );
        assert!(instruction.branch().is_none());
        assert!(instruction.store().is_some());
        assert_eq!(instruction.store().unwrap().address(), 0x603);
        assert_eq!(instruction.store().unwrap().variable(), 0x80);
        assert_eq!(instruction.next_address(), 0x604);
    }

    #[test]
    fn test_decode_instruction_var_two_op() {
        let mut map = test_map(3);
        // Put instruction above dynamic memory
        // MOD -> (result)
        map[0x600] = 0xD8;
        // Operand types - large, large
        map[0x601] = 0x0F;
        // Operands
        map[0x602] = 0x12;
        map[0x603] = 0x34;
        map[0x604] = 0x56;
        map[0x605] = 0x78;
        // Store
        map[0x606] = 0x80;

        let zmachine = mock_zmachine(map);

        let instruction = assert_ok!(decode_instruction(&zmachine, 0x600));
        assert_eq!(instruction.address(), 0x600);
        assert_eq!(instruction.opcode().version(), 3);
        assert_eq!(instruction.opcode().opcode(), 0xD8);
        assert_eq!(instruction.opcode().instruction(), 0x18);
        assert_eq!(instruction.opcode().form(), &OpcodeForm::Var);
        assert_eq!(
            instruction.operands(),
            &[
                operand(OperandType::LargeConstant, 0x1234),
                operand(OperandType::LargeConstant, 0x5678)
            ]
        );
        assert!(instruction.branch().is_none());
        assert!(instruction.store().is_some());
        assert_eq!(instruction.store().unwrap().address(), 0x606);
        assert_eq!(instruction.store().unwrap().variable(), 0x80);
        assert_eq!(instruction.next_address(), 0x607);
    }

    #[test]
    fn test_decode_instruction_var_var() {
        let mut map = test_map(3);
        // Put instruction above dynamic memory
        // PRINT_NUM
        map[0x600] = 0xE6;
        // Operand types - large
        map[0x601] = 0x3F;
        // Operands
        map[0x602] = 0x12;
        map[0x603] = 0x34;

        let zmachine = mock_zmachine(map);

        let instruction = assert_ok!(decode_instruction(&zmachine, 0x600));
        assert_eq!(instruction.address(), 0x600);
        assert_eq!(instruction.opcode().version(), 3);
        assert_eq!(instruction.opcode().opcode(), 0xE6);
        assert_eq!(instruction.opcode().instruction(), 0x6);
        assert_eq!(instruction.opcode().form(), &OpcodeForm::Var);
        assert_eq!(
            instruction.operands(),
            &[operand(OperandType::LargeConstant, 0x1234)]
        );
        assert!(instruction.branch().is_none());
        assert!(instruction.store().is_none());
        assert_eq!(instruction.next_address(), 0x604);
    }

    #[test]
    fn test_decode_instruction_ext() {
        let mut map = test_map(5);
        // Put instruction above dynamic memory
        // LOG_SHIFT -> (result)
        map[0x600] = 0xBE;
        map[0x601] = 0x02;
        // Operand types - small, small
        map[0x602] = 0x5F;
        // Operands
        map[0x603] = 0x12;
        map[0x604] = 0x34;
        // Store
        map[0x605] = 0x80;

        let zmachine = mock_zmachine(map);

        let instruction = assert_ok!(decode_instruction(&zmachine, 0x600));
        assert_eq!(instruction.address(), 0x600);
        assert_eq!(instruction.opcode().version(), 5);
        assert_eq!(instruction.opcode().opcode(), 0x02);
        assert_eq!(instruction.opcode().instruction(), 0x02);
        assert_eq!(instruction.opcode().form(), &OpcodeForm::Ext);
        assert_eq!(
            instruction.operands(),
            &[
                operand(OperandType::SmallConstant, 0x12),
                operand(OperandType::SmallConstant, 0x34)
            ]
        );
        assert!(instruction.branch().is_none());
        assert!(instruction.store().is_some());
        assert_eq!(instruction.store().unwrap().address(), 0x605);
        assert_eq!(instruction.store().unwrap().variable(), 0x80);
        assert_eq!(instruction.next_address(), 0x606);
    }
}
