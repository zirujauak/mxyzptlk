use crate::executor::context::{error::ContextError, Context};

use super::*;

pub fn save(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
    let operands = operand_values(context, instruction)?;
    todo!()
}

pub fn restore(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
    let operands = operand_values(context, instruction)?;
    todo!()
}

pub fn log_shift(
    context: &mut Context,
    instruction: &Instruction,
) -> Result<usize, ContextError> {
    let operands = operand_values(context, instruction)?;
    let value = operands[0];
    let places = operands[1] as i16;
    let new_value = if places < 0 && places > -16 {
        u16::overflowing_shr(value, places.abs() as u32).0
    } else if places > 0 && places < 16 {
        u16::overflowing_shl(value, places as u32).0
    } else if places == 0 {
        value
    } else {
        todo!()
    };

    store_result(context, instruction, new_value)?;
    Ok(instruction.next_address())
}

pub fn art_shift(
    context: &mut Context,
    instruction: &Instruction,
) -> Result<usize, ContextError> {
    let operands = operand_values(context, instruction)?;
    let value = operands[0] as i16;
    let places = operands[1] as i16;
    let new_value = if places < 0 && places > -16 {
        i16::overflowing_shr(value, places.abs() as u32).0
    } else if places > 0 && places < 16 {
        i16::overflowing_shl(value, places as u32).0
    } else if places == 0 {
        value
    } else {
        todo!()
    };

    store_result(context, instruction, new_value as u16)?;
    Ok(instruction.next_address())
}

pub fn set_font(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
    let operands = operand_values(context, instruction)?;
    todo!()
}

pub fn draw_picture(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
    let operands = operand_values(context, instruction)?;
    todo!()
}

pub fn picture_data(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
    let operands = operand_values(context, instruction)?;
    todo!()
}

pub fn erase_picture(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
    let operands = operand_values(context, instruction)?;
    todo!()
}

pub fn set_margins(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
    let operands = operand_values(context, instruction)?;
    todo!()
}

pub fn save_undo(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
    let operands = operand_values(context, instruction)?;
    todo!()
}

pub fn restore_undo(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
    let operands = operand_values(context, instruction)?;
    todo!()
}

pub fn print_unicode(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
    let operands = operand_values(context, instruction)?;
    todo!()
}

pub fn check_unicode(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
    let operands = operand_values(context, instruction)?;
    todo!()
}

pub fn set_true_colour(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
    let operands = operand_values(context, instruction)?;
    todo!()
}

pub fn move_window(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
    let operands = operand_values(context, instruction)?;
    todo!()
}

pub fn window_size(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
    let operands = operand_values(context, instruction)?;
    todo!()
}

pub fn window_style(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
    let operands = operand_values(context, instruction)?;
    todo!()
}

pub fn get_wind_prop(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
    let operands = operand_values(context, instruction)?;
    todo!()
}

pub fn scroll_window(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
    let operands = operand_values(context, instruction)?;
    todo!()
}

pub fn pop_stack(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
    let operands = operand_values(context, instruction)?;
    todo!()
}

pub fn read_mouse(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
    let operands = operand_values(context, instruction)?;
    todo!()
}

pub fn mouse_window(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
    let operands = operand_values(context, instruction)?;
    todo!()
}

pub fn push_stack(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
    let operands = operand_values(context, instruction)?;
    todo!()
}

pub fn put_wind_prop(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
    let operands = operand_values(context, instruction)?;
    todo!()
}

pub fn print_form(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
    let operands = operand_values(context, instruction)?;
    todo!()
}

pub fn make_menu(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
    let operands = operand_values(context, instruction)?;
    todo!()
}

pub fn picture_table(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
    let operands = operand_values(context, instruction)?;
    todo!()
}

pub fn buffer_screen(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
    let operands = operand_values(context, instruction)?;
    todo!()
}