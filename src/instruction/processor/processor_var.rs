use std::cmp::Ordering;

use crate::{
    error::{ErrorCode, RuntimeError},
    instruction::{processor::store_result, Instruction},
    object::property,
    text,
    zmachine::{io::screen::Interrupt, state::header::HeaderField, ZMachine},
};

use super::{branch, call_fn, operand_values};

pub fn call_vs(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let address = zmachine.packed_routine_address(operands[0])?;
    let arguments = &operands[1..].to_vec();

    call_fn(
        zmachine,
        address,
        instruction.next_address(),
        arguments,
        instruction.store().copied(),
    )
}

pub fn storew(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let address = operands[0] as isize + (operands[1] as i16 * 2) as isize;
    zmachine.write_word(address as usize, operands[2])?;
    Ok(instruction.next_address())
}

pub fn storeb(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let address = operands[0] as isize + (operands[1] as i16) as isize;
    zmachine.write_byte(address as usize, operands[2] as u8)?;
    Ok(instruction.next_address())
}

pub fn put_prop(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;

    property::set_property(
        zmachine,
        operands[0] as usize,
        operands[1] as u8,
        operands[2],
    )?;
    Ok(instruction.next_address())
}

fn terminators(zmachine: &ZMachine) -> Result<Vec<u16>, RuntimeError> {
    let mut terminators = vec!['\r' as u16];

    if zmachine.version() > 4 {
        let mut table_addr = zmachine.header_word(HeaderField::TerminatorTable)? as usize;
        loop {
            let b = zmachine.read_byte(table_addr)?;
            if b == 0 {
                break;
            } else if (130..155).contains(&b) || b >= 252 {
                terminators.push(b as u16);
            }
            table_addr += 1;
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

pub fn read(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;

    let text_buffer = operands[0] as usize;

    if let Some(r) = zmachine.read_interrupt_result() {
        zmachine.clear_read_interrupt();
        if r == 1 {
            zmachine.write_byte(text_buffer + 1, 0)?;
            store_result(zmachine, instruction, 0)?;
            return Ok(instruction.next_address());
        }
    }

    let parse = if operands.len() > 1 {
        operands[1] as usize
    } else {
        0
    };

    let len = if zmachine.version() < 5 {
        zmachine.read_byte(text_buffer)? - 1
    } else {
        zmachine.read_byte(text_buffer)?
    } as usize;

    let timeout = if operands.len() > 2 { operands[2] } else { 0 };
    let routine = if timeout > 0 && operands.len() > 2 {
        zmachine.set_read_interrupt_pending();
        zmachine.packed_routine_address(operands[3])?
    } else {
        0
    };

    let mut existing_input = Vec::new();

    match zmachine.version().cmp(&4) {
        Ordering::Less => zmachine.status_line()?,
        Ordering::Greater => {
            let existing_len = zmachine.read_byte(text_buffer + 1)? as usize;
            for i in 0..existing_len {
                existing_input.push(zmachine.read_byte(text_buffer + 2 + i)? as u16);
            }
            if zmachine.input_interrupt_print() {
                zmachine.print(&existing_input)?;
            }
        }
        _ => {}
    }
    // if zmachine.version() < 4 {
    //     // V3 show status line before input
    //     zmachine.status_line()?;
    // } else if zmachine.version() > 4 {
    //     // text buffer may contain existing input
    //     let existing_len = zmachine.read_byte(text_buffer + 1)? as usize;
    //     for i in 0..existing_len {
    //         existing_input.push(zmachine.read_byte(text_buffer + 2 + i)? as u16);
    //     }
    //     if zmachine.input_interrupt_print() {
    //         zmachine.print(&existing_input)?;
    //     }
    // }

    zmachine.clear_input_interrupt_print();

    let terminators = terminators(zmachine)?;
    let input_buffer = zmachine.read_line(&existing_input, len, &terminators, timeout * 100)?;
    let terminator = input_buffer.last().filter(|&x| terminators.contains(x));
    //     c.filter
    //     if terminators.contains(c) {
    //         Some(c)
    //     } else {
    //         None
    //     }
    // } else {
    //     None
    // };

    // If there was no terminator, then input was interrupted
    if terminator.is_none() {
        // Store any input that was read before the interrupt
        zmachine.write_byte(text_buffer + 1, input_buffer.len() as u8)?;
        for (i, b) in input_buffer.iter().enumerate() {
            zmachine.write_byte(text_buffer + 2 + i, *b as u8)?;
        }

        debug!(target: "app::input", "READ interrupted");

        if zmachine.sound_interrupt().is_some() {
            if !zmachine.is_sound_playing() {
                debug!(target: "app::input", "Sound interrupt firing");
                zmachine.clear_read_interrupt();
                return zmachine.call_sound_interrupt(instruction.address());
            }
        } else if routine > 0 {
            debug!(target: "app::input", "Read interrupt firing");
            return zmachine.call_read_interrupt(routine, instruction.address());
        } else {
            return Err(RuntimeError::new(
                ErrorCode::System,
                "Read returned no terminator, but there is no interrupt to run".to_string(),
            ));
        }
    }

    let end = input_buffer.len()
        - match terminator {
            Some(_) => 1,
            None => 0,
        };

    // Store input to the text buffer
    if zmachine.version() < 5 {
        // Store the buffer contents
        for (i, b) in input_buffer.iter().enumerate() {
            if i < end {
                zmachine.write_byte(text_buffer + 1 + i, to_lower_case(*b))?;
            }
        }
        // Terminated by a 0
        zmachine.write_byte(text_buffer + 1 + end, 0)?;
    } else {
        // Store the buffer length
        zmachine.write_byte(text_buffer + 1, end as u8)?;
        for (i, b) in input_buffer.iter().enumerate() {
            if i < end {
                zmachine.write_byte(text_buffer + 2 + i, to_lower_case(*b))?;
            }
        }
    }

    // Lexical analysis
    if parse > 0 || zmachine.version() < 5 {
        let dictionary = zmachine.header_word(HeaderField::Dictionary)? as usize;
        text::parse_text(zmachine, text_buffer, parse, dictionary, false)?;
    }

    if zmachine.version() > 4 {
        if let Some(t) = terminator {
            store_result(zmachine, instruction, *t)?;
        } else {
            store_result(zmachine, instruction, 0)?;
        }
    }

    Ok(instruction.next_address())
}

pub fn print_char(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    zmachine.print(&vec![operands[0]])?;
    // context.print_string(format!("{}", (operands[0] as u8) as char));
    Ok(instruction.next_address())
}

pub fn print_num(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let s = format!("{}", operands[0] as i16);
    let mut text = Vec::new();
    for c in s.chars() {
        text.push(c as u16);
    }
    zmachine.print(&text)?;
    Ok(instruction.next_address())
}

pub fn random(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;

    let range = operands[0] as i16;
    if range < 1 {
        if range == 0 || range.abs() >= 1000 {
            zmachine.seed(range.unsigned_abs())
        } else if range.abs() < 1000 {
            zmachine.predictable(range.unsigned_abs())
        }
        store_result(zmachine, instruction, 0)?;
    } else {
        let value = zmachine.random(range as u16);
        store_result(zmachine, instruction, value)?;
    }

    Ok(instruction.next_address())
}

pub fn push(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    zmachine.push(operands[0])?;
    Ok(instruction.next_address())
}

pub fn pull(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let value = zmachine.variable(0)?;

    // If pulling to the stack, need to remove what was underneath the
    // value pulled before pushing it back.  This effectively discards
    // the second value in the stack.
    if operands[0] == 0 {
        zmachine.variable(0)?;
        // zmachine.state.current_frame_mut()?.local_variable(0)?;
    }

    zmachine.set_variable(operands[0] as u8, value)?;
    Ok(instruction.next_address())
}

pub fn split_window(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    zmachine.split_window(operands[0])?;

    Ok(instruction.next_address())
}

pub fn set_window(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    zmachine.set_window(operands[0])?;

    Ok(instruction.next_address())
}

pub fn call_vs2(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let address = zmachine.packed_routine_address(operands[0])?;
    let arguments = operands[1..operands.len()].to_vec();

    call_fn(
        zmachine,
        address,
        instruction.next_address,
        &arguments,
        instruction.store().copied(),
    )
}

pub fn erase_window(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    zmachine.erase_window(operands[0] as i16)?;
    Ok(instruction.next_address())
}

pub fn erase_line(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    if operands[0] == 1 {
        zmachine.erase_line()?;
    }

    Ok(instruction.next_address())
}

pub fn set_cursor(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    zmachine.set_cursor(operands[0], operands[1])?;
    Ok(instruction.next_address())
}

pub fn set_text_style(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    zmachine.set_text_style(operands[0])?;
    Ok(instruction.next_address())
}

pub fn buffer_mode(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    zmachine.buffer_mode(operands[0])?;
    Ok(instruction.next_address())
}

pub fn output_stream(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let stream = operands[0] as i16;
    let table = if stream == 3 {
        Some(operands[1] as usize)
    } else {
        None
    };

    zmachine.output_stream(stream, table)?;
    Ok(instruction.next_address())
}

pub fn input_stream(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    let _operands = operand_values(zmachine, instruction)?;
    error!(target: "app::instruction", "INPUT_STREAM not implemented, instruction ignored");
    Ok(instruction.next_address())
}

pub fn sound_effect(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    let operands: Vec<u16> = operand_values(zmachine, instruction)?;
    let number = operands[0];
    match number {
        1 | 2 => zmachine.beep()?,
        _ => {
            let effect = operands[1];
            match effect {
                1 => {
                    // Do nothing?
                }
                2 => {
                    let (volume, repeats) = if operands.len() > 2 {
                        (operands[2] & 0xFF, (operands[2] >> 8) & 0xFF)
                    } else {
                        (255, 1)
                    };
                    let routine = if operands.len() > 3 {
                        Some(zmachine.packed_routine_address(operands[3])?)
                    } else {
                        None
                    };

                    zmachine.play_sound(number, volume as u8, repeats as u8, routine)?
                }
                3 | 4 => zmachine.stop_sound()?,
                _ => {
                    return Err(RuntimeError::new(
                        ErrorCode::System,
                        format!("Invalid SOUND_EFFECT effect {}", effect),
                    ))
                }
            }
        }
    }

    Ok(instruction.next_address())
}

pub fn read_char(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    if !operands.is_empty() && operands[0] != 1 {
        return Err(RuntimeError::new(
            ErrorCode::Instruction,
            format!("READ_CHAR argument 1 must be 1, was {}", operands[0]),
        ));
    }

    if let Some(v) = zmachine.read_interrupt_result() {
        zmachine.clear_read_interrupt();
        if v == 1 {
            store_result(zmachine, instruction, 0)?;
            return Ok(instruction.next_address());
        }
    }

    let timeout = if operands.len() > 1 { operands[1] } else { 0 };
    let routine = if timeout > 0 && operands.len() > 2 {
        zmachine.set_read_interrupt_pending();
        zmachine.packed_routine_address(operands[2])?
    } else {
        0
    };

    let key = zmachine.read_key(timeout * 100)?;
    match key.zchar() {
        Some(c) => {
            store_result(zmachine, instruction, c)?;
            Ok(instruction.next_address())
        }
        None => {
            if let Some(i) = key.interrupt() {
                match i {
                    Interrupt::ReadTimeout => {
                        zmachine.call_read_interrupt(routine, instruction.address())
                    }
                    Interrupt::Sound => zmachine.call_sound_interrupt(instruction.address()),
                }
            } else {
                Err(RuntimeError::new(
                    ErrorCode::System,
                    "read_key return no character or interrupt".to_string(),
                ))
            }
        }
    }
}

pub fn scan_table(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;

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
            zmachine.read_word(address)?
        } else {
            zmachine.read_byte(address)? as u16
        };

        if value == operands[0] {
            store_result(zmachine, instruction, address as u16)?;
            condition = true;
            break;
        }
    }

    if !condition {
        store_result(zmachine, instruction, 0)?;
    }

    branch(zmachine, instruction, condition)
}

pub fn not(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    store_result(zmachine, instruction, !operands[0])?;
    Ok(instruction.next_address())
}

pub fn call_vn(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let address = zmachine.packed_routine_address(operands[0])?;
    let arguments = &operands[1..].to_vec();

    call_fn(
        zmachine,
        address,
        instruction.next_address(),
        arguments,
        instruction.store().copied(),
    )
}

pub fn call_vn2(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let address = zmachine.packed_routine_address(operands[0])?;
    let arguments = &operands[1..].to_vec();

    call_fn(
        zmachine,
        address,
        instruction.next_address(),
        arguments,
        instruction.store().copied(),
    )
}

pub fn tokenise(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let text_buffer = operands[0] as usize;
    let parse_buffer = operands[1] as usize;
    let dictionary = if operands.len() > 2 {
        operands[2] as usize
    } else {
        zmachine.header_word(HeaderField::Dictionary)? as usize
    };
    let flag = if operands.len() > 3 {
        operands[3] > 0
    } else {
        false
    };

    text::parse_text(zmachine, text_buffer, parse_buffer, dictionary, flag)?;
    Ok(instruction.next_address())
}

pub fn encode_text(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let text_buffer = operands[0] as usize;
    let length = operands[1] as usize;
    let from = operands[2] as usize;
    let dest_buffer = operands[3] as usize;

    let mut zchars = Vec::new();
    for i in 0..length {
        zchars.push(zmachine.read_byte(text_buffer + from + i)? as u16);
    }

    let encoded_text = text::encode_text(&mut zchars, 3);

    for (i, w) in encoded_text.iter().enumerate() {
        zmachine.write_word(dest_buffer + (i * 2), *w)?
    }

    Ok(instruction.next_address())
}

pub fn copy_table(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;

    let src = operands[0] as usize;
    let dst = operands[1] as usize;
    let len = operands[2] as i16;

    if dst == 0 {
        for i in 0..len as usize {
            zmachine.write_byte(src + i, 0)?;
        }
    } else if len > 0 && dst > src && dst < src + len as usize {
        for i in (0..len as usize).rev() {
            zmachine.write_byte(dst + i, zmachine.read_byte(src + i)?)?;
        }
    } else {
        for i in 0..len.unsigned_abs() as usize {
            zmachine.write_byte(dst + i, zmachine.read_byte(src + i)?)?;
        }
    }

    Ok(instruction.next_address())
}

pub fn print_table(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let table = operands[0] as usize;
    let width = operands[1] as usize;
    let height = if operands.len() > 2 { operands[2] } else { 1 };
    let skip = if operands.len() > 3 { operands[3] } else { 0 } as usize;

    let origin = zmachine.cursor()?;
    let rows = zmachine.rows();
    for i in 0..height as usize {
        if origin.0 + i as u16 > zmachine.rows() {
            zmachine.new_line()?;
            zmachine.set_cursor(rows, origin.1)?;
        } else {
            zmachine.set_cursor(origin.0 + i as u16, origin.1)?;
        }
        let mut text = Vec::new();
        for j in 0..width {
            let offset = i * (width + skip);
            text.push(zmachine.read_byte(table + offset + j)? as u16);
        }
        zmachine.print(&text)?;
    }

    Ok(instruction.next_address())
}

pub fn check_arg_count(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;

    branch(
        zmachine,
        instruction,
        zmachine.argument_count()? >= operands[0] as u8,
    )
}
