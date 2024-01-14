//! [2OP](https://inform-fiction.org/zmachine/standards/z1point1/sect14.html#2OP)
//! instructions: long and variable form instructions that have two (or, in some cases, more) operands.

use super::*;
use crate::error::RuntimeError;
use crate::object::{self, attribute, property};
use crate::zmachine::ZMachine;

/// [JE](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#je): branches if
/// operands 0 is equal to any of the subsequent operands.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing the [InstructionResult] or a [RuntimeError]
pub fn je(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    for i in 1..operands.len() {
        if operands[0] as i16 == operands[i] as i16 {
            return InstructionResult::new(branch(zmachine, instruction, true)?);
        }
    }

    InstructionResult::new(branch(zmachine, instruction, false)?)
}

/// [JL](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#jl): branches if
/// operand 0 is less than operand 1
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing the [InstructionResult] or a [RuntimeError]
pub fn jl(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    InstructionResult::new(branch(
        zmachine,
        instruction,
        (operands[0] as i16) < (operands[1] as i16),
    )?)
}

/// [JG](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#jg): branches if
/// operand 0 is greater than operand 1
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing the [InstructionResult] or a [RuntimeError]
pub fn jg(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    InstructionResult::new(branch(
        zmachine,
        instruction,
        (operands[0] as i16) > (operands[1] as i16),
    )?)
}

/// [DEC_CHK](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#dec_chk): decrements
/// the variable specified by operand 0 in place and branches if the decremented value is less
/// than operand 1.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing the [InstructionResult] or a [RuntimeError]
pub fn dec_chk(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let val = zmachine.peek_variable(operands[0] as u8)? as i16;
    let new_val = i16::overflowing_sub(val, 1);
    zmachine.set_variable_indirect(operands[0] as u8, new_val.0 as u16)?;
    InstructionResult::new(branch(
        zmachine,
        instruction,
        new_val.0 < operands[1] as i16,
    )?)
}

/// [INC_CHK](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#inc_chk): increments
/// the variable specified by operand 0 in place and branches if the decremented value is greater
/// than operand 1.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing the [InstructionResult] or a [RuntimeError]
pub fn inc_chk(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let val = zmachine.peek_variable(operands[0] as u8)? as i16;
    let new_val = i16::overflowing_add(val, 1);
    zmachine.set_variable_indirect(operands[0] as u8, new_val.0 as u16)?;
    InstructionResult::new(branch(
        zmachine,
        instruction,
        new_val.0 > operands[1] as i16,
    )?)
}

/// [JIN](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#jin): branches if
/// the parent of the object in operand 0 is the object in operand 1.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing the [InstructionResult] or a [RuntimeError]
pub fn jin(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    InstructionResult::new(branch(
        zmachine,
        instruction,
        object::parent(zmachine, operands[0] as usize)? == (operands[1] as usize),
    )?)
}

/// [TEST](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#test): branches if
/// operand 0 bitwise AND operand 1 is equal to operand 1.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing the [InstructionResult] or a [RuntimeError]
pub fn test(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    InstructionResult::new(branch(
        zmachine,
        instruction,
        operands[0] & operands[1] == operands[1],
    )?)
}

/// [OR](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#or): stores the
/// bitwise OR of operand 0 with operand 1.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing the [InstructionResult] or a [RuntimeError]
pub fn or(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let mut result = operands[0];
    for w in operands[1..].iter() {
        result |= *w
    }

    store_result(zmachine, instruction, result)?;
    InstructionResult::new(Address(instruction.next_address))
}

/// [AND](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#and): stores the
/// bitwise AND of operand 0 with operand 1.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing the [InstructionResult] or a [RuntimeError]
pub fn and(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let mut result = operands[0];
    for w in operands[1..].iter() {
        result &= *w
    }

    store_result(zmachine, instruction, result)?;
    InstructionResult::new(Address(instruction.next_address))
}

/// [TEST_ATTR](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#test_attr): branches if
/// the object in operand 0 has the attribute in operand 1.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing the [InstructionResult] or a [RuntimeError]
pub fn test_attr(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let condition = attribute::value(zmachine, operands[0] as usize, operands[1] as u8)?;
    InstructionResult::new(branch(zmachine, instruction, condition)?)
}

/// [SET_ATTR](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#set_attr): sets the attribute
/// in operand 1 on the object in operand 0.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing the [InstructionResult] or a [RuntimeError]
pub fn set_attr(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    if operands[0] > 0 {
        attribute::set(zmachine, operands[0] as usize, operands[1] as u8)?;
    }

    InstructionResult::new(Address(instruction.next_address))
}

/// [CLEAR_ATTR](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#clear_attr): clears the attribute
/// in operand 1 on the object in operand 0.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing the [InstructionResult] or a [RuntimeError]
pub fn clear_attr(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    if operands[0] > 0 {
        attribute::clear(zmachine, operands[0] as usize, operands[1] as u8)?;
    }
    InstructionResult::new(Address(instruction.next_address))
}

/// [STORE](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#store): sets the variable
/// referenced by operand 0 to the value in operand 1
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing the [InstructionResult] or a [RuntimeError]
pub fn store(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    zmachine.set_variable_indirect(operands[0] as u8, operands[1])?;
    InstructionResult::new(Address(instruction.next_address))
}

/// [INSERT_OBJ](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#insert_obj): inserts the
/// object in operand 0 as the first child of the object in operand 1.  
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing the [InstructionResult] or a [RuntimeError]
pub fn insert_obj(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
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
                        return fatal_error!(
                            ErrorCode::InvalidObjectTree,
                            "Unable to find previous sibling of object {} in parent {}",
                            object,
                            old_parent
                        );
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

    InstructionResult::new(Address(instruction.next_address))
}

/// [LOADW](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#loadw): stores the word
/// value from the word array at the byte address in operand 0, indexed by operand 1.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing the [InstructionResult] or a [RuntimeError]
pub fn loadw(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let address = (operands[0] as isize + (operands[1] as i16 * 2) as isize) as usize;
    store_result(zmachine, instruction, zmachine.read_word(address)?)?;
    InstructionResult::new(Address(instruction.next_address))
}

/// [LOADB](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#loadb): stores the byte
/// value from the byte array at the byte address in operand 0, indexed by operand 1.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing the [InstructionResult] or a [RuntimeError]
pub fn loadb(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let address = (operands[0] as isize + (operands[1] as i16) as isize) as usize;
    store_result(zmachine, instruction, zmachine.read_byte(address)? as u16)?;
    InstructionResult::new(Address(instruction.next_address))
}

/// [GET_PROP](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#get_prop): stores the 1-
/// or 2-byte value of the property in operand 1 of the object in operand 0.  If the object does not
/// have the requested property, the default property value is stored.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing the [InstructionResult] or a [RuntimeError]
pub fn get_prop(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;

    if operands[0] == 0 {
        store_result(zmachine, instruction, 0)?;
    } else {
        let value = property::property(zmachine, operands[0] as usize, operands[1] as u8)?;
        store_result(zmachine, instruction, value)?;
    }

    InstructionResult::new(Address(instruction.next_address))
}

