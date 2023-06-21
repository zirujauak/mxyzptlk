use crate::iff::quetzal::Quetzal;

use super::*;

pub fn save(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    if operands.len() > 0 {
        Err(RuntimeError::new(
            ErrorCode::UnimplementedInstruction,
            "SAVE table not implemented".to_string(),
        ))
    } else {
        let quetzal = Quetzal::from_state(state, instruction.store().unwrap().address);
        state.prompt_and_write("Save to: ", "ifzs", &quetzal.to_vec())?;
        store_result(state, instruction, 1)?;
        Ok(instruction.next_address())
    }
}

pub fn restore(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    if operands.len() > 0 {
        Err(RuntimeError::new(
            ErrorCode::UnimplementedInstruction,
            "SAVE table not implemented".to_string(),
        ))
    } else {
        let data = state.prompt_and_read("Restore from: ", "ifzs")?;
        let quetzal = Quetzal::from_vec(&data).unwrap();
        match state.restore(quetzal) {
            Ok(address) => {
                let i = decoder::decode_instruction(state.memory(), address - 3)?;
                store_result(state, instruction, 2)?;
                Ok(i.next_address())
            }
            Err(_) => branch(state, instruction, false),
        }
    }
}

pub fn log_shift(
    state: &mut State,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    let value = operands[0];
    let places = operands[1] as i16;
    let new_value = if places < 0 && places > -16 {
        u16::overflowing_shr(value, places.abs() as u32).0
    } else if places > 0 && places < 16 {
        u16::overflowing_shl(value, places as u32).0
    } else if places == 0 {
        value
    } else {
        error!(target: "app::instruction", "LOG_SHIFT {:04x} {}?!", value, places);
        0
    };

    store_result(state, instruction, new_value)?;
    Ok(instruction.next_address())
}

pub fn art_shift(
    state: &mut State,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    let value = operands[0] as i16;
    let places = operands[1] as i16;
    let new_value = if places < 0 && places > -16 {
        i16::overflowing_shr(value, places.abs() as u32).0
    } else if places > 0 && places < 16 {
        i16::overflowing_shl(value, places as u32).0
    } else if places == 0 {
        value
    } else {
        error!(target: "app::instruction", "ART_SHIFT {:04x} {}?!", value, places);
        0
    };

    store_result(state, instruction, new_value as u16)?;
    Ok(instruction.next_address())
}

// pub fn set_font(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     todo!()
// }

// pub fn draw_picture(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     todo!()
// }

// pub fn picture_data(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     todo!()
// }

// pub fn erase_picture(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     todo!()
// }

// pub fn set_margins(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     todo!()
// }

pub fn save_undo(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    state.save_undo(instruction)?;
    store_result(state, instruction, 1)?;
    Ok(instruction.next_address())
}

pub fn restore_undo(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    match state.restore_undo() {
        Ok(pc) => {
            trace!(target: "app::trace", "state.restore_undo() -> ${:04x}", pc);
            if pc == 0 {
                store_result(state, instruction, 0)?;
                Ok(instruction.next_address())
            } else {
                let i = decoder::decode_instruction(state.memory(), pc - 3)?;
                trace!(target: "app::target", "{}", i);
                store_result(state, &i, 2)?;
                Ok(i.next_address())
            }
        }
        Err(e) => {
            trace!(target: "app::trace", "{}", e);
            store_result(state, instruction, 0)?;
            Ok(instruction.next_address())
        }
    }
}

// pub fn print_unicode(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     todo!()
// }

// pub fn check_unicode(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     todo!()
// }

// pub fn set_true_colour(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     todo!()
// }

// pub fn move_window(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     todo!()
// }

// pub fn window_size(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     todo!()
// }

// pub fn window_style(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     todo!()
// }

// pub fn get_wind_prop(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     todo!()
// }

// pub fn scroll_window(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     todo!()
// }

// pub fn pop_stack(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     todo!()
// }

// pub fn read_mouse(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     todo!()
// }

// pub fn mouse_window(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     todo!()
// }

// pub fn push_stack(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     todo!()
// }

// pub fn put_wind_prop(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     todo!()
// }

// pub fn print_form(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     todo!()
// }

// pub fn make_menu(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     todo!()
// }

// pub fn picture_table(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     todo!()
// }

// pub fn buffer_screen(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     todo!()
// }
