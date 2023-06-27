use crate::{
    error::{ErrorCode, RuntimeError},
    zmachine::{
        instruction::Instruction,
        object::{
            self, child, parent,
            property::{property_length, short_name},
            set_child, set_parent, set_sibling, sibling,
        },
        text::{as_text, from_vec},
        ZMachine,
    },
};

use super::{branch, operand_values, packed_routine_address, packed_string_address, store_result};

pub fn jz(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    branch(zmachine, instruction, operands[0] == 0)
}

pub fn get_sibling(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let sibling = object::sibling(&zmachine.state, operands[0] as usize)?;
    store_result(zmachine, instruction, sibling as u16)?;
    branch(zmachine, instruction, sibling != 0)
}

pub fn get_child(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let child = object::child(&zmachine.state, operands[0] as usize)?;
    store_result(zmachine, instruction, child as u16)?;
    branch(zmachine, instruction, child != 0)
}

pub fn get_parent(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let parent = object::parent(&zmachine.state, operands[0] as usize)?;
    store_result(zmachine, instruction, parent as u16)?;
    Ok(instruction.next_address())
}

pub fn get_prop_len(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let len = property_length(&zmachine.state, operands[0] as usize)?;
    store_result(zmachine, instruction, len as u16)?;
    Ok(instruction.next_address())
}

pub fn inc(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let val = zmachine.state.peek_variable(operands[0] as u8)?;
    let new_val = i16::overflowing_add(val as i16, 1);
    zmachine
        .state
        .set_variable_indirect(operands[0] as u8, new_val.0 as u16)?;
    Ok(instruction.next_address())
}

pub fn dec(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let val = zmachine.state.peek_variable(operands[0] as u8)?;
    let new_val = i16::overflowing_sub(val as i16, 1);
    zmachine
        .state
        .set_variable_indirect(operands[0] as u8, new_val.0 as u16)?;
    Ok(instruction.next_address())
}

pub fn print_addr(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let text = as_text(&zmachine.state, operands[0] as usize)?;

    zmachine.print(&text)?;
    Ok(instruction.next_address())
}

pub fn call_1s(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let address = packed_routine_address(zmachine.state.memory(), operands[0])?;

    zmachine.call_routine(
        address,
        &vec![],
        instruction.store,
        instruction.next_address(),
    )
}

pub fn remove_obj(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let object = operands[0] as usize;
    if object > 0 {
        let parent = parent(&zmachine.state, object)?;
        if parent != 0 {
            let parent_child = child(&zmachine.state, parent)?;
            if parent_child == object {
                let sibling = sibling(&zmachine.state, object)?;
                set_child(&mut zmachine.state, parent, sibling)?;
            } else {
                let mut sibling = parent_child;
                while sibling != 0 && object::sibling(&zmachine.state, sibling)? != object {
                    sibling = object::sibling(&zmachine.state, sibling)?;
                }

                if sibling == 0 {
                    return Err(RuntimeError::new(
                        ErrorCode::ObjectTreeState,
                        "Unable to find previous sibling of removed object".to_string(),
                    ));
                }

                let o = object::sibling(&zmachine.state, object)?;
                set_sibling(
                    &mut zmachine.state,
                    sibling,
                    o,
                )?;
            }

            set_parent(&mut zmachine.state, object, 0)?;
            set_sibling(&mut zmachine.state, object, 0)?;
        }
    }

    Ok(instruction.next_address())
}

pub fn print_obj(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let ztext = short_name(&zmachine.state, operands[0] as usize)?;
    let text = from_vec(&zmachine.state, &ztext)?;
    zmachine.print(&text)?;
    // context.print_string(text);
    Ok(instruction.next_address())
}

pub fn ret(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    zmachine.return_routine(operands[0])
}

pub fn jump(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let address = (instruction.next_address() as isize) + (operands[0] as i16) as isize - 2;
    Ok(address as usize)
}

pub fn print_paddr(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let address = packed_string_address(zmachine.state.memory(), operands[0])?;
    let text = as_text(&zmachine.state, address)?;
    zmachine.print(&text)?;
    // context.print_string(text);
    Ok(instruction.next_address())
}

pub fn load(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let value = zmachine.state.peek_variable(operands[0] as u8)?;
    store_result(zmachine, instruction, value)?;
    Ok(instruction.next_address())
}

pub fn not(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let value = !operands[0];
    store_result(zmachine, instruction, value)?;
    Ok(instruction.next_address())
}

pub fn call_1n(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let address = packed_routine_address(zmachine.state.memory(), operands[0])?;
    zmachine.call_routine(address, &vec![], None, instruction.next_address())
}
