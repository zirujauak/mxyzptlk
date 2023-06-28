use super::*;
use crate::error::RuntimeError;
use crate::object::{self, attribute, property};
use crate::zmachine::ZMachine;

pub fn je(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    for i in 1..operands.len() {
        if operands[0] as i16 == operands[i] as i16 {
            return branch(zmachine, instruction, true);
        }
    }

    branch(zmachine, instruction, false)
}

pub fn jl(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    branch(
        zmachine,
        instruction,
        (operands[0] as i16) < (operands[1] as i16),
    )
}

pub fn jg(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    branch(
        zmachine,
        instruction,
        (operands[0] as i16) > (operands[1] as i16),
    )
}

pub fn dec_chk(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let val = zmachine.peek_variable(operands[0] as u8)? as i16;
    let new_val = i16::overflowing_sub(val, 1);
    zmachine.set_variable_indirect(operands[0] as u8, new_val.0 as u16)?;
    branch(zmachine, instruction, new_val.0 < operands[1] as i16)
}

pub fn inc_chk(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let val = zmachine.peek_variable(operands[0] as u8)? as i16;
    let new_val = i16::overflowing_add(val, 1);
    zmachine.set_variable_indirect(operands[0] as u8, new_val.0 as u16)?;
    branch(zmachine, instruction, new_val.0 > operands[1] as i16)
}

pub fn jin(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    branch(
        zmachine,
        instruction,
        object::parent(zmachine, operands[0] as usize)? == (operands[1] as usize),
    )
}

pub fn test(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    branch(
        zmachine,
        instruction,
        operands[0] & operands[1] == operands[1],
    )
}

pub fn or(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let mut result = operands[0];
    for i in 1..operands.len() {
        result = result | operands[i]
    }

    store_result(zmachine, instruction, result)?;
    Ok(instruction.next_address())
}

pub fn and(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let mut result = operands[0];
    for i in 1..operands.len() {
        result = result & operands[i]
    }

    store_result(zmachine, instruction, result)?;
    Ok(instruction.next_address())
}

pub fn loadw(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let address = (operands[0] as isize + (operands[1] as i16 * 2) as isize) as usize;
    store_result(zmachine, instruction, zmachine.read_word(address)?)?;
    Ok(instruction.next_address())
}

pub fn loadb(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let address = (operands[0] as isize + (operands[1] as i16) as isize) as usize;
    store_result(zmachine, instruction, zmachine.read_byte(address)? as u16)?;
    Ok(instruction.next_address())
}

pub fn store(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    zmachine.set_variable_indirect(operands[0] as u8, operands[1])?;
    Ok(instruction.next_address())
}

pub fn insert_obj(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;

    let object = operands[0] as usize;
    if object != 0 {
        let new_parent = operands[1] as usize;
        let old_parent = object::parent(zmachine, object)?;

        if old_parent != new_parent {
            if old_parent != 0 {
                let old_parent_child = object::child(zmachine, old_parent)?;

                if old_parent_child == object {
                    let o = object::sibling(zmachine, object)?;
                    object::set_child(zmachine, old_parent, o)?;
                } else {
                    let mut sibling = old_parent_child;
                    while sibling != 0 && object::sibling(zmachine, sibling)? != object {
                        sibling = object::sibling(zmachine, sibling)?;
                    }

                    if sibling == 0 {
                        return Err(RuntimeError::new(
                            ErrorCode::ObjectTreeState,
                            format!(
                                "Unable to find previous sibling of object {} in parent {}",
                                object, old_parent
                            ),
                        ));
                    }

                    let o = object::sibling(zmachine, object)?;
                    object::set_sibling(zmachine, sibling, o)?;
                }
            }

            let o = object::child(zmachine, new_parent)?;
            object::set_sibling(zmachine, object, o)?;
            object::set_child(zmachine, new_parent, object)?;
            object::set_parent(zmachine, object, new_parent)?;
        }
    }

    Ok(instruction.next_address())
}

pub fn test_attr(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let condition = operands[0] > 0
        && attribute::value(zmachine, operands[0] as usize, operands[1] as u8)?;
    branch(zmachine, instruction, condition)
}

pub fn set_attr(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    if operands[0] > 0 {
        attribute::set(zmachine, operands[0] as usize, operands[1] as u8)?;
    }

    Ok(instruction.next_address())
}

pub fn clear_attr(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    if operands[0] > 0 {
        attribute::clear(zmachine, operands[0] as usize, operands[1] as u8)?;
    }

    Ok(instruction.next_address())
}

pub fn get_prop(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;

    if operands[0] == 0 {
        store_result(zmachine, instruction, 0)?;
    } else {
        let value = property::property(zmachine, operands[0] as usize, operands[1] as u8)?;
        store_result(zmachine, instruction, value)?;
    }

    Ok(instruction.next_address())
}

pub fn get_prop_addr(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;

    if operands[0] == 0 {
        store_result(zmachine, instruction, 0)?;
    } else {
        let value = property::property_data_address(
            zmachine,
            operands[0] as usize,
            operands[1] as u8,
        )?;
        store_result(zmachine, instruction, value as u16)?;
    }

    Ok(instruction.next_address())
}

pub fn get_next_prop(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;

    if operands[0] == 0 {
        store_result(zmachine, instruction, 0)?;
    } else {
        let value =
            property::next_property(zmachine, operands[0] as usize, operands[1] as u8)?;
        store_result(zmachine, instruction, value as u16)?;
    }

    Ok(instruction.next_address())
}

pub fn add(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;

    let mut value = operands[0] as i16;
    for i in 1..operands.len() {
        value = i16::overflowing_add(value, operands[i] as i16).0;
    }

    store_result(zmachine, instruction, value as u16)?;
    Ok(instruction.next_address())
}

pub fn sub(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;

    let mut value = operands[0] as i16;
    for i in 1..operands.len() {
        value = i16::overflowing_sub(value, operands[i] as i16).0;
    }

    store_result(zmachine, instruction, value as u16)?;
    Ok(instruction.next_address())
}

pub fn mul(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;

    let mut value = operands[0] as i16;
    for i in 1..operands.len() {
        value = i16::overflowing_mul(value, operands[i] as i16).0;
    }

    store_result(zmachine, instruction, value as u16)?;
    Ok(instruction.next_address())
}

pub fn div(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;

    let mut value = operands[0] as i16;
    for i in 1..operands.len() {
        value = i16::overflowing_div(value, operands[i] as i16).0;
    }

    store_result(zmachine, instruction, value as u16)?;
    Ok(instruction.next_address())
}

pub fn modulus(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;

    let mut value = operands[0] as i16;
    for i in 1..operands.len() {
        value = i16::overflowing_rem(value, operands[i] as i16).0;
    }

    store_result(zmachine, instruction, value as u16)?;
    Ok(instruction.next_address())
}

pub fn call_2s(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;

    let address = zmachine.packed_routine_address(operands[0])?;
    let arguments = vec![operands[1]];

    call_fn(
        zmachine,
        address,
        instruction.next_address(),
        &arguments,
        instruction.store().copied(),
    )
}

pub fn call_2n(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;

    let address = zmachine.packed_routine_address(operands[0])?;
    let arguments = vec![operands[1]];

    call_fn(
        zmachine,
        address,
        instruction.next_address(),
        &arguments,
        None,
    )
}

pub fn set_colour(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    zmachine.set_colors(operands[0], operands[1])?;
    Ok(instruction.next_address())
}

pub fn throw(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;

    let result = operands[0];
    let depth = operands[1];

    zmachine.throw(depth, result)
}