/// [GET_PROP_ADDR](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#get_prop_addr): stores the byte
/// address of the property in operand 1 for the object in operand 0.  If the object does not have
/// the property, 0 is stored instead.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing the [InstructionResult] or a [RuntimeError]
pub fn get_prop_addr(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;

    if operands[0] == 0 {
        store_result(zmachine, instruction, 0)?;
    } else {
        let value =
            property::property_data_address(zmachine, operands[0] as usize, operands[1] as u8)?;
        store_result(zmachine, instruction, value as u16)?;
    }

    InstructionResult::new(Address(instruction.next_address))
}

/// [GET_NEXT_PROP](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#get_next_prop): stores the next
/// property after the property in operand 1 for the object in operand 1.  If there is no next property, 0 is
/// stored.  If operand 1 is 0, the first property number on the object is returned.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing the [InstructionResult] or a [RuntimeError]
pub fn get_next_prop(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;

    if operands[0] == 0 {
        store_result(zmachine, instruction, 0)?;
    } else {
        let value = property::next_property(zmachine, operands[0] as usize, operands[1] as u8)?;
        store_result(zmachine, instruction, value as u16)?;
    }

    InstructionResult::new(Address(instruction.next_address))
}

/// [ADD](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#add): performs signed
/// addition, storing the result of adding operand 0 to operand 1.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing the [InstructionResult] or a [RuntimeError]
pub fn add(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;

    let mut value = operands[0] as i16;
    for w in operands[1..].iter() {
        value = i16::overflowing_add(value, *w as i16).0;
    }

    store_result(zmachine, instruction, value as u16)?;
    InstructionResult::new(Address(instruction.next_address))
}

/// [SUB](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#sub): performs signed
/// subtraction, storing the result of subtracting operand 0 from operand 1.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing the [InstructionResult] or a [RuntimeError]
pub fn sub(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;

    let mut value = operands[0] as i16;
    for w in operands[1..].iter() {
        value = i16::overflowing_sub(value, *w as i16).0;
    }

    store_result(zmachine, instruction, value as u16)?;
    InstructionResult::new(Address(instruction.next_address))
}

/// [MUL](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#mul): performs signed
/// multiplication, storing the result of multiplying operand 0 by operand 1.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing the [InstructionResult] or a [RuntimeError]
pub fn mul(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;

    let mut value = operands[0] as i16;
    for w in operands[1..].iter() {
        value = i16::overflowing_mul(value, *w as i16).0;
    }

    store_result(zmachine, instruction, value as u16)?;
    InstructionResult::new(Address(instruction.next_address))
}

/// [DIV](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#div): performs signed
/// division, storing the result of dividing operand 0 by operand 1.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing the [InstructionResult] or a [RuntimeError]
pub fn div(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;

    let mut value = operands[0] as i16;
    for w in operands[1..].iter() {
        // Divide by zero
        if *w == 0 {
            return fatal_error!(
                ErrorCode::DivideByZero,
                "Divide by zero: {}, {:?}",
                instruction,
                operands
            );
        }
        value = i16::overflowing_div(value, *w as i16).0;
    }

    store_result(zmachine, instruction, value as u16)?;
    InstructionResult::new(Address(instruction.next_address))
}

/// [MOD](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#mod): performs signed
/// division, storing the remainder of dividing operand 0 by operand 1.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing the [InstructionResult] or a [RuntimeError]
pub fn modulus(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;

    let mut value = operands[0] as i16;
    for w in operands[1..].iter() {
        if *w == 0 {
            return fatal_error!(
                ErrorCode::DivideByZero,
                "Divide by zero: {}, {:?}",
                instruction,
                operands
            );
        }
        value = i16::overflowing_rem(value, *w as i16).0;
    }

    store_result(zmachine, instruction, value as u16)?;
    InstructionResult::new(Address(instruction.next_address))
}

/// [CALL_2S](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#call_2s): Calls
/// a routine with a single argument and stores the result.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing the [InstructionResult] or a [RuntimeError]
pub fn call_2s(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let address = zmachine.packed_routine_address(operands[0])?;

    InstructionResult::new(call_routine(
        zmachine,
        address,
        instruction.next_address,
        &vec![operands[1]],
        instruction.store,
    )?)
}

/// [CALL_2N](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#call_2n): Calls
/// a routine with a single argument and will not store the result.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing the [InstructionResult] or a [RuntimeError]
pub fn call_2n(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;

    let address = zmachine.packed_routine_address(operands[0])?;
    let arguments = vec![operands[1]];

    InstructionResult::new(call_routine(
        zmachine,
        address,
        instruction.next_address,
        &arguments,
        None,
    )?)
}

/// [SET_COLOUR](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#set_colour): Sets the
/// foreground and background colours to the values in operand 0 and operand 1, respectively.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing the [InstructionResult] or a [RuntimeError]
pub fn set_colour(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    InstructionResult::set_colour(Address(instruction.next_address), operands[0], operands[1])
}

/// [THROW](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#throw): Resets
/// the stack to the previously caught frame pointer in operand 1, returning the
/// value in operand 0.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing the [InstructionResult] or a [RuntimeError]

pub fn throw(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;

    let result = operands[0];
    let depth = operands[1];

    InstructionResult::new(zmachine.throw(depth, result)?)
}

#[cfg(test)]
mod tests {
    use crate::{
        assert_ok_eq,
        instruction::{
            processor::{dispatch, Opcode},
            OpcodeForm, OperandCount, OperandType,
        },
        object::{self, attribute},
        test_util::*,
    };

    fn opcode_2op(version: u8, instruction: u8) -> Opcode {
        Opcode::new(
            version,
            instruction,
            instruction,
            OpcodeForm::Long,
            OperandCount::_2OP,
        )
    }

    fn opcode_var(version: u8, instruction: u8) -> Opcode {
        Opcode::new(
            version,
            instruction,
            instruction,
            OpcodeForm::Var,
            OperandCount::_2OP,
        )
    }

