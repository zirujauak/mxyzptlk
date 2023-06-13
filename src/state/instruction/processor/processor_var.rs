use std::time::{SystemTime, UNIX_EPOCH};

use super::*;

pub fn call_vs(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    let address = packed_routine_address(state.memory(), operands[0])?;
    let arguments = &operands[1..].to_vec();

    call_fn(
        state,
        address,
        instruction.next_address(),
        arguments,
        instruction.store().copied(),
    )
}

// pub fn storew(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     let address = operands[0] as isize + (operands[1] as i16 * 2) as isize;
//     context.write_word(address as usize, operands[2])?;
//     Ok(instruction.next_address())
// }

pub fn storeb(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    let address = operands[0] as isize + (operands[1] as i16) as isize;
    state.write_byte(address as usize, operands[2] as u8)?;
    Ok(instruction.next_address())
}

// pub fn put_prop(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;

//     property::set_property(
//         context,
//         operands[0] as usize,
//         operands[1] as u8,
//         operands[2],
//     )?;
//     Ok(instruction.next_address())
// }

// pub fn read(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;

//     let text_buffer = operands[0] as usize;
//     let parse = if operands.len() > 1 {
//         operands[1] as usize
//     } else {
//         0
//     };

//     let len = if context.version() < 5 {
//         context.read_byte(text_buffer)? - 1
//     } else {
//         context.read_byte(text_buffer)?
//     };

//     let timeout = if operands.len() > 2 { operands[2] } else { 0 };
//     let routine = if operands.len() > 2 { operands[3] } else { 0 };

//     if context.version() < 4 {
//         // TODO: Show status line
//     }

//     // Read input
//     let mut input = context.read_line(timeout)?;
//     // TODO: unwrap()
//     let terminator = match input.pop() {
//         Some(code) => code,
//         None => InputCode::new(0 as char, 0 as char, None, None)
//     };

//     Ok(())

// }

// pub fn print_char(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     context.print_string(format!("{}", (operands[0] as u8) as char));
//     Ok(instruction.next_address())
// }

// pub fn print_num(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;

//     context.print_string(format!("{}", operands[0] as i16));
//     Ok(instruction.next_address())
// }

// pub fn random(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;

//     let range = operands[0] as i16;
//     if range < 1 {
//         if range == 0 || range.abs() >= 1000 {
//             context.seed(range.abs() as u16)
//         } else if range.abs() < 1000 {
//             context.predictable(range.abs() as u16)
//         }
//         store_result(context, instruction, 0)?;
//     } else {
//         let value = context.interpreter_mut().rng_mut().random(range as u16);
//         store_result(context, instruction, value)?;
//     }

//     Ok(instruction.next_address())
// }

// pub fn push(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     context.current_frame_mut().push(operands[0]);
//     Ok(instruction.next_address())
// }

// pub fn pull(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     let value = context.variable(0)?;

//     if operands[0] == 0 {
//         context.current_frame_mut().pop()?;
//     }

//     context.set_variable(operands[0] as u8, value)?;
//     Ok(instruction.next_address())
// }

// pub fn split_window(
//     context: &mut Context,
//     instruction: &Instruction,
// ) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     todo!()
// }

// pub fn set_window(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     todo!()
// }

// pub fn call_vs2(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     let address = packed_routine_address(context, operands[0]);
//     let arguments = operands[1..operands.len()].to_vec();
//     call_fn(
//         context,
//         address,
//         instruction.next_address(),
//         &arguments,
//         instruction.store(),
//     )
// }

// pub fn erase_window(
//     context: &mut Context,
//     instruction: &Instruction,
// ) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     todo!()
// }

// pub fn erase_line(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     todo!()
// }

// pub fn set_cursor(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     todo!()
// }

// pub fn set_text_style(
//     context: &mut Context,
//     instruction: &Instruction,
// ) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     todo!()
// }

// pub fn buffer_mode(
//     context: &mut Context,
//     instruction: &Instruction,
// ) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     todo!()
// }

// pub fn output_stream(
//     context: &mut Context,
//     instruction: &Instruction,
// ) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     todo!()
// }

// pub fn input_stream(
//     context: &mut Context,
//     instruction: &Instruction,
// ) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     todo!()
// }

// pub fn sound_effect(
//     context: &mut Context,
//     instruction: &Instruction,
// ) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     todo!()
// }

// pub fn read_char(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     todo!()
// }

// pub fn scan_table(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;

//     let scan = if operands.len() == 4 && operands[3] & 0x80 == 0 {
//         1
//     } else {
//         2
//     };

//     let entry_size = if operands.len() == 4 {
//         operands[3] & 0x3f
//     } else {
//         2
//     } as usize;

//     let len = operands[2] as usize;
//     let mut condition = false;
//     for i in 0..len {
//         let address = operands[1] as usize + (i * entry_size);
//         let value = if scan == 2 {
//             context.read_word(address)?
//         } else {
//             context.read_byte(address)? as u16
//         };

//         if value == operands[0] {
//             store_result(context, instruction, address as u16);
//             condition = true;
//             break;
//         }
//     }

//     if condition == false {
//         store_result(context, instruction, 0);
//     }

//     branch(context, instruction, condition)
// }

// pub fn not(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     store_result(context, instruction, !operands[0])?;
//     Ok(instruction.next_address())
// }

// pub fn call_vn(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     let address = packed_routine_address(context, operands[0]);
//     let arguments = &operands[1..].to_vec();

//     call_fn(
//         context,
//         address,
//         instruction.next_address(),
//         arguments,
//         None,
//     )
// }

// pub fn call_vn2(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     let address = packed_routine_address(context, operands[0]);
//     let arguments = &operands[1..].to_vec();

//     call_fn(
//         context,
//         address,
//         instruction.next_address(),
//         arguments,
//         None,
//     )
// }

// pub fn tokenise(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     todo!()
// }

// pub fn encode_text(
//     context: &mut Context,
//     instruction: &Instruction,
// ) -> Result<usize, ContextError> {
//     todo!()
// }

// pub fn copy_table(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;

//     let src = operands[0] as usize;
//     let dst = operands[1] as usize;
//     let len = operands[2] as i16;

//     if dst == 0 {
//         for i in 0..len as usize {
//             context.write_byte(src + i, 0)?;
//         }
//     } else {
//         if len > 0 && dst > src && dst < src + len as usize {
//             for i in (0..len as usize).rev() {
//                 context.write_byte(dst + i, context.read_byte(src + i)?)?;
//             }
//         } else {
//             for i in 0..len.abs() as usize {
//                 context.write_byte(dst + i, context.read_byte(src + i)?)?;
//             }
//         }
//     }

//     Ok(instruction.next_address())
// }

// pub fn print_table(
//     context: &mut Context,
//     instruction: &Instruction,
// ) -> Result<usize, ContextError> {
//     todo!()
// }

// pub fn check_arg_count(
//     context: &mut Context,
//     instruction: &Instruction,
// ) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;

//     branch(
//         context,
//         instruction,
//         context.current_frame().argument_count() >= operands[0] as u8,
//     )
// }
