use super::*;
use crate::error::*;
use crate::state::State;

pub fn rtrue(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    state.return_routine(1)
}

// pub fn rfalse(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     context.return_fn(0)
// }

// fn literal_text(context: &Context, address: usize) -> Result<Vec<u16>, ContextError> {
//     let mut text = Vec::new();
//     let mut done = false;

//     while !done {
//         let w = context.read_word(address + (text.len() * 2))?;
//         done = w & 0x8000 == 0x8000;
//         text.push(w);
//     }

//     Ok(text)
// }
// pub fn print(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let ztext = literal_text(context, instruction.address() + 1)?;
//     let text = text::from_vec(context, &ztext)?;

//     context.print_string(text);
//     Ok(instruction.next_address() + (ztext.len() * 2))
// }

// pub fn print_ret(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let ztext = literal_text(context, instruction.address + 1)?;
//     let text = text::from_vec(context, &ztext)?;

//     context.print_string(text);
//     context.interpreter_mut().screen_device_mut().new_line();

//     context.return_fn(1)
// }

// pub fn save(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     todo!()
// }

// pub fn restore(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     todo!()
// }

// pub fn restart(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     todo!()
// }

// pub fn ret_popped(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let value = context.current_frame_mut().pop()?;
//     context.return_fn(value)
// }

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

// pub fn new_line(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     context.new_line();
//     Ok(instruction.next_address())
// }

// pub fn show_status(
//     context: &mut Context,
//     instruction: &Instruction,
// ) -> Result<usize, ContextError> {
//     todo!()
// }

// pub fn verify(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let expected = header::checksum(context.memory_map());
//     let checksum = context.checksum();

//     branch(context, instruction, expected == checksum)
// }

// pub fn piracy(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     branch(context, instruction, true)
// }
