use super::*;
use crate::state::{object::property, text};

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

pub fn storew(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    let address = operands[0] as isize + (operands[1] as i16 * 2) as isize;
    state.write_word(address as usize, operands[2])?;
    Ok(instruction.next_address())
}

pub fn storeb(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    let address = operands[0] as isize + (operands[1] as i16) as isize;
    state.write_byte(address as usize, operands[2] as u8)?;
    Ok(instruction.next_address())
}

pub fn put_prop(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;

    property::set_property(state, operands[0] as usize, operands[1] as u8, operands[2])?;
    Ok(instruction.next_address())
}

fn terminators(state: &State) -> Result<Vec<u16>, RuntimeError> {
    let mut terminators = vec!['\r' as u16];

    if header::field_byte(state.memory(), HeaderField::Version)? > 4 {
        let mut table_addr =
            header::field_word(state.memory(), HeaderField::TerminatorTable)? as usize;
        loop {
            let b = state.read_byte(table_addr)?;
            if b == 0 {
                break;
            } else if (b >= 129 && b <= 154) || b >= 252 {
                terminators.push(b as u16);
            }
            table_addr = table_addr + 1;
        }
    }

    Ok(terminators)
}

pub fn to_lower_case(c: u16) -> u8 {
    // Uppercase ASCII is 0x41 - 0x5A
    if c > 0x40 && c < 0x5b {
        // Lowercase ASCII is 0x61 - 0x7A, so OR 0x20 to convert
        (c | 0x20) as u8
    } else {
        c as u8
    }
}

fn store_parsed_entry(
    state: &mut State,
    word: &Vec<char>,
    word_start: usize,
    entry_address: usize,
    entry: u16,
) -> Result<(), RuntimeError> {
    info!(target: "app::input", "READ: dictionary for {:?} => stored to ${:04x}: {:#04x}/{}/{}", word, entry_address, entry, word.len(), word_start);
    state.write_word(entry_address, entry as u16)?;
    state.write_byte(entry_address + 2, word.len() as u8)?;
    state.write_byte(entry_address + 3, word_start as u8)?;
    Ok(())
}

pub fn read(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;

    let text_buffer = operands[0] as usize;

    if let Some(v) = state.input_interrupt() {
        if v == 1 {
            state.write_byte(text_buffer + 1, 0)?;
            store_result(state, &instruction, 0)?;
            return Ok(instruction.next_address());
        }
    }

    let parse = if operands.len() > 1 {
        operands[1] as usize
    } else {
        0
    };

    info!(target: "app::input", "READ: text buffer ${:04x} / parse buffer ${:04x}", text_buffer, parse);
    let version = header::field_byte(state.memory(), HeaderField::Version)?;

    let len = if version < 5 {
        state.read_byte(text_buffer)? - 1
    } else {
        state.read_byte(text_buffer)?
    } as usize;

    let timeout = if operands.len() > 2 { operands[2] } else { 0 };
    let routine = if timeout > 0 && operands.len() > 2 {
        packed_routine_address(state.memory(), operands[3])?
    } else {
        0
    };

    let mut existing_input = Vec::new();

    if version < 4 {
        // V3 show status line before input
        state.status_line()?;
    }

    if version > 4 {
        // text buffer may contain existing input
        let existing_len = state.read_byte(text_buffer + 1)? as usize;
        for i in 0..existing_len {
            existing_input.push(state.read_byte(text_buffer + 2 + i)? as u16);
        }
        if state.input_interrupt_print {
            state.print(&existing_input)?;
        }
    }

    info!(target: "app::input", "READ initial input: {:?}", existing_input);

    let terminators = terminators(state)?;
    info!(target: "app::input", "READ terminators: {:?}", terminators);

    let input_buffer = state.read_line(&existing_input, len, &terminators, timeout * 100)?;
    let terminator = if let Some(c) = input_buffer.last() {
        if terminators.contains(c) {
            Some(c)
        } else {
            None
        }
    } else {
        None
    };

    info!(target: "app::input", "READ: input {:?}", input_buffer);
    info!(target: "app::input", "READ: terminator {:?}", terminator);

    if let None = terminator {
        state.write_byte(text_buffer + 1, input_buffer.len() as u8)?;
        for i in 0..input_buffer.len() {
            state.write_byte(text_buffer + 2 + i, input_buffer[i] as u8)?;
        }
        return state.read_interrupt(routine, instruction.address());
    }

    let end = input_buffer.len()
        - match terminator {
            Some(_) => 1,
            None => 0,
        };

    info!(target: "app::input", "READ: {} characters", end);
    // Store input to the text buffer
    if version < 5 {
        info!(target: "app::input", "READ: write input buffer to ${:04x}", text_buffer + 1);
        // Store the buffer contents
        for i in 0..end {
            state.write_byte(text_buffer + 1 + i, to_lower_case(input_buffer[i]))?;
        }
        // Terminated by a 0
        state.write_byte(text_buffer + 1 + end, 0)?;
    } else {
        info!(target: "app::input", "READ: write input buffer to ${:04x}", text_buffer + 2);
        // Store the buffer length
        state.write_byte(text_buffer + 1, end as u8)?;
        for i in 0..end {
            state.write_byte(text_buffer + 2 + i, to_lower_case(input_buffer[i]))?;
        }
    }

    // Lexical analysis
    if parse > 0 || version < 5 {
        let dictionary = header::field_word(state.memory(), HeaderField::Dictionary)? as usize;
        text::parse_text(state, version, text_buffer, parse, dictionary, false)?;
    }

    if version > 4 {
        if let Some(t) = terminator {
            info!(target: "app::input", "Store terminator {}", *t);
            store_result(state, instruction, *t)?;
        } else {
            store_result(state, instruction, 0)?;
        }
    }

    Ok(instruction.next_address())
}