    #[test]
    fn test_je_2op_true() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_branch_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x1234),
                operand(OperandType::LargeConstant, 0x1234),
            ],
            opcode_2op(3, 1),
            0x406,
            branch(0x405, true, 0x40a),
        );

        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x40a);
    }

    #[test]
    fn test_je_2op_false() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_branch_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x1234),
                operand(OperandType::LargeConstant, 0x1235),
            ],
            opcode_2op(3, 1),
            0x406,
            branch(0x405, true, 0x40a),
        );

        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x406);
    }

    #[test]
    fn test_je_var_true() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_branch_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x1234),
                operand(OperandType::LargeConstant, 0x1235),
                operand(OperandType::LargeConstant, 0x1236),
                operand(OperandType::LargeConstant, 0x1234),
            ],
            opcode_var(3, 1),
            0x406,
            branch(0x405, true, 0x40a),
        );

        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x40a);
    }

    #[test]
    fn test_je_var_false() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_branch_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x1234),
                operand(OperandType::LargeConstant, 0x1235),
                operand(OperandType::LargeConstant, 0x1236),
                operand(OperandType::LargeConstant, 0x1237),
            ],
            opcode_var(3, 1),
            0x406,
            branch(0x405, true, 0x40a),
        );

        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x406);
    }

    #[test]
    fn test_jl_true() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_branch_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0xFFFE),
                operand(OperandType::LargeConstant, 0xFFFF),
            ],
            opcode_2op(3, 2),
            0x406,
            branch(0x405, true, 0x40a),
        );

        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x40a);
    }

    #[test]
    fn test_jl_false() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_branch_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x0000),
                operand(OperandType::LargeConstant, 0xFFFF),
            ],
            opcode_2op(3, 2),
            0x406,
            branch(0x405, true, 0x40a),
        );

        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x406);
    }

    #[test]
    fn test_jg_true() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_branch_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0xFFFF),
                operand(OperandType::LargeConstant, 0xFFFE),
            ],
            opcode_2op(3, 3),
            0x406,
            branch(0x405, true, 0x40a),
        );

        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x40a);
    }

    #[test]
    fn test_jg_false() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_branch_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0xFFFF),
                operand(OperandType::LargeConstant, 0x0000),
            ],
            opcode_2op(3, 3),
            0x406,
            branch(0x405, true, 0x40a),
        );

        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x406);
    }

    #[test]
    fn test_dec_chk_true() {
        let mut map = test_map(3);
        set_variable(&mut map, 0x80, 0x00);
        let mut zmachine = mock_zmachine(map);
        let i = mock_branch_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 0x80),
                operand(OperandType::SmallConstant, 0x00),
            ],
            opcode_2op(3, 4),
            0x405,
            branch(0x404, true, 0x40a),
        );

        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x40a);
        assert_ok_eq!(zmachine.variable(0x80), 0xFFFF);
    }

    #[test]
    fn test_dec_chk_false() {
        let mut map = test_map(3);
        set_variable(&mut map, 0x80, 0x01);
        let mut zmachine = mock_zmachine(map);
        let i = mock_branch_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 0x80),
                operand(OperandType::SmallConstant, 0x00),
            ],
            opcode_2op(3, 4),
            0x405,
            branch(0x404, true, 0x40a),
        );

        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x405);
        assert_ok_eq!(zmachine.variable(0x80), 0x00);
    }

    #[test]
    fn test_inc_chk_true() {
        let mut map = test_map(3);
        set_variable(&mut map, 0x80, 0x00);
        let mut zmachine = mock_zmachine(map);
        let i = mock_branch_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 0x80),
                operand(OperandType::SmallConstant, 0x00),
            ],
            opcode_2op(3, 5),
            0x405,
            branch(0x404, true, 0x40a),
        );

        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x40a);
        assert_ok_eq!(zmachine.variable(0x80), 0x01);
    }

    #[test]
    fn test_inc_chk_false() {
        let mut map = test_map(3);
        set_variable(&mut map, 0x80, 0xFFFF);
        let mut zmachine = mock_zmachine(map);
        let i = mock_branch_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 0x80),
                operand(OperandType::SmallConstant, 0x00),
            ],
            opcode_2op(3, 5),
            0x405,
            branch(0x404, true, 0x40a),
        );

        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x405);
        assert_ok_eq!(zmachine.variable(0x80), 0x00);
    }

    #[test]
    fn test_jin_true() {
        let mut map = test_map(3);
        mock_object(&mut map, 1, vec![], (0, 1, 2));
        mock_object(&mut map, 2, vec![], (1, 3, 4));

        let mut zmachine = mock_zmachine(map);
        let i = mock_branch_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 0x02),
                operand(OperandType::SmallConstant, 0x01),
            ],
            opcode_2op(3, 6),
            0x405,
            branch(0x404, true, 0x40a),
        );

        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x40a);
    }

    #[test]
    fn test_jin_false() {
        let mut map = test_map(3);
        mock_object(&mut map, 1, vec![], (0, 1, 2));
        mock_object(&mut map, 2, vec![], (1, 3, 4));
        mock_object(&mut map, 4, vec![], (2, 5, 0));

        let mut zmachine = mock_zmachine(map);
        let i = mock_branch_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 0x03),
                operand(OperandType::SmallConstant, 0x01),
            ],
            opcode_2op(3, 6),
            0x405,
            branch(0x404, true, 0x40a),
        );

        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x405);
    }

    #[test]
    fn test_test_true() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_branch_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0xA596),
                operand(OperandType::LargeConstant, 0x8182),
            ],
            opcode_2op(3, 7),
            0x406,
            branch(0x405, true, 0x40a),
        );

        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x40a);
    }

    #[test]
    fn test_test_false() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_branch_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0xA596),
                operand(OperandType::LargeConstant, 0x8181),
            ],
            opcode_2op(3, 7),
            0x406,
            branch(0x405, true, 0x40a),
        );

        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x406);
    }

    #[test]
    fn test_or() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x1200),
                operand(OperandType::SmallConstant, 0xFE),
            ],
            opcode_2op(3, 8),
            0x405,
            store(0x404, 0x80),
        );

        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x405);
        assert_ok_eq!(zmachine.variable(0x80), 0x12FE);
    }

    #[test]
    fn test_and() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0xAAAA),
                operand(OperandType::LargeConstant, 0x5555),
            ],
            opcode_2op(3, 9),
            0x406,
            store(0x405, 0x80),
        );

        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x406);
        assert_ok_eq!(zmachine.variable(0x80), 0x00);
    }

    #[test]
    fn test_test_attr_v3_true() {
        let mut map = test_map(3);
        // Set attributes 0, 4, 9, 14, 19, 24, and 29
        mock_attributes(&mut map, 1, &[0x88, 0x42, 0x10, 0x84]);
        let mut zmachine = mock_zmachine(map);
        let i = mock_branch_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 0x01),
                operand(OperandType::SmallConstant, 24),
            ],
            opcode_2op(3, 10),
            0x404,
            branch(0x403, true, 0x40a),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x40a);
    }

    #[test]
    fn test_test_attr_v3_false() {
        let mut map = test_map(3);
        // Set attributes 0, 4, 9, 14, 19, 24, and 29
        mock_attributes(&mut map, 1, &[0x88, 0x42, 0x10, 0x84]);
        let mut zmachine = mock_zmachine(map);
        let i = mock_branch_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 0x01),
                operand(OperandType::SmallConstant, 23),
            ],
            opcode_2op(3, 10),
            0x404,
            branch(0x403, true, 0x40a),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x404);
    }

    #[test]
    fn test_test_attr_v4_true() {
        let mut map = test_map(4);
        // Set attributes 0, 4, 9, 14, 19, 24, 29, 34, 39, 44
        mock_attributes(&mut map, 1, &[0x88, 0x42, 0x10, 0x84, 0x21, 0x08]);
        let mut zmachine = mock_zmachine(map);
        let i = mock_branch_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 0x01),
                operand(OperandType::SmallConstant, 39),
            ],
            opcode_2op(4, 10),
            0x404,
            branch(0x403, true, 0x40a),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x40a);
    }

    #[test]
    fn test_test_attr_v4_false() {
        let mut map = test_map(4);
        // Set attributes 0, 4, 9, 14, 19, 24, 29, 34, 39, 44
        mock_attributes(&mut map, 1, &[0x88, 0x42, 0x10, 0x84, 0x21, 0x08]);
        let mut zmachine = mock_zmachine(map);
        let i = mock_branch_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 0x01),
                operand(OperandType::SmallConstant, 40),
            ],
            opcode_2op(4, 10),
            0x404,
            branch(0x403, true, 0x40a),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x404);
    }

    #[test]
    fn test_test_attr_v4_invalid() {
        let mut map = test_map(4);
        // Set attributes 0, 4, 9, 14, 19, 24, 29, 34, 39, 44
        mock_attributes(&mut map, 1, &[0x88, 0x42, 0x10, 0x84, 0x21, 0x08]);
        let mut zmachine = mock_zmachine(map);
        let i = mock_branch_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 0x01),
                operand(OperandType::SmallConstant, 48),
            ],
            opcode_2op(4, 10),
            0x404,
            branch(0x403, true, 0x40a),
        );
        assert!(dispatch(&mut zmachine, &i).is_err());
    }

    #[test]
    fn test_set_attr_v3() {
        let mut map = test_map(3);
        // Set attributes 0, 4, 9, 14, 19, 24
        mock_attributes(&mut map, 1, &[0x88, 0x42, 0x10, 0x84]);
        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 0x01),
                operand(OperandType::SmallConstant, 20),
            ],
            opcode_2op(3, 11),
            0x404,
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x404);
        assert!(attribute::value(&zmachine, 1, 19).is_ok_and(|x| x));
        assert!(attribute::value(&zmachine, 1, 20).is_ok_and(|x| x));
        assert!(attribute::value(&zmachine, 1, 21).is_ok_and(|x| !x));
    }

    #[test]
    fn test_set_attr_v4() {
        let mut map = test_map(4);
        // Set attributes 0, 4, 9, 14, 19, 24, 29, 34, 39, 44
        mock_attributes(&mut map, 1, &[0x88, 0x42, 0x10, 0x84, 0x21, 0x08]);
        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 0x01),
                operand(OperandType::SmallConstant, 47),
            ],
            opcode_2op(4, 11),
            0x404,
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x404);
        assert!(attribute::value(&zmachine, 1, 46).is_ok_and(|x| !x));
        assert!(attribute::value(&zmachine, 1, 47).is_ok_and(|x| x));
    }

    #[test]
    fn test_set_attr_v4_invalid() {
        let mut map = test_map(4);
        // Set attributes 0, 4, 9, 14, 19, 24, 29, 34, 39, 44
        mock_attributes(&mut map, 1, &[0x88, 0x42, 0x10, 0x84, 0x21, 0x08]);
        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 0x01),
                operand(OperandType::SmallConstant, 48),
            ],
            opcode_2op(4, 11),
            0x404,
        );
        assert!(dispatch(&mut zmachine, &i).is_err());
    }

    #[test]
    fn test_clear_attr_v3() {
        let mut map = test_map(3);
        // Set attributes 0, 4, 9, 14, 19, 24
        mock_attributes(&mut map, 1, &[0x88, 0x42, 0x10, 0x84]);
        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 0x01),
                operand(OperandType::SmallConstant, 19),
            ],
            opcode_2op(3, 12),
            0x404,
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x404);
        assert!(attribute::value(&zmachine, 1, 18).is_ok_and(|x| !x));
        assert!(attribute::value(&zmachine, 1, 19).is_ok_and(|x| !x));
        assert!(attribute::value(&zmachine, 1, 20).is_ok_and(|x| !x));
        assert!(attribute::value(&zmachine, 1, 14).is_ok_and(|x| x));
    }

    #[test]
    fn test_clear_attr_v4() {
        let mut map = test_map(4);
        // Set attributes 0, 4, 9, 14, 19, 24, 29, 34, 39, 44
        mock_attributes(&mut map, 1, &[0x88, 0x42, 0x10, 0x84, 0x21, 0x08]);
        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 0x01),
                operand(OperandType::SmallConstant, 44),
            ],
            opcode_2op(4, 12),
            0x404,
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x404);
        assert!(attribute::value(&zmachine, 1, 43).is_ok_and(|x| !x));
        assert!(attribute::value(&zmachine, 1, 44).is_ok_and(|x| !x));
        assert!(attribute::value(&zmachine, 1, 45).is_ok_and(|x| !x));
    }

    #[test]
    fn test_clear_attr_v4_invalid() {
        let mut map = test_map(4);
        // Set attributes 0, 4, 9, 14, 19, 24, 29, 34, 39, 44, and 47
        mock_attributes(&mut map, 1, &[0x88, 0x42, 0x10, 0x84, 0x21, 0x09]);
        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 0x01),
                operand(OperandType::SmallConstant, 48),
            ],
            opcode_2op(4, 12),
            0x404,
        );
        assert!(dispatch(&mut zmachine, &i).is_err());
    }

    #[test]
    fn test_store() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 0x80),
                operand(OperandType::LargeConstant, 0xFEDC),
            ],
            opcode_2op(3, 13),
            0x404,
        );

        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x404);
        assert_ok_eq!(zmachine.variable(0x80), 0xFEDC);
    }

    #[test]
    fn test_store_sp() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        assert!(zmachine.set_variable(0, 0x1234).is_ok());
        assert!(zmachine.set_variable(0, 0x5678).is_ok());
        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 0x00),
                operand(OperandType::LargeConstant, 0xFEDC),
            ],
            opcode_2op(3, 13),
            0x404,
        );

        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x404);
        assert_ok_eq!(zmachine.variable(0), 0xFEDC);
        assert_ok_eq!(zmachine.variable(0), 0x1234);
        assert!(zmachine.variable(0).is_err());
    }

    #[test]
    fn test_insert_obj_first_child() {
        let mut map = test_map(3);
        mock_object(&mut map, 1, vec![], (0, 0, 2));
        mock_object(&mut map, 2, vec![], (1, 3, 4));
        mock_object(&mut map, 5, vec![], (0, 0, 6));
        mock_object(&mut map, 6, vec![], (5, 7, 8));
        mock_object(&mut map, 7, vec![], (5, 9, 10));
        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 0x06),
                operand(OperandType::SmallConstant, 0x01),
            ],
            opcode_2op(3, 14),
            0x403,
        );

        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x403);
        assert_ok_eq!(object::child(&zmachine, 1), 6);
        assert_ok_eq!(object::parent(&zmachine, 6), 1);
        assert_ok_eq!(object::sibling(&zmachine, 6), 2);
        assert_ok_eq!(object::child(&zmachine, 6), 8);
        assert_ok_eq!(object::parent(&zmachine, 2), 1);
        assert_ok_eq!(object::child(&zmachine, 5), 7);
        assert_ok_eq!(object::parent(&zmachine, 7), 5);
        assert_ok_eq!(object::sibling(&zmachine, 7), 9);
    }

    #[test]
    fn test_insert_obj_middle_child() {
        let mut map = test_map(3);
        mock_object(&mut map, 1, vec![], (0, 0, 2));
        mock_object(&mut map, 2, vec![], (1, 3, 4));
        mock_object(&mut map, 5, vec![], (0, 0, 6));
        mock_object(&mut map, 6, vec![], (5, 7, 8));
        mock_object(&mut map, 7, vec![], (5, 9, 10));
        mock_object(&mut map, 9, vec![], (5, 0, 0));

        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 0x07),
                operand(OperandType::SmallConstant, 0x01),
            ],
            opcode_2op(3, 14),
            0x403,
        );

        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x403);
        assert_ok_eq!(object::child(&zmachine, 1), 7);
        assert_ok_eq!(object::parent(&zmachine, 7), 1);
        assert_ok_eq!(object::sibling(&zmachine, 7), 2);
        assert_ok_eq!(object::parent(&zmachine, 2), 1);
        assert_ok_eq!(object::child(&zmachine, 5), 6);
        assert_ok_eq!(object::parent(&zmachine, 6), 5);
        assert_ok_eq!(object::sibling(&zmachine, 6), 9);
        assert_ok_eq!(object::parent(&zmachine, 9), 5);
    }

    #[test]
    fn test_insert_obj_last_child() {
        let mut map = test_map(3);
        mock_object(&mut map, 1, vec![], (0, 0, 2));
        mock_object(&mut map, 2, vec![], (1, 3, 4));
        mock_object(&mut map, 5, vec![], (0, 0, 6));
        mock_object(&mut map, 6, vec![], (5, 7, 8));
        mock_object(&mut map, 7, vec![], (5, 9, 10));
        mock_object(&mut map, 9, vec![], (5, 0, 0));

        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 0x09),
                operand(OperandType::SmallConstant, 0x01),
            ],
            opcode_2op(3, 14),
            0x403,
        );

        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x403);
        assert_ok_eq!(object::child(&zmachine, 1), 9);
        assert_ok_eq!(object::parent(&zmachine, 9), 1);
        assert_ok_eq!(object::sibling(&zmachine, 9), 2);
        assert_ok_eq!(object::parent(&zmachine, 2), 1);
        assert_ok_eq!(object::child(&zmachine, 5), 6);
        assert_ok_eq!(object::parent(&zmachine, 6), 5);
        assert_ok_eq!(object::sibling(&zmachine, 6), 7);
        assert_ok_eq!(object::parent(&zmachine, 7), 5);
        assert_ok_eq!(object::sibling(&zmachine, 7), 0);
    }

    #[test]
    fn test_loadw() {
        let mut map = test_map(3);
        map[0x608] = 0x12;
        map[0x609] = 0x34;
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x600),
                operand(OperandType::SmallConstant, 0x04),
            ],
            opcode_2op(3, 15),
            0x405,
            store(0x404, 0x80),
        );

        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x405);
        assert_ok_eq!(zmachine.variable(0x80), 0x1234);
    }

    #[test]
    fn test_loadb() {
        let mut map = test_map(3);
        map[0x604] = 0x12;
        map[0x605] = 0x34;
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x600),
                operand(OperandType::SmallConstant, 0x04),
            ],
            opcode_2op(3, 16),
            0x405,
            store(0x404, 0x80),
        );

        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x405);
        assert_ok_eq!(zmachine.variable(0x80), 0x12);
    }

    #[test]
    fn test_get_prop_v3_byte() {
        let mut map = test_map(3);
        mock_default_properties(&mut map);
        mock_object(&mut map, 1, vec![], (0, 0, 0));
        mock_properties(
            &mut map,
            1,
            &[
                (20, &vec![0x12, 0x34]),
                (15, &vec![0x56]),
                (10, &vec![0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x11, 0x22]),
            ],
        );
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 0x01),
                operand(OperandType::SmallConstant, 15),
            ],
            opcode_2op(3, 17),
            0x404,
            store(0x403, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x404);
        assert_ok_eq!(zmachine.variable(0x80), 0x56);
    }

    #[test]
    fn test_get_prop_v3_word() {
        let mut map = test_map(3);
        mock_default_properties(&mut map);
        mock_object(&mut map, 1, vec![], (0, 0, 0));
        mock_properties(
            &mut map,
            1,
            &[
                (20, &vec![0x12, 0x34]),
                (15, &vec![0x56]),
                (10, &vec![0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x11, 0x22]),
            ],
        );
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 0x01),
                operand(OperandType::SmallConstant, 20),
            ],
            opcode_2op(3, 17),
            0x404,
            store(0x403, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x404);
        assert_ok_eq!(zmachine.variable(0x80), 0x1234);
    }

    #[test]
    fn test_get_prop_v3_default() {
        let mut map = test_map(3);
        mock_default_properties(&mut map);
        mock_object(&mut map, 1, vec![], (0, 0, 0));
        mock_properties(
            &mut map,
            1,
            &[
                (20, &vec![0x12, 0x34]),
                (15, &vec![0x56]),
                (10, &vec![0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x11, 0x22]),
            ],
        );
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 0x01),
                operand(OperandType::SmallConstant, 21),
            ],
            opcode_2op(3, 17),
            0x404,
            store(0x403, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x404);
        assert_ok_eq!(zmachine.variable(0x80), 0x414);
    }

    #[test]
    fn test_get_prop_v3_too_long() {
        let mut map = test_map(3);
        mock_default_properties(&mut map);
        mock_object(&mut map, 1, vec![], (0, 0, 0));
        mock_properties(
            &mut map,
            1,
            &[
                (20, &vec![0x12, 0x34]),
                (15, &vec![0x56]),
                (10, &vec![0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x11, 0x22]),
            ],
        );
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 0x01),
                operand(OperandType::SmallConstant, 10),
            ],
            opcode_2op(3, 17),
            0x404,
            store(0x403, 0x80),
        );
        assert!(dispatch(&mut zmachine, &i).is_err());
    }

    #[test]
    fn test_get_prop_v4_byte() {
        let mut map = test_map(4);
        mock_default_properties(&mut map);
        mock_object(&mut map, 1, vec![], (0, 0, 0));
        mock_properties(
            &mut map,
            1,
            &[
                (20, &vec![0x12, 0x34]),
                (15, &vec![0x56]),
                (10, &vec![0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x11, 0x22]),
            ],
        );
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 0x01),
                operand(OperandType::SmallConstant, 15),
            ],
            opcode_2op(4, 17),
            0x404,
            store(0x403, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x404);
        assert_ok_eq!(zmachine.variable(0x80), 0x56);
    }

    #[test]
    fn test_get_prop_v4_word() {
        let mut map = test_map(4);
        mock_default_properties(&mut map);
        mock_object(&mut map, 1, vec![], (0, 0, 0));
        mock_properties(
            &mut map,
            1,
            &[
                (20, &vec![0x12, 0x34]),
                (15, &vec![0x56]),
                (10, &vec![0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x11, 0x22]),
            ],
        );
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 0x01),
                operand(OperandType::SmallConstant, 20),
            ],
            opcode_2op(4, 17),
            0x404,
            store(0x403, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x404);
        assert_ok_eq!(zmachine.variable(0x80), 0x1234);
    }

    #[test]
    fn test_get_prop_v4_default() {
        let mut map = test_map(4);
        mock_default_properties(&mut map);
        mock_object(&mut map, 1, vec![], (0, 0, 0));
        mock_properties(
            &mut map,
            1,
            &[
                (20, &vec![0x12, 0x34]),
                (15, &vec![0x56]),
                (10, &vec![0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x11, 0x22]),
            ],
        );
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 0x01),
                operand(OperandType::SmallConstant, 60),
            ],
            opcode_2op(4, 17),
            0x404,
            store(0x403, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x404);
        assert_ok_eq!(zmachine.variable(0x80), 0xb3b);
    }

    #[test]
    fn test_get_prop_v4_too_long() {
        let mut map = test_map(3);
        mock_default_properties(&mut map);
        mock_object(&mut map, 1, vec![], (0, 0, 0));
        mock_properties(
            &mut map,
            1,
            &[
                (20, &vec![0x12, 0x34]),
                (15, &vec![0x56]),
                (10, &vec![0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x11, 0x22]),
            ],
        );
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 0x01),
                operand(OperandType::SmallConstant, 10),
            ],
            opcode_2op(4, 17),
            0x404,
            store(0x403, 0x80),
        );
        assert!(dispatch(&mut zmachine, &i).is_err());
    }

    #[test]
    fn test_get_prop_addr_v3() {
        let mut map = test_map(3);
        mock_default_properties(&mut map);
        mock_object(&mut map, 1, vec![], (0, 0, 0));
        mock_properties(
            &mut map,
            1,
            &[
                (20, &vec![0x12, 0x34]),
                (15, &vec![0x56]),
                (10, &vec![0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x11, 0x22]),
            ],
        );
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 0x01),
                operand(OperandType::SmallConstant, 15),
            ],
            opcode_2op(3, 18),
            0x404,
            store(0x403, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x404);
        assert_ok_eq!(zmachine.variable(0x80), 0x305);
    }

    #[test]
    fn test_get_prop_addr_v3_no_prop() {
        let mut map = test_map(3);
        mock_default_properties(&mut map);
        mock_object(&mut map, 1, vec![], (0, 0, 0));
        mock_properties(
            &mut map,
            1,
            &[
                (20, &vec![0x12, 0x34]),
                (15, &vec![0x56]),
                (10, &vec![0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x11, 0x22]),
            ],
        );
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 0x01),
                operand(OperandType::SmallConstant, 16),
            ],
            opcode_2op(3, 18),
            0x404,
            store(0x403, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x404);
        assert_ok_eq!(zmachine.variable(0x80), 0);
    }

    #[test]
    fn test_get_prop_addr_object_0() {
        let mut map = test_map(3);
        mock_default_properties(&mut map);
        mock_object(&mut map, 1, vec![], (0, 0, 0));
        mock_properties(
            &mut map,
            1,
            &[
                (20, &vec![0x12, 0x34]),
                (15, &vec![0x56]),
                (10, &vec![0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x11, 0x22]),
            ],
        );
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 0),
                operand(OperandType::SmallConstant, 10),
            ],
            opcode_2op(3, 18),
            0x404,
            store(0x403, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x404);
        assert_ok_eq!(zmachine.variable(0x80), 0);
    }

    #[test]
    fn test_get_prop_addr_v4() {
        let mut map = test_map(4);
        mock_default_properties(&mut map);
        mock_object(&mut map, 1, vec![], (0, 0, 0));
        mock_properties(
            &mut map,
            1,
            &[
                (20, &vec![0x12, 0x34]),
                (15, &vec![0x56]),
                (10, &vec![0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x11, 0x22]),
            ],
        );
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 0x01),
                operand(OperandType::SmallConstant, 10),
            ],
            opcode_2op(4, 18),
            0x404,
            store(0x403, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x404);
        assert_ok_eq!(zmachine.variable(0x80), 0x308);
    }

    #[test]
    fn test_get_prop_addr_v4_no_prop() {
        let mut map = test_map(4);
        mock_default_properties(&mut map);
        mock_object(&mut map, 1, vec![], (0, 0, 0));
        mock_properties(
            &mut map,
            1,
            &[
                (20, &vec![0x12, 0x34]),
                (15, &vec![0x56]),
                (10, &vec![0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x11, 0x22]),
            ],
        );
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 0x01),
                operand(OperandType::SmallConstant, 16),
            ],
            opcode_2op(4, 18),
            0x404,
            store(0x403, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x404);
        assert_ok_eq!(zmachine.variable(0x80), 0);
    }

    #[test]
    fn test_get_next_prop_v3() {
        let mut map = test_map(3);
        mock_default_properties(&mut map);
        mock_object(&mut map, 1, vec![], (0, 0, 0));
        mock_properties(
            &mut map,
            1,
            &[
                (20, &vec![0x12, 0x34]),
                (15, &vec![0x56]),
                (10, &vec![0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x11, 0x22]),
            ],
        );
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 0x01),
                operand(OperandType::SmallConstant, 15),
            ],
            opcode_2op(3, 19),
            0x404,
            store(0x403, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x404);
        assert_ok_eq!(zmachine.variable(0x80), 0x0A);
    }

    #[test]
    fn test_get_next_prop_v3_none() {
        let mut map = test_map(3);
        mock_default_properties(&mut map);
        mock_object(&mut map, 1, vec![], (0, 0, 0));
        mock_properties(
            &mut map,
            1,
            &[
                (20, &vec![0x12, 0x34]),
                (15, &vec![0x56]),
                (10, &vec![0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x11, 0x22]),
            ],
        );
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 0x01),
                operand(OperandType::SmallConstant, 10),
            ],
            opcode_2op(3, 19),
            0x404,
            store(0x403, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x404);
        assert_ok_eq!(zmachine.variable(0x80), 0);
    }

    #[test]
    fn test_get_next_prop_object_0() {
        let mut map = test_map(3);
        mock_default_properties(&mut map);
        mock_object(&mut map, 1, vec![], (0, 0, 0));
        mock_properties(
            &mut map,
            1,
            &[
                (20, &vec![0x12, 0x34]),
                (15, &vec![0x56]),
                (10, &vec![0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x11, 0x22]),
            ],
        );
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 0),
                operand(OperandType::SmallConstant, 10),
            ],
            opcode_2op(3, 19),
            0x404,
            store(0x403, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x404);
        assert_ok_eq!(zmachine.variable(0x80), 0);
    }

    #[test]
    fn test_get_next_prop_v3_0() {
        let mut map = test_map(3);
        mock_default_properties(&mut map);
        mock_object(&mut map, 1, vec![], (0, 0, 0));
        mock_properties(
            &mut map,
            1,
            &[
                (20, &vec![0x12, 0x34]),
                (15, &vec![0x56]),
                (10, &vec![0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x11, 0x22]),
            ],
        );
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 0x01),
                operand(OperandType::SmallConstant, 0),
            ],
            opcode_2op(3, 19),
            0x404,
            store(0x403, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x404);
        assert_ok_eq!(zmachine.variable(0x80), 0x14);
    }

    #[test]
    fn test_get_next_prop_v3_no_start_prop() {
        let mut map = test_map(3);
        mock_default_properties(&mut map);
        mock_object(&mut map, 1, vec![], (0, 0, 0));
        mock_properties(
            &mut map,
            1,
            &[
                (20, &vec![0x12, 0x34]),
                (15, &vec![0x56]),
                (10, &vec![0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x11, 0x22]),
            ],
        );
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 0x01),
                operand(OperandType::SmallConstant, 12),
            ],
            opcode_2op(3, 19),
            0x404,
            store(0x403, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x404);
        assert_ok_eq!(zmachine.variable(0x80), 0);
    }

    #[test]
    fn test_get_next_prop_v4() {
        let mut map = test_map(4);
        mock_default_properties(&mut map);
        mock_object(&mut map, 1, vec![], (0, 0, 0));
        mock_properties(
            &mut map,
            1,
            &[
                (20, &vec![0x12, 0x34]),
                (15, &vec![0x56]),
                (10, &vec![0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x11, 0x22]),
            ],
        );
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 0x01),
                operand(OperandType::SmallConstant, 15),
            ],
            opcode_2op(4, 19),
            0x404,
            store(0x403, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x404);
        assert_ok_eq!(zmachine.variable(0x80), 0x0A);
    }

    #[test]
    fn test_get_next_prop_v4_none() {
        let mut map = test_map(4);
        mock_default_properties(&mut map);
        mock_object(&mut map, 1, vec![], (0, 0, 0));
        mock_properties(
            &mut map,
            1,
            &[
                (20, &vec![0x12, 0x34]),
                (15, &vec![0x56]),
                (10, &vec![0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x11, 0x22]),
            ],
        );
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 0x01),
                operand(OperandType::SmallConstant, 10),
            ],
            opcode_2op(4, 19),
            0x404,
            store(0x403, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x404);
        assert_ok_eq!(zmachine.variable(0x80), 0);
    }

    #[test]
    fn test_get_next_prop_v4_0() {
        let mut map = test_map(3);
        mock_default_properties(&mut map);
        mock_object(&mut map, 1, vec![], (0, 0, 0));
        mock_properties(
            &mut map,
            1,
            &[
                (20, &vec![0x12, 0x34]),
                (15, &vec![0x56]),
                (10, &vec![0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x11, 0x22]),
            ],
        );
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 0x01),
                operand(OperandType::SmallConstant, 0),
            ],
            opcode_2op(3, 19),
            0x404,
            store(0x403, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x404);
        assert_ok_eq!(zmachine.variable(0x80), 0x14);
    }

    #[test]
    fn test_add() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x1234),
                operand(OperandType::LargeConstant, 0x123),
            ],
            opcode_2op(3, 20),
            0x406,
            store(0x405, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x406);
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0x1357))
    }

    #[test]
    fn test_add_negative() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x1234),
                operand(OperandType::LargeConstant, 0xFFFF),
            ],
            opcode_2op(3, 20),
            0x406,
            store(0x405, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x406);
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0x1233))
    }

    #[test]
    fn test_add_overflow() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x7FFF),
                operand(OperandType::LargeConstant, 0x1),
            ],
            opcode_2op(3, 20),
            0x406,
            store(0x405, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x406);
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0x8000))
    }

    #[test]
    fn test_sub() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x1234),
                operand(OperandType::LargeConstant, 0x123),
            ],
            opcode_2op(3, 21),
            0x406,
            store(0x405, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x406);
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0x1111))
    }

    #[test]
    fn test_sub_negative() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x1234),
                operand(OperandType::LargeConstant, 0xFFFF),
            ],
            opcode_2op(3, 21),
            0x406,
            store(0x405, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x406);
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0x1235))
    }

    #[test]
    fn test_sub_overflow() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x8000),
                operand(OperandType::LargeConstant, 0x1),
            ],
            opcode_2op(3, 21),
            0x406,
            store(0x405, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x406);
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0x7FFF))
    }

    #[test]
    fn test_mul() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x1234),
                operand(OperandType::LargeConstant, 0x2),
            ],
            opcode_2op(3, 22),
            0x406,
            store(0x405, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x406);
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0x2468))
    }

    #[test]
    fn test_mul_negative() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0xFFFF),
                operand(OperandType::LargeConstant, 0xFFFF),
            ],
            opcode_2op(3, 22),
            0x406,
            store(0x405, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x406);
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0x1))
    }

    #[test]
    fn test_mul_overflow() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x8000),
                operand(OperandType::LargeConstant, 0xFFFF),
            ],
            opcode_2op(3, 22),
            0x406,
            store(0x405, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x406);
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0x8000))
    }

    #[test]
    fn test_div() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x2468),
                operand(OperandType::LargeConstant, 0x2),
            ],
            opcode_2op(3, 23),
            0x406,
            store(0x405, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x406);
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0x1234))
    }

    #[test]
    fn test_div_negative() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x2),
                operand(OperandType::LargeConstant, 0xFFFE),
            ],
            opcode_2op(3, 23),
            0x406,
            store(0x405, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x406);
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0xFFFF))
    }

    #[test]
    fn test_div_overflow() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x8000),
                operand(OperandType::LargeConstant, 0xFFFF),
            ],
            opcode_2op(3, 23),
            0x406,
            store(0x405, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x406);
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0x8000))
    }

    #[test]
    fn test_div_by_0() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x8000),
                operand(OperandType::LargeConstant, 0),
            ],
            opcode_2op(3, 23),
            0x406,
            store(0x405, 0x80),
        );
        assert!(dispatch(&mut zmachine, &i).is_err());
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0))
    }

    #[test]
    fn test_mod() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 13),
                operand(OperandType::LargeConstant, 5),
            ],
            opcode_2op(3, 24),
            0x406,
            store(0x405, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x406);
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 3))
    }

    #[test]
    fn test_mod_negative1() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0xFFF3),
                operand(OperandType::LargeConstant, 0x5),
            ],
            opcode_2op(3, 24),
            0x406,
            store(0x405, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x406);
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0xFFFD))
    }

    #[test]
    fn test_mod_negative2() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 13),
                operand(OperandType::LargeConstant, 0xFFFB),
            ],
            opcode_2op(3, 24),
            0x406,
            store(0x405, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x406);
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 3))
    }

    #[test]
    fn test_mod_by_0() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x8000),
                operand(OperandType::LargeConstant, 0),
            ],
            opcode_2op(3, 24),
            0x406,
            store(0x405, 0x80),
        );
        assert!(dispatch(&mut zmachine, &i).is_err());
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0))
    }

    #[test]
    fn test_call_2s_v4() {
        let mut map = test_map(4);
        mock_routine(&mut map, 0x600, &[0x1234, 0x5678, 0x9abc]);
        let mut zmachine = mock_zmachine(map);
        assert!(zmachine.push(0xabcd).is_ok());
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x180),
                operand(OperandType::LargeConstant, 0xF0AD),
            ],
            opcode_2op(4, 25),
            0x406,
            store(0x405, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x607);
        assert_ok_eq!(zmachine.variable(1), 0xF0AD);
        assert_ok_eq!(zmachine.variable(2), 0x5678);
        assert_ok_eq!(zmachine.variable(3), 0x9abc);
        assert!(zmachine.variable(0).is_err());
        assert_ok_eq!(zmachine.return_routine(0x9876), 0x406);
        assert_ok_eq!(zmachine.variable(0x80), 0x9876);
        assert_ok_eq!(zmachine.variable(0), 0xabcd);
    }

    #[test]
    fn test_call_2s_v5() {
        let mut map = test_map(5);
        mock_routine(&mut map, 0x600, &[0x1234, 0x5678, 0x9abc]);
        let mut zmachine = mock_zmachine(map);
        assert!(zmachine.push(0xabcd).is_ok());
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x180),
                operand(OperandType::LargeConstant, 0xF0AD),
            ],
            opcode_2op(5, 25),
            0x406,
            store(0x405, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x601);
        assert_ok_eq!(zmachine.variable(1), 0xF0AD);
        assert_ok_eq!(zmachine.variable(2), 0);
        assert_ok_eq!(zmachine.variable(3), 0);
        assert!(zmachine.variable(0).is_err());
        assert_ok_eq!(zmachine.return_routine(0x9876), 0x406);
        assert_ok_eq!(zmachine.variable(0x80), 0x9876);
        assert_ok_eq!(zmachine.variable(0), 0xabcd);
    }

    #[test]
    fn test_call_2s_v8() {
        let mut map = test_map(8);
        mock_routine(&mut map, 0x600, &[0x1234, 0x5678, 0x9abc]);
        let mut zmachine = mock_zmachine(map);
        assert!(zmachine.push(0xabcd).is_ok());
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0xC0),
                operand(OperandType::LargeConstant, 0xF0AD),
            ],
            opcode_2op(8, 25),
            0x406,
            store(0x405, 0x80),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x601);
        assert_ok_eq!(zmachine.variable(1), 0xF0AD);
        assert_ok_eq!(zmachine.variable(2), 0);
        assert_ok_eq!(zmachine.variable(3), 0);
        assert!(zmachine.variable(0).is_err());
        assert_ok_eq!(zmachine.return_routine(0x9876), 0x406);
        assert_ok_eq!(zmachine.variable(0x80), 0x9876);
        assert_ok_eq!(zmachine.variable(0), 0xabcd);
    }

    #[test]
    fn test_call_2n_v5() {
        let mut map = test_map(5);
        mock_routine(&mut map, 0x600, &[0x1234, 0x5678, 0x9abc]);
        let mut zmachine = mock_zmachine(map);
        assert!(zmachine.push(0xabcd).is_ok());
        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x180),
                operand(OperandType::LargeConstant, 0xF0AD),
            ],
            opcode_2op(5, 26),
            0x406,
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x601);
        assert_ok_eq!(zmachine.variable(1), 0xF0AD);
        assert_ok_eq!(zmachine.variable(2), 0);
        assert_ok_eq!(zmachine.variable(3), 0);
        assert!(zmachine.variable(0).is_err());
        assert_ok_eq!(zmachine.return_routine(0x9876), 0x406);
        assert_ok_eq!(zmachine.variable(0x80), 0);
        assert_ok_eq!(zmachine.variable(0), 0xabcd);
    }

    #[test]
    fn test_call_2n_v8() {
        let mut map = test_map(8);
        mock_routine(&mut map, 0x600, &[0x1234, 0x5678, 0x9abc]);
        let mut zmachine = mock_zmachine(map);
        assert!(zmachine.push(0xabcd).is_ok());
        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0xC0),
                operand(OperandType::LargeConstant, 0xF0AD),
            ],
            opcode_2op(8, 26),
            0x406,
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x601);
        assert_ok_eq!(zmachine.variable(1), 0xF0AD);
        assert_ok_eq!(zmachine.variable(2), 0);
        assert_ok_eq!(zmachine.variable(3), 0);
        assert!(zmachine.variable(0).is_err());
        assert_ok_eq!(zmachine.return_routine(0x9876), 0x406);
        assert_ok_eq!(zmachine.variable(0x80), 0);
        assert_ok_eq!(zmachine.variable(0), 0xabcd);
    }

    #[test]
    fn test_set_colour() {
        let map = test_map(5);
        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 2),
                operand(OperandType::SmallConstant, 3),
            ],
            opcode_2op(5, 27),
            0x403,
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x403);
        assert_eq!(colors(), (2, 3));
    }

    #[test]
    fn test_throw() {
        let map = test_map(5);
        let mut zmachine = mock_zmachine(map);
        mock_frame(&mut zmachine, 0x500, Some(0x80), 0x401);
        mock_frame(&mut zmachine, 0x600, None, 0x501);
        mock_frame(&mut zmachine, 0x700, Some(0x81), 0x601);
        assert_eq!(zmachine.frame_count(), 4);
        let i = mock_instruction(
            0x701,
            vec![
                operand(OperandType::LargeConstant, 0x1234),
                operand(OperandType::SmallConstant, 2),
            ],
            opcode_2op(5, 28),
            0x705,
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x401);
        assert_eq!(zmachine.frame_count(), 1);
        assert_ok_eq!(zmachine.variable(0x80), 0x1234);
    }
}
