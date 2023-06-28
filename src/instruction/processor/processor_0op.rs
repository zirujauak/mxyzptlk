use crate::error::{ErrorCode, RuntimeError};
use crate::instruction::{decoder, Instruction};
use crate::text;
use crate::zmachine::state::header::HeaderField;
use crate::zmachine::ZMachine;

use super::branch;
use super::store_result;

pub fn rtrue(zmachine: &mut ZMachine, _instruction: &Instruction) -> Result<usize, RuntimeError> {
    zmachine.return_routine(1)
}

pub fn rfalse(zmachine: &mut ZMachine, _instruction: &Instruction) -> Result<usize, RuntimeError> {
    zmachine.return_routine(0)
}

pub fn print(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let ztext = zmachine.string_literal(instruction.address() + 1)?;
    let text = text::from_vec(zmachine, &ztext)?;
    zmachine.print(&text)?;
    Ok(instruction.next_address() + (ztext.len() * 2))
}

pub fn print_ret(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    let ztext = zmachine.string_literal(instruction.address + 1)?;
    let text = text::from_vec(zmachine, &ztext)?;

    zmachine.print(&text)?;
    zmachine.new_line()?;

    zmachine.return_routine(1)
}

fn save_result(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
    success: bool,
) -> Result<usize, RuntimeError> {
    if zmachine.version() == 3 {
        branch(zmachine, instruction, success)
    } else {
        store_result(zmachine, instruction, if success { 1 } else { 0 })?;
        Ok(instruction.next_address())
    }
}

pub fn save(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let pc = if zmachine.version() == 3 {
        match instruction.branch() {
            Some(b) => b.address,
            None => {
                return Err(RuntimeError::new(
                    ErrorCode::Save,
                    "V3 SAVE should be a branch instruction".to_string(),
                ))
            }
        }
    } else {
        match instruction.store() {
            Some(r) => r.address,
            None => {
                return Err(RuntimeError::new(
                    ErrorCode::Save,
                    "V4 SAVE should be a store instruction".to_string(),
                ))
            }
        }
    };

    let save_data = zmachine.save(pc)?;
    match zmachine.prompt_and_write("Save to: ", "ifzs", &save_data) {
        Ok(_) => save_result(zmachine, instruction, true),
        Err(e) => {
            zmachine.print(
                &format!("Error writing save file: {}\r", e)
                    .chars()
                    .map(|c| c as u16)
                    .collect(),
            )?;
            save_result(zmachine, instruction, false)
        }
    }
}

pub fn restore(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    match zmachine.prompt_and_read("Restore from: ", "ifzs") {
        Ok(save_data) => {
            match zmachine.restore(save_data) {
                Ok(address) => {
                    match address {
                        Some(a) => {
                            let i = decoder::decode_instruction(zmachine, a - 1)?;
                            if zmachine.version() == 3 {
                                // V3 is a branch
                                branch(zmachine, &i, true)
                            } else {
                                // V4 is a store
                                store_result(zmachine, instruction, 2)?;
                                Ok(i.next_address())
                            }
                        }
                        None => branch(zmachine, instruction, false),
                    }
                }
                Err(e) => {
                    zmachine.print(
                        &format!("Error reading: {}\r", e)
                            .chars()
                            .map(|c| c as u16)
                            .collect(),
                    )?;
                    if zmachine.version() == 3 {
                        branch(zmachine, instruction, false)
                    } else {
                        store_result(zmachine, instruction, 0)?;
                        Ok(instruction.next_address())
                    }
                }
            }
        }
        Err(e) => {
            zmachine.print(
                &format!("Error reading: {}\r", e)
                    .chars()
                    .map(|c| c as u16)
                    .collect(),
            )?;
            if zmachine.version() == 3 {
                branch(zmachine, instruction, false)
            } else {
                store_result(zmachine, instruction, 0)?;
                Ok(instruction.next_address())
            }
        }
    }
}

pub fn restart(zmachine: &mut ZMachine, _instruction: &Instruction) -> Result<usize, RuntimeError> {
    zmachine.restart()
}

pub fn ret_popped(
    zmachine: &mut ZMachine,
    _instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    let value = zmachine.variable(0)?;
    zmachine.return_routine(value)
}

pub fn pop(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    zmachine.variable(0)?;
    Ok(instruction.next_address())
}

pub fn catch(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let depth = zmachine.frame_count();
    store_result(zmachine, instruction, depth as u16)?;
    Ok(instruction.next_address())
}

pub fn quit(zmachine: &mut ZMachine, _instruction: &Instruction) -> Result<usize, RuntimeError> {
    zmachine.quit()?;
    Ok(0)
}

pub fn new_line(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    zmachine.new_line()?;
    // context.new_line();
    Ok(instruction.next_address())
}

pub fn show_status(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    zmachine.status_line()?;
    Ok(instruction.next_address())
}

pub fn verify(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let expected = zmachine.header_word(HeaderField::Checksum)?;
    let checksum = zmachine.checksum()?;

    branch(zmachine, instruction, expected == checksum)
}

pub fn piracy(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    branch(zmachine, instruction, true)
}
