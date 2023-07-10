use crate::error::{ErrorCode, RuntimeError};
use crate::instruction::{decoder, Instruction};
use crate::text;
use crate::zmachine::state::header::HeaderField;
use crate::zmachine::ZMachine;

use super::branch;
use super::store_result;

pub fn rtrue(zmachine: &mut ZMachine, _instruction: &Instruction) -> Result<usize, RuntimeError> {
    zmachine.return_routine(1)
}

pub fn rfalse(zmachine: &mut ZMachine, _instruction: &Instruction) -> Result<usize, RuntimeError> {
    zmachine.return_routine(0)
}

pub fn print(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let ztext = zmachine.string_literal(instruction.address() + 1)?;
    let text = text::from_vec(zmachine, &ztext)?;
    zmachine.print(&text)?;
    Ok(instruction.next_address() + (ztext.len() * 2))
}

pub fn print_ret(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    let ztext = zmachine.string_literal(instruction.address + 1)?;
    let text = text::from_vec(zmachine, &ztext)?;

    zmachine.print(&text)?;
    zmachine.new_line()?;

    zmachine.return_routine(1)
}

pub fn nop(_zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    Ok(instruction.next_address())
}

fn save_result(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
    success: bool,
) -> Result<usize, RuntimeError> {
    if zmachine.version() == 3 {
        branch(zmachine, instruction, success)
    } else {
        store_result(zmachine, instruction, if success { 1 } else { 0 })?;
        Ok(instruction.next_address())
    }
}

pub fn save(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let pc = if zmachine.version() == 3 {
        match instruction.branch() {
            Some(b) => b.address(),
            None => {
                return Err(RuntimeError::new(
                    ErrorCode::Save,
                    "V3 SAVE should be a branch instruction".to_string(),
                ))
            }
        }
    } else {
        match instruction.store() {
            Some(r) => r.address,
            None => {
                return Err(RuntimeError::new(
                    ErrorCode::Save,
                    "V4 SAVE should be a store instruction".to_string(),
                ))
            }
        }
    };

    match zmachine.save(pc) {
        Ok(_) => save_result(zmachine, instruction, true),
        Err(_) => save_result(zmachine, instruction, false),
    }
}

pub fn restore(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    match zmachine.restore() {
        Ok(address) => {
            match address {
                Some(a) => {
                    let i = decoder::decode_instruction(zmachine, a - 1)?;
                    if zmachine.version() == 3 {
                        // V3 is a branch
                        branch(zmachine, &i, true)
                    } else {
                        // V4 is a store
                        store_result(zmachine, instruction, 2)?;
                        Ok(i.next_address())
                    }
                }
                None => {
                    if zmachine.version() == 3 {
                        branch(zmachine, instruction, false)
                    } else {
                        store_result(zmachine, instruction, 0)?;
                        Ok(instruction.next_address())
                    }
                }
            }
        }
        Err(e) => {
            zmachine.print_str(format!("Error reading: {}\r", e))?;
            if zmachine.version() == 3 {
                branch(zmachine, instruction, false)
            } else {
                store_result(zmachine, instruction, 0)?;
                Ok(instruction.next_address())
            }
        }
    }
}

pub fn restart(zmachine: &mut ZMachine, _instruction: &Instruction) -> Result<usize, RuntimeError> {
    zmachine.restart()
}

pub fn ret_popped(
    zmachine: &mut ZMachine,
    _instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    let value = zmachine.variable(0)?;
    zmachine.return_routine(value)
}

pub fn pop(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    zmachine.variable(0)?;
    Ok(instruction.next_address())
}

pub fn catch(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let depth = zmachine.frame_count();
    store_result(zmachine, instruction, depth as u16)?;
    Ok(instruction.next_address())
}

pub fn quit(zmachine: &mut ZMachine, _instruction: &Instruction) -> Result<usize, RuntimeError> {
    zmachine.quit()?;
    Ok(0)
}

pub fn new_line(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    zmachine.new_line()?;
    // context.new_line();
    Ok(instruction.next_address())
}

pub fn show_status(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    zmachine.status_line()?;
    Ok(instruction.next_address())
}

pub fn verify(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let expected = zmachine.header_word(HeaderField::Checksum)?;
    let checksum = zmachine.checksum()?;

    branch(zmachine, instruction, expected == checksum)
}

