use crate::recoverable_error;

use super::*;

pub fn save(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    if !operands.is_empty() {
        info!(target: "app::instruction", "SAVE auxiliary data not implemented yet");
        store_result(zmachine, instruction, 0)?;
    } else {
        // unwrap() should be safe here because this is a store instruction
        match zmachine.save(instruction.store().unwrap().address()) {
            Ok(_) => {
                store_result(zmachine, instruction, 1)?;
            }
            Err(_) => {
                store_result(zmachine, instruction, 0)?;
            }
        }
    }
    Ok(instruction.next_address())
}

pub fn restore(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    if !operands.is_empty() {
        info!(target: "app::instruction", "RESTORE auxiliary data not implemented yet");
        store_result(zmachine, instruction, 0)?;
        Ok(instruction.next_address())
    } else {
        match zmachine.restore() {
            Ok(address) => match address {
                Some(a) => {
                    let i = decoder::decode_instruction(zmachine, a - 3)?;
                    store_result(zmachine, &i, 2)?;
                    Ok(i.next_address())
                }
                None => {
                    store_result(zmachine, instruction, 0)?;
                    Ok(instruction.next_address())
                }
            },
            Err(e) => {
                zmachine.print_str(format!("Error restoring: {}\r", e))?;
                store_result(zmachine, instruction, 0)?;
                Ok(instruction.next_address())
            }
        }
    }
}

pub fn log_shift(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let value = operands[0];
    let places = operands[1] as i16;
    let new_value = if places < 0 && places > -16 {
        u16::overflowing_shr(value, places.unsigned_abs() as u32).0
    } else if places > 0 && places < 16 {
        u16::overflowing_shl(value, places as u32).0
    } else if places == 0 {
        value
    } else {
        // Store a 0 here before returning an error so the user may
        // opt to recover
        store_result(zmachine, instruction, 0)?;
        return recoverable_error!(ErrorCode::InvalidShift, "Invalid shift bits {}", places);
    };

    store_result(zmachine, instruction, new_value)?;
    Ok(instruction.next_address())
}

pub fn art_shift(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let value = operands[0] as i16;
    let places = operands[1] as i16;
    let new_value = if places < 0 && places > -16 {
        i16::overflowing_shr(value, places.unsigned_abs() as u32).0
    } else if places > 0 && places < 16 {
        i16::overflowing_shl(value, places as u32).0
    } else if places == 0 {
        value
    } else {
        // Store a 0 or -1 here before returning an error so the user may
        // opt to recover
        store_result(
            zmachine,
            instruction,
            if value < 0 && places < 0 { 0xFFFF } else { 0 },
        )?;
        return recoverable_error!(ErrorCode::InvalidShift, "Invalid shift bits {}", places);
    };

    store_result(zmachine, instruction, new_value as u16)?;
    Ok(instruction.next_address())
}

pub fn set_font(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let result = zmachine.set_font(operands[0])?;
    store_result(zmachine, instruction, result)?;
    Ok(instruction.next_address())
}

pub fn save_undo(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    // unwrap() should be safe here because this is a store instruction
    zmachine.save_undo(instruction.store().unwrap().address())?;
    store_result(zmachine, instruction, 1)?;
    Ok(instruction.next_address())
}

pub fn restore_undo(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    match zmachine.restore_undo() {
        Ok(pc) => match pc {
            Some(address) => {
                let i = decoder::decode_instruction(zmachine, address - 3)?;
                store_result(zmachine, &i, 2)?;
                Ok(i.next_address())
            }
            None => {
                store_result(zmachine, instruction, 0)?;
                Ok(instruction.next_address())
            }
        },
        Err(_) => {
            store_result(zmachine, instruction, 0)?;
            Ok(instruction.next_address())
        }
    }
}

// pub fn print_unicode(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     todo!()
// }

// pub fn check_unicode(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     todo!()
// }

