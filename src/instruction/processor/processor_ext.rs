use super::*;

pub fn save(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    if operands.len() > 0 {
        Err(RuntimeError::new(
            ErrorCode::UnimplementedInstruction,
            "SAVE table not implemented".to_string(),
        ))
    } else {
        // unwrap() should be safe here because this is a store instruction
        match zmachine.save(instruction.store().unwrap().address()) {
            Ok(_) => {
                store_result(zmachine, instruction, 1)?;
            }
            Err(_) => {
                store_result(zmachine, instruction, 0)?;
            }
        }
        Ok(instruction.next_address())
    }
}

pub fn restore(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    if operands.len() > 0 {
        Err(RuntimeError::new(
            ErrorCode::UnimplementedInstruction,
            "RESTORE table not implemented".to_string(),
        ))
    } else {
        match zmachine.restore() {
            Ok(address) => match address {
                Some(a) => {
                    let i = decoder::decode_instruction(zmachine, a - 3)?;
                    store_result(zmachine, &i, 2)?;
                    Ok(i.next_address())
                }
                None => {
                    store_result(zmachine, instruction, 0)?;
                    Ok(instruction.next_address())
                }
            },
            Err(e) => {
                zmachine.print_str(format!("Error restoring: {}\r", e))?;
                store_result(zmachine, instruction, 0)?;
                Ok(instruction.next_address())
            }
        }
    }
}

pub fn log_shift(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let value = operands[0];
    let places = operands[1] as i16;
    let new_value = if places < 0 && places > -16 {
        u16::overflowing_shr(value, places.abs() as u32).0
    } else if places > 0 && places < 16 {
        u16::overflowing_shl(value, places as u32).0
    } else if places == 0 {
        value
    } else {
        error!(target: "app::instruction", "Invalid places for LOG_SHIFT {:04x} {} [-15..15]", value, places);
        0
    };

    store_result(zmachine, instruction, new_value)?;
    Ok(instruction.next_address())
}

pub fn art_shift(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let value = operands[0] as i16;
    let places = operands[1] as i16;
    let new_value = if places < 0 && places > -16 {
        i16::overflowing_shr(value, places.abs() as u32).0
    } else if places > 0 && places < 16 {
        i16::overflowing_shl(value, places as u32).0
    } else if places == 0 {
        value
    } else {
        error!(target: "app::instruction", "Invalid places for ART_SHIFT {:04x} {} [-15..15]", value, places);
        0
    };

    store_result(zmachine, instruction, new_value as u16)?;
    Ok(instruction.next_address())
}

pub fn set_font(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let result = zmachine.set_font(operands[0])?;
    store_result(zmachine, instruction, result)?;
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

pub fn save_undo(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    // unwrap() should be safe here because this is a store instruction
    zmachine.save_undo(instruction.store().unwrap().address())?;
    store_result(zmachine, instruction, 1)?;
    Ok(instruction.next_address())
}

pub fn restore_undo(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    match zmachine.restore_undo() {
        Ok(pc) => match pc {
            Some(address) => {
                let i = decoder::decode_instruction(zmachine, address - 3)?;
                store_result(zmachine, &i, 2)?;
                Ok(i.next_address())
            }
            None => {
                store_result(zmachine, instruction, 0)?;
                Ok(instruction.next_address())
            }
        },
        Err(_) => {
            store_result(zmachine, instruction, 0)?;
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
