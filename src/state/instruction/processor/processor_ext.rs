use super::*;

pub fn save(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    if operands.len() > 0 {
        Err(RuntimeError::new(
            ErrorCode::UnimplementedInstruction,
            "SAVE table not implemented".to_string(),
        ))
    } else {
        let save_data = state.prepare_save(instruction.store().unwrap().address)?;
        match state.prompt_and_write("Save to: ", "ifzs", &save_data) {
            Ok(_) => {
                store_result(state, instruction, 1)?;
            }
            Err(e) => {
                state.print(
                    &format!("Error writing: {}\r", e)
                        .chars()
                        .map(|c| c as u16)
                        .collect(),
                )?;
                store_result(state, instruction, 0)?;
            }
        }
        Ok(instruction.next_address())
    }
}

pub fn restore(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    if operands.len() > 0 {
        Err(RuntimeError::new(
            ErrorCode::UnimplementedInstruction,
            "RESTORE table not implemented".to_string(),
        ))
    } else {
        match state.prompt_and_read("Restore from: ", "ifzs") {
            Ok(save_data) => match state.restore(save_data) {
                Ok(address) => match address {
                    Some(a) => {
                        let i = decoder::decode_instruction(state.memory(), a - 3)?;
                        store_result(state, &i, 2)?;
                        Ok(i.next_address())
                    }
                    None => {
                        store_result(state, instruction, 0)?;
                        Ok(instruction.next_address())
                    }
                },
                Err(e) => {
                    state.print(
                        &format!("Error restoring: {}\r", e)
                            .chars()
                            .map(|c| c as u16)
                            .collect(),
                    )?;
                    store_result(state, instruction, 0)?;
                    Ok(instruction.next_address())
                }
            },
            Err(e) => {
                state.print(
                    &format!("Error reading: {}\r", e)
                        .chars()
                        .map(|c| c as u16)
                        .collect(),
                )?;
                store_result(state, instruction, 0)?;
                Ok(instruction.next_address())
            }
        }
    }
}

pub fn log_shift(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
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

pub fn art_shift(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
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

pub fn set_font(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    let result = state.set_font(operands[0])?;
    store_result(state, instruction, result)?;
    Ok(instruction.next_address())
}

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
        Ok(pc) => match pc {
            Some(address) => {
                let i = decoder::decode_instruction(state.memory(), address - 3)?;
                store_result(state, &i, 2)?;
                Ok(i.next_address())
            }
            None => {
                store_result(state, instruction, 0)?;
                Ok(instruction.next_address())
            }
        },
        Err(e) => {
            error!(target: "app::quetzal", "Error restoring from undo state: {}", e);
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