pub fn print_char(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    state.print(&vec![operands[0]])?;
    // context.print_string(format!("{}", (operands[0] as u8) as char));
    Ok(instruction.next_address())
}

pub fn print_num(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    let s = format!("{}", operands[0] as i16);
    let mut text = Vec::new();
    for c in s.chars() {
        text.push(c as u16);
    }
    state.print(&text)?;
    Ok(instruction.next_address())
}

pub fn random(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;

    let range = operands[0] as i16;
    if range < 1 {
        if range == 0 || range.abs() >= 1000 {
            state.seed(range.abs() as u16)
        } else if range.abs() < 1000 {
            state.predictable(range.abs() as u16)
        }
        store_result(state, instruction, 0)?;
    } else {
        let value = state.random(range as u16);
        store_result(state, instruction, value)?;
    }

    Ok(instruction.next_address())
}

pub fn push(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    state.push(operands[0])?;
    Ok(instruction.next_address())
}

pub fn pull(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    let value = state.variable(0)?;

    if operands[0] == 0 {
        state.frame_stack.current_frame_mut()?.pop()?;
    }

    state.set_variable(operands[0] as u8, value)?;
    Ok(instruction.next_address())
}

pub fn split_window(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    state.split_window(operands[0])?;

    Ok(instruction.next_address())
}

pub fn set_window(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    state.set_window(operands[0])?;

    Ok(instruction.next_address())
}

pub fn call_vs2(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    let address = packed_routine_address(state.memory(), operands[0])?;
    let arguments = operands[1..operands.len()].to_vec();

    state.call_routine(
        address,
        &arguments,
        instruction.store,
        instruction.next_address(),
    )
}

pub fn erase_window(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    state.erase_window(operands[0] as i16)?;
    Ok(instruction.next_address())
}

// pub fn erase_line(context: &mut Context, instruction: &Instruction) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     todo!()
// }

pub fn set_cursor(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    state.set_cursor(operands[0], operands[1])?;
    Ok(instruction.next_address())
}

pub fn set_text_style(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    state.set_text_style(operands[0])?;
    Ok(instruction.next_address())
}

pub fn buffer_mode(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    state.buffer_mode(operands[0])?;
    Ok(instruction.next_address())
}

pub fn output_stream(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    let stream = operands[0] as i16;
    let table = if stream == 3 {
        Some(operands[1] as usize)
    } else {
        None
    };

    state.output_stream(stream, table)?;
    Ok(instruction.next_address())
}

// pub fn input_stream(
//     context: &mut Context,
//     instruction: &Instruction,
// ) -> Result<usize, ContextError> {
//     let operands = operand_values(context, instruction)?;
//     todo!()
// }

pub fn sound_effect(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands: Vec<u16> = operand_values(state, instruction)?;

    match operands[0] {
        1 => state.beep()?,
        2 => {
            state.beep()?;
            state.beep()?
        }
        _ => trace!(target: "app::trace", "SOUND_EFFECT not fully implemented"),
    }

    Ok(instruction.next_address())
}

