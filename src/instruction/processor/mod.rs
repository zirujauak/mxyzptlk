use crate::error::*;
use crate::zmachine::ZMachine;

use super::*;

mod processor_0op;
mod processor_1op;
mod processor_2op;
mod processor_ext;
mod processor_var;

fn operand_value(zmachine: &mut ZMachine, operand: &Operand) -> Result<u16, RuntimeError> {
    match operand.operand_type() {
        OperandType::SmallConstant | OperandType::LargeConstant => Ok(operand.value()),
        OperandType::Variable => zmachine.variable(operand.value() as u8),
    }
}

fn operand_values(
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
        info!(target: "app::instruction", "{}", l);
    }
    Ok(v)
}

fn branch(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
    condition: bool,
) -> Result<usize, RuntimeError> {
    match instruction.branch() {
        Some(b) => {
            if condition == b.condition() {
                match b.branch_address {
                    0 => zmachine.return_routine(0), // return false
                    1 => zmachine.return_routine(1), // return true,
                    _ => Ok(b.branch_address()),
                }
            } else {
                Ok(instruction.next_address())
            }
        }
        None => Ok(instruction.next_address()),
    }
}

fn store_result(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
    value: u16,
) -> Result<(), RuntimeError> {
    match instruction.store() {
        Some(s) => zmachine.set_variable(s.variable, value),
        None => Ok(()),
    }
}

fn call_fn(
    zmachine: &mut ZMachine,
    address: usize,
    return_addr: usize,
    arguments: &Vec<u16>,
    result: Option<StoreResult>,
) -> Result<usize, RuntimeError> {
    match address {
        0 | 1 => {
            if let Some(r) = result {
                zmachine.set_variable(r.variable, address as u16)?
            }

            Ok(return_addr)
        }
        _ => zmachine.call_routine(address, arguments, result, return_addr),
    }
}

