// use super::{Instruction, Operand, OperandType, StoreResult, OperandCount, OpcodeForm};
// use crate::executor::context::{error::ContextError, header, Context};

use crate::error::*;
use crate::state::header;
use crate::state::header::*;
use crate::state::instruction::*;
use crate::state::memory::Memory;
use crate::state::State;

mod processor_0op;
mod processor_1op;
mod processor_2op;
mod processor_ext;
mod processor_var;

fn operand_value(state: &mut State, operand: &Operand) -> Result<u16, RuntimeError> {
    match operand.operand_type() {
        OperandType::SmallConstant | OperandType::LargeConstant => Ok(operand.value()),
        OperandType::Variable => state.variable(operand.value() as u8),
    }
}

fn operand_values(state: &mut State, instruction: &Instruction) -> Result<Vec<u16>, RuntimeError> {
    let mut v = Vec::new();
    let mut s = "Operands: ".to_string();
    for o in instruction.operands() {
        let value = operand_value(state, &o);
        match value {
            Ok(val) => {
                match o.operand_type() {
                    OperandType::SmallConstant => s.push_str(&format!(" {:#02x}", val)),
                    _ => s.push_str(&format!(" {:#04x}", val)),
                };
                v.push(val)
            }
            Err(e) => return Err(e),
        }
    }

    info!(target: "app::instruction", "{}", s);
    Ok(v)
}

fn branch(
    state: &mut State,
    instruction: &Instruction,
    condition: bool,
) -> Result<usize, RuntimeError> {
    match instruction.branch() {
        Some(b) => {
            if condition == b.condition {
                match b.branch_address {
                    0 => state.return_routine(0), // return false
                    1 => state.return_routine(1), // return true,
                    _ => Ok(b.branch_address),
                }
            } else {
                Ok(instruction.next_address())
            }
        }
        None => Ok(instruction.next_address()),
    }
}

fn store_result(
    state: &mut State,
    instruction: &Instruction,
    value: u16,
) -> Result<(), RuntimeError> {
    match instruction.store() {
        Some(s) => state.set_variable(s.variable, value),
        None => Ok(()),
    }
}

fn call_fn(
    state: &mut State,
    address: usize,
    return_addr: usize,
    arguments: &Vec<u16>,
    result: Option<StoreResult>,
) -> Result<usize, RuntimeError> {
    match address {
        0 | 1 => {
            match result {
                Some(r) => match state.set_variable(r.variable, address as u16) {
                    Ok(_) => (),
                    Err(e) => return Err(e),
                },
                None => (),
            }

            Ok(return_addr)
        }
        _ => state.call_routine(address, arguments, result, return_addr),
    }
}

fn packed_routine_address(memory: &Memory, address: u16) -> Result<usize, RuntimeError> {
    let version = header::field_byte(memory, HeaderField::Version)?;
    match version {
        1 | 2 | 3 => Ok(address as usize * 2),
        4 | 5 => Ok(address as usize * 4),
        7 => Ok((address as usize * 4)
            + (header::field_word(memory, HeaderField::RoutinesOffset)? as usize * 8)),
        8 => Ok(address as usize * 8),
        _ => Err(RuntimeError::new(
            ErrorCode::UnsupportedVersion,
            format!("Unsupported version: {}", version),
        )),
    }
}

fn packed_string_address(memory: &Memory, address: u16) -> Result<usize, RuntimeError> {
    let version = header::field_byte(memory, HeaderField::Version)?;
    match version {
        1 | 2 | 3 => Ok(address as usize * 2),
        4 | 5 => Ok(address as usize * 4),
        7 => Ok((address as usize * 4)
            + (header::field_word(memory, HeaderField::StringsOffset)? as usize * 8)),
        8 => Ok(address as usize * 8),
        // TODO: error
        _ => Err(RuntimeError::new(
            ErrorCode::UnsupportedVersion,
            format!("Unsupported version: {}", version),
        )),
    }
}