pub fn read_char(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    if operands[0] != 1 {
        return Err(RuntimeError::new(
            ErrorCode::Instruction,
            format!("READ_CHAR argument 1 must be 1, was {}", operands[0]),
        ));
    }

    if let Some(interrupt) = state.input_interrupt() {
        if interrupt == 1 {
            store_result(state, instruction, 0)?;
            return Ok(instruction.next_address());
        }
    }

    let timeout = if operands.len() > 1 { operands[1] } else { 0 };
    let routine = if timeout > 0 && operands.len() > 2 {
        packed_routine_address(state.memory(), operands[2])?
    } else {
        0
    };

    let key = match state.read_key(timeout * 100)? {
        Some(key) => key,
        None => {
            if routine > 0 {
                return state.read_interrupt(routine, instruction.address());
            } else {
                return Err(RuntimeError::new(
                    ErrorCode::System,
                    "read_key returned None without a timeout".to_string(),
                ));
            }
        }
    };
    store_result(state, instruction, key)?;

    Ok(instruction.next_address())
}

pub fn scan_table(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;

    let scan = if operands.len() == 4 && operands[3] & 0x80 == 0 {
        1
    } else {
        2
    };

    let entry_size = if operands.len() == 4 {
        operands[3] & 0x3f
    } else {
        2
    } as usize;

    let len = operands[2] as usize;
    let mut condition = false;
    for i in 0..len {
        let address = operands[1] as usize + (i * entry_size);
        let value = if scan == 2 {
            state.read_word(address)?
        } else {
            state.read_byte(address)? as u16
        };

        if value == operands[0] {
            store_result(state, instruction, address as u16)?;
            condition = true;
            break;
        }
    }

    if condition == false {
        store_result(state, instruction, 0)?;
    }

    branch(state, instruction, condition)
}

pub fn not(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    store_result(state, instruction, !operands[0])?;
    Ok(instruction.next_address())
}

pub fn call_vn(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    let address = packed_routine_address(state.memory(), operands[0])?;
    let arguments = &operands[1..].to_vec();

    state.call_routine(
        address,
        arguments,
        instruction.store,
        instruction.next_address(),
    )
}

pub fn call_vn2(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    let address = packed_routine_address(state.memory(), operands[0])?;
    let arguments = &operands[1..].to_vec();

    state.call_routine(address, arguments, None, instruction.next_address())
}

pub fn tokenise(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    let text_buffer = operands[0] as usize;
    let parse_buffer = operands[1] as usize;
    let dictionary = if operands.len() > 2 {
        operands[2] as usize
    } else {
        header::field_word(state.memory(), HeaderField::Dictionary)? as usize
    };
    let flag = if operands.len() > 3 {
        operands[3] > 0
    } else {
        false
    };

    text::parse_text(
        state,
        header::field_byte(state.memory(), HeaderField::Version)?,
        text_buffer,
        parse_buffer,
        dictionary,
        flag,
    )?;
    Ok(instruction.next_address())
}

pub fn encode_text(
    state: &mut State,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    let text_buffer = operands[0] as usize;
    let length = operands[1] as usize;
    let from = operands[2] as usize;
    let dest_buffer = operands[3] as usize;

    let mut zchars = Vec::new();
    for i in 0..length {
        zchars.push(state.read_byte(text_buffer + from + i)? as u16);
    }

    let encoded_text = text::encode_text(&zchars, 3);
    
    info!(target: "app::input", "Encoded text: {:04x} {:04x} {:04x}", encoded_text[0], encoded_text[1], encoded_text[2]);
    for i in 0..encoded_text.len() {
        state.write_word(dest_buffer + (i * 2), encoded_text[i])?
    }

    Ok(instruction.next_address())
}

pub fn copy_table(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;

    let src = operands[0] as usize;
    let dst = operands[1] as usize;
    let len = operands[2] as i16;

    if dst == 0 {
        for i in 0..len as usize {
            state.write_byte(src + i, 0)?;
        }
    } else {
        if len > 0 && dst > src && dst < src + len as usize {
            for i in (0..len as usize).rev() {
                state.write_byte(dst + i, state.read_byte(src + i)?)?;
            }
        } else {
            for i in 0..len.abs() as usize {
                state.write_byte(dst + i, state.read_byte(src + i)?)?;
            }
        }
    }

    Ok(instruction.next_address())
}

pub fn print_table(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    let table = operands[0] as usize;
    let width = operands[1] as usize;
    let height = if operands.len() > 2 { operands[2] } else { 1 } as usize;
    let skip = if operands.len() > 3 { operands[3] } else { 0 } as usize;

    let origin = state.cursor()?;
    for i in 0..height as usize {
        state.set_cursor(origin.0 + i as u16, origin.1)?;
        for j in 0..width {
            let offset = i * (width + skip);
            state.print(&vec![state.read_byte(table + offset + j)? as u16])?;
        }
    }

    Ok(instruction.next_address())
}

pub fn check_arg_count(
    state: &mut State,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;

    branch(
        state,
        instruction,
        state.frame_stack().current_frame()?.argument_count() >= operands[0] as u8,
    )
}