// pub fn set_true_colour(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     todo!()
// }

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use crate::{
        assert_ok_eq,
        instruction::{processor::dispatch, Opcode, OpcodeForm, OperandCount, OperandType},
        test_util::*,
    };

    fn opcode(instruction: u8) -> Opcode {
        Opcode::new(5, 0xBE, instruction, OpcodeForm::Ext, OperandCount::_VAR)
    }

    #[test]
    fn test_save() {
        input(&[
            '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}', 'v', '5', '.', 'i', 'f',
            'z', 's',
        ]);

        let map = test_map(5);
        let i = mock_store_instruction(0x480, vec![], opcode(0), 0x483, store(0x482, 0x80));
        let mut zmachine = mock_zmachine(map);
        let a = dispatch(&mut zmachine, &i);
        assert!(Path::new("test-v5.ifzs").exists());
        assert!(fs::remove_file(Path::new("test-v5.ifzs")).is_ok());
        assert_ok_eq!(a, 0x483);
        assert_ok_eq!(zmachine.variable(0x80), 1);
    }

    #[test]
    fn test_save_fail() {
        input(&[
            '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}',
            '\u{8}', '\u{8}', '\u{8}', '/', 'x', '/', 'x',
        ]);

        let map = test_map(5);
        let i = mock_store_instruction(0x480, vec![], opcode(0), 0x483, store(0x482, 0x80));
        let mut zmachine = mock_zmachine(map);
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x483);
        assert_ok_eq!(zmachine.variable(0x80), 0);
    }

    #[test]
    fn test_restore_v5() {
        input(&[
            '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}', 'v', '5', 'r', '.', 'i',
            'f', 'z', 's',
        ]);

        // Save a file
        let mut map = test_map(5);
        // Set up the save instruction for the restore to decode
        map[0x480] = 0xbe;
        map[0x481] = 0x00;
        map[0x482] = 0xFF;
        map[0x483] = 0x80;

        set_variable(&mut map, 0x80, 0xFF);
        set_variable(&mut map, 0x81, 0xFE);

        let mut zmachine = mock_zmachine(map);

        let i = mock_store_instruction(0x480, vec![], opcode(0), 0x484, store(0x483, 0x80));

        let a = dispatch(&mut zmachine, &i);
        assert!(Path::new("test-v5r.ifzs").exists());
        assert_ok_eq!(a, 0x484);
        assert_ok_eq!(zmachine.variable(0x80), 0x01);

        let i2 = mock_store_instruction(0x484, vec![], opcode(1), 0x486, store(0x485, 0x81));
        input(&[
            '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}',
            '\u{8}', '\u{8}', '\u{8}', 't', 'e', 's', 't', '-', 'v', '5', 'r', '.', 'i', 'f', 'z',
            's',
        ]);

        let a = dispatch(&mut zmachine, &i2);
        assert!(fs::remove_file(Path::new("test-v5r.ifzs")).is_ok());
        assert_ok_eq!(a, 0x484);
        assert_ok_eq!(zmachine.variable(0x80), 0x02);
    }

    #[test]
    fn test_restore_v5_fail() {
        let mut map = test_map(5);

        set_variable(&mut map, 0x80, 0xFF);
        set_variable(&mut map, 0x81, 0xFE);

        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(0x484, vec![], opcode(1), 0x486, store(0x485, 0x81));

        let a = dispatch(&mut zmachine, &i);
        assert_ok_eq!(a, 0x486);
        assert_ok_eq!(zmachine.variable(0x81), 0);
    }

    #[test]
    fn test_log_shift_0() {
        let map = test_map(5);

        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x0001),
                operand(OperandType::SmallConstant, 0),
            ],
            opcode(2),
            0x487,
            store(0x486, 0x81),
        );

        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x487);
        assert_ok_eq!(zmachine.variable(0x81), 1);
    }

    #[test]
    fn test_log_shift_left() {
        let map = test_map(5);

        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x0001),
                operand(OperandType::SmallConstant, 15),
            ],
            opcode(2),
            0x487,
            store(0x486, 0x81),
        );

        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x487);
        assert_ok_eq!(zmachine.variable(0x81), 0x8000);
    }

    #[test]
    fn test_log_shift_right() {
        let map = test_map(5);

        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x8000),
                operand(OperandType::SmallConstant, 0xFFF1),
            ],
            opcode(2),
            0x487,
            store(0x486, 0x81),
        );

        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x487);
        assert_ok_eq!(zmachine.variable(0x81), 0x1);
    }

    #[test]
    fn test_log_shift_left_overflow() {
        let map = test_map(5);

        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x8000),
                operand(OperandType::SmallConstant, 16),
            ],
            opcode(2),
            0x487,
            store(0x486, 0x81),
        );

        assert!(dispatch(&mut zmachine, &i).is_err());
        assert_ok_eq!(zmachine.variable(0x81), 0);
    }

    #[test]
    fn test_log_shift_right_overflow() {
        let map = test_map(5);

        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x8000),
                operand(OperandType::SmallConstant, 0xFFF0),
            ],
            opcode(2),
            0x487,
            store(0x486, 0x81),
        );

        assert!(dispatch(&mut zmachine, &i).is_err());
        assert_ok_eq!(zmachine.variable(0x81), 0);
    }

    #[test]
    fn test_art_shift_0() {
        let map = test_map(5);

        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x0001),
                operand(OperandType::SmallConstant, 0),
            ],
            opcode(3),
            0x487,
            store(0x486, 0x81),
        );

        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x487);
        assert_ok_eq!(zmachine.variable(0x81), 1);
    }

    #[test]
    fn test_art_shift_left() {
        let map = test_map(5);

        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x0001),
                operand(OperandType::SmallConstant, 15),
            ],
            opcode(3),
            0x487,
            store(0x486, 0x81),
        );

        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x487);
        assert_ok_eq!(zmachine.variable(0x81), 0x8000);
    }

    #[test]
    fn test_art_shift_right() {
        let map = test_map(5);

        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x8000),
                operand(OperandType::SmallConstant, 0xFFF1),
            ],
            opcode(3),
            0x487,
            store(0x486, 0x81),
        );

        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x487);
        assert_ok_eq!(zmachine.variable(0x81), 0xFFFF);
    }

    #[test]
    fn test_art_shift_left_overflow() {
        let map = test_map(5);

        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x8000),
                operand(OperandType::SmallConstant, 16),
            ],
            opcode(3),
            0x487,
            store(0x486, 0x81),
        );

        assert!(dispatch(&mut zmachine, &i).is_err());
        assert_ok_eq!(zmachine.variable(0x81), 0);
    }

    #[test]
    fn test_art_shift_right_overflow() {
        let map = test_map(5);

        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x8000),
                operand(OperandType::SmallConstant, 0xFFF0),
            ],
            opcode(3),
            0x487,
            store(0x486, 0x81),
        );

        assert!(dispatch(&mut zmachine, &i).is_err());
        assert_ok_eq!(zmachine.variable(0x81), 0xFFFF);
    }

    #[test]
    fn test_set_font() {
        let map = test_map(5);

        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 0x3)],
            opcode(4),
            0x484,
            store(0x483, 0x81),
        );

        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x484);
        assert_ok_eq!(zmachine.variable(0x81), 1);
    }

    #[test]
    fn test_set_font_0() {
        let map = test_map(5);

        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 0)],
            opcode(4),
            0x484,
            store(0x483, 0x81),
        );

        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x484);
        assert_ok_eq!(zmachine.variable(0x81), 1);
    }

    #[test]
    fn test_set_font_invalid() {
        let map = test_map(5);

        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 9)],
            opcode(4),
            0x484,
            store(0x483, 0x81),
        );

        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x484);
        assert_ok_eq!(zmachine.variable(0x81), 0);
    }

    #[test]
    fn test_save_undo() {
        let map = test_map(5);

        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(0x400, vec![], opcode(9), 0x483, store(0x482, 0x81));

        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x483);
        assert_ok_eq!(zmachine.variable(0x81), 1);
    }

    #[test]
    fn test_restore_undo() {
        let mut map = test_map(5);
        // Put the save instruction into memory for the restore
        map[0x400] = 0xBE;
        map[0x401] = 0x0A;
        map[0x402] = 0xFF;
        map[0x403] = 0x80;

        let mut zmachine = mock_zmachine(map);

        // Save current state
        let i = mock_store_instruction(0x400, vec![], opcode(9), 0x404, store(0x403, 0x81));
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x404);
        assert_ok_eq!(zmachine.variable(0x81), 1);

        // Simulate a function call to change state
        mock_frame(&mut zmachine, 0x600, None, 0x500);
        assert_eq!(zmachine.frame_count(), 2);

        let i = mock_store_instruction(0x600, vec![], opcode(10), 0x604, store(0x603, 0x81));
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x404);
        assert_eq!(zmachine.frame_count(), 1);
        assert_ok_eq!(zmachine.variable(0x80), 2);
    }

    #[test]
    fn test_restore_undo_fail() {
        let map = test_map(5);
        let mut zmachine = mock_zmachine(map);

        // No undo save
        let i = mock_store_instruction(0x600, vec![], opcode(10), 0x604, store(0x603, 0x81));
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x604);
        assert_eq!(zmachine.frame_count(), 1);
        assert_ok_eq!(zmachine.variable(0x81), 0);
    }
}
