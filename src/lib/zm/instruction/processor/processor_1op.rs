use crate::{
    error::{ErrorCode, RuntimeError},
    fatal_error,
    instruction::{Instruction, InstructionResult, NextAddress},
    object::{self, property},
    text,
    zmachine::ZMachine,
};

use super::{branch, operand_values, store_result};

pub fn jz(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    InstructionResult::new(branch(zmachine, instruction, operands[0] == 0)?)
}

pub fn get_sibling(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let sibling = object::sibling(zmachine, operands[0] as usize)?;
    store_result(zmachine, instruction, sibling as u16)?;
    InstructionResult::new(branch(zmachine, instruction, sibling != 0)?)
}

pub fn get_child(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let child = object::child(zmachine, operands[0] as usize)?;
    store_result(zmachine, instruction, child as u16)?;
    InstructionResult::new(branch(zmachine, instruction, child != 0)?)
}

pub fn get_parent(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let parent = object::parent(zmachine, operands[0] as usize)?;
    store_result(zmachine, instruction, parent as u16)?;
    InstructionResult::new(NextAddress::Address(instruction.next_address()))
}

pub fn get_prop_len(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let len = property::property_length(zmachine, operands[0] as usize)?;
    store_result(zmachine, instruction, len as u16)?;
    InstructionResult::new(NextAddress::Address(instruction.next_address()))
}

pub fn inc(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let val = zmachine.peek_variable(operands[0] as u8)?;
    let new_val = i16::overflowing_add(val as i16, 1);
    zmachine.set_variable_indirect(operands[0] as u8, new_val.0 as u16)?;
    InstructionResult::new(NextAddress::Address(instruction.next_address()))
}

pub fn dec(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let val = zmachine.peek_variable(operands[0] as u8)?;
    let new_val = i16::overflowing_sub(val as i16, 1);
    zmachine.set_variable_indirect(operands[0] as u8, new_val.0 as u16)?;
    InstructionResult::new(NextAddress::Address(instruction.next_address()))
}

pub fn print_addr(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let text = text::as_text(zmachine, operands[0] as usize, false)?;

    InstructionResult::print(NextAddress::Address(instruction.next_address), text)
    // Ok(InstructionResult::new(
    //     Directive::Print,
    //     DirectiveRequest::print(&text),
    //     instruction.next_address,
    // ))
    // zmachine.print(&text)?;
    // Ok(instruction.next_address())
}

pub fn call_1s(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let address = zmachine.packed_routine_address(operands[0])?;

    InstructionResult::new(zmachine.call_routine(
        address,
        &vec![],
        instruction.store,
        instruction.next_address(),
    )?)
}

pub fn remove_obj(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let object = operands[0] as usize;
    if object > 0 {
        let parent = object::parent(zmachine, object)?;
        if parent != 0 {
            let parent_child = object::child(zmachine, parent)?;
            if parent_child == object {
                let sibling = object::sibling(zmachine, object)?;
                object::set_child(zmachine, parent, sibling)?;
            } else {
                let mut sibling = parent_child;
                while sibling != 0 && object::sibling(zmachine, sibling)? != object {
                    sibling = object::sibling(zmachine, sibling)?;
                }

                if sibling == 0 {
                    return fatal_error!(
                        ErrorCode::InvalidObjectTree,
                        "Unable to find previous sibling of removed object that is not the first child"
                    );
                }

                let o = object::sibling(zmachine, object)?;
                object::set_sibling(zmachine, sibling, o)?;
            }

            object::set_parent(zmachine, object, 0)?;
            object::set_sibling(zmachine, object, 0)?;
        }
    }

    InstructionResult::new(NextAddress::Address(instruction.next_address()))
}

pub fn print_obj(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let ztext = property::short_name(zmachine, operands[0] as usize)?;
    let text = text::from_vec(zmachine, &ztext, false)?;

    InstructionResult::print(NextAddress::Address(instruction.next_address), text)
    // zmachine.print(&text)?;
    // Ok(instruction.next_address())
}

pub fn ret(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;

    InstructionResult::new(zmachine.return_routine(operands[0])?)
}

pub fn jump(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let address = (instruction.next_address() as isize) + (operands[0] as i16) as isize - 2;
    InstructionResult::new(NextAddress::Address(address as usize))
}

pub fn print_paddr(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let address = zmachine.packed_string_address(operands[0])?;
    let text = text::as_text(zmachine, address, false)?;
    zmachine.output(&text, NextAddress::Address(instruction.next_address), false)
    // if zmachine.is_read_interrupt()? {
    //     zmachine.set_redraw_input()?;
    // }

    // InstructionResult::print(NextAddress::Address(instruction.next_address), text)
}

