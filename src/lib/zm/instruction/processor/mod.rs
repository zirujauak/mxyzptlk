use crate::zmachine::ZMachine;
use crate::{error::*, fatal_error};

use super::*;

pub mod processor_0op;
mod processor_1op;
mod processor_2op;
pub mod processor_ext;
pub mod processor_var;

fn operand_value(zmachine: &mut ZMachine, operand: &Operand) -> Result<u16, RuntimeError> {
    match operand.operand_type() {
        OperandType::SmallConstant | OperandType::LargeConstant => Ok(operand.value()),
        OperandType::Variable => zmachine.variable(operand.value() as u8),
    }
}

pub fn operand_values(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<Vec<u16>, RuntimeError> {
    let mut v = Vec::new();
    let mut l = "Operand values: ".to_string();
    for o in instruction.operands() {
        let value = operand_value(zmachine, o)?;
        match o.operand_type {
            OperandType::SmallConstant => l.push_str(&format!(" #{:02x}", value as u8)),
            _ => l.push_str(&format!(" #{:04x}", value)),
        }
        v.push(value)
    }
    if !v.is_empty() {
        debug!(target: "app::instruction", "{}", l);
    }
    Ok(v)
}

pub fn branch(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
    condition: bool,
) -> Result<NextAddress, RuntimeError> {
    match instruction.branch() {
        Some(b) => {
            if condition == b.condition() {
                match b.branch_address {
                    0 => zmachine.return_routine(0), // return false
                    1 => zmachine.return_routine(1), // return true,
                    _ => Ok(NextAddress::Address(b.branch_address())),
                }
            } else {
                Ok(NextAddress::Address(instruction.next_address()))
            }
        }
        None => Ok(NextAddress::Address(instruction.next_address())),
    }
}

fn store_result(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
    value: u16,
) -> Result<(), RuntimeError> {
    match instruction.store() {
        Some(s) => zmachine.set_variable(s.variable(), value),
        None => Ok(()),
    }
}

fn call_fn(
    zmachine: &mut ZMachine,
    address: usize,
    return_addr: usize,
    arguments: &Vec<u16>,
    result: Option<StoreResult>,
) -> Result<NextAddress, RuntimeError> {
    match address {
        0 | 1 => {
            if let Some(r) = result {
                zmachine.set_variable(r.variable(), address as u16)?
            }

            Ok(NextAddress::Address(return_addr))
        }
        _ => zmachine.call_routine(address, arguments, result, return_addr),
    }
}

pub fn dispatch(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    debug!(target: "app::instruction", "dispatch: {}", instruction);
    match instruction.opcode().form() {
        OpcodeForm::Ext => match (zmachine.version(), instruction.opcode().instruction()) {
            // V6 opcodes have been omitted
            (5, 0x00) | (7, 0x00) | (8, 0x00) => processor_ext::save_pre(zmachine, instruction),
            (5, 0x01) | (7, 0x01) | (8, 0x01) => processor_ext::restore_pre(zmachine, instruction),
            (5, 0x02) | (7, 0x02) | (8, 0x02) => processor_ext::log_shift(zmachine, instruction),
            (5, 0x03) | (7, 0x03) | (8, 0x03) => processor_ext::art_shift(zmachine, instruction),
            (5, 0x04) | (7, 0x04) | (8, 0x04) => processor_ext::set_font_pre(zmachine, instruction),
            (5, 0x09) | (7, 0x09) | (8, 0x09) => processor_ext::save_undo(zmachine, instruction),
            (5, 0x0a) | (7, 0x0a) | (8, 0x0a) => processor_ext::restore_undo(zmachine, instruction),
            //         (5, 0x0b) | (7, 0x0b) | (8, 0x0b) => processor_ext::print_unicode(context, instruction),
            //         (5, 0x0c) | (7, 0x0c) | (8, 0x0c) => processor_ext::check_unicode(context, instruction),
            //         (5, 0x0d) | (7, 0x0d) | (8, 0x0d) => processor_ext::set_true_colour(context, instruction),
            (_, _) => fatal_error!(
                ErrorCode::UnimplementedInstruction,
                "Unimplemented EXT instruction: {}",
                instruction.opcode()
            ),
        },
        _ => match instruction.opcode().operand_count() {
            OperandCount::_0OP => match (zmachine.version(), instruction.opcode().instruction()) {
                (_, 0x0) => processor_0op::rtrue(zmachine, instruction),
                (_, 0x1) => processor_0op::rfalse(zmachine, instruction),
                (_, 0x2) => processor_0op::print(zmachine, instruction),
                (_, 0x3) => processor_0op::print_ret(zmachine, instruction),
                (_, 0x4) => processor_0op::nop(zmachine, instruction),
                (3, 0x5) | (4, 0x5) => processor_0op::save_pre(zmachine, instruction),
                (3, 0x6) | (4, 0x6) => processor_0op::restore_pre(zmachine, instruction),
                (_, 0x7) => processor_0op::restart(zmachine, instruction),
                (_, 0x8) => processor_0op::ret_popped(zmachine, instruction),
                (3, 0x9) | (4, 0x9) => processor_0op::pop(zmachine, instruction),
                (_, 0x9) => processor_0op::catch(zmachine, instruction),
                (_, 0xa) => processor_0op::quit(zmachine, instruction),
                (_, 0xb) => processor_0op::new_line(zmachine, instruction),
                (3, 0xc) => processor_0op::show_status(zmachine, instruction),
                (_, 0xd) => processor_0op::verify(zmachine, instruction),
                (_, 0xf) => processor_0op::piracy(zmachine, instruction),
                (_, _) => fatal_error!(
                    ErrorCode::UnimplementedInstruction,
                    "Unimplemented instruction: {}",
                    instruction.opcode()
                ),
            },
            OperandCount::_1OP => match (zmachine.version(), instruction.opcode().instruction()) {
                (_, 0x0) => processor_1op::jz(zmachine, instruction),
                (_, 0x1) => processor_1op::get_sibling(zmachine, instruction),
                (_, 0x2) => processor_1op::get_child(zmachine, instruction),
                (_, 0x3) => processor_1op::get_parent(zmachine, instruction),
                (_, 0x4) => processor_1op::get_prop_len(zmachine, instruction),
                (_, 0x5) => processor_1op::inc(zmachine, instruction),
                (_, 0x6) => processor_1op::dec(zmachine, instruction),
                (_, 0x7) => processor_1op::print_addr(zmachine, instruction),
                (4, 0x8) | (5, 0x8) | (7, 0x8) | (8, 0x8) => {
                    processor_1op::call_1s(zmachine, instruction)
                }
                (_, 0x9) => processor_1op::remove_obj(zmachine, instruction),
                (_, 0xa) => processor_1op::print_obj(zmachine, instruction),
                (_, 0xb) => processor_1op::ret(zmachine, instruction),
                (_, 0xc) => processor_1op::jump(zmachine, instruction),
                (_, 0xd) => processor_1op::print_paddr(zmachine, instruction),
                (_, 0xe) => processor_1op::load(zmachine, instruction),
                (3, 0xf) | (4, 0xf) => processor_1op::not(zmachine, instruction),
                (_, 0xf) => processor_1op::call_1n(zmachine, instruction),
                (_, _) => fatal_error!(
                    ErrorCode::UnimplementedInstruction,
                    "Unimplemented instruction: {}",
                    instruction.opcode()
                ),
            },
            OperandCount::_2OP => match (zmachine.version(), instruction.opcode().instruction()) {
                (_, 0x01) => processor_2op::je(zmachine, instruction),
                (_, 0x02) => processor_2op::jl(zmachine, instruction),
                (_, 0x03) => processor_2op::jg(zmachine, instruction),
                (_, 0x04) => processor_2op::dec_chk(zmachine, instruction),
                (_, 0x05) => processor_2op::inc_chk(zmachine, instruction),
                (_, 0x06) => processor_2op::jin(zmachine, instruction),
                (_, 0x07) => processor_2op::test(zmachine, instruction),
                (_, 0x08) => processor_2op::or(zmachine, instruction),
                (_, 0x09) => processor_2op::and(zmachine, instruction),
                (_, 0x0a) => processor_2op::test_attr(zmachine, instruction),
                (_, 0x0b) => processor_2op::set_attr(zmachine, instruction),
                (_, 0x0c) => processor_2op::clear_attr(zmachine, instruction),
                (_, 0x0d) => processor_2op::store(zmachine, instruction),
                (_, 0x0e) => processor_2op::insert_obj(zmachine, instruction),
                (_, 0x0f) => processor_2op::loadw(zmachine, instruction),
                (_, 0x10) => processor_2op::loadb(zmachine, instruction),
                (_, 0x11) => processor_2op::get_prop(zmachine, instruction),
                (_, 0x12) => processor_2op::get_prop_addr(zmachine, instruction),
                (_, 0x13) => processor_2op::get_next_prop(zmachine, instruction),
                (_, 0x14) => processor_2op::add(zmachine, instruction),
                (_, 0x15) => processor_2op::sub(zmachine, instruction),
                (_, 0x16) => processor_2op::mul(zmachine, instruction),
                (_, 0x17) => processor_2op::div(zmachine, instruction),
                (_, 0x18) => processor_2op::modulus(zmachine, instruction),
                (4, 0x19) | (5, 0x19) | (7, 0x19) | (8, 0x19) => {
                    processor_2op::call_2s(zmachine, instruction)
                }
                (5, 0x1a) | (7, 0x1a) | (8, 0x1a) => processor_2op::call_2n(zmachine, instruction),
                (5, 0x1b) | (7, 0x1b) | (8, 0x1b) => {
                    processor_2op::set_colour(zmachine, instruction)
                }
                (5, 0x1c) | (7, 0x1c) | (8, 0x1c) => processor_2op::throw(zmachine, instruction),
                (_, _) => fatal_error!(
                    ErrorCode::UnimplementedInstruction,
                    "Unimplemented instruction: {}",
                    instruction.opcode()
                ),
            },
            OperandCount::_VAR => match (zmachine.version(), instruction.opcode().instruction()) {
                (_, 0x00) => processor_var::call_vs(zmachine, instruction),
                (_, 0x01) => processor_var::storew(zmachine, instruction),
                (_, 0x02) => processor_var::storeb(zmachine, instruction),
                (_, 0x03) => processor_var::put_prop(zmachine, instruction),
                (_, 0x04) => processor_var::read_pre(zmachine, instruction),
                (_, 0x05) => processor_var::print_char(zmachine, instruction),
                (_, 0x06) => processor_var::print_num(zmachine, instruction),
                (_, 0x07) => processor_var::random(zmachine, instruction),
                (_, 0x08) => processor_var::push(zmachine, instruction),
                (_, 0x09) => processor_var::pull(zmachine, instruction),
                (_, 0x0a) => processor_var::split_window(zmachine, instruction),
                (_, 0x0b) => processor_var::set_window(zmachine, instruction),
                (4, 0x0c) | (5, 0x0c) | (7, 0x0c) | (8, 0x0c) => {
                    processor_var::call_vs2(zmachine, instruction)
                }
                (4, 0x0d) | (5, 0x0d) | (7, 0x0d) | (8, 0x0d) => {
                    processor_var::erase_window(zmachine, instruction)
                }
                (4, 0x0e) | (5, 0x0e) | (7, 0x0e) | (8, 0x0e) => {
                    processor_var::erase_line(zmachine, instruction)
                }
                (4, 0x0f) | (5, 0x0f) | (7, 0x0f) | (8, 0x0f) => {
                    processor_var::set_cursor(zmachine, instruction)
                }
                (4, 0x10) | (5, 0x10) | (7, 0x10) | (8, 0x10) => {
                    processor_var::get_cursor_pre(zmachine, instruction)
                }
                (4, 0x11) | (5, 0x11) | (7, 0x11) | (8, 0x11) => {
                    processor_var::set_text_style(zmachine, instruction)
                }
                (4, 0x12) | (5, 0x12) | (7, 0x12) | (8, 0x12) => {
                    processor_var::buffer_mode(zmachine, instruction)
                }
                (_, 0x13) => processor_var::output_stream(zmachine, instruction),
                (_, 0x14) => processor_var::input_stream(zmachine, instruction),
                (_, 0x15) => processor_var::sound_effect_pre(zmachine, instruction),
                (4, 0x16) | (5, 0x16) | (7, 0x16) | (8, 0x16) => {
                    processor_var::read_char_pre(zmachine, instruction)
                }
                (4, 0x17) | (5, 0x17) | (7, 0x17) | (8, 0x17) => {
                    processor_var::scan_table(zmachine, instruction)
                }
                (5, 0x18) | (7, 0x18) | (8, 0x18) => processor_var::not(zmachine, instruction),
                (5, 0x19) | (7, 0x19) | (8, 0x19) => processor_var::call_vn(zmachine, instruction),
                (5, 0x1a) | (7, 0x1a) | (8, 0x1a) => processor_var::call_vn2(zmachine, instruction),
                (5, 0x1b) | (7, 0x1b) | (8, 0x1b) => processor_var::tokenise(zmachine, instruction),
                (5, 0x1c) | (7, 0x1c) | (8, 0x1c) => {
                    processor_var::encode_text(zmachine, instruction)
                }
                (5, 0x1d) | (7, 0x1d) | (8, 0x1d) => {
                    processor_var::copy_table(zmachine, instruction)
                }
                (5, 0x1e) | (7, 0x1e) | (8, 0x1e) => {
                    processor_var::print_table(zmachine, instruction)
                }
                (5, 0x1f) | (7, 0x1f) | (8, 0x1f) => {
                    processor_var::check_arg_count(zmachine, instruction)
                }
                (_, _) => fatal_error!(
                    ErrorCode::UnimplementedInstruction,
                    "Unimplemented instruction: {}",
                    instruction.opcode()
                ),
            },
        },
    }
}

#[cfg(test)]
pub mod tests {
    use crate::{assert_ok, assert_ok_eq, test_util::*};

    use super::*;

    #[test]
    fn test_operand_value() {
        // Set up a simple memory map with global var 0x80 set
        let mut v = test_map(5);
        set_variable(&mut v, 0x80, 0x789A);
        let mut zmachine = mock_zmachine(v);

        let o_small_constant = Operand::new(OperandType::SmallConstant, 0x12);
        let o_large_constant = Operand::new(OperandType::LargeConstant, 0x3456);
        let o_variable = Operand::new(OperandType::Variable, 0x80);
        assert_ok_eq!(operand_value(&mut zmachine, &o_small_constant), 0x12);
        assert_ok_eq!(operand_value(&mut zmachine, &o_large_constant), 0x3456);
        assert_ok_eq!(operand_value(&mut zmachine, &o_variable), 0x789A);
    }

    #[test]
    fn test_operand_values() {
        let mut v = test_map(5);
        set_variable(&mut v, 0x80, 0x789A);
        set_variable(&mut v, 0x81, 0x1357);
        let mut zmachine = mock_zmachine(v);
        let o_small_constant = Operand::new(OperandType::SmallConstant, 0x12);
        let o_large_constant = Operand::new(OperandType::LargeConstant, 0x3456);
        let o_variable1 = Operand::new(OperandType::Variable, 0x80);
        let o_variable2 = Operand::new(OperandType::Variable, 0x81);
        let i = mock_instruction(
            0x480,
            vec![o_variable1, o_large_constant, o_small_constant, o_variable2],
            Opcode::new(5, 0, 0, OpcodeForm::Var, OperandCount::_VAR),
            0,
        );

        let operands = assert_ok!(operand_values(&mut zmachine, &i));
        assert_eq!(operands[0], 0x789A);
        assert_eq!(operands[1], 0x3456);
        assert_eq!(operands[2], 0x12);
        assert_eq!(operands[3], 0x1357);

        let i = mock_instruction(
            0x480,
            vec![],
            Opcode::new(5, 0, 0, OpcodeForm::Var, OperandCount::_VAR),
            0,
        );
        assert!(operand_values(&mut zmachine, &i).is_ok_and(|x| x.is_empty()));
    }

    #[test]
    fn test_branch_rfalse() {
        let mut v = test_map(5);
        set_variable(&mut v, 0x80, 0xFF);
        let mut zmachine = mock_zmachine(v);
        mock_frame(&mut zmachine, 0x500, Some(0x80), 0x480);
        let i = mock_branch(true, 0, 0x502);
        assert_ok_eq!(processor::branch(&mut zmachine, &i, true), 0x480);
        assert_ok_eq!(zmachine.variable(0x80), 0);
    }

    #[test]
    fn test_branch_rfalse_no_branch() {
        let mut v = test_map(5);
        set_variable(&mut v, 0x80, 0xFF);
        let mut zmachine = mock_zmachine(v);
        mock_frame(&mut zmachine, 0x500, Some(0x80), 0x480);
        let i = mock_branch(false, 0, 0x502);
        assert_ok_eq!(processor::branch(&mut zmachine, &i, true), 0x502);
        assert_ok_eq!(zmachine.variable(0x80), 0xFF);
    }

    #[test]
    fn test_branch_rtrue() {
        let mut v = test_map(5);
        set_variable(&mut v, 0x80, 0xFF);
        let mut zmachine = mock_zmachine(v);
        mock_frame(&mut zmachine, 0x500, Some(0x80), 0x480);
        let i = mock_branch(true, 1, 0x502);
        assert_ok_eq!(processor::branch(&mut zmachine, &i, true), 0x480);
        assert_ok_eq!(zmachine.variable(0x80), 1);
    }

    #[test]
    fn test_branch_rtrue_no_branch() {
        let mut v = test_map(5);
        set_variable(&mut v, 0x80, 0xFF);
        let mut zmachine = mock_zmachine(v);
        mock_frame(&mut zmachine, 0x500, Some(0x80), 0x480);
        let i = mock_branch(false, 1, 0x502);
        assert_ok_eq!(processor::branch(&mut zmachine, &i, true), 0x502);
        assert_ok_eq!(zmachine.variable(0x80), 0xFF);
    }

    #[test]
    fn test_branch() {
        let v = test_map(5);
        let mut zmachine = mock_zmachine(v);
        let i = mock_branch(true, 0x500, 0x482);
        assert_ok_eq!(processor::branch(&mut zmachine, &i, true), 0x500);
    }

    #[test]
    fn test_branch_no_branch() {
        let v = test_map(5);
        let mut zmachine = mock_zmachine(v);
        let i = mock_branch(true, 0x500, 0x482);
        assert_ok_eq!(processor::branch(&mut zmachine, &i, false), 0x482);
    }

    // #[test]
    // fn test_branch_not_a_branch_instruction() {
    //     let v = test_map(5);
    //     let mut zmachine = mock_zmachine(v);
    //     let i = mock_instruction(
    //         0x480,
    //         vec![],
    //         Opcode::new(5, 0, 0, OpcodeForm::Var, OperandCount::_VAR),
    //         0x482,
    //     );
    //     let a = processor::branch(&mut zmachine, &i, false);
    //     assert!(a.is_ok());
    //     assert_eq!(a.unwrap(), 0x482);
    // }

    #[test]
    fn test_store_result() {
        let mut v = test_map(5);
        set_variable(&mut v, 0x80, 0xFF);
        let mut zmachine = mock_zmachine(v);
        let i = mock_store_result(Some(0x80), 0x482);
        let a = store_result(&mut zmachine, &i, 0x12);
        assert!(a.is_ok());
        let v = zmachine.variable(0x80);
        assert!(v.is_ok());
        assert_eq!(v.unwrap(), 0x12);
    }

    #[test]
    fn test_store_result_no_result() {
        let mut v = test_map(5);
        set_variable(&mut v, 0x80, 0xFF);
        let mut zmachine = mock_zmachine(v);
        let i = mock_store_result(None, 0x482);
        let a = store_result(&mut zmachine, &i, 0x12);
        assert!(a.is_ok());
        let v = zmachine.variable(0x80);
        assert!(v.is_ok());
        assert_eq!(v.unwrap(), 0xFF);
    }

    #[test]
    fn test_store_result_not_a_store_result_instruction() {
        let mut v = test_map(5);
        set_variable(&mut v, 0x80, 0xFF);
        let mut zmachine = mock_zmachine(v);
        let i = mock_instruction(
            0x480,
            vec![],
            Opcode::new(5, 0, 0, OpcodeForm::Var, OperandCount::_VAR),
            0x482,
        );
        let a = store_result(&mut zmachine, &i, 0x12);
        assert!(a.is_ok());
    }

    // #[test]
    // fn test_call_fn_rfalse() {
    //     let mut v = test_map(5);
    //     set_variable(&mut v, 0x80, 0xFF);
    //     let mut zmachine = mock_zmachine(v);
    //     let a = call_fn(
    //         &mut zmachine,
    //         0,
    //         0x482,
    //         &vec![],
    //         Some(StoreResult::new(0, 0x80)),
    //     );
    //     assert!(a.is_ok());
    //     assert_eq!(a.unwrap(), 0x482);
    //     let v = zmachine.variable(0x80);
    //     assert!(v.is_ok());
    //     assert_eq!(v.unwrap(), 0x00);
    // }

    // #[test]
    // fn test_call_fn_rtrue() {
    //     let mut v = test_map(5);
    //     set_variable(&mut v, 0x80, 0xFF);
    //     let mut zmachine = mock_zmachine(v);
    //     let a = call_fn(
    //         &mut zmachine,
    //         1,
    //         0x482,
    //         &vec![],
    //         Some(StoreResult::new(0, 0x80)),
    //     );
    //     assert!(a.is_ok());
    //     assert_eq!(a.unwrap(), 0x482);
    //     let v = zmachine.variable(0x80);
    //     assert!(v.is_ok());
    //     assert_eq!(v.unwrap(), 0x01);
    // }

    // #[test]
    // fn test_call_fn() {
    //     let v = test_map(5);
    //     let mut zmachine = mock_zmachine(v);
    //     assert_eq!(zmachine.frame_count(), 1);
    //     let a = call_fn(
    //         &mut zmachine,
    //         0x500,
    //         0x482,
    //         &vec![],
    //         Some(StoreResult::new(0, 0x80)),
    //     );
    //     assert!(a.is_ok());
    //     assert_eq!(a.unwrap(), 0x501);
    //     assert_eq!(zmachine.frame_count(), 2);
    // }
}