pub fn piracy(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    branch(zmachine, instruction, true)
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use crate::instruction::{
        processor::{dispatch, tests::*},
        Opcode, OpcodeForm, OperandCount,
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
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x482));
        assert_eq!(zmachine.frame_count(), 1);
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0x01));
    }

    #[test]
    fn test_rfalse() {
        let mut v = test_map(5);
        set_variable(&mut v, 0x80, 0xFF);
        let mut zmachine = mock_zmachine(v);
        mock_frame(&mut zmachine, 0x500, Some(0x80), 0x482);
        assert_eq!(zmachine.frame_count(), 2);
        let i = mock_instruction(0x500, vec![], opcode(3, 1), 0x501);
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x482));
        assert_eq!(zmachine.frame_count(), 1);
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0x00));
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
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x485));
        assert_print("Hello");
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
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x482));
        assert_eq!(zmachine.frame_count(), 1);
        assert_print("Hello");
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0x01));
    }

    #[test]
    fn test_nop() {
        let v = test_map(3);
        let mut zmachine = mock_zmachine(v);
        let i = mock_instruction(0x400, vec![], opcode(3, 4), 0x401);
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x401));
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
        assert!(a.is_ok_and(|x| x == 0x484));
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
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x482));
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
        assert!(a.is_ok_and(|x| x == 0x483));

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
        assert!(a.is_ok_and(|x| x == 0x489));
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
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x482));
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
        assert!(a.is_ok_and(|x| x == 0x483));
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0x01));
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
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x483));
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0x00));
    }

    #[test]
    fn test_save_v4_bad_instruction() {
        let mut map = test_map(4);
        set_variable(&mut map, 0x80, 0xFF);

        let i = mock_instruction(0x480, vec![], opcode(4, 5), 0x483);
        let mut zmachine = mock_zmachine(map);
        assert!(dispatch(&mut zmachine, &i).is_err());
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0xFF));
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
        assert!(a.is_ok_and(|x| x == 0x482));
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0x01));

        let i2 = mock_store_instruction(0x484, vec![], opcode(4, 6), 0x486, store(0x485, 0x81));
        input(&[
            '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}', '\u{8}',
            '\u{8}', '\u{8}', '\u{8}', 't', 'e', 's', 't', '-', 'v', '4', 'r', '.', 'i', 'f', 'z',
            's',
        ]);

        let a = dispatch(&mut zmachine, &i2);
        assert!(fs::remove_file(Path::new("test-v4r.ifzs")).is_ok());
        assert!(a.is_ok_and(|x| x == 0x482));
        assert!(zmachine.variable(0x81).is_ok_and(|x| x == 0x02));
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
        assert!(a.is_ok_and(|x| x == 0x482));
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0x01));

        let i2 = mock_store_instruction(0x484, vec![], opcode(4, 6), 0x486, store(0x485, 0x81));

        assert!(dispatch(&mut zmachine, &i2).is_ok_and(|x| x == 0x486));
        assert!(zmachine.variable(0x81).is_ok_and(|x| x == 0x00));
    }

    #[test]
    fn test_restart() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);

        let i = mock_instruction(0x480, vec![], opcode(3, 7), 0x481);

        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x400));
    }

    #[test]
    fn test_ret_popped() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        mock_frame(&mut zmachine, 0x500, Some(0x80), 0x402);
        assert!(zmachine.push(0x1122).is_ok());
        assert!(zmachine.push(0x3344).is_ok());

        let i = mock_instruction(0x501, vec![], opcode(3, 8), 0x502);

        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x402));
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0x3344));
        assert!(zmachine.variable(0).is_err());
    }

    #[test]
    fn test_pop() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        assert!(zmachine.push(0x1122).is_ok());
        assert!(zmachine.push(0x3344).is_ok());

        let i = mock_instruction(0x501, vec![], opcode(3, 9), 0x502);

        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x502));
        assert!(zmachine.peek_variable(0).is_ok_and(|x| x == 0x1122));
    }

    #[test]
    fn test_catch() {
        let map = test_map(5);
        let mut zmachine = mock_zmachine(map);
        mock_frame(&mut zmachine, 0x480, None, 0x404);
        mock_frame(&mut zmachine, 0x500, None, 0x404);
        let i = mock_store_instruction(0x500, vec![], opcode(5, 9), 0x502, store(0x501, 0x80));

        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x502));
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 3));
    }

    #[test]
    fn test_quit() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(0x400, vec![], opcode(3, 10), 0x401);
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0));
    }

    #[test]
    fn test_new_line() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        assert!(zmachine.set_cursor(1, 1).is_ok());
        let i = mock_instruction(0x400, vec![], opcode(3, 11), 0x401);
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x401));
        let cursor = zmachine.cursor().unwrap();
        assert_eq!(cursor, (2, 1));
    }

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
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x401));
        assert_print(
            " Status Object                                                         -246/4567",
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
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x401));
        assert_print(
            " Status Object                                                          12:00 AM",
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
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x401));
        assert_print(
            " Status Object                                                          12:00 PM",
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
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x401));
        assert_print(
            " Status Object                                                           1:59 AM",
        );
    }

    #[test]
    fn test_verify() {
        let mut map = test_map(3);
        // Put some data in the map
        for i in 0x40..0x800 {
            map[i] = i as u8;
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
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x40a));
    }

    #[test]
    fn test_verify_fail() {
        let mut map = test_map(3);
        // Put some data in the map
        for i in 0x40..0x800 {
            map[i] = i as u8;
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
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x402));
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
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x40a));
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
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x402));
    }
}
