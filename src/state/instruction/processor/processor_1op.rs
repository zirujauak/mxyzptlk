use super::*;
use crate::error::*;
use crate::state::memory::Memory;
use crate::state::State;

// pub fn jz(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     branch(context, instruction, operands[0] == 0)
// }

// pub fn get_sibling(
//     context: &mut Context,
//     instruction: &Instruction,
// ) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     let sibling = object::sibling(context, operands[0] as usize)?;
//     store_result(context, instruction, sibling as u16)?;
//     branch(context, instruction, sibling != 0)
// }

// pub fn get_child(
//     context: &mut Context,
//     instruction: &Instruction,
// ) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     let child = object::child(context, operands[0] as usize)?;
//     store_result(context, instruction, child as u16)?;
//     branch(context, instruction, child != 0)
// }

// pub fn get_parent(
//     context: &mut Context,
//     instruction: &Instruction,
// ) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     let parent = object::parent(context, operands[0] as usize)?;
//     store_result(context, instruction, parent as u16)?;
//     Ok(instruction.next_address())
// }

// pub fn get_prop_len(
//     context: &mut Context,
//     instruction: &Instruction,
// ) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     let len = property::property_length(context, operands[0] as usize)?;
//     store_result(context, instruction, len as u16)?;
//     Ok(instruction.next_address())
// }

pub fn inc(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    let val = state.peek_variable(operands[0] as u8)?;
    let new_val = i16::overflowing_add(val as i16, 1);
    state.set_variable_indirect(operands[0] as u8, new_val.0 as u16)?;
    Ok(instruction.next_address())
}

// pub fn dec(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     let val = context.peek_variable(operands[0] as u8)?;
//     let new_val = i16::overflowing_sub(val as i16, 1);
//     context.set_variable_indirect(operands[0] as u8, new_val.0 as u16)?;
//     Ok(instruction.next_address())
// }

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

// pub fn print_obj(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     let ztext = property::short_name(context, operands[0] as usize)?;
//     let text = text::from_vec(context, &ztext)?;

//     context.print_string(text);
//     Ok(instruction.next_address())
// }

// pub fn ret(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     context.return_fn(operands[0])
// }

// pub fn jump(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     let address = (instruction.next_address() as isize) + (operands[0] as i16) as isize - 2;
//     Ok(address as usize)
// }

// pub fn print_paddr(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     let address = packed_string_address(context, operands[0]);
//     let text = text::as_text(context, address)?;

//     context.print_string(text);
//     Ok(instruction.next_address())
// }

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
