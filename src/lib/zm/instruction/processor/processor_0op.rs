//! [0OP](https://inform-fiction.org/zmachine/standards/z1point1/sect14.html#0OP)
//! instructions: short form instructions that have no operands.

use crate::error::{ErrorCode, RuntimeError};
use crate::instruction::{Instruction, InstructionResult, NextAddress::Address};
use crate::zmachine::header::HeaderField;
use crate::zmachine::{RequestType, ZMachine};
use crate::{fatal_error, text};

use super::{branch, store_result};

/// [RTRUE](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#rtrue): return true (1) from the current routine
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing the [InstructionResult] or a [RuntimeError]
pub fn rtrue(
    zmachine: &mut ZMachine,
    _instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    InstructionResult::new(zmachine.return_routine(1)?)
}

/// [RFALSE](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#rfalse): return false (0) from the current routine
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing the [InstructionResult] or a [RuntimeError]
pub fn rfalse(
    zmachine: &mut ZMachine,
    _instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    InstructionResult::new(zmachine.return_routine(0)?)
}

/// [PRINT](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#print): prints the inline ztext directly following the opcode.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing the [InstructionResult] or a [RuntimeError]
pub fn print(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let ztext = zmachine.string_literal(instruction.address + 1)?;
    let text = text::from_vec(zmachine, &ztext, false)?;
    zmachine.output(
        &text,
        Address(instruction.next_address + (ztext.len() * 2)),
        RequestType::Print,
    )
}

/// [PRINT_RET](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#print_ret): prints the inline ztext directly following the opcode followed by a
/// new line, then returns true from the current routine.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing the [InstructionResult] or a [RuntimeError]
pub fn print_ret(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let ztext = zmachine.string_literal(instruction.address + 1)?;
    let text = text::from_vec(zmachine, &ztext, false)?;
    let a = zmachine.return_routine(1)?;
    zmachine.output(&text, a, RequestType::PrintRet)
}

/// [NOP](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#nop): goes nowhere, does nothing.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing the [InstructionResult] or a [RuntimeError]
pub fn nop(
    _zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    InstructionResult::new(Address(instruction.next_address))
}

/// [SAVE](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#save): records current game state to a vector of zbytes (u8) to be saved to
/// a storage medium by the interpreter
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing an [InstructionResult] with a [RequestType::Save] interpreter
/// request or a [RuntimeError]
pub fn save_pre(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let pc = if zmachine.version() == 3 {
        match &instruction.branch {
            Some(b) => b.address,
            None => {
                return fatal_error!(
                    ErrorCode::InvalidInstruction,
                    "V3 SAVE should be a branch instruction"
                )
            }
        }
    } else {
        match instruction.store {
            Some(r) => r.address,
            None => {
                return fatal_error!(
                    ErrorCode::InvalidInstruction,
                    "V4 SAVE should be a store instruction"
                )
            }
        }
    };

    let save_data = zmachine.save_state(pc)?;
    InstructionResult::save(Address(instruction.address), zmachine.name(), save_data)
}

/// [RESTORE](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#restore): records current game state to a vector of zbytes (u8) to be saved to
/// a storage medium by the interpreter
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing an [InstructionResult] with a [RequestType::Restore] interpreter
/// request or a [RuntimeError]
pub fn restore_pre(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    // Interpreter will handle prompting for and loading the restore data
    InstructionResult::restore(Address(instruction.next_address), zmachine.name())
}

/// [RESTART](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#restart): resets
/// start and restarts exeuction from the initial address.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing an [InstructionResult] with a [RequestType::Restart] interpreter
/// request or a [RuntimeError]
pub fn restart(
    zmachine: &mut ZMachine,
    _instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    InstructionResult::restart(Address(zmachine.restart()?))
}

/// [RET_POPPED](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#ret_popped): pops
/// the value from the top of the stack and returns it as the result of the current
/// routine.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing an [InstructionResult] or a [RuntimeError]
pub fn ret_popped(
    zmachine: &mut ZMachine,
    _instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let value = zmachine.variable(0)?;
    InstructionResult::new(zmachine.return_routine(value)?)
}

/// [POP](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#pop): pops
/// the stack and throws away the result.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing an [InstructionResult] or a [RuntimeError]
pub fn pop(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    zmachine.variable(0)?;
    InstructionResult::new(Address(instruction.next_address))
}

/// [CATCH](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#catch): stores
/// the current frame pointer.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing an [InstructionResult] or a [RuntimeError]
pub fn catch(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let depth = zmachine.frame_count();
    store_result(zmachine, instruction, depth as u16)?;
    InstructionResult::new(Address(instruction.next_address))
}