pub fn dispatch(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    info!(target: "app::instruction", "{}", instruction);
    let version = header::field_byte(state.memory(), HeaderField::Version)?;
    match instruction.opcode().form() {
        OpcodeForm::Ext => match instruction.opcode().instruction() {
            0x00 => processor_ext::save(state, instruction),
            0x01 => processor_ext::restore(state, instruction),
            0x02 => processor_ext::log_shift(state, instruction),
            0x03 => processor_ext::art_shift(state, instruction),
            // 0x04 => processor_ext::set_font(context, instruction),
            //         0x05 => processor_ext::draw_picture(context, instruction),
            //         0x06 => processor_ext::picture_data(context, instruction),
            //         0x07 => processor_ext::erase_picture(context, instruction),
            //         0x08 => processor_ext::set_margins(context, instruction),
            0x09 => processor_ext::save_undo(state, instruction),
            0x0a => processor_ext::restore_undo(state, instruction),
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
                0x0 => processor_0op::rtrue(state, instruction),
                0x1 => processor_0op::rfalse(state, instruction),
                0x2 => processor_0op::print(state, instruction),
                0x3 => processor_0op::print_ret(state, instruction),
                0x5 => processor_0op::save(state, instruction),
                0x6 => processor_0op::restore(state, instruction),
                0x7 => processor_0op::restart(state, instruction),
                0x8 => processor_0op::ret_popped(state, instruction),
                //             0x9 => if context.version() < 5 {
                //                 processor_0op::pop(context, instruction)
                //             } else {
                //                 processor_0op::catch(context, instruction)
                //             },
                0xa => processor_0op::quit(state, instruction),
                0xb => processor_0op::new_line(state, instruction),
                0xc => processor_0op::show_status(state, instruction),
                //             0xd => processor_0op::verify(context, instruction),
                0xf => processor_0op::piracy(state, instruction),
                _ => Err(RuntimeError::new(
                    ErrorCode::UnimplementedInstruction,
                    format!("Unimplemented instruction: {}", instruction.opcode()),
                )),
            },
            OperandCount::_1OP => match instruction.opcode().instruction() {
                0x0 => processor_1op::jz(state, instruction),
                0x1 => processor_1op::get_sibling(state, instruction),
                0x2 => processor_1op::get_child(state, instruction),
                0x3 => processor_1op::get_parent(state, instruction),
                0x4 => processor_1op::get_prop_len(state, instruction),
                0x5 => processor_1op::inc(state, instruction),
                0x6 => processor_1op::dec(state, instruction),
                0x7 => processor_1op::print_addr(state, instruction),
                0x8 => processor_1op::call_1s(state, instruction),
                0x9 => processor_1op::remove_obj(state, instruction),
                0xa => processor_1op::print_obj(state, instruction),
                0xb => processor_1op::ret(state, instruction),
                0xc => processor_1op::jump(state, instruction),
                0xd => processor_1op::print_paddr(state, instruction),
                0xe => processor_1op::load(state, instruction),
                0xf => {
                    if version < 5 {
                        processor_1op::not(state, instruction)
                    } else {
                        processor_1op::call_1n(state, instruction)
                    }
                }
                _ => Err(RuntimeError::new(
                    ErrorCode::UnimplementedInstruction,
                    format!("Unimplemented instruction: {}", instruction.opcode()),
                )),
            },
            OperandCount::_2OP => match instruction.opcode().instruction() {
                0x01 => processor_2op::je(state, instruction),
                0x02 => processor_2op::jl(state, instruction),
                0x03 => processor_2op::jg(state, instruction),
                0x04 => processor_2op::dec_chk(state, instruction),
                0x05 => processor_2op::inc_chk(state, instruction),
                0x06 => processor_2op::jin(state, instruction),
                0x07 => processor_2op::test(state, instruction),
                0x08 => processor_2op::or(state, instruction),
                0x09 => processor_2op::and(state, instruction),
                0x0a => processor_2op::test_attr(state, instruction),
                0x0b => processor_2op::set_attr(state, instruction),
                0x0c => processor_2op::clear_attr(state, instruction),
                0x0d => processor_2op::store(state, instruction),
                0x0e => processor_2op::insert_obj(state, instruction),
                0x0f => processor_2op::loadw(state, instruction),
                0x10 => processor_2op::loadb(state, instruction),
                0x11 => processor_2op::get_prop(state, instruction),
                0x12 => processor_2op::get_prop_addr(state, instruction),
                0x13 => processor_2op::get_next_prop(state, instruction),
                0x14 => processor_2op::add(state, instruction),
                0x15 => processor_2op::sub(state, instruction),
                0x16 => processor_2op::mul(state, instruction),
                0x17 => processor_2op::div(state, instruction),
                0x18 => processor_2op::modulus(state, instruction),
                0x19 => processor_2op::call_2s(state, instruction),
                0x1a => processor_2op::call_2n(state, instruction),
                //             0x1b => processor_2op::set_colour(context, instruction),
                //             0x1c => processor_2op::throw(context, instruction),
                _ => Err(RuntimeError::new(
                    ErrorCode::UnimplementedInstruction,
                    format!("Unimplemented instruction: {}", instruction.opcode()),
                )),
            },
            OperandCount::_VAR => match instruction.opcode().instruction() {
                0x00 => processor_var::call_vs(state, instruction),
                0x01 => processor_var::storew(state, instruction),
                0x02 => processor_var::storeb(state, instruction),
                0x03 => processor_var::put_prop(state, instruction),
                0x04 => processor_var::read(state, instruction),
                0x05 => processor_var::print_char(state, instruction),
                0x06 => processor_var::print_num(state, instruction),
                0x07 => processor_var::random(state, instruction),
                0x08 => processor_var::push(state, instruction),
                0x09 => processor_var::pull(state, instruction),
                0x0a => processor_var::split_window(state, instruction),
                0x0b => processor_var::set_window(state, instruction),
                0x0c => processor_var::call_vs2(state, instruction),
                0x0d => processor_var::erase_window(state, instruction),
                //             0x0e => processor_var::erase_line(context, instruction),
                0x0f => processor_var::set_cursor(state, instruction),
                0x11 => processor_var::set_text_style(state, instruction),
                0x12 => processor_var::buffer_mode(state, instruction),
                0x13 => processor_var::output_stream(state, instruction),
                //             0x14 => processor_var::input_stream(context, instruction),
                0x15 => processor_var::sound_effect(state, instruction),
                0x16 => processor_var::read_char(state, instruction),
                0x17 => processor_var::scan_table(state, instruction),
                0x18 => processor_var::not(state, instruction),
                0x19 => processor_var::call_vn(state, instruction),
                //             0x1a => processor_var::call_vn2(context, instruction),
                //             0x1b => processor_var::tokenise(context, instruction),
                //             0x1c => processor_var::encode_text(context, instruction),
                //             0x1d => processor_var::copy_table(context, instruction),
                //             0x1e => processor_var::print_table(context, instruction),
                0x1f => processor_var::check_arg_count(state, instruction),
                _ => Err(RuntimeError::new(
                    ErrorCode::UnimplementedInstruction,
                    format!("Unimplemented instruction: {}", instruction.opcode()),
                )),
            },
        },
    }
}
