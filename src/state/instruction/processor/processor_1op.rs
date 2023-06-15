use super::*;
use crate::state::object;
use crate::state::object::property;
use crate::state::text;
use crate::state::State;

pub fn jz(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    branch(state, instruction, operands[0] == 0)
}

pub fn get_sibling(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    let sibling = object::sibling(state, operands[0] as usize)?;
    store_result(state, instruction, sibling as u16)?;
    branch(state, instruction, sibling != 0)
}

pub fn get_child(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    let child = object::child(state, operands[0] as usize)?;
    store_result(state, instruction, child as u16)?;
    branch(state, instruction, child != 0)
}

pub fn get_parent(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    let parent = object::parent(state, operands[0] as usize)?;
    store_result(state, instruction, parent as u16)?;
    Ok(instruction.next_address())
}

pub fn get_prop_len(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    let len = property::property_length(state, operands[0] as usize)?;
    store_result(state, instruction, len as u16)?;
    Ok(instruction.next_address())
}

pub fn inc(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    let val = state.peek_variable(operands[0] as u8)?;
    let new_val = i16::overflowing_add(val as i16, 1);
    state.set_variable_indirect(operands[0] as u8, new_val.0 as u16)?;
    Ok(instruction.next_address())
}

pub fn dec(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    let val = state.peek_variable(operands[0] as u8)?;
    let new_val = i16::overflowing_sub(val as i16, 1);
    state.set_variable_indirect(operands[0] as u8, new_val.0 as u16)?;
    Ok(instruction.next_address())
}

// pub fn print_addr(
//     context: &mut Context,
//     instruction: &Instruction,
// ) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     let text = text::as_text(context, operands[0] as usize)?;

//     context.print_string(text);
//     Ok(instruction.next_address())
// }

// pub fn call_1s(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     let address = packed_routine_address(context, operands[0]);

//     call_fn(context, address, instruction.next_address(), &vec![], instruction.store())
// }

// pub fn remove_obj(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     let object = operands[0] as usize;
//     if object > 0 {
//         let parent = object::parent(context, object)?;
//         if parent != 0 {
//             let parent_child = object::child(context, parent)?;
//             if parent_child == object {
//                 let sibling = object::sibling(context, object)?;
//                 object::set_child(context, parent, sibling)?;
//             } else {
//                 let mut sibling = parent_child;
//                 while sibling != 0 && object::sibling(context, sibling)? != object {
//                     sibling = object::sibling(context, sibling)?;
//                 }

//                 if sibling == 0 {
//                     panic!("Inconsistent object tree");
//                 }

//                 object::set_sibling(context, sibling, object::sibling(context, object)?)?;
//             }

//             object::set_parent(context, object, 0)?;
//             object::set_sibling(context, object, 0)?;
//         }
//     }

//     Ok(instruction.next_address())
// }

pub fn print_obj(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    let ztext = property::short_name(state, operands[0] as usize)?;
    let text = text::from_vec(state, &ztext)?;
    state.print(&text)?;
    // context.print_string(text);
    Ok(instruction.next_address())
}

pub fn ret(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    state.return_routine(operands[0])
}

pub fn jump(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    let address = (instruction.next_address() as isize) + (operands[0] as i16) as isize - 2;
    Ok(address as usize)
}

pub fn print_paddr(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    let address = packed_string_address(state.memory(), operands[0])?;
    let text = text::as_text(state, address)?;
    state.print(&text)?;
    // context.print_string(text);
    Ok(instruction.next_address())
}

// pub fn load(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     let value = context.peek_variable(operands[0] as u8)?;
//     store_result(context, instruction, value)?;
//     Ok(instruction.next_address())
// }

// pub fn not(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     let value = !operands[0];
//     store_result(context, instruction, value)?;
//     Ok(instruction.next_address())
// }

// pub fn call_1n(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     let address = packed_routine_address(context, operands[0]);
//     call_fn(context, address, instruction.next_address(), &vec![], None)
// }