pub fn load(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let value = zmachine.peek_variable(operands[0] as u8)?;
    store_result(zmachine, instruction, value)?;
    InstructionResult::new(NextAddress::Address(instruction.next_address()))
}

pub fn not(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let value = !operands[0];
    store_result(zmachine, instruction, value)?;
    InstructionResult::new(NextAddress::Address(instruction.next_address()))
}

pub fn call_1n(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let address = zmachine.packed_routine_address(operands[0])?;
    InstructionResult::new(zmachine.call_routine(
        address,
        &vec![],
        None,
        instruction.next_address(),
    )?)
}

#[cfg(test)]
mod tests {
    use crate::{
        assert_ok_eq, assert_print,
        instruction::{processor::dispatch, Opcode, OpcodeForm, OperandCount, OperandType},
        object,
        test_util::*,
    };

    fn opcode(version: u8, instruction: u8) -> Opcode {
        Opcode::new(
            version,
            instruction,
            instruction,
            OpcodeForm::Short,
            OperandCount::_1OP,
        )
    }

    #[test]
    fn test_jz_true() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);

        let i = mock_branch_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 0)],
            opcode(3, 0),
            0x403,
            branch(0x402, true, 0x40a),
        );

        let a = dispatch(&mut zmachine, &i);
        assert_ok_eq!(a, 0x40a);
    }

    #[test]
    fn test_jz_false() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);

        let i = mock_branch_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 1)],
            opcode(3, 0),
            0x403,
            branch(0x402, true, 0x40a),
        );

        let a = dispatch(&mut zmachine, &i);
        assert_ok_eq!(a, 0x403);
    }

    #[test]
    fn test_get_sibling_v3_true() {
        let mut map = test_map(3);
        // Sibling
        //   4     18    E        7     12    14
        // 0 00100 11000 01110  0 00111 10010 10100
        // 130E                 1E54
        //   C     5     5
        // 1 01100 00101 00101
        // B0A5

        mock_object(&mut map, 1, vec![0x130E, 0x1E54, 0xB0A5], (4, 2, 5));
        let mut zmachine = mock_zmachine(map);
        let i = mock_branch_store_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 1)],
            opcode(3, 1),
            0x403,
            branch(0x401, true, 0x40a),
            store(0x402, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x40a);
        assert_ok_eq!(zmachine.variable(0x80), 0x02);
    }

    #[test]
    fn test_get_sibling_v3_false() {
        let mut map = test_map(3);
        set_variable(&mut map, 0x80, 0xFFFF);

        mock_object(&mut map, 1, vec![0x130E, 0x1E54, 0xB0A5], (4, 0, 5));
        let mut zmachine = mock_zmachine(map);
        let i = mock_branch_store_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 1)],
            opcode(3, 1),
            0x403,
            branch(0x401, true, 0x40a),
            store(0x402, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x403);
        assert_ok_eq!(zmachine.variable(0x80), 0x00);
    }

    #[test]
    fn test_get_sibling_v4_true() {
        let mut map = test_map(4);

        mock_object(&mut map, 1, vec![0x130E, 0x1E54, 0xB0A5], (4, 2, 5));
        let mut zmachine = mock_zmachine(map);
        let i = mock_branch_store_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 1)],
            opcode(4, 1),
            0x403,
            branch(0x401, true, 0x40a),
            store(0x402, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x40a);
        assert_ok_eq!(zmachine.variable(0x80), 0x02);
    }

    #[test]
    fn test_get_sibling_v4_false() {
        let mut map = test_map(4);
        set_variable(&mut map, 0x80, 0xFFFF);

        mock_object(&mut map, 1, vec![0x130E, 0x1E54, 0xB0A5], (4, 0, 5));
        let mut zmachine = mock_zmachine(map);
        let i = mock_branch_store_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 1)],
            opcode(4, 1),
            0x403,
            branch(0x401, true, 0x40a),
            store(0x402, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x403);
        assert_ok_eq!(zmachine.variable(0x80), 0x00);
    }

    #[test]
    fn test_get_sibling_0() {
        let mut map = test_map(3);
        mock_object(&mut map, 1, vec![0x130E, 0x1E54, 0xB0A5], (4, 2, 5));
        let mut zmachine = mock_zmachine(map);
        let i = mock_branch_store_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 0)],
            opcode(3, 1),
            0x403,
            branch(0x401, true, 0x40a),
            store(0x402, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x403);
        assert_ok_eq!(zmachine.variable(0x80), 0);
    }

    #[test]
    fn test_get_child_v3_true() {
        let mut map = test_map(3);
        // Child
        //   4     8     D        E     12    9
        // 0 00100 01000 01101  1 01110 10010 01001
        // 110D                 BA49

        mock_object(&mut map, 1, vec![0x110D, 0xBA49], (4, 2, 5));
        let mut zmachine = mock_zmachine(map);
        let i = mock_branch_store_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 1)],
            opcode(3, 2),
            0x403,
            branch(0x401, true, 0x40a),
            store(0x402, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x40a);
        assert_ok_eq!(zmachine.variable(0x80), 0x05);
    }

    #[test]
    fn test_get_child_v3_false() {
        let mut map = test_map(3);
        set_variable(&mut map, 0x80, 0xFFFF);

        mock_object(&mut map, 1, vec![0x110D, 0xBA49], (4, 2, 0));
        let mut zmachine = mock_zmachine(map);
        let i = mock_branch_store_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 1)],
            opcode(4, 2),
            0x403,
            branch(0x401, true, 0x40a),
            store(0x402, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x403);
        assert_ok_eq!(zmachine.variable(0x80), 0x00);
    }

    #[test]
    fn test_get_child_v4_true() {
        let mut map = test_map(4);

        mock_object(&mut map, 1, vec![0x110D, 0xBA49], (4, 2, 5));
        let mut zmachine = mock_zmachine(map);
        let i = mock_branch_store_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 1)],
            opcode(4, 2),
            0x403,
            branch(0x401, true, 0x40a),
            store(0x402, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x40a);
        assert_ok_eq!(zmachine.variable(0x80), 0x05);
    }

    #[test]
    fn test_get_child_v4_false() {
        let mut map = test_map(4);
        set_variable(&mut map, 0x80, 0xFFFF);

        mock_object(&mut map, 1, vec![0x110D, 0xBA49], (4, 2, 0));
        let mut zmachine = mock_zmachine(map);
        let i = mock_branch_store_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 1)],
            opcode(4, 2),
            0x403,
            branch(0x401, true, 0x40a),
            store(0x402, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x403);
        assert_ok_eq!(zmachine.variable(0x80), 0x00);
    }

    #[test]
    fn test_get_child_0() {
        let mut map = test_map(3);
        set_variable(&mut map, 0x80, 0xFFFF);

        mock_object(&mut map, 1, vec![0x110D, 0xBA49], (4, 2, 0));
        let mut zmachine = mock_zmachine(map);
        let i = mock_branch_store_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 0)],
            opcode(4, 2),
            0x403,
            branch(0x401, true, 0x40a),
            store(0x402, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x403);
        assert_ok_eq!(zmachine.variable(0x80), 0);
    }

    #[test]
    fn test_get_parent_v3() {
        let mut map = test_map(3);
        set_variable(&mut map, 0x80, 0xFFFF);
        // Parent
        //   4     15    6        17    A     13
        // 0 00100 10101 00110  0 10111 01010 10011
        // 12A6                 5D53
        //   19    5     5
        // 1 11001 00101 00101
        // E4A5

        mock_object(&mut map, 1, vec![0x12A6, 0x5E54, 0xE4A5], (4, 2, 5));
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 1)],
            opcode(3, 3),
            0x403,
            store(0x402, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x403);
        assert_ok_eq!(zmachine.variable(0x80), 0x04);
    }

    #[test]
    fn test_get_parent_v4() {
        let mut map = test_map(4);
        set_variable(&mut map, 0x80, 0xFFFF);
        // Parent
        //   4     15    6        16    A     14
        // 0 00100 10101 00110  0 10110 01010 10100
        // 12A6                 5954
        //   19    5     5
        // 1 11001 00101 00101
        // E4A5

        mock_object(&mut map, 1, vec![0x12A6, 0x5954, 0xE4A5], (4, 2, 5));
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 1)],
            opcode(4, 3),
            0x403,
            store(0x402, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x403);
        assert_ok_eq!(zmachine.variable(0x80), 0x04);
    }

    #[test]
    fn test_get_parent_0() {
        let mut map = test_map(4);
        set_variable(&mut map, 0x80, 0xFFFF);
        // Parent
        //   4     15    6        16    A     14
        // 0 00100 10101 00110  0 10110 01010 10100
        // 12A6                 5954
        //   19    5     5
        // 1 11001 00101 00101
        // E4A5

        mock_object(&mut map, 1, vec![0x12A6, 0x5954, 0xE4A5], (4, 2, 5));
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 0)],
            opcode(4, 3),
            0x403,
            store(0x402, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x403);
        assert_ok_eq!(zmachine.variable(0x80), 0);
    }

    #[test]
    fn test_get_prop_len_v3() {
        let mut map = test_map(3);
        set_variable(&mut map, 0x80, 0xFFFF);
        map[0x300] = 0x2C;
        // Object
        //   4     14    7        F     A     8
        // 0 00100 10100 00111  0 01111 01010 01011
        // 1287                 3D4B
        //   19    5     5
        // 1 11001 00101 00101
        // E4A5
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![operand(OperandType::LargeConstant, 0x301)],
            opcode(3, 4),
            0x403,
            store(0x402, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x403);
        assert_ok_eq!(zmachine.variable(0x80), 2);
    }

    #[test]
    fn test_get_prop_len_v4_short() {
        let mut map = test_map(4);
        set_variable(&mut map, 0x80, 0xFFFF);
        map[0x300] = 0x3A;
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![operand(OperandType::LargeConstant, 0x301)],
            opcode(4, 4),
            0x403,
            store(0x402, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x403);
        assert_ok_eq!(zmachine.variable(0x80), 1);
    }

    #[test]
    fn test_get_prop_len_v4_long() {
        let mut map = test_map(4);
        set_variable(&mut map, 0x80, 0xFFFF);
        map[0x300] = 0x7A;
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![operand(OperandType::LargeConstant, 0x301)],
            opcode(4, 4),
            0x403,
            store(0x402, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x403);
        assert_ok_eq!(zmachine.variable(0x80), 2);
    }

    #[test]
    fn test_get_prop_len_v4_extended() {
        let mut map = test_map(4);
        set_variable(&mut map, 0x80, 0xFFFF);
        map[0x300] = 0xBA;
        map[0x301] = 0xBF;
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![operand(OperandType::LargeConstant, 0x302)],
            opcode(4, 4),
            0x403,
            store(0x402, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x403);
        assert_ok_eq!(zmachine.variable(0x80), 63);
    }

    #[test]
    fn test_get_prop_len_v4_64() {
        let mut map = test_map(4);
        set_variable(&mut map, 0x80, 0xFFFF);
        map[0x300] = 0xBA;
        map[0x301] = 0x80;
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![operand(OperandType::LargeConstant, 0x302)],
            opcode(4, 4),
            0x403,
            store(0x402, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x403);
        assert_ok_eq!(zmachine.variable(0x80), 64);
    }

    #[test]
    fn test_get_prop_len_no_prop() {
        let mut map = test_map(3);
        set_variable(&mut map, 0x80, 0xFFFF);
        map[0x300] = 0x2C;
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![operand(OperandType::LargeConstant, 0)],
            opcode(3, 4),
            0x403,
            store(0x402, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x403);
        assert_ok_eq!(zmachine.variable(0x80), 0);
    }

    #[test]
    fn test_inc() {
        let mut map = test_map(3);
        set_variable(&mut map, 0x80, 0x1234);
        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 0x80)],
            opcode(3, 5),
            0x402,
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x402);
        assert_ok_eq!(zmachine.variable(0x80), 0x1235);
    }

    #[test]
    fn test_inc_overflow() {
        let mut map = test_map(3);
        set_variable(&mut map, 0x80, 0x7FFF);
        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 0x80)],
            opcode(3, 5),
            0x402,
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x402);
        assert_ok_eq!(zmachine.variable(0x80), 0x8000);
    }

    #[test]
    fn test_inc_sp() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        assert!(zmachine.push(0x1234).is_ok());
        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 0)],
            opcode(3, 5),
            0x402,
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x402);
        assert_ok_eq!(zmachine.variable(0), 0x1235);
        assert!(zmachine.variable(0).is_err());
    }

    #[test]
    fn test_dec() {
        let mut map = test_map(3);
        set_variable(&mut map, 0x80, 0x1234);
        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 0x80)],
            opcode(3, 6),
            0x402,
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x402);
        assert_ok_eq!(zmachine.variable(0x80), 0x1233);
    }

    #[test]
    fn test_dec_overflow() {
        let mut map = test_map(3);
        set_variable(&mut map, 0x80, 0x0000);
        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 0x80)],
            opcode(3, 6),
            0x402,
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x402);
        assert_ok_eq!(zmachine.variable(0x80), 0xFFFF);
    }

    #[test]
    fn test_dec_sp() {
        let mut map = test_map(3);
        set_variable(&mut map, 0x80, 0x7FFF);
        let mut zmachine = mock_zmachine(map);
        assert!(zmachine.push(0x1234).is_ok());
        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 0)],
            opcode(3, 6),
            0x402,
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x402);
        assert_ok_eq!(zmachine.variable(0), 0x1233);
        assert!(zmachine.variable(0).is_err());
    }

    #[test]
    fn test_print_addr() {
        let mut map = test_map(3);
        // Hello
        map[0x600] = 0x11;
        map[0x601] = 0xaa;
        map[0x602] = 0xc6;
        map[0x603] = 0x34;

        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::LargeConstant, 0x600)],
            opcode(3, 7),
            0x403,
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x403);
        assert_print!("Hello");
    }

    #[test]
    fn test_call_1s_v4() {
        let mut map = test_map(4);
        mock_routine(&mut map, 0x600, &[0x1234, 0x5678, 0x9abc, 0xdef0]);

        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![operand(OperandType::LargeConstant, 0x180)],
            opcode(4, 8),
            0x404,
            store(0x403, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x609);
        assert!(zmachine.variable(0).is_err());
        assert_ok_eq!(zmachine.variable(1), 0x1234);
        assert_ok_eq!(zmachine.variable(2), 0x5678);
        assert_ok_eq!(zmachine.variable(3), 0x9abc);
        assert_ok_eq!(zmachine.variable(4), 0xdef0);
        assert!(zmachine.variable(5).is_err());
        assert_ok_eq!(zmachine.return_routine(0xF0AD), 0x404);
        assert_ok_eq!(zmachine.variable(0x80), 0xF0AD);
    }

    #[test]
    fn test_call_1s_v5() {
        let mut map = test_map(5);
        mock_routine(&mut map, 0x600, &[0x1234, 0x5678, 0x9abc, 0xdef0]);

        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![operand(OperandType::LargeConstant, 0x180)],
            opcode(5, 8),
            0x404,
            store(0x403, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x601);
        assert!(zmachine.variable(0).is_err());
        assert_ok_eq!(zmachine.variable(1), 0);
        assert_ok_eq!(zmachine.variable(2), 0);
        assert_ok_eq!(zmachine.variable(3), 0);
        assert_ok_eq!(zmachine.variable(4), 0);
        assert!(zmachine.variable(5).is_err());
        assert_ok_eq!(zmachine.return_routine(0xF0AD), 0x404);
        assert_ok_eq!(zmachine.variable(0x80), 0xF0AD);
    }

    #[test]
    fn test_call_1s_v8() {
        let mut map = test_map(8);
        mock_routine(&mut map, 0x600, &[0x1234, 0x5678, 0x9abc, 0xdef0]);

        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![operand(OperandType::LargeConstant, 0xC0)],
            opcode(8, 8),
            0x404,
            store(0x403, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x601);
        assert!(zmachine.variable(0).is_err());
        assert_ok_eq!(zmachine.variable(1), 0);
        assert_ok_eq!(zmachine.variable(2), 0);
        assert_ok_eq!(zmachine.variable(3), 0);
        assert_ok_eq!(zmachine.variable(4), 0);
        assert!(zmachine.variable(5).is_err());
        assert_ok_eq!(zmachine.return_routine(0xF0AD), 0x404);
        assert_ok_eq!(zmachine.variable(0x80), 0xF0AD);
    }

    #[test]
    fn test_remove_obj_v3_first_child() {
        let mut map = test_map(3);
        mock_object(&mut map, 1, vec![0x12A6, 0x5954, 0xE4A5], (4, 5, 2));
        mock_object(&mut map, 2, vec![0x110D, 0xBA49], (1, 3, 5));
        mock_object(&mut map, 3, vec![0x130E, 0x1E54, 0xB0A5], (1, 6, 7));

        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 2)],
            opcode(3, 9),
            0x402,
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x402);
        assert_ok_eq!(object::child(&zmachine, 1), 3);
        assert_ok_eq!(object::parent(&zmachine, 2), 0);
        assert_ok_eq!(object::sibling(&zmachine, 2), 0);
        assert_ok_eq!(object::child(&zmachine, 2), 5);
        assert_ok_eq!(object::parent(&zmachine, 3), 1);
        assert_ok_eq!(object::sibling(&zmachine, 3), 6);
        assert_ok_eq!(object::child(&zmachine, 3), 7);
    }

    #[test]
    fn test_remove_obj_v3_middle_child() {
        let mut map = test_map(3);
        mock_object(&mut map, 1, vec![0x12A6, 0x5954, 0xE4A5], (4, 5, 2));
        mock_object(&mut map, 2, vec![0x110D, 0xBA49], (1, 3, 5));
        mock_object(&mut map, 3, vec![0x130E, 0x1E54, 0xB0A5], (1, 6, 7));
        mock_object(&mut map, 6, vec![0x1287, 0x3D4B, 0xE4A5], (1, 8, 9));

        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 3)],
            opcode(3, 9),
            0x402,
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x402);
        assert_ok_eq!(object::child(&zmachine, 1), 2);
        assert_ok_eq!(object::parent(&zmachine, 2), 1);
        assert_ok_eq!(object::sibling(&zmachine, 2), 6);
        assert_ok_eq!(object::child(&zmachine, 2), 5);
        assert_ok_eq!(object::parent(&zmachine, 3), 0);
        assert_ok_eq!(object::sibling(&zmachine, 3), 0);
        assert_ok_eq!(object::child(&zmachine, 3), 7);
        assert_ok_eq!(object::parent(&zmachine, 6), 1);
        assert_ok_eq!(object::sibling(&zmachine, 6), 8);
        assert_ok_eq!(object::child(&zmachine, 6), 9);
    }

    #[test]
    fn test_remove_obj_v3_last_child() {
        let mut map = test_map(3);
        mock_object(&mut map, 1, vec![0x12A6, 0x5954, 0xE4A5], (4, 5, 2));
        mock_object(&mut map, 2, vec![0x110D, 0xBA49], (1, 3, 5));
        mock_object(&mut map, 3, vec![0x130E, 0x1E54, 0xB0A5], (1, 6, 7));

        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 3)],
            opcode(3, 9),
            0x402,
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x402);
        assert_ok_eq!(object::child(&zmachine, 1), 2);
        assert_ok_eq!(object::parent(&zmachine, 2), 1);
        assert_ok_eq!(object::sibling(&zmachine, 2), 6);
        assert_ok_eq!(object::child(&zmachine, 2), 5);
        assert_ok_eq!(object::parent(&zmachine, 3), 0);
        assert_ok_eq!(object::sibling(&zmachine, 3), 0);
        assert_ok_eq!(object::child(&zmachine, 3), 7);
    }

    #[test]
    fn test_remove_obj_no_parent() {
        let mut map = test_map(3);
        mock_object(&mut map, 1, vec![0x12A6, 0x5954, 0xE4A5], (4, 5, 2));
        mock_object(&mut map, 2, vec![0x110D, 0xBA49], (1, 0, 5));
        mock_object(&mut map, 3, vec![0x130E, 0x1E54, 0xB0A5], (0, 6, 7));

        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 3)],
            opcode(3, 9),
            0x402,
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x402);
        assert_ok_eq!(object::child(&zmachine, 1), 2);
        assert_ok_eq!(object::parent(&zmachine, 2), 1);
        assert_ok_eq!(object::sibling(&zmachine, 2), 0);
        assert_ok_eq!(object::child(&zmachine, 2), 5);
        assert_ok_eq!(object::parent(&zmachine, 3), 0);
        assert_ok_eq!(object::sibling(&zmachine, 3), 6);
        assert_ok_eq!(object::child(&zmachine, 3), 7);
    }

    #[test]
    fn test_remove_obj_object_0() {
        let mut map = test_map(3);
        mock_object(&mut map, 1, vec![0x12A6, 0x5954, 0xE4A5], (4, 5, 2));
        mock_object(&mut map, 2, vec![0x110D, 0xBA49], (1, 3, 5));
        mock_object(&mut map, 3, vec![0x130E, 0x1E54, 0xB0A5], (1, 6, 7));

        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 0)],
            opcode(3, 9),
            0x402,
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x402);
        assert_ok_eq!(object::child(&zmachine, 1), 2);
        assert_ok_eq!(object::parent(&zmachine, 2), 1);
        assert_ok_eq!(object::sibling(&zmachine, 2), 3);
        assert_ok_eq!(object::child(&zmachine, 2), 5);
        assert_ok_eq!(object::parent(&zmachine, 3), 1);
        assert_ok_eq!(object::sibling(&zmachine, 3), 6);
        assert_ok_eq!(object::child(&zmachine, 3), 7);
    }

    #[test]
    fn test_remove_obj_v4_first_child() {
        let mut map = test_map(4);
        mock_object(&mut map, 1, vec![0x12A6, 0x5954, 0xE4A5], (4, 5, 2));
        mock_object(&mut map, 2, vec![0x110D, 0xBA49], (1, 3, 5));
        mock_object(&mut map, 3, vec![0x130E, 0x1E54, 0xB0A5], (1, 6, 7));

        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 2)],
            opcode(4, 9),
            0x402,
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x402);
        assert_ok_eq!(object::child(&zmachine, 1), 3);
        assert_ok_eq!(object::parent(&zmachine, 2), 0);
        assert_ok_eq!(object::sibling(&zmachine, 2), 0);
        assert_ok_eq!(object::child(&zmachine, 2), 5);
        assert_ok_eq!(object::parent(&zmachine, 3), 1);
        assert_ok_eq!(object::sibling(&zmachine, 3), 6);
        assert_ok_eq!(object::child(&zmachine, 3), 7);
    }

    #[test]
    fn test_remove_obj_v4_middle_child() {
        let mut map = test_map(4);
        mock_object(&mut map, 1, vec![0x12A6, 0x5954, 0xE4A5], (4, 5, 2));
        mock_object(&mut map, 2, vec![0x110D, 0xBA49], (1, 3, 5));
        mock_object(&mut map, 3, vec![0x130E, 0x1E54, 0xB0A5], (1, 6, 7));
        mock_object(&mut map, 6, vec![0x1287, 0x3D4B, 0xE4A5], (1, 8, 9));

        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 3)],
            opcode(4, 9),
            0x402,
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x402);
        assert_ok_eq!(object::child(&zmachine, 1), 2);
        assert_ok_eq!(object::parent(&zmachine, 2), 1);
        assert_ok_eq!(object::sibling(&zmachine, 2), 6);
        assert_ok_eq!(object::child(&zmachine, 2), 5);
        assert_ok_eq!(object::parent(&zmachine, 3), 0);
        assert_ok_eq!(object::sibling(&zmachine, 3), 0);
        assert_ok_eq!(object::child(&zmachine, 3), 7);
        assert_ok_eq!(object::parent(&zmachine, 6), 1);
        assert_ok_eq!(object::sibling(&zmachine, 6), 8);
        assert_ok_eq!(object::child(&zmachine, 6), 9);
    }

    #[test]
    fn test_remove_obj_v4_last_child() {
        let mut map = test_map(4);
        mock_object(&mut map, 1, vec![0x12A6, 0x5954, 0xE4A5], (4, 5, 2));
        mock_object(&mut map, 2, vec![0x110D, 0xBA49], (1, 3, 5));
        mock_object(&mut map, 3, vec![0x130E, 0x1E54, 0xB0A5], (1, 6, 7));

        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 3)],
            opcode(4, 9),
            0x402,
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x402);
        assert_ok_eq!(object::child(&zmachine, 1), 2);
        assert_ok_eq!(object::parent(&zmachine, 2), 1);
        assert_ok_eq!(object::sibling(&zmachine, 2), 6);
        assert_ok_eq!(object::child(&zmachine, 2), 5);
        assert_ok_eq!(object::parent(&zmachine, 3), 0);
        assert_ok_eq!(object::sibling(&zmachine, 3), 0);
        assert_ok_eq!(object::child(&zmachine, 3), 7);
    }

    #[test]
    fn test_remove_obj_invalid_tree() {
        let mut map = test_map(3);
        mock_object(&mut map, 1, vec![0x12A6, 0x5954, 0xE4A5], (4, 5, 2));
        mock_object(&mut map, 2, vec![0x110D, 0xBA49], (1, 0, 5));
        mock_object(&mut map, 3, vec![0x130E, 0x1E54, 0xB0A5], (1, 6, 7));

        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 3)],
            opcode(3, 9),
            0x402,
        );
        assert!(dispatch(&mut zmachine, &i).is_err());
    }

    #[test]
    fn test_print_obj_v3() {
        let mut map = test_map(3);
        mock_object(&mut map, 1, vec![0x12A6, 0x5D53, 0xE4A5], (4, 5, 2));

        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 1)],
            opcode(3, 10),
            0x402,
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x402);
        assert_print!("Parent");
    }

    #[test]
    fn test_print_obj_v4() {
        let mut map = test_map(4);
        mock_object(&mut map, 1, vec![0x12A6, 0x5D53, 0xE4A5], (4, 5, 2));

        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 1)],
            opcode(4, 10),
            0x402,
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x402);
        assert_print!("Parent");
    }

    #[test]
    fn test_ret_store() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        assert!(zmachine.set_variable(0, 0x1234).is_ok());
        mock_frame(&mut zmachine, 0x500, Some(0x80), 0x400);
        let i = mock_instruction(
            0x501,
            vec![operand(OperandType::LargeConstant, 0x5678)],
            opcode(3, 11),
            0x501,
        );
        assert!(zmachine.peek_variable(0).is_err());
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x400);
        assert_ok_eq!(zmachine.variable(0x80), 0x5678);
        assert_ok_eq!(zmachine.peek_variable(0), 0x1234);
    }

    #[test]
    fn test_ret_no_store() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        assert!(zmachine.set_variable(0, 0x1234).is_ok());
        mock_frame(&mut zmachine, 0x500, None, 0x400);
        let i = mock_instruction(
            0x501,
            vec![operand(OperandType::LargeConstant, 0x5678)],
            opcode(3, 11),
            0x501,
        );
        assert!(zmachine.peek_variable(0).is_err());
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x400);
        assert_ok_eq!(zmachine.variable(0x80), 0);
        assert_ok_eq!(zmachine.peek_variable(0), 0x1234);
    }

    #[test]
    fn test_jump_forward() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x401,
            vec![operand(OperandType::LargeConstant, 0xFF)],
            opcode(3, 12),
            0x404,
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x501);
    }

    #[test]
    fn test_jump_backward() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x401,
            vec![operand(OperandType::LargeConstant, 0xFEFF)],
            opcode(3, 12),
            0x404,
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x301);
    }

    #[test]
    fn test_print_paddr_v3() {
        let mut map = test_map(3);
        // Hello
        map[0x600] = 0x11;
        map[0x601] = 0xaa;
        map[0x602] = 0xc6;
        map[0x603] = 0x34;

        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::LargeConstant, 0x300)],
            opcode(3, 13),
            0x403,
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x403);
        assert_print!("Hello");
    }

    #[test]
    fn test_print_paddr_v4() {
        let mut map = test_map(4);
        // Hello
        map[0x600] = 0x11;
        map[0x601] = 0xaa;
        map[0x602] = 0xc6;
        map[0x603] = 0x34;

        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::LargeConstant, 0x180)],
            opcode(4, 13),
            0x403,
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x403);
        assert_print!("Hello");
    }

    #[test]
    fn test_print_paddr_v8() {
        let mut map = test_map(8);
        // Hello
        map[0x600] = 0x11;
        map[0x601] = 0xaa;
        map[0x602] = 0xc6;
        map[0x603] = 0x34;

        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 0xC0)],
            opcode(8, 13),
            0x403,
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x403);
        assert_print!("Hello");
    }

    #[test]
    fn test_load() {
        let mut map = test_map(3);
        set_variable(&mut map, 0x80, 0x1234);
        set_variable(&mut map, 0x81, 0x5678);
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 0x81)],
            opcode(3, 14),
            0x403,
            store(0x402, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x403);
        assert_ok_eq!(zmachine.variable(0x80), 0x5678);
        assert_ok_eq!(zmachine.variable(0x81), 0x5678);
    }

    #[test]
    pub fn test_not() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![operand(OperandType::LargeConstant, 0xF0A5)],
            opcode(3, 15),
            0x404,
            store(0x403, 0x080),
        );

        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x404);
        assert_ok_eq!(zmachine.variable(0x80), 0x0F5A);
    }

    #[test]
    pub fn test_call_1n_v5() {
        let mut map = test_map(5);
        mock_routine(&mut map, 0x600, &[0x1234, 0x5678, 0x9abc, 0xdef0]);

        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::LargeConstant, 0x180)],
            opcode(5, 15),
            0x404,
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x601);
        assert!(zmachine.variable(0).is_err());
        assert_ok_eq!(zmachine.variable(1), 0);
        assert_ok_eq!(zmachine.variable(2), 0);
        assert_ok_eq!(zmachine.variable(3), 0);
        assert_ok_eq!(zmachine.variable(4), 0);
        assert!(zmachine.variable(5).is_err());
        assert_ok_eq!(zmachine.return_routine(0xF0AD), 0x404);
        assert_ok_eq!(zmachine.variable(0x80), 0);
    }

    #[test]
    pub fn test_call_1n_v8() {
        let mut map = test_map(8);
        mock_routine(&mut map, 0x600, &[0x1234, 0x5678, 0x9abc, 0xdef0]);

        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::LargeConstant, 0xC0)],
            opcode(8, 15),
            0x404,
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x601);
        assert!(zmachine.variable(0).is_err());
        assert_ok_eq!(zmachine.variable(1), 0);
        assert_ok_eq!(zmachine.variable(2), 0);
        assert_ok_eq!(zmachine.variable(3), 0);
        assert_ok_eq!(zmachine.variable(4), 0);
        assert!(zmachine.variable(5).is_err());
        assert_ok_eq!(zmachine.return_routine(0xF0AD), 0x404);
        assert_ok_eq!(zmachine.variable(0x80), 0);
    }
}
