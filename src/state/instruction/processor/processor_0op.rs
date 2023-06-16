use super::*;
use crate::state::State;
use crate::state::text;

pub fn rtrue(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    state.return_routine(1)
}

pub fn rfalse(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
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

    todo!()
}

pub fn restore(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    todo!()
}

// pub fn restart(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     todo!()
// }

pub fn ret_popped(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
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

// pub fn quit(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     pancurses::reset_shell_mode();
//     pancurses::curs_set(1);
//     process::exit(0);
// }

pub fn new_line(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    state.new_line()?;
    // context.new_line();
    Ok(instruction.next_address())
}

pub fn show_status(
    state: &mut State,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
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
