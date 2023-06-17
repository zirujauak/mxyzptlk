use super::*;
use crate::iff::quetzal::Quetzal;
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
    let version = header::field_byte(state.memory(), HeaderField::Version)?;
    let quetzal = Quetzal::from_state(
        state,
        if version == 3 {
            instruction.branch().unwrap().address
        } else {
            instruction.store().unwrap().address
        },
    );
    state.prompt_and_write("Save to: ", "ifzs", &quetzal.to_vec())?;

    if version == 3 {
        branch(state, instruction, true)
    } else {
        store_result(state, instruction, 1)?;
        Ok(instruction.next_address())
    }
}

pub fn restore(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let data = state.prompt_and_read("Restore from: ", "ifzs")?;
    let quetzal = Quetzal::from_vec(&data).unwrap();
    match state.restore(quetzal) {
        Ok(address) => {
            let i = decoder::decode_instruction(state.memory(), address - 1)?;
            if header::field_byte(state.memory(), HeaderField::Version)? == 3 {
                // V3 is a branch
                branch(state, &i, true)
            } else {
                // V4 is a store
                store_result(state, instruction, 2)?;
                Ok(i.next_address())
            }
        }
        Err(_) => branch(state, instruction, false),
    }
}

pub fn restart(state: &mut State, _instruction: &Instruction) -> Result<usize, RuntimeError> {
    state.restart()
}

pub fn ret_popped(state: &mut State, _instruction: &Instruction) -> Result<usize, RuntimeError> {
    let value = state.frame_stack.current_frame_mut()?.pop()?;
    state.return_routine(value)
}

// pub fn pop(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     context.current_frame_mut().pop()?;
//     Ok(instruction.next_address())
// }

// pub fn catch(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     todo!()
// }

pub fn quit(state: &mut State, _instruction: &Instruction) -> Result<usize, RuntimeError> {
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

// pub fn verify(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let expected = header::checksum(context.memory_map());
//     let checksum = context.checksum();

//     branch(context, instruction, expected == checksum)
// }

// pub fn piracy(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     branch(context, instruction, true)
// }