/// [QUIT](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#quit): halts
/// execution and exits.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing an [InstructionResult] with a [RequestType::Quit] interpreter request
/// or a [RuntimeError]
pub fn quit(
    _zmachine: &mut ZMachine,
    _instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    InstructionResult::quit()
}

/// [NEW_LINE](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#new_line): prints
/// a new line.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing an [InstructionResult] with a [RequestType::NewLine] interpreter request
/// or a [RuntimeError]
pub fn new_line(
    _zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    InstructionResult::new_line(Address(instruction.next_address))
}

/// [SHOW_STATUS](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#show_status): prints
/// the current status line.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing an [InstructionResult] with a [RequestType::ShowStatus] interpreter request
/// or a [RuntimeError]
pub fn show_status(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let (left, right) = zmachine.status_line()?;
    InstructionResult::show_status(Address(instruction.next_address), left, right)
}

/// [VERIFY](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#verify): calculates
/// the checksum of the zcode and branches if the result matches the checksum in the header.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing an [InstructionResult] or a [RuntimeError]
pub fn verify(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let expected = zmachine.header_word(HeaderField::Checksum)?;
    let checksum = zmachine.checksum()?;

    InstructionResult::new(branch(zmachine, instruction, expected == checksum)?)
}

/// [PIRACY](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#piracy): performs
/// a piracy check.  Always branches.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing an [InstructionResult] or a [RuntimeError]
pub fn piracy(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    InstructionResult::new(branch(zmachine, instruction, true)?)
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use crate::{
        assert_ok_eq, assert_print,
        instruction::{processor::dispatch, Opcode, OpcodeForm, OperandCount},
        test_util::*,
    };

    fn opcode(version: u8, instruction: u8) -> Opcode {
        Opcode::new(
            version,
            instruction,
            instruction,
            OpcodeForm::Short,
            OperandCount::_0OP,
        )
    }

    #[test]
    fn test_rtrue() {
        let mut v = test_map(5);
        set_variable(&mut v, 0x80, 0xFF);
        let mut zmachine = mock_zmachine(v);
        mock_frame(&mut zmachine, 0x500, Some(0x80), 0x482);
        assert_eq!(zmachine.frame_count(), 2);
        let i = mock_instruction(0x500, vec![], opcode(3, 0), 0x501);
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x482);
        assert_eq!(zmachine.frame_count(), 1);
        assert_ok_eq!(zmachine.variable(0x80), 0x01);
    }

    #[test]
    fn test_rfalse() {
        let mut v = test_map(5);
        set_variable(&mut v, 0x80, 0xFF);
        let mut zmachine = mock_zmachine(v);
        mock_frame(&mut zmachine, 0x500, Some(0x80), 0x482);
        assert_eq!(zmachine.frame_count(), 2);
        let i = mock_instruction(0x500, vec![], opcode(3, 1), 0x501);
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x482);
        assert_eq!(zmachine.frame_count(), 1);
        assert_ok_eq!(zmachine.variable(0x80), 0x00);
    }

    #[test]
    fn test_print() {
        let mut v = test_map(5);
        // H e l l o
        v[0x481] = 0x11;
        v[0x482] = 0xaa;
        v[0x483] = 0xc6;
        v[0x484] = 0x34;

        let mut zmachine = mock_zmachine(v);
        let i = mock_instruction(0x480, vec![], opcode(3, 2), 0x481);
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x485);
        assert_print!("Hello");
    }

    #[test]
    fn test_print_ret() {
        let mut v = test_map(5);
        set_variable(&mut v, 0x80, 0xFF);

        // H e l l o
        v[0x502] = 0x11;
        v[0x503] = 0xaa;
        v[0x504] = 0xc6;
        v[0x505] = 0x34;

        let mut zmachine = mock_zmachine(v);
        mock_frame(&mut zmachine, 0x500, Some(0x80), 0x482);
        let i = mock_instruction(0x501, vec![], opcode(3, 3), 0x502);
        assert_eq!(zmachine.frame_count(), 2);
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x482);
        assert_eq!(zmachine.frame_count(), 1);
        assert_print!("Hello");
        assert_ok_eq!(zmachine.variable(0x80), 0x01);
    }

    #[test]
    fn test_nop() {
        let v = test_map(3);
        let mut zmachine = mock_zmachine(v);
        let i = mock_instruction(0x400, vec![], opcode(3, 4), 0x401);
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x401);
    }

    #[test]
    fn test_save_v3() {
        input(&[
            '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}', 'v', '3', '.', 'i', 'f',
            'z', 's',
        ]);

        let map = test_map(3);
        let i = mock_branch_instruction(
            0x480,
            vec![],
            opcode(3, 5),
            0x482,
            branch(0x481, true, 0x484),
        );
        let mut zmachine = mock_zmachine(map);
        let a = dispatch(&mut zmachine, &i);
        assert!(Path::new("test-v3.ifzs").exists());
        assert!(fs::remove_file(Path::new("test-v3.ifzs")).is_ok());
        assert_ok_eq!(a, 0x484);
    }

    #[test]
    fn test_save_v3_fail() {
        input(&[
            '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}',
            '\u{8}', '\u{8}', '\u{8}', '/', 'x', '/', 'x',
        ]);

        let map = test_map(3);
        let i = mock_branch_instruction(
            0x480,
            vec![],
            opcode(3, 5),
            0x482,
            branch(0x481, true, 0x484),
        );
        let mut zmachine = mock_zmachine(map);
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x482);
    }

    #[test]
    fn test_save_v3_bad_instruction() {
        let map = test_map(3);
        let i = mock_instruction(0x480, vec![], opcode(3, 5), 0x482);
        let mut zmachine = mock_zmachine(map);

        let a = dispatch(&mut zmachine, &i);
        assert!(a.is_err());
    }

    #[test]
    fn test_restore_v3() {
        // Save a file
        input(&[
            '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}', 'v', '3', 'r', '.', 'i',
            'f', 'z', 's',
        ]);

        let mut map = test_map(3);
        // Set up the save instruction for the restore to decode
        map[0x480] = 0xb5;
        map[0x481] = 0xc9;

        let i = mock_branch_instruction(
            0x480,
            vec![],
            opcode(3, 5),
            0x482,
            branch(0x481, true, 0x483),
        );

        let mut zmachine = mock_zmachine(map);
        let a = dispatch(&mut zmachine, &i);
        assert!(Path::new("test-v3r.ifzs").exists());
        assert_ok_eq!(a, 0x483);

        input(&[
            '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}',
            '\u{8}', '\u{8}', '\u{8}', 't', 'e', 's', 't', '-', 'v', '3', 'r', '.', 'i', 'f', 'z',
            's',
        ]);
        let i = mock_branch_instruction(
            0x480,
            vec![],
            opcode(3, 6),
            0x482,
            branch(0x481, true, 0x490),
        );
        let a = dispatch(&mut zmachine, &i);
        assert!(fs::remove_file(Path::new("test-v3r.ifzs")).is_ok());
        assert_ok_eq!(a, 0x489);
    }

    #[test]
    fn test_restore_v3_fail() {
        let map = test_map(3);

        let mut zmachine = mock_zmachine(map);
        let i = mock_branch_instruction(
            0x480,
            vec![],
            opcode(3, 6),
            0x482,
            branch(0x481, true, 0x490),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x482);
    }

    #[test]
    fn test_save_v4() {
        // Accept default save file name
        input(&[
            '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}', 'v', '4', '.', 'i', 'f',
            'z', 's',
        ]);

        let mut map = test_map(4);
        set_variable(&mut map, 0x80, 0xFF);

        let i = mock_store_instruction(0x480, vec![], opcode(4, 5), 0x483, store(0x481, 0x80));
        let mut zmachine = mock_zmachine(map);
        let a = dispatch(&mut zmachine, &i);
        assert!(Path::new("test-v4.ifzs").exists());
        assert!(fs::remove_file(Path::new("test-v4.ifzs")).is_ok());
        assert_ok_eq!(a, 0x483);
        assert_ok_eq!(zmachine.variable(0x80), 0x01);
    }

    #[test]
    fn test_save_v4_fail() {
        input(&[
            '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}',
            '\u{8}', '\u{8}', '\u{8}', '/', 'x', '/', 'x',
        ]);
        let mut map = test_map(4);
        set_variable(&mut map, 0x80, 0xFF);

        let i = mock_store_instruction(0x480, vec![], opcode(4, 5), 0x483, store(0x481, 0x80));
        let mut zmachine = mock_zmachine(map);
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x483);
        assert_ok_eq!(zmachine.variable(0x80), 0x00);
    }

    #[test]
    fn test_save_v4_bad_instruction() {
        let mut map = test_map(4);
        set_variable(&mut map, 0x80, 0xFF);

        let i = mock_instruction(0x480, vec![], opcode(4, 5), 0x483);
        let mut zmachine = mock_zmachine(map);
        assert!(dispatch(&mut zmachine, &i).is_err());
        assert_ok_eq!(zmachine.variable(0x80), 0xFF);
    }

    #[test]
    fn test_restore_v4() {
        input(&[
            '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}', 'v', '4', 'r', '.', 'i',
            'f', 'z', 's',
        ]);

        // Save a file
        let mut map = test_map(4);
        // Set up the save instruction for the restore to decode
        map[0x480] = 0xb5;
        map[0x481] = 0x80;

        set_variable(&mut map, 0x80, 0xFF);
        set_variable(&mut map, 0x81, 0xFE);

        let mut zmachine = mock_zmachine(map.clone());

        let i = mock_store_instruction(0x480, vec![], opcode(4, 5), 0x482, store(0x481, 0x80));

        let a = dispatch(&mut zmachine, &i);
        assert!(Path::new("test-v4r.ifzs").exists());
        assert_ok_eq!(a, 0x482);
        assert_ok_eq!(zmachine.variable(0x80), 0x01);

        let i2 = mock_store_instruction(0x484, vec![], opcode(4, 6), 0x486, store(0x485, 0x81));
        input(&[
            '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}',
            '\u{8}', '\u{8}', '\u{8}', 't', 'e', 's', 't', '-', 'v', '4', 'r', '.', 'i', 'f', 'z',
            's',
        ]);

        let a = dispatch(&mut zmachine, &i2);
        assert!(fs::remove_file(Path::new("test-v4r.ifzs")).is_ok());
        assert_ok_eq!(a, 0x482);
        assert_ok_eq!(zmachine.variable(0x81), 0x02);
    }

    #[test]
    fn test_restore_v4_fail() {
        // Save a file
        let mut map = test_map(4);
        // Set up the save instruction for the restore to decode
        map[0x480] = 0xb5;
        map[0x481] = 0x80;

        set_variable(&mut map, 0x80, 0xFF);
        set_variable(&mut map, 0x81, 0xFE);

        let mut zmachine = mock_zmachine(map);

        let i = mock_store_instruction(0x480, vec![], opcode(4, 5), 0x482, store(0x481, 0x80));

        let a = dispatch(&mut zmachine, &i);
        assert!(Path::new("test-01.ifzs").exists());
        assert!(fs::remove_file(Path::new("test-01.ifzs")).is_ok());
        assert_ok_eq!(a, 0x482);
        assert_ok_eq!(zmachine.variable(0x80), 0x01);

        let i2 = mock_store_instruction(0x484, vec![], opcode(4, 6), 0x486, store(0x485, 0x81));

        assert_ok_eq!(dispatch(&mut zmachine, &i2), 0x486);
        assert_ok_eq!(zmachine.variable(0x81), 0x00);
    }

    #[test]
    fn test_restart() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);

        let i = mock_instruction(0x480, vec![], opcode(3, 7), 0x481);

        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x400);
    }

    #[test]
    fn test_ret_popped() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        mock_frame(&mut zmachine, 0x500, Some(0x80), 0x402);
        assert!(zmachine.push(0x1122).is_ok());
        assert!(zmachine.push(0x3344).is_ok());

        let i = mock_instruction(0x501, vec![], opcode(3, 8), 0x502);

        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x402);
        assert_ok_eq!(zmachine.variable(0x80), 0x3344);
        assert!(zmachine.variable(0).is_err());
    }

    #[test]
    fn test_pop() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        assert!(zmachine.push(0x1122).is_ok());
        assert!(zmachine.push(0x3344).is_ok());

        let i = mock_instruction(0x501, vec![], opcode(3, 9), 0x502);

        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x502);
        assert_ok_eq!(zmachine.peek_variable(0), 0x1122);
    }

    #[test]
    fn test_catch() {
        let map = test_map(5);
        let mut zmachine = mock_zmachine(map);
        mock_frame(&mut zmachine, 0x480, None, 0x404);
        mock_frame(&mut zmachine, 0x500, None, 0x404);
        let i = mock_store_instruction(0x500, vec![], opcode(5, 9), 0x502, store(0x501, 0x80));

        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x502);
        assert_ok_eq!(zmachine.variable(0x80), 3);
    }

    #[test]
    fn test_quit() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(0x400, vec![], opcode(3, 10), 0x401);
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0);
    }

    // #[test]
    // fn test_new_line() {
    //     let map = test_map(3);
    //     let mut zmachine = mock_zmachine(map);
    //     assert!(zmachine.set_cursor(2, 1).is_ok());
    //     let i = mock_instruction(0x400, vec![], opcode(3, 11), 0x401);
    //     assert_ok_eq!(dispatch(&mut zmachine, &i), 0x401);
    //     let cursor = zmachine.cursor().unwrap();
    //     assert_eq!(cursor, (3, 1));
    // }

    #[test]
    fn test_show_status_score() {
        let mut map = test_map(3);

        // Short name: Status Object
        mock_object(
            &mut map,
            1,
            vec![0x1319, 0x1B3A, 0x6004, 0x50EF, 0xA919],
            (0, 0, 0),
        );

        // Set the object, score, and turn vars
        set_variable(&mut map, 0x10, 1);
        set_variable(&mut map, 0x11, 0xFF0A);
        set_variable(&mut map, 0x12, 4567);

        let mut zmachine = mock_zmachine(map);

        let i = mock_instruction(0x400, vec![], opcode(3, 12), 0x401);
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x401);
        assert_print!(
            " Status Object                                                         -99/4567 "
        );
    }

    #[test]
    fn test_show_status_time_am() {
        let mut map = test_map(3);

        // Set the timed game flag bit
        map[0x01] = 0x02;

        // Short name: Status Object
        mock_object(
            &mut map,
            1,
            vec![0x1319, 0x1B3A, 0x6004, 0x50EF, 0xA919],
            (0, 0, 0),
        );

        // Set the object, score, and turn vars
        set_variable(&mut map, 0x10, 1);
        set_variable(&mut map, 0x11, 0);
        set_variable(&mut map, 0x12, 0);

        let mut zmachine = mock_zmachine(map);

        let i = mock_instruction(0x400, vec![], opcode(3, 12), 0x401);
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x401);
        assert_print!(
            " Status Object                                                         12:00 AM "
        );
    }

    #[test]
    fn test_show_status_time_pm() {
        let mut map = test_map(3);

        // Set the timed game flag bit
        map[0x01] = 0x02;

        // Short name: Status Object

        mock_object(
            &mut map,
            1,
            vec![0x1319, 0x1B3A, 0x6004, 0x50EF, 0xA919],
            (0, 0, 0),
        );

        // Set the object, score, and turn vars
        set_variable(&mut map, 0x10, 1);
        set_variable(&mut map, 0x11, 12);
        set_variable(&mut map, 0x12, 0);

        let mut zmachine = mock_zmachine(map);

        let i = mock_instruction(0x400, vec![], opcode(3, 12), 0x401);
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x401);
        assert_print!(
            " Status Object                                                         12:00 PM "
        );
    }

    #[test]
    fn test_show_status_time_padding() {
        let mut map = test_map(3);

        // Set the timed game flag bit
        map[0x01] = 0x02;

        // Short name: Status Object
        mock_object(
            &mut map,
            1,
            vec![0x1319, 0x1B3A, 0x6004, 0x50EF, 0xA919],
            (0, 0, 0),
        );

        // Set the object, score, and turn vars
        set_variable(&mut map, 0x10, 1);
        set_variable(&mut map, 0x11, 1);
        set_variable(&mut map, 0x12, 59);

        let mut zmachine = mock_zmachine(map);

        let i = mock_instruction(0x400, vec![], opcode(3, 12), 0x401);
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x401);
        assert_print!(
            " Status Object                                                          1:59 AM "
        );
    }

    #[test]
    fn test_verify() {
        let mut map = test_map(3);
        // Put some data in the map
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        // Add the checksum
        map[0x1C] = 0xf6;
        map[0x1D] = 0x20;

        let mut zmachine = mock_zmachine(map);
        let i = mock_branch_instruction(
            0x400,
            vec![],
            opcode(3, 13),
            0x402,
            branch(0x401, true, 0x40a),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x40a);
    }

    #[test]
    fn test_verify_fail() {
        let mut map = test_map(3);
        // Put some data in the map
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        // Add the checksum
        map[0x1C] = 0xf6;
        map[0x1D] = 0x21;

        let mut zmachine = mock_zmachine(map);
        let i = mock_branch_instruction(
            0x400,
            vec![],
            opcode(3, 13),
            0x402,
            branch(0x401, true, 0x40a),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x402);
    }

    #[test]
    fn test_piracy() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);

        let i = mock_branch_instruction(
            0x400,
            vec![],
            opcode(3, 15),
            0x402,
            branch(0x401, true, 0x40a),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x40a);
    }

    #[test]
    fn test_piracy_fail() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);

        let i = mock_branch_instruction(
            0x400,
            vec![],
            opcode(3, 15),
            0x402,
            branch(0x401, false, 0x40a),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x402);
    }
}
