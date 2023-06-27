use super::*;
use crate::state::text;
use crate::state::State;

pub fn rtrue(state: &mut State, _instruction: &Instruction) -> Result<usize, RuntimeError> {
    state.return_routine(1)
}

pub fn rfalse(state: &mut State, _instruction: &Instruction) -> Result<usize, RuntimeError> {
    state.return_routine(0)
}

fn literal_text(state: &State, address: usize) -> Result<Vec<u16>, RuntimeError> {
    let mut text = Vec::new();
    let mut done = false;

    while !done {
        let w = state.memory().read_word(address + (text.len() * 2))?;
        done = w & 0x8000 == 0x8000;
        text.push(w);
    }

    Ok(text)
}

pub fn print(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let ztext = literal_text(state, instruction.address() + 1)?;
    let text = text::from_vec(state, &ztext)?;
    state.print(&text)?;
    Ok(instruction.next_address() + (ztext.len() * 2))
}

pub fn print_ret(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let ztext = literal_text(state, instruction.address + 1)?;
    let text = text::from_vec(state, &ztext)?;

    state.print(&text)?;
    state.new_line()?;

    state.return_routine(1)
}

pub fn save(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let pc = if state.version == 3 {
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

    let save_data = state.prepare_save(pc)?;
    match state.prompt_and_write("Save to: ", "ifzs", &save_data) {
        Ok(_) => {
            if state.version == 3 {
                branch(state, instruction, true)
            } else {
                store_result(state, instruction, 1)?;
                Ok(instruction.next_address())
            }
        }
        Err(e) => {
            state.print(
                &format!("Error writing save file: {}\r", e)
                    .chars()
                    .map(|c| c as u16)
                    .collect(),
            )?;
            if state.version == 3 {
                branch(state, instruction, false)
            } else {
                store_result(state, instruction, 0)?;
                Ok(instruction.next_address())
            }
        }
    }
}

pub fn restore(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    match state.prompt_and_read("Restore from: ", "ifzs") {
        Ok(save_data) => {
            match state.restore(save_data) {
                Ok(address) => {
                    match address {
                        Some(a) => {
                            let i = decoder::decode_instruction(state.memory(), a - 1)?;
                            if state.version == 3 {
                                // V3 is a branch
                                branch(state, &i, true)
                            } else {
                                // V4 is a store
                                store_result(state, instruction, 2)?;
                                Ok(i.next_address())
                            }
                        }
                        None => branch(state, instruction, false),
                    }
                }
                Err(e) => {
                    state.print(
                        &format!("Error reading: {}\r", e)
                            .chars()
                            .map(|c| c as u16)
                            .collect(),
                    )?;
                    if state.version == 3 {
                        branch(state, instruction, false)
                    } else {
                        store_result(state, instruction, 0)?;
                        Ok(instruction.next_address())
                    }
                }
            }
        }
        Err(e) => {
            state.print(
                &format!("Error reading: {}\r", e)
                    .chars()
                    .map(|c| c as u16)
                    .collect(),
            )?;
            if state.version == 3 {
                branch(state, instruction, false)
            } else {
                store_result(state, instruction, 0)?;
                Ok(instruction.next_address())
            }
        }
    }
}

pub fn restart(state: &mut State, _instruction: &Instruction) -> Result<usize, RuntimeError> {
    state.restart()
}

pub fn ret_popped(state: &mut State, _instruction: &Instruction) -> Result<usize, RuntimeError> {
    let value = state.variable(0)?;
    state.return_routine(value)
}

pub fn pop(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    state.variable(0)?;
    Ok(instruction.next_address())
}

pub fn catch(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let depth = state.frames();
    store_result(state, instruction, depth as u16)?;
    Ok(instruction.next_address())
}

pub fn quit(state: &mut State, _instruction: &Instruction) -> Result<usize, RuntimeError> {
    state.quit()?;
    Ok(0)
}

pub fn new_line(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    state.new_line()?;
    // context.new_line();
    Ok(instruction.next_address())
}

pub fn show_status(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    state.status_line()?;
    Ok(instruction.next_address())
}

pub fn verify(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let expected = header::field_word(state, HeaderField::Checksum)?;
    let checksum = state.checksum()?;

    branch(state, instruction, expected == checksum)
}

pub fn piracy(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    branch(state, instruction, true)
}
