use super::*;
use crate::error::*;
use crate::state::object::attribute;
use crate::state::object::property;
use crate::state::State;

pub fn je(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    for i in 1..operands.len() {
        if operands[0] as i16 == operands[i] as i16 {
            return branch(state, instruction, true);
        }
    }

    branch(state, instruction, false)
}

pub fn jl(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    branch(
        state,
        instruction,
        (operands[0] as i16) < (operands[1] as i16),
    )
}

// pub fn jg(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     branch(
//         context,
//         instruction,
//         (operands[0] as i16) > (operands[1] as i16),
//     )
// }

// pub fn dec_chk(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     let val = context.peek_variable(operands[0] as u8)? as i16;
//     let new_val = i16::overflowing_sub(val, 1);
//     context.set_variable_indirect(operands[0] as u8, new_val.0 as u16)?;
//     branch(context, instruction, new_val.0 < operands[1] as i16)
// }

// pub fn inc_chk(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     let val = context.peek_variable(operands[0] as u8)? as i16;
//     let new_val = i16::overflowing_add(val, 1);
//     context.set_variable_indirect(operands[0] as u8, new_val.0 as u16)?;
//     branch(context, instruction, new_val.0 > operands[1] as i16)
// }

// pub fn jin(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     branch(
//         context,
//         instruction,
//         object::parent(context, operands[0] as usize)? == (operands[1] as usize),
//     )
// }

// pub fn test(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     branch(
//         context,
//         instruction,
//         operands[0] & operands[1] == operands[1],
//     )
// }

// pub fn or(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     let mut result = operands[0];
//     for i in 1..operands.len() {
//         result = result | operands[i]
//     }

//     store_result(context, instruction, result)?;
//     Ok(instruction.next_address())
// }

pub fn and(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    let mut result = operands[0];
    for i in 1..operands.len() {
        result = result & operands[i]
    }

    store_result(state, instruction, result)?;
    Ok(instruction.next_address())
}

pub fn loadw(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    let address = (operands[0] as isize + (operands[1] as i16 * 2) as isize) as usize;
    store_result(state, instruction, state.read_word(address)?)?;
    Ok(instruction.next_address())
}

pub fn loadb(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    let address = (operands[0] as isize + (operands[1] as i16) as isize) as usize;
    store_result(state, instruction, state.read_byte(address)? as u16)?;
    Ok(instruction.next_address())
}

pub fn store(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    state.set_variable_indirect(operands[0] as u8, operands[1])?;
    Ok(instruction.next_address())
}

// pub fn insert_obj(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;

//     let object = operands[0] as usize;
//     if object != 0 {
//         let new_parent = operands[1] as usize;
//         let old_parent = object::parent(context, object)?;

//         if old_parent != new_parent {
//             if old_parent != 0 {
//                 let old_parent_child = object::child(context, old_parent)?;

//                 if old_parent_child == object {
//                     object::set_child(context, old_parent, object::sibling(context, object)?)?;
//                 } else {
//                     let mut sibling = old_parent_child;
//                     while sibling != 0 && object::sibling(context, sibling)? != object {
//                         sibling = object::sibling(context, sibling)?;
//                     }

//                     if sibling == 0 {
//                         panic!("Inconsistent object tree");
//                     }

//                     object::set_sibling(context, sibling, object::sibling(context, object)?)?;
//                 }
//             }

//             object::set_sibling(context, object, object::child(context, new_parent)?)?;
//             object::set_child(context, new_parent, object)?;
//             object::set_parent(context, object, new_parent)?;
//         }
//     }

//     Ok(instruction.next_address())
// }

pub fn test_attr(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    let condition =
        operands[0] > 0 && attribute::value(state, operands[0] as usize, operands[1] as u8)?;
    branch(state, instruction, condition)
}

pub fn set_attr(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    if operands[0] > 0 {
        attribute::set(state, operands[0] as usize, operands[1] as u8)?;
    }

    Ok(instruction.next_address())
}

pub fn clear_attr(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    if operands[0] > 0 {
        attribute::clear(state, operands[0] as usize, operands[1] as u8)?;
    }

    Ok(instruction.next_address())
}

pub fn get_prop(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;

    if operands[0] == 0 {
        store_result(state, instruction, 0)?;
    } else {
        let value = property::property(state, operands[0] as usize, operands[1] as u8)?;
        store_result(state, instruction, value)?;
    }

    Ok(instruction.next_address())
}

// pub fn get_prop_addr(
//     context: &mut Context,
//     instruction: &Instruction,
// ) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;

//     if operands[0] == 0 {
//         store_result(context, instruction, 0)?;
//     } else {
//         let value = property::property_data_addr(context, operands[0] as usize, operands[1] as u8)?;
//         store_result(context, instruction, value as u16)?;
//     }

//     Ok(instruction.next_address())
// }

// pub fn get_next_prop(
//     context: &mut Context,
//     instruction: &Instruction,
// ) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;

//     if operands[0] == 0 {
//         store_result(context, instruction, 0)?;
//     } else {
//         let value = property::next_property(context, operands[0] as usize, operands[1] as u8)?;
//         store_result(context, instruction, value as u16)?;
//     }

//     Ok(instruction.next_address())
// }

// pub fn add(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;

//     let mut value = operands[0] as i16;
//     for i in 1..operands.len() {
//         value = i16::overflowing_add(value, operands[i] as i16).0;
//     }

//     trace!(
//         "ADD {} + {} = {}",
//         operands[0] as i16,
//         operands[1] as i16,
//         value as i16
//     );
//     store_result(context, instruction, value as u16)?;
//     Ok(instruction.next_address())
// }

// pub fn sub(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;

//     let mut value = operands[0] as i16;
//     for i in 1..operands.len() {
//         value = i16::overflowing_sub(value, operands[i] as i16).0;
//     }

//     store_result(context, instruction, value as u16);
//     Ok(instruction.next_address())
// }

// pub fn mul(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;

//     let mut value = operands[0] as i16;
//     for i in 1..operands.len() {
//         value = i16::overflowing_mul(value, operands[i] as i16).0;
//     }

//     store_result(context, instruction, value as u16);
//     Ok(instruction.next_address())
// }

// pub fn div(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;

//     let mut value = operands[0] as i16;
//     for i in 1..operands.len() {
//         value = i16::overflowing_div(value, operands[i] as i16).0;
//     }

//     store_result(context, instruction, value as u16);
//     Ok(instruction.next_address())
// }

// pub fn modulus(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;

//     let mut value = operands[0] as i16;
//     for i in 1..operands.len() {
//         value = i16::overflowing_rem(value, operands[i] as i16).0;
//     }

//     store_result(context, instruction, value as u16);
//     Ok(instruction.next_address())
// }

// pub fn call_2s(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;

//     let address = packed_routine_address(context, operands[0]);
//     let arguments = vec![operands[1]];

//     call_fn(
//         context,
//         address,
//         instruction.next_address(),
//         &arguments,
//         instruction.store,
//     )
// }

// pub fn call_2n(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;

//     let address = packed_routine_address(context, operands[0]);
//     let arguments = vec![operands[1]];

//     call_fn(
//         context,
//         address,
//         instruction.next_address(),
//         &arguments,
//         None,
//     )
// }

// pub fn set_colour(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     todo!()
// }

// pub fn throw(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;

//     let value = operands[0];
//     let index = operands[1];

//     todo!()
//     // context.throw(index, value)
// }