pub fn dispatch(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    info!(target: "app::instruction", "dispatch: {}", instruction);
    match instruction.opcode().form() {
        OpcodeForm::Ext => match instruction.opcode().instruction() {
            0x00 => processor_ext::save(zmachine, instruction),
            0x01 => processor_ext::restore(zmachine, instruction),
            0x02 => processor_ext::log_shift(zmachine, instruction),
            0x03 => processor_ext::art_shift(zmachine, instruction),
            0x04 => processor_ext::set_font(zmachine, instruction),
            //         0x05 => processor_ext::draw_picture(context, instruction),
            //         0x06 => processor_ext::picture_data(context, instruction),
            //         0x07 => processor_ext::erase_picture(context, instruction),
            //         0x08 => processor_ext::set_margins(context, instruction),
            0x09 => processor_ext::save_undo(zmachine, instruction),
            0x0a => processor_ext::restore_undo(zmachine, instruction),
            //         0x0b => processor_ext::print_unicode(context, instruction),
            //         0x0c => processor_ext::check_unicode(context, instruction),
            //         0x0d => processor_ext::set_true_colour(context, instruction),
            //         0x10 => processor_ext::move_window(context, instruction),
            //         0x11 => processor_ext::window_size(context, instruction),
            //         0x12 => processor_ext::window_style(context, instruction),
            //         0x13 => processor_ext::get_wind_prop(context, instruction),
            //         0x14 => processor_ext::scroll_window(context, instruction),
            //         0x15 => processor_ext::pop_stack(context, instruction),
            //         0x16 => processor_ext::read_mouse(context, instruction),
            //         0x17 => processor_ext::mouse_window(context, instruction),
            //         0x18 => processor_ext::push_stack(context, instruction),
            //         0x19 => processor_ext::put_wind_prop(context, instruction),
            //         0x1a => processor_ext::print_form(context, instruction),
            //         0x1b => processor_ext::make_menu(context, instruction),
            //         0x1c => processor_ext::picture_table(context, instruction),
            //         0x1d => processor_ext::buffer_screen(context, instruction),
            _ => Err(RuntimeError::new(
                ErrorCode::UnimplementedInstruction,
                format!("Unimplemented EXT instruction: {}", instruction.opcode()),
            )),
        },
        _ => match instruction.opcode().operand_count() {
            OperandCount::_0OP => match instruction.opcode().instruction() {
                0x0 => processor_0op::rtrue(zmachine, instruction),
                0x1 => processor_0op::rfalse(zmachine, instruction),
                0x2 => processor_0op::print(zmachine, instruction),
                0x3 => processor_0op::print_ret(zmachine, instruction),
                0x5 => processor_0op::save(zmachine, instruction),
                0x6 => processor_0op::restore(zmachine, instruction),
                0x7 => processor_0op::restart(zmachine, instruction),
                0x8 => processor_0op::ret_popped(zmachine, instruction),
                0x9 => {
                    if zmachine.version() < 5 {
                        processor_0op::pop(zmachine, instruction)
                    } else {
                        processor_0op::catch(zmachine, instruction)
                    }
                }
                0xa => processor_0op::quit(zmachine, instruction),
                0xb => processor_0op::new_line(zmachine, instruction),
                0xc => processor_0op::show_status(zmachine, instruction),
                0xd => processor_0op::verify(zmachine, instruction),
                0xf => processor_0op::piracy(zmachine, instruction),
                _ => Err(RuntimeError::new(
                    ErrorCode::UnimplementedInstruction,
                    format!("Unimplemented instruction: {}", instruction.opcode()),
                )),
            },
            OperandCount::_1OP => match instruction.opcode().instruction() {
                0x0 => processor_1op::jz(zmachine, instruction),
                0x1 => processor_1op::get_sibling(zmachine, instruction),
                0x2 => processor_1op::get_child(zmachine, instruction),
                0x3 => processor_1op::get_parent(zmachine, instruction),
                0x4 => processor_1op::get_prop_len(zmachine, instruction),
                0x5 => processor_1op::inc(zmachine, instruction),
                0x6 => processor_1op::dec(zmachine, instruction),
                0x7 => processor_1op::print_addr(zmachine, instruction),
                0x8 => processor_1op::call_1s(zmachine, instruction),
                0x9 => processor_1op::remove_obj(zmachine, instruction),
                0xa => processor_1op::print_obj(zmachine, instruction),
                0xb => processor_1op::ret(zmachine, instruction),
                0xc => processor_1op::jump(zmachine, instruction),
                0xd => processor_1op::print_paddr(zmachine, instruction),
                0xe => processor_1op::load(zmachine, instruction),
                0xf => {
                    if zmachine.version() < 5 {
                        processor_1op::not(zmachine, instruction)
                    } else {
                        processor_1op::call_1n(zmachine, instruction)
                    }
                }
                _ => Err(RuntimeError::new(
                    ErrorCode::UnimplementedInstruction,
                    format!("Unimplemented instruction: {}", instruction.opcode()),
                )),
            },
            OperandCount::_2OP => match instruction.opcode().instruction() {
                0x01 => processor_2op::je(zmachine, instruction),
                0x02 => processor_2op::jl(zmachine, instruction),
                0x03 => processor_2op::jg(zmachine, instruction),
                0x04 => processor_2op::dec_chk(zmachine, instruction),
                0x05 => processor_2op::inc_chk(zmachine, instruction),
                0x06 => processor_2op::jin(zmachine, instruction),
                0x07 => processor_2op::test(zmachine, instruction),
                0x08 => processor_2op::or(zmachine, instruction),
                0x09 => processor_2op::and(zmachine, instruction),
                0x0a => processor_2op::test_attr(zmachine, instruction),
                0x0b => processor_2op::set_attr(zmachine, instruction),
                0x0c => processor_2op::clear_attr(zmachine, instruction),
                0x0d => processor_2op::store(zmachine, instruction),
                0x0e => processor_2op::insert_obj(zmachine, instruction),
                0x0f => processor_2op::loadw(zmachine, instruction),
                0x10 => processor_2op::loadb(zmachine, instruction),
                0x11 => processor_2op::get_prop(zmachine, instruction),
                0x12 => processor_2op::get_prop_addr(zmachine, instruction),
                0x13 => processor_2op::get_next_prop(zmachine, instruction),
                0x14 => processor_2op::add(zmachine, instruction),
                0x15 => processor_2op::sub(zmachine, instruction),
                0x16 => processor_2op::mul(zmachine, instruction),
                0x17 => processor_2op::div(zmachine, instruction),
                0x18 => processor_2op::modulus(zmachine, instruction),
                0x19 => processor_2op::call_2s(zmachine, instruction),
                0x1a => processor_2op::call_2n(zmachine, instruction),
                0x1b => processor_2op::set_colour(zmachine, instruction),
                0x1c => processor_2op::throw(zmachine, instruction),
                _ => Err(RuntimeError::new(
                    ErrorCode::UnimplementedInstruction,
                    format!("Unimplemented instruction: {}", instruction.opcode()),
                )),
            },
            OperandCount::_VAR => match instruction.opcode().instruction() {
                0x00 => processor_var::call_vs(zmachine, instruction),
                0x01 => processor_var::storew(zmachine, instruction),
                0x02 => processor_var::storeb(zmachine, instruction),
                0x03 => processor_var::put_prop(zmachine, instruction),
                0x04 => processor_var::read(zmachine, instruction),
                0x05 => processor_var::print_char(zmachine, instruction),
                0x06 => processor_var::print_num(zmachine, instruction),
                0x07 => processor_var::random(zmachine, instruction),
                0x08 => processor_var::push(zmachine, instruction),
                0x09 => processor_var::pull(zmachine, instruction),
                0x0a => processor_var::split_window(zmachine, instruction),
                0x0b => processor_var::set_window(zmachine, instruction),
                0x0c => processor_var::call_vs2(zmachine, instruction),
                0x0d => processor_var::erase_window(zmachine, instruction),
                0x0e => processor_var::erase_line(zmachine, instruction),
                0x0f => processor_var::set_cursor(zmachine, instruction),
                0x11 => processor_var::set_text_style(zmachine, instruction),
                0x12 => processor_var::buffer_mode(zmachine, instruction),
                0x13 => processor_var::output_stream(zmachine, instruction),
                0x14 => processor_var::input_stream(zmachine, instruction),
                0x15 => processor_var::sound_effect(zmachine, instruction),
                0x16 => processor_var::read_char(zmachine, instruction),
                0x17 => processor_var::scan_table(zmachine, instruction),
                0x18 => processor_var::not(zmachine, instruction),
                0x19 => processor_var::call_vn(zmachine, instruction),
                0x1a => processor_var::call_vn2(zmachine, instruction),
                0x1b => processor_var::tokenise(zmachine, instruction),
                0x1c => processor_var::encode_text(zmachine, instruction),
                0x1d => processor_var::copy_table(zmachine, instruction),
                0x1e => processor_var::print_table(zmachine, instruction),
                0x1f => processor_var::check_arg_count(zmachine, instruction),
                _ => Err(RuntimeError::new(
                    ErrorCode::UnimplementedInstruction,
                    format!("Unimplemented instruction: {}", instruction.opcode()),
                )),
            },
        },
    }
}
