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
            if zmachine.version() == 4 {
                let len = zmachine.read_byte(text_buffer)? as usize - 1;
                for i in 0..len {
                    zmachine.write_byte(text_buffer + i + 1, 0)?;
                }
            } else {
                zmachine.write_byte(text_buffer + 1, 0)?;
                store_result(zmachine, instruction, 0)?;
            }
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

    match zmachine.version() {
        3 => zmachine.status_line()?,
        4 => {
            let mut i = 1;
            loop {
                let b = zmachine.read_byte(text_buffer + i)? as u16;
                if b == 0 {
                    break;
                }
                existing_input.push(b);
                i += 1;
            }
            if zmachine.input_interrupt_print() {
                zmachine.print(&existing_input)?;
            }
        }
        _ => {
            let existing_len = zmachine.read_byte(text_buffer + 1)? as usize;
            for i in 0..existing_len {
                existing_input.push(zmachine.read_byte(text_buffer + 2 + i)? as u16);
            }
            if zmachine.input_interrupt_print() {
                zmachine.print(&existing_input)?;
            }
        }
    }

    zmachine.clear_input_interrupt_print();

    let terminators = terminators(zmachine)?;
    let input_buffer = zmachine.read_line(&existing_input, len, &terminators, timeout * 100)?;
    let terminator = input_buffer.last().filter(|&x| terminators.contains(x));

    // If there was no terminator, then input was interrupted
    // TODO: match this and save the unwrapped terminator when it is Some
    // to use later.
    if terminator.is_none() {
        // Store any input that was read before the interrupt
        if zmachine.version() == 4 {
            for (i, b) in input_buffer.iter().enumerate() {
                zmachine.write_byte(text_buffer + 1 + i, *b as u8)?;
            }
            zmachine.write_byte(text_buffer + 1 + input_buffer.len(), 0)?;
        } else {
            zmachine.write_byte(text_buffer + 1, input_buffer.len() as u8)?;
            for (i, b) in input_buffer.iter().enumerate() {
                zmachine.write_byte(text_buffer + 2 + i, *b as u8)?;
            }
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
        // unwrap() is safe here as terminator is checked for none earlier
        store_result(zmachine, instruction, *terminator.unwrap())?;
    }

    Ok(instruction.next_address())
}

pub fn print_char(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    zmachine.print(&vec![operands[0]])?;
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

pub fn get_cursor(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<usize, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let (row, column) = zmachine.cursor()?;
    zmachine.write_word(operands[0] as usize, row)?;
    zmachine.write_word(operands[0] as usize + 2, column)?;
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

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use crate::{
        instruction::{
            processor::{
                dispatch,
                tests::{
                    assert_print, beep, branch, buffer_mode, erase_line, erase_window, input,
                    mock_branch_instruction, mock_branch_store_instruction, mock_custom_dictionary,
                    mock_dictionary, mock_instruction, mock_object, mock_properties, mock_routine,
                    mock_store_instruction, mock_zmachine, operand, output_stream, play_sound,
                    set_input_delay, set_input_timeout, set_split, set_variable, split, store,
                    style, test_map, window,
                },
            },
            Opcode, OpcodeForm, OperandCount, OperandType,
        },
        object::property,
    };

    fn opcode(version: u8, instruction: u8) -> Opcode {
        Opcode::new(
            version,
            instruction,
            instruction,
            OpcodeForm::Var,
            OperandCount::_VAR,
        )
    }

    #[test]
    fn test_call_v3() {
        let mut map = test_map(3);
        mock_routine(
            &mut map,
            0x600,
            &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
        );
        let mut zmachine = mock_zmachine(map);
        assert!(zmachine.push(1).is_ok());
        let i = mock_store_instruction(
            0x401,
            vec![
                operand(OperandType::LargeConstant, 0x300),
                operand(OperandType::SmallConstant, 0x12),
                operand(OperandType::LargeConstant, 0x3456),
                operand(OperandType::LargeConstant, 0xABCD),
            ],
            opcode(3, 0),
            0x409,
            store(0x408, 0),
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x61f));
        assert!(zmachine.peek_variable(0).is_err());
        assert!(zmachine.variable(1).is_ok_and(|x| x == 0x12));
        assert!(zmachine.variable(2).is_ok_and(|x| x == 0x3456));
        assert!(zmachine.variable(3).is_ok_and(|x| x == 0xABCD));
        assert!(zmachine.variable(4).is_ok_and(|x| x == 4));
        assert!(zmachine.variable(5).is_ok_and(|x| x == 5));
        assert!(zmachine.variable(6).is_ok_and(|x| x == 6));
        assert!(zmachine.variable(7).is_ok_and(|x| x == 7));
        assert!(zmachine.variable(8).is_ok_and(|x| x == 8));
        assert!(zmachine.variable(9).is_ok_and(|x| x == 9));
        assert!(zmachine.variable(10).is_ok_and(|x| x == 10));
        assert!(zmachine.variable(11).is_ok_and(|x| x == 11));
        assert!(zmachine.variable(12).is_ok_and(|x| x == 12));
        assert!(zmachine.variable(13).is_ok_and(|x| x == 13));
        assert!(zmachine.variable(14).is_ok_and(|x| x == 14));
        assert!(zmachine.variable(15).is_ok_and(|x| x == 15));
        assert!(zmachine.return_routine(0xF0AD).is_ok_and(|x| x == 0x409));
        assert!(zmachine.variable(0).is_ok_and(|x| x == 0xF0AD));
        assert!(zmachine.variable(0).is_ok_and(|x| x == 1));
    }

    #[test]
    fn test_call_vs_v4() {
        let mut map = test_map(4);
        mock_routine(
            &mut map,
            0x600,
            &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
        );
        let mut zmachine = mock_zmachine(map);
        assert!(zmachine.push(1).is_ok());
        let i = mock_store_instruction(
            0x401,
            vec![
                operand(OperandType::LargeConstant, 0x180),
                operand(OperandType::SmallConstant, 0x12),
                operand(OperandType::LargeConstant, 0x3456),
                operand(OperandType::LargeConstant, 0xABCD),
            ],
            opcode(4, 0),
            0x409,
            store(0x408, 0),
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x61f));
        assert!(zmachine.peek_variable(0).is_err());
        assert!(zmachine.variable(1).is_ok_and(|x| x == 0x12));
        assert!(zmachine.variable(2).is_ok_and(|x| x == 0x3456));
        assert!(zmachine.variable(3).is_ok_and(|x| x == 0xABCD));
        assert!(zmachine.variable(4).is_ok_and(|x| x == 4));
        assert!(zmachine.variable(5).is_ok_and(|x| x == 5));
        assert!(zmachine.variable(6).is_ok_and(|x| x == 6));
        assert!(zmachine.variable(7).is_ok_and(|x| x == 7));
        assert!(zmachine.variable(8).is_ok_and(|x| x == 8));
        assert!(zmachine.variable(9).is_ok_and(|x| x == 9));
        assert!(zmachine.variable(10).is_ok_and(|x| x == 10));
        assert!(zmachine.variable(11).is_ok_and(|x| x == 11));
        assert!(zmachine.variable(12).is_ok_and(|x| x == 12));
        assert!(zmachine.variable(13).is_ok_and(|x| x == 13));
        assert!(zmachine.variable(14).is_ok_and(|x| x == 14));
        assert!(zmachine.variable(15).is_ok_and(|x| x == 15));
        assert!(zmachine.return_routine(0xF0AD).is_ok_and(|x| x == 0x409));
        assert!(zmachine.variable(0).is_ok_and(|x| x == 0xF0AD));
        assert!(zmachine.variable(0).is_ok_and(|x| x == 1));
    }

    #[test]
    fn test_call_vs_v5() {
        let mut map = test_map(5);
        mock_routine(
            &mut map,
            0x600,
            &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
        );
        let mut zmachine = mock_zmachine(map);
        assert!(zmachine.push(1).is_ok());
        let i = mock_store_instruction(
            0x401,
            vec![
                operand(OperandType::LargeConstant, 0x180),
                operand(OperandType::SmallConstant, 0x12),
                operand(OperandType::LargeConstant, 0x3456),
                operand(OperandType::LargeConstant, 0xABCD),
            ],
            opcode(5, 0),
            0x409,
            store(0x408, 0),
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x601));
        assert!(zmachine.peek_variable(0).is_err());
        assert!(zmachine.variable(1).is_ok_and(|x| x == 0x12));
        assert!(zmachine.variable(2).is_ok_and(|x| x == 0x3456));
        assert!(zmachine.variable(3).is_ok_and(|x| x == 0xABCD));
        assert!(zmachine.variable(4).is_ok_and(|x| x == 0));
        assert!(zmachine.variable(5).is_ok_and(|x| x == 0));
        assert!(zmachine.variable(6).is_ok_and(|x| x == 0));
        assert!(zmachine.variable(7).is_ok_and(|x| x == 0));
        assert!(zmachine.variable(8).is_ok_and(|x| x == 0));
        assert!(zmachine.variable(9).is_ok_and(|x| x == 0));
        assert!(zmachine.variable(10).is_ok_and(|x| x == 0));
        assert!(zmachine.variable(11).is_ok_and(|x| x == 0));
        assert!(zmachine.variable(12).is_ok_and(|x| x == 0));
        assert!(zmachine.variable(13).is_ok_and(|x| x == 0));
        assert!(zmachine.variable(14).is_ok_and(|x| x == 0));
        assert!(zmachine.variable(15).is_ok_and(|x| x == 0));
        assert!(zmachine.return_routine(0xF0AD).is_ok_and(|x| x == 0x409));
        assert!(zmachine.variable(0).is_ok_and(|x| x == 0xF0AD));
        assert!(zmachine.variable(0).is_ok_and(|x| x == 1));
    }

    #[test]
    fn test_call_vs_v8() {
        let mut map = test_map(8);
        mock_routine(
            &mut map,
            0x600,
            &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
        );
        let mut zmachine = mock_zmachine(map);
        assert!(zmachine.push(1).is_ok());
        let i = mock_store_instruction(
            0x401,
            vec![
                operand(OperandType::LargeConstant, 0xC0),
                operand(OperandType::SmallConstant, 0x12),
                operand(OperandType::LargeConstant, 0x3456),
                operand(OperandType::LargeConstant, 0xABCD),
            ],
            opcode(8, 0),
            0x409,
            store(0x408, 0),
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x601));
        assert!(zmachine.peek_variable(0).is_err());
        assert!(zmachine.variable(1).is_ok_and(|x| x == 0x12));
        assert!(zmachine.variable(2).is_ok_and(|x| x == 0x3456));
        assert!(zmachine.variable(3).is_ok_and(|x| x == 0xABCD));
        assert!(zmachine.variable(4).is_ok_and(|x| x == 0));
        assert!(zmachine.variable(5).is_ok_and(|x| x == 0));
        assert!(zmachine.variable(6).is_ok_and(|x| x == 0));
        assert!(zmachine.variable(7).is_ok_and(|x| x == 0));
        assert!(zmachine.variable(8).is_ok_and(|x| x == 0));
        assert!(zmachine.variable(9).is_ok_and(|x| x == 0));
        assert!(zmachine.variable(10).is_ok_and(|x| x == 0));
        assert!(zmachine.variable(11).is_ok_and(|x| x == 0));
        assert!(zmachine.variable(12).is_ok_and(|x| x == 0));
        assert!(zmachine.variable(13).is_ok_and(|x| x == 0));
        assert!(zmachine.variable(14).is_ok_and(|x| x == 0));
        assert!(zmachine.variable(15).is_ok_and(|x| x == 0));
        assert!(zmachine.return_routine(0xF0AD).is_ok_and(|x| x == 0x409));
        assert!(zmachine.variable(0).is_ok_and(|x| x == 0xF0AD));
        assert!(zmachine.variable(0).is_ok_and(|x| x == 1));
    }

    #[test]
    fn test_storew() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x380),
                operand(OperandType::SmallConstant, 0x4),
                operand(OperandType::LargeConstant, 0x1234),
            ],
            opcode(3, 1),
            0x406,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x406));
        assert!(zmachine.read_word(0x388).is_ok_and(|x| x == 0x1234));
    }

    #[test]
    fn test_storeb() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x380),
                operand(OperandType::SmallConstant, 0x4),
                operand(OperandType::SmallConstant, 0x56),
            ],
            opcode(3, 2),
            0x405,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x405));
        assert!(zmachine.read_byte(0x384).is_ok_and(|x| x == 0x56));
    }

    #[test]
    fn test_put_prop_v3_byte() {
        let mut map = test_map(3);
        mock_object(&mut map, 1, vec![], (0, 0, 0));
        mock_properties(
            &mut map,
            1,
            &[
                (20, &vec![0x12, 0x34]),
                (15, &vec![0x56]),
                (10, &vec![0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x11, 0x22]),
            ],
        );
        let mut zmachine = mock_zmachine(map);
        assert!(property::property(&zmachine, 1, 15).is_ok_and(|x| x == 0x56));
        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 1),
                operand(OperandType::SmallConstant, 15),
                operand(OperandType::LargeConstant, 0xFFFE),
            ],
            opcode(3, 3),
            0x404,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x404));
        assert!(property::property(&zmachine, 1, 15).is_ok_and(|x| x == 0xFE));
    }

    #[test]
    fn test_put_prop_v3_word() {
        let mut map = test_map(3);
        mock_object(&mut map, 1, vec![], (0, 0, 0));
        mock_properties(
            &mut map,
            1,
            &[
                (20, &vec![0x12, 0x34]),
                (15, &vec![0x56]),
                (10, &vec![0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x11, 0x22]),
            ],
        );
        let mut zmachine = mock_zmachine(map);
        assert!(property::property(&zmachine, 1, 20).is_ok_and(|x| x == 0x1234));
        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 1),
                operand(OperandType::SmallConstant, 20),
                operand(OperandType::LargeConstant, 0xFEDC),
            ],
            opcode(3, 3),
            0x405,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x405));
        assert!(property::property(&zmachine, 1, 20).is_ok_and(|x| x == 0xFEDC));
    }

    #[test]
    fn test_put_prop_invalid() {
        let mut map = test_map(3);
        mock_object(&mut map, 1, vec![], (0, 0, 0));
        mock_properties(
            &mut map,
            1,
            &[
                (20, &vec![0x12, 0x34]),
                (15, &vec![0x56]),
                (10, &vec![0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x11, 0x22]),
            ],
        );
        let mut zmachine = mock_zmachine(map);
        assert!(property::property(&zmachine, 1, 20).is_ok_and(|x| x == 0x1234));
        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 1),
                operand(OperandType::SmallConstant, 21),
                operand(OperandType::LargeConstant, 0xFEDC),
            ],
            opcode(3, 3),
            0x405,
        );
        assert!(dispatch(&mut zmachine, &i).is_err());
    }

    #[test]
    fn test_put_prop_v4_byte() {
        let mut map = test_map(4);
        mock_object(&mut map, 1, vec![], (0, 0, 0));
        mock_properties(
            &mut map,
            1,
            &[
                (40, &vec![0x12, 0x34]),
                (35, &vec![0x56]),
                (10, &vec![0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x11, 0x22]),
            ],
        );
        let mut zmachine = mock_zmachine(map);
        assert!(property::property(&zmachine, 1, 35).is_ok_and(|x| x == 0x56));
        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 1),
                operand(OperandType::SmallConstant, 35),
                operand(OperandType::LargeConstant, 0xFFFE),
            ],
            opcode(4, 3),
            0x404,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x404));
        assert!(property::property(&zmachine, 1, 35).is_ok_and(|x| x == 0xFE));
    }

    #[test]
    fn test_put_prop_v4_word() {
        let mut map = test_map(4);
        mock_object(&mut map, 1, vec![], (0, 0, 0));
        mock_properties(
            &mut map,
            1,
            &[
                (40, &vec![0x12, 0x34]),
                (35, &vec![0x56]),
                (10, &vec![0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x11, 0x22]),
            ],
        );
        let mut zmachine = mock_zmachine(map);
        assert!(property::property(&zmachine, 1, 40).is_ok_and(|x| x == 0x1234));
        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 1),
                operand(OperandType::SmallConstant, 40),
                operand(OperandType::LargeConstant, 0xFEDC),
            ],
            opcode(4, 3),
            0x405,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x405));
        assert!(property::property(&zmachine, 1, 40).is_ok_and(|x| x == 0xFEDC));
    }

    #[test]
    fn test_sread_v3() {
        let mut map = test_map(3);
        mock_dictionary(&mut map);
        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x380),
                operand(OperandType::LargeConstant, 0x3A0),
            ],
            opcode(3, 4),
            0x405,
        );

        input(&['I', 'n', 'v', 'e', 'n', 't', 'o', 'r', 'y']);

        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x405));
        // Text buffer
        assert!(zmachine.read_byte(0x381).is_ok_and(|x| x == b'i'));
        assert!(zmachine.read_byte(0x382).is_ok_and(|x| x == b'n'));
        assert!(zmachine.read_byte(0x383).is_ok_and(|x| x == b'v'));
        assert!(zmachine.read_byte(0x384).is_ok_and(|x| x == b'e'));
        assert!(zmachine.read_byte(0x385).is_ok_and(|x| x == b'n'));
        assert!(zmachine.read_byte(0x386).is_ok_and(|x| x == b't'));
        assert!(zmachine.read_byte(0x387).is_ok_and(|x| x == b'o'));
        assert!(zmachine.read_byte(0x388).is_ok_and(|x| x == b'r'));
        assert!(zmachine.read_byte(0x389).is_ok_and(|x| x == b'y'));
        assert!(zmachine.read_byte(0x38a).is_ok_and(|x| x == 0));
        // Parse buffer
        assert!(zmachine.read_byte(0x3A1).is_ok_and(|x| x == 1));
        assert!(zmachine.read_word(0x3A2).is_ok_and(|x| x == 0x310));
        assert!(zmachine.read_byte(0x3A4).is_ok_and(|x| x == 9));
        assert!(zmachine.read_byte(0x3A5).is_ok_and(|x| x == 1));
    }

    #[test]
    fn test_sread_v4() {
        let mut map = test_map(4);
        mock_dictionary(&mut map);
        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x380),
                operand(OperandType::LargeConstant, 0x3A0),
            ],
            opcode(4, 4),
            0x405,
        );

        input(&['I', 'n', 'v', 'e', 'n', 't', 'o', 'r', 'y']);

        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x405));
        // Text buffer
        assert!(zmachine.read_byte(0x381).is_ok_and(|x| x == b'i'));
        assert!(zmachine.read_byte(0x382).is_ok_and(|x| x == b'n'));
        assert!(zmachine.read_byte(0x383).is_ok_and(|x| x == b'v'));
        assert!(zmachine.read_byte(0x384).is_ok_and(|x| x == b'e'));
        assert!(zmachine.read_byte(0x385).is_ok_and(|x| x == b'n'));
        assert!(zmachine.read_byte(0x386).is_ok_and(|x| x == b't'));
        assert!(zmachine.read_byte(0x387).is_ok_and(|x| x == b'o'));
        assert!(zmachine.read_byte(0x388).is_ok_and(|x| x == b'r'));
        assert!(zmachine.read_byte(0x389).is_ok_and(|x| x == b'y'));
        assert!(zmachine.read_byte(0x38a).is_ok_and(|x| x == 0));
        // Parse buffer
        assert!(zmachine.read_byte(0x3A1).is_ok_and(|x| x == 1));
        assert!(zmachine.read_word(0x3A2).is_ok_and(|x| x == 0x310));
        assert!(zmachine.read_byte(0x3A4).is_ok_and(|x| x == 9));
        assert!(zmachine.read_byte(0x3A5).is_ok_and(|x| x == 1));
    }

    #[test]
    fn test_sread_v4_interrupt() {
        let mut map = test_map(4);
        mock_dictionary(&mut map);
        mock_routine(&mut map, 0x600, &[0x1234, 0x5678]);
        let mut zmachine = mock_zmachine(map);
        // Read with a 3 second timeout
        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x380),
                operand(OperandType::LargeConstant, 0x3A0),
                operand(OperandType::LargeConstant, 30),
                operand(OperandType::LargeConstant, 0x180),
            ],
            opcode(4, 4),
            0x405,
        );

        input(&['I', 'n', 'v', 'e', 'n', 't', 'o', 'r', 'y']);
        // Wait 500ms before each key press
        set_input_delay(501);

        // Input was interrupted
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x605));
        // Text buffer
        assert!(zmachine.read_byte(0x381).is_ok_and(|x| x == b'I'));
        assert!(zmachine.read_byte(0x382).is_ok_and(|x| x == b'n'));
        assert!(zmachine.read_byte(0x383).is_ok_and(|x| x == b'v'));
        assert!(zmachine.read_byte(0x384).is_ok_and(|x| x == b'e'));
        assert!(zmachine.read_byte(0x385).is_ok_and(|x| x == b'n'));
        assert!(zmachine.read_byte(0x386).is_ok_and(|x| x == b't'));
        assert!(zmachine.read_byte(0x387).is_ok_and(|x| x == 0));
        // Parse buffer
        assert!(zmachine.read_byte(0x3A1).is_ok_and(|x| x == 0));
    }

    #[test]
    fn test_sread_v4_interrupt_continue() {
        let mut map = test_map(4);
        mock_dictionary(&mut map);
        mock_routine(&mut map, 0x600, &[0x1234, 0x5678]);
        // Text buffer from previous READ
        map[0x381] = b'I';
        map[0x382] = b'n';
        map[0x383] = b'v';
        map[0x384] = b'e';
        map[0x385] = b'n';
        map[0x386] = b't';
        map[0x387] = 0;

        let mut zmachine = mock_zmachine(map);
        zmachine.set_read_interrupt_pending();
        assert!(zmachine.call_read_interrupt(0x600, 0x400).is_ok());
        assert!(zmachine.return_routine(0).is_ok());

        // Read with a 3 second timeout
        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x380),
                operand(OperandType::LargeConstant, 0x3A0),
                operand(OperandType::LargeConstant, 30),
                operand(OperandType::LargeConstant, 0x180),
            ],
            opcode(4, 4),
            0x405,
        );

        input(&['o', 'r', 'y']);

        // Input was interrupted
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x405));
        // Text buffer
        assert!(zmachine.read_byte(0x381).is_ok_and(|x| x == b'i'));
        assert!(zmachine.read_byte(0x382).is_ok_and(|x| x == b'n'));
        assert!(zmachine.read_byte(0x383).is_ok_and(|x| x == b'v'));
        assert!(zmachine.read_byte(0x384).is_ok_and(|x| x == b'e'));
        assert!(zmachine.read_byte(0x385).is_ok_and(|x| x == b'n'));
        assert!(zmachine.read_byte(0x386).is_ok_and(|x| x == b't'));
        assert!(zmachine.read_byte(0x387).is_ok_and(|x| x == b'o'));
        assert!(zmachine.read_byte(0x388).is_ok_and(|x| x == b'r'));
        assert!(zmachine.read_byte(0x389).is_ok_and(|x| x == b'y'));
        assert!(zmachine.read_byte(0x38a).is_ok_and(|x| x == 0));
        // Parse buffer
        assert!(zmachine.read_byte(0x3A1).is_ok_and(|x| x == 1));
        assert!(zmachine.read_word(0x3A2).is_ok_and(|x| x == 0x310));
        assert!(zmachine.read_byte(0x3A4).is_ok_and(|x| x == 9));
        assert!(zmachine.read_byte(0x3A5).is_ok_and(|x| x == 1));
    }

    #[test]
    fn test_sread_v4_interrupt_stop() {
        let mut map = test_map(4);
        mock_dictionary(&mut map);
        mock_routine(&mut map, 0x600, &[0x1234, 0x5678]);
        // Text buffer from previous READ
        map[0x381] = b'I';
        map[0x382] = b'n';
        map[0x383] = b'v';
        map[0x384] = b'e';
        map[0x385] = b'n';
        map[0x386] = b't';
        map[0x387] = 0;

        let mut zmachine = mock_zmachine(map);
        zmachine.set_read_interrupt_pending();
        assert!(zmachine.call_read_interrupt(0x600, 0x400).is_ok());
        assert!(zmachine.return_routine(1).is_ok());

        // Read with a 3 second timeout
        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x380),
                operand(OperandType::LargeConstant, 0x3A0),
                operand(OperandType::LargeConstant, 30),
                operand(OperandType::LargeConstant, 0x180),
            ],
            opcode(4, 4),
            0x405,
        );

        // Input was interrupted
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x405));
        // Text buffer
        assert!(zmachine.read_byte(0x381).is_ok_and(|x| x == 0));
        assert!(zmachine.read_byte(0x382).is_ok_and(|x| x == 0));
        assert!(zmachine.read_byte(0x383).is_ok_and(|x| x == 0));
        assert!(zmachine.read_byte(0x384).is_ok_and(|x| x == 0));
        assert!(zmachine.read_byte(0x385).is_ok_and(|x| x == 0));
        assert!(zmachine.read_byte(0x386).is_ok_and(|x| x == 0));
        assert!(zmachine.read_byte(0x387).is_ok_and(|x| x == 0));
        // Parse buffer
        assert!(zmachine.read_byte(0x3A1).is_ok_and(|x| x == 0));
    }

    #[test]
    fn test_aread_v5() {
        let mut map = test_map(5);
        mock_dictionary(&mut map);
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x380),
                operand(OperandType::LargeConstant, 0x3A0),
            ],
            opcode(5, 4),
            0x406,
            store(0x405, 0x80),
        );

        input(&['I', 'n', 'v', 'e', 'n', 't', 'o', 'r', 'y']);

        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x406));
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == b'\r' as u16));
        // Text buffer
        assert!(zmachine.read_byte(0x381).is_ok_and(|x| x == 9));
        assert!(zmachine.read_byte(0x382).is_ok_and(|x| x == b'i'));
        assert!(zmachine.read_byte(0x383).is_ok_and(|x| x == b'n'));
        assert!(zmachine.read_byte(0x384).is_ok_and(|x| x == b'v'));
        assert!(zmachine.read_byte(0x385).is_ok_and(|x| x == b'e'));
        assert!(zmachine.read_byte(0x386).is_ok_and(|x| x == b'n'));
        assert!(zmachine.read_byte(0x387).is_ok_and(|x| x == b't'));
        assert!(zmachine.read_byte(0x388).is_ok_and(|x| x == b'o'));
        assert!(zmachine.read_byte(0x389).is_ok_and(|x| x == b'r'));
        assert!(zmachine.read_byte(0x38a).is_ok_and(|x| x == b'y'));
        // Parse buffer
        assert!(zmachine.read_byte(0x3A1).is_ok_and(|x| x == 1));
        assert!(zmachine.read_word(0x3A2).is_ok_and(|x| x == 0x310));
        assert!(zmachine.read_byte(0x3A4).is_ok_and(|x| x == 9));
        assert!(zmachine.read_byte(0x3A5).is_ok_and(|x| x == 2));
    }

    #[test]
    fn test_aread_v5_no_parse() {
        let mut map = test_map(5);
        mock_dictionary(&mut map);
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x380),
                operand(OperandType::LargeConstant, 0),
            ],
            opcode(5, 4),
            0x406,
            store(0x405, 0x80),
        );

        input(&['I', 'n', 'v', 'e', 'n', 't', 'o', 'r', 'y']);

        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x406));
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == b'\r' as u16));
        // Text buffer
        assert!(zmachine.read_byte(0x381).is_ok_and(|x| x == 9));
        assert!(zmachine.read_byte(0x382).is_ok_and(|x| x == b'i'));
        assert!(zmachine.read_byte(0x383).is_ok_and(|x| x == b'n'));
        assert!(zmachine.read_byte(0x384).is_ok_and(|x| x == b'v'));
        assert!(zmachine.read_byte(0x385).is_ok_and(|x| x == b'e'));
        assert!(zmachine.read_byte(0x386).is_ok_and(|x| x == b'n'));
        assert!(zmachine.read_byte(0x387).is_ok_and(|x| x == b't'));
        assert!(zmachine.read_byte(0x388).is_ok_and(|x| x == b'o'));
        assert!(zmachine.read_byte(0x389).is_ok_and(|x| x == b'r'));
        assert!(zmachine.read_byte(0x38a).is_ok_and(|x| x == b'y'));
        // Parse buffer
        assert!(zmachine.read_byte(0x3A1).is_ok_and(|x| x == 0));
    }

    #[test]
    fn test_aread_v5_interrupt() {
        let mut map = test_map(5);
        mock_dictionary(&mut map);
        mock_routine(&mut map, 0x600, &[0x1234, 0x5678]);
        let mut zmachine = mock_zmachine(map);
        // Read with a 3 second timeout
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x380),
                operand(OperandType::LargeConstant, 0x3A0),
                operand(OperandType::LargeConstant, 30),
                operand(OperandType::LargeConstant, 0x180),
            ],
            opcode(5, 4),
            0x409,
            store(0x408, 0x80),
        );

        input(&['I', 'n', 'v', 'e', 'n', 't', 'o', 'r', 'y']);
        // Wait 500ms before each key press
        set_input_delay(501);

        // Input was interrupted
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x601));
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0));
        // Text buffer
        assert!(zmachine.read_byte(0x381).is_ok_and(|x| x == 6));
        assert!(zmachine.read_byte(0x382).is_ok_and(|x| x == b'I'));
        assert!(zmachine.read_byte(0x383).is_ok_and(|x| x == b'n'));
        assert!(zmachine.read_byte(0x384).is_ok_and(|x| x == b'v'));
        assert!(zmachine.read_byte(0x385).is_ok_and(|x| x == b'e'));
        assert!(zmachine.read_byte(0x386).is_ok_and(|x| x == b'n'));
        assert!(zmachine.read_byte(0x387).is_ok_and(|x| x == b't'));
        // Parse buffer
        assert!(zmachine.read_byte(0x3A1).is_ok_and(|x| x == 0));
    }

    #[test]
    fn test_aread_v5_interrupt_continue() {
        let mut map = test_map(5);
        mock_dictionary(&mut map);
        mock_routine(&mut map, 0x600, &[0x1234, 0x5678]);
        // Text buffer from previous READ
        map[0x381] = 6;
        map[0x382] = b'i';
        map[0x383] = b'n';
        map[0x384] = b'v';
        map[0x385] = b'e';
        map[0x386] = b'n';
        map[0x387] = b't';

        let mut zmachine = mock_zmachine(map);
        zmachine.set_read_interrupt_pending();
        assert!(zmachine.call_read_interrupt(0x600, 0x400).is_ok());
        assert!(zmachine.return_routine(0).is_ok());

        // Read with a 3 second timeout
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x380),
                operand(OperandType::LargeConstant, 0x3A0),
                operand(OperandType::LargeConstant, 30),
                operand(OperandType::LargeConstant, 0x180),
            ],
            opcode(5, 4),
            0x409,
            store(0x408, 0x80),
        );

        input(&['o', 'r', 'y']);

        // Input was interrupted
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x409));
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == b'\r' as u16));
        // Text buffer
        assert!(zmachine.read_byte(0x381).is_ok_and(|x| x == 9));
        assert!(zmachine.read_byte(0x382).is_ok_and(|x| x == b'i'));
        assert!(zmachine.read_byte(0x383).is_ok_and(|x| x == b'n'));
        assert!(zmachine.read_byte(0x384).is_ok_and(|x| x == b'v'));
        assert!(zmachine.read_byte(0x385).is_ok_and(|x| x == b'e'));
        assert!(zmachine.read_byte(0x386).is_ok_and(|x| x == b'n'));
        assert!(zmachine.read_byte(0x387).is_ok_and(|x| x == b't'));
        assert!(zmachine.read_byte(0x388).is_ok_and(|x| x == b'o'));
        assert!(zmachine.read_byte(0x389).is_ok_and(|x| x == b'r'));
        assert!(zmachine.read_byte(0x38a).is_ok_and(|x| x == b'y'));
        // Parse buffer
        assert!(zmachine.read_byte(0x3A1).is_ok_and(|x| x == 1));
        assert!(zmachine.read_word(0x3A2).is_ok_and(|x| x == 0x310));
        assert!(zmachine.read_byte(0x3A4).is_ok_and(|x| x == 9));
        assert!(zmachine.read_byte(0x3A5).is_ok_and(|x| x == 2));
    }

    #[test]
    fn test_aread_v5_interrupt_stop() {
        let mut map = test_map(5);
        set_variable(&mut map, 0x80, 0xFF);
        mock_dictionary(&mut map);
        mock_routine(&mut map, 0x600, &[0x1234, 0x5678]);
        // Text buffer from previous READ
        map[0x381] = 6;
        map[0x382] = b'i';
        map[0x383] = b'n';
        map[0x384] = b'v';
        map[0x385] = b'e';
        map[0x386] = b'n';
        map[0x387] = b't';

        let mut zmachine = mock_zmachine(map);
        zmachine.set_read_interrupt_pending();
        assert!(zmachine.call_read_interrupt(0x600, 0x400).is_ok());
        assert!(zmachine.return_routine(1).is_ok());

        // Read with a 3 second timeout
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x380),
                operand(OperandType::LargeConstant, 0x3A0),
                operand(OperandType::LargeConstant, 30),
                operand(OperandType::LargeConstant, 0x180),
            ],
            opcode(5, 4),
            0x409,
            store(0x408, 0x80),
        );

        // Input was interrupted
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x409));
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0));
        // Text buffer
        assert!(zmachine.read_byte(0x381).is_ok_and(|x| x == 0));
        // Parse buffer
        assert!(zmachine.read_byte(0x3A1).is_ok_and(|x| x == 0));
    }

    #[test]
    fn test_print_char() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::LargeConstant, b'@' as u16)],
            opcode(3, 5),
            0x403,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x403));
        assert_print("@");
    }

    #[test]
    fn test_print_num() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::LargeConstant, 0x7FFF)],
            opcode(3, 6),
            0x403,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x403));
        assert_print("32767");
    }

    #[test]
    fn test_print_num_negative() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::LargeConstant, 0x8000)],
            opcode(3, 6),
            0x403,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x403));
        assert_print("-32768");
    }

    #[test]
    fn test_random() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);

        let i = mock_store_instruction(
            0x400,
            vec![operand(OperandType::LargeConstant, 0x7FFF)],
            opcode(3, 7),
            0x404,
            store(0x403, 0x080),
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x404));
        assert!(zmachine
            .variable(0x80)
            .is_ok_and(|x| (1..=0x7FFF).contains(&x)));
    }

    #[test]
    fn test_random_random_seed() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![operand(OperandType::LargeConstant, 0)],
            opcode(3, 7),
            0x404,
            store(0x403, 0x080),
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x404));
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0));

        let i = mock_store_instruction(
            0x400,
            vec![operand(OperandType::LargeConstant, 0x7FFF)],
            opcode(3, 7),
            0x404,
            store(0x403, 0x080),
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x404));
        assert!(zmachine
            .variable(0x80)
            .is_ok_and(|x| (1..=0x7FFF).contains(&x)));
    }

    #[test]
    fn test_random_predictable() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![operand(OperandType::LargeConstant, 0xFFF8)],
            opcode(3, 7),
            0x404,
            store(0x403, 0x080),
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x404));
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0));
        let i = mock_store_instruction(
            0x400,
            vec![operand(OperandType::LargeConstant, 8)],
            opcode(3, 7),
            0x404,
            store(0x403, 0x080),
        );
        for r in 1..9 {
            assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x404));
            assert!(zmachine.variable(0x80).is_ok_and(|x| x == r % 8));
        }
    }

    #[test]
    fn test_random_seeded() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![operand(OperandType::LargeConstant, 0x8001)],
            opcode(3, 7),
            0x404,
            store(0x403, 0x080),
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x404));
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0));
        let i = mock_store_instruction(
            0x400,
            vec![operand(OperandType::LargeConstant, 32767)],
            opcode(3, 7),
            0x404,
            store(0x403, 0x080),
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x404));
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0x4DD5));
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x404));
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0x0AD5));
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x404));
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0x3D5E));
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x404));
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0x0F57));
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x404));
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0x12E1));
    }

    #[test]
    fn test_push() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        assert!(zmachine.peek_variable(0).is_err());
        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::LargeConstant, 0x1234)],
            opcode(3, 8),
            0x403,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x403));
        assert!(zmachine.peek_variable(0).is_ok_and(|x| x == 0x1234));
    }

    #[test]
    fn test_pull() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        assert!(zmachine.push(0x1234).is_ok());
        assert!(zmachine.push(0x5678).is_ok());
        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 0x80)],
            opcode(3, 9),
            0x403,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x403));
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0x5678));
        assert!(zmachine.peek_variable(0).is_ok_and(|x| x == 0x1234));
    }

    #[test]
    fn test_pull_sp() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        assert!(zmachine.push(0x1234).is_ok());
        assert!(zmachine.push(0x5678).is_ok());
        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 0x00)],
            opcode(3, 9),
            0x403,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x403));
        assert!(zmachine.variable(0).is_ok_and(|x| x == 0x5678));
        assert!(zmachine.peek_variable(0).is_err());
    }

    #[test]
    fn test_split_window() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 0xC)],
            opcode(3, 10),
            0x402,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x402));
        assert_eq!(split(), 0xC);
    }

    #[test]
    fn test_set_window() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 0xC)],
            opcode(3, 10),
            0x402,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x402));
        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 0x1)],
            opcode(3, 11),
            0x402,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x402));
        assert_eq!(window(), 1);
    }

    #[test]
    fn test_call_vs2_v4() {
        let mut map = test_map(4);
        mock_routine(&mut map, 0x600, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        let mut zmachine = mock_zmachine(map);
        assert!(zmachine.push(0x1234).is_ok());
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x180),
                operand(OperandType::LargeConstant, 0x1111),
                operand(OperandType::SmallConstant, 0x22),
                operand(OperandType::LargeConstant, 0x3333),
                operand(OperandType::SmallConstant, 0x44),
                operand(OperandType::LargeConstant, 0x5555),
                operand(OperandType::SmallConstant, 0x66),
                operand(OperandType::LargeConstant, 0x7777),
            ],
            opcode(4, 12),
            0x411,
            store(0x410, 0x80),
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x615));
        assert!(zmachine.peek_variable(0).is_err());
        assert!(zmachine.variable(1).is_ok_and(|x| x == 0x1111));
        assert!(zmachine.variable(2).is_ok_and(|x| x == 0x22));
        assert!(zmachine.variable(3).is_ok_and(|x| x == 0x3333));
        assert!(zmachine.variable(4).is_ok_and(|x| x == 0x44));
        assert!(zmachine.variable(5).is_ok_and(|x| x == 0x5555));
        assert!(zmachine.variable(6).is_ok_and(|x| x == 0x66));
        assert!(zmachine.variable(7).is_ok_and(|x| x == 0x7777));
        assert!(zmachine.variable(8).is_ok_and(|x| x == 0x8));
        assert!(zmachine.variable(9).is_ok_and(|x| x == 0x9));
        assert!(zmachine.variable(10).is_ok_and(|x| x == 0xA));
        assert!(zmachine.variable(11).is_err());
        assert!(zmachine.return_routine(0x5678).is_ok_and(|x| x == 0x411));
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0x5678));
    }

    #[test]
    fn test_call_vs2_v5() {
        let mut map = test_map(5);
        mock_routine(&mut map, 0x600, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        let mut zmachine = mock_zmachine(map);
        assert!(zmachine.push(0x1234).is_ok());
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x180),
                operand(OperandType::LargeConstant, 0x1111),
                operand(OperandType::SmallConstant, 0x22),
                operand(OperandType::LargeConstant, 0x3333),
                operand(OperandType::SmallConstant, 0x44),
                operand(OperandType::LargeConstant, 0x5555),
                operand(OperandType::SmallConstant, 0x66),
                operand(OperandType::LargeConstant, 0x7777),
            ],
            opcode(5, 12),
            0x411,
            store(0x410, 0x80),
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x601));
        assert!(zmachine.peek_variable(0).is_err());
        assert!(zmachine.variable(1).is_ok_and(|x| x == 0x1111));
        assert!(zmachine.variable(2).is_ok_and(|x| x == 0x22));
        assert!(zmachine.variable(3).is_ok_and(|x| x == 0x3333));
        assert!(zmachine.variable(4).is_ok_and(|x| x == 0x44));
        assert!(zmachine.variable(5).is_ok_and(|x| x == 0x5555));
        assert!(zmachine.variable(6).is_ok_and(|x| x == 0x66));
        assert!(zmachine.variable(7).is_ok_and(|x| x == 0x7777));
        assert!(zmachine.variable(8).is_ok_and(|x| x == 0));
        assert!(zmachine.variable(9).is_ok_and(|x| x == 0));
        assert!(zmachine.variable(10).is_ok_and(|x| x == 0));
        assert!(zmachine.variable(11).is_err());
        assert!(zmachine.return_routine(0x5678).is_ok_and(|x| x == 0x411));
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0x5678));
    }

    #[test]
    fn test_call_vs2_v8() {
        let mut map = test_map(8);
        mock_routine(&mut map, 0x600, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        let mut zmachine = mock_zmachine(map);
        assert!(zmachine.push(0x1234).is_ok());
        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0xC0),
                operand(OperandType::LargeConstant, 0x1111),
                operand(OperandType::SmallConstant, 0x22),
                operand(OperandType::LargeConstant, 0x3333),
                operand(OperandType::SmallConstant, 0x44),
                operand(OperandType::LargeConstant, 0x5555),
                operand(OperandType::SmallConstant, 0x66),
                operand(OperandType::LargeConstant, 0x7777),
            ],
            opcode(8, 12),
            0x411,
            store(0x410, 0x80),
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x601));
        assert!(zmachine.peek_variable(0).is_err());
        assert!(zmachine.variable(1).is_ok_and(|x| x == 0x1111));
        assert!(zmachine.variable(2).is_ok_and(|x| x == 0x22));
        assert!(zmachine.variable(3).is_ok_and(|x| x == 0x3333));
        assert!(zmachine.variable(4).is_ok_and(|x| x == 0x44));
        assert!(zmachine.variable(5).is_ok_and(|x| x == 0x5555));
        assert!(zmachine.variable(6).is_ok_and(|x| x == 0x66));
        assert!(zmachine.variable(7).is_ok_and(|x| x == 0x7777));
        assert!(zmachine.variable(8).is_ok_and(|x| x == 0));
        assert!(zmachine.variable(9).is_ok_and(|x| x == 0));
        assert!(zmachine.variable(10).is_ok_and(|x| x == 0));
        assert!(zmachine.variable(11).is_err());
        assert!(zmachine.return_routine(0x5678).is_ok_and(|x| x == 0x411));
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0x5678));
    }

    #[test]
    fn test_erase_window_0() {
        let map = test_map(4);
        let mut zmachine = mock_zmachine(map);
        set_split(12);
        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 0)],
            opcode(4, 13),
            0x402,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x402));
        assert_eq!(split(), 12);
        assert_eq!(erase_window(), [0]);
    }

    #[test]
    fn test_erase_window_1() {
        let map = test_map(4);
        let mut zmachine = mock_zmachine(map);
        set_split(12);
        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 1)],
            opcode(4, 13),
            0x402,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x402));
        assert_eq!(split(), 12);
        assert_eq!(erase_window(), [1]);
    }

    #[test]
    fn test_erase_window_both() {
        let map = test_map(4);
        let mut zmachine = mock_zmachine(map);
        set_split(12);
        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 0xFFFE)],
            opcode(4, 13),
            0x402,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x402));
        assert_eq!(split(), 12);
        assert_eq!(erase_window(), [1, 0]);
    }

    #[test]
    fn test_erase_window_unsplit() {
        let map = test_map(4);
        let mut zmachine = mock_zmachine(map);
        set_split(12);
        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 0xFFFF)],
            opcode(4, 13),
            0x402,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x402));
        assert_eq!(split(), 0);
        assert_eq!(erase_window(), [0]);
    }

    #[test]
    fn test_erase_line() {
        let map = test_map(4);
        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 1)],
            opcode(4, 14),
            0x402,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x402));
        assert_eq!(split(), 0);
        assert!(erase_line());
    }

    #[test]
    fn test_set_cursor() {
        let map = test_map(4);
        let mut zmachine = mock_zmachine(map);
        let mut c = (0, 0);
        assert!(zmachine.cursor().is_ok_and(|x| {
            c = x;
            true
        }));

        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, c.0 - 1),
                operand(OperandType::SmallConstant, c.1 + 1),
            ],
            opcode(4, 15),
            0x403,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x403));
        assert!(zmachine
            .cursor()
            .is_ok_and(|x| x.0 == c.0 - 1 && x.1 == c.1 + 1));
    }

    #[test]
    fn test_get_cursor() {
        let map = test_map(4);
        let mut zmachine = mock_zmachine(map);

        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::LargeConstant, 0x300)],
            opcode(4, 16),
            0x403,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x403));
        assert!(zmachine.read_word(0x300).is_ok_and(|x| x == 24));
        assert!(zmachine.read_word(0x302).is_ok_and(|x| x == 1));
    }

    #[test]
    fn test_set_text_style() {
        let map = test_map(4);
        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 1)],
            opcode(4, 17),
            0x402,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x402));
        assert_eq!(split(), 0);
        assert_eq!(style(), 1);
    }

    #[test]
    fn test_set_text_additive() {
        let map = test_map(4);
        let mut zmachine = mock_zmachine(map);
        assert!(zmachine.set_text_style(2).is_ok());
        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 1)],
            opcode(4, 17),
            0x402,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x402));
        assert_eq!(split(), 0);
        assert_eq!(style(), 3);
    }

    #[test]
    fn test_set_text_style_multiple() {
        let map = test_map(4);
        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 3)],
            opcode(4, 17),
            0x402,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x402));
        assert_eq!(split(), 0);
        assert_eq!(style(), 3);
    }

    #[test]
    fn test_set_text_style_roman() {
        let map = test_map(4);
        let mut zmachine = mock_zmachine(map);
        assert!(zmachine.set_text_style(0xF).is_ok());
        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 0)],
            opcode(4, 17),
            0x402,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x402));
        assert_eq!(split(), 0);
        assert_eq!(style(), 0);
    }

    #[test]
    fn test_buffer_mode() {
        let map = test_map(4);
        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 1)],
            opcode(4, 18),
            0x402,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x402));
        assert_eq!(buffer_mode(), 1);
    }

    #[test]
    fn test_output_stream_enable_2() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 2)],
            opcode(4, 19),
            0x402,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x402));
        assert!(Path::new("test-01.txt").exists());
        assert!(fs::remove_file(Path::new("test-01.txt")).is_ok());
        assert_eq!(output_stream(), (3, None));
    }

    #[test]
    fn test_output_stream_disable_1() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 0xFFFF)],
            opcode(4, 19),
            0x402,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x402));
        assert_eq!(output_stream(), (0, None));
    }

    #[test]
    fn test_output_stream_enable_3() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 3),
                operand(OperandType::LargeConstant, 0x300),
            ],
            opcode(4, 19),
            0x403,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x403));
        assert_eq!(output_stream(), (5, Some(0x300)));
    }

    #[test]
    fn test_input_stream_nop() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 2)],
            opcode(4, 20),
            0x402,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x402));
    }

    #[test]
    fn test_sound_effect_1() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 1)],
            opcode(4, 21),
            0x402,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x402));
        assert!(beep());
    }

    #[test]
    fn test_sound_effect_2() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 2)],
            opcode(4, 21),
            0x402,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x402));
        assert!(beep());
    }

    #[test]
    fn test_sound_effect_3() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 3),
                operand(OperandType::SmallConstant, 2),
                operand(OperandType::LargeConstant, 0x20),
            ],
            opcode(4, 21),
            0x405,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x405));
        assert_eq!(play_sound(), (128, 0x20, 1));
    }

    #[test]
    fn test_sound_effect_3_change_volume() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 3),
                operand(OperandType::SmallConstant, 2),
                operand(OperandType::LargeConstant, 0x20),
            ],
            opcode(4, 21),
            0x405,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x405));
        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 3),
                operand(OperandType::SmallConstant, 2),
                operand(OperandType::LargeConstant, 0x30),
            ],
            opcode(4, 21),
            0x405,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x405));
        assert_eq!(play_sound(), (0, 0x30, 0));
    }

    #[test]
    fn test_sound_effect_3_stop() {
        let map = test_map(3);
        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 3),
                operand(OperandType::SmallConstant, 2),
                operand(OperandType::LargeConstant, 0x20),
            ],
            opcode(4, 21),
            0x405,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x405));
        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 3),
                operand(OperandType::SmallConstant, 3),
            ],
            opcode(4, 21),
            0x405,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x405));
        assert_eq!(play_sound(), (0, 0, 0));
    }

    #[test]
    fn test_sound_effect_v5_with_repeat() {
        let map = test_map(5);
        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 4),
                operand(OperandType::SmallConstant, 2),
                operand(OperandType::LargeConstant, 0x1020),
            ],
            opcode(5, 21),
            0x405,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x405));
        assert_eq!(play_sound(), (256, 0x20, 0x10));
    }

    #[test]
    fn test_sound_effect_v5_default_repeat() {
        let map = test_map(5);
        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 4),
                operand(OperandType::SmallConstant, 2),
                operand(OperandType::LargeConstant, 0x0020),
            ],
            opcode(5, 21),
            0x405,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x405));
        assert_eq!(play_sound(), (256, 0x20, 5));
    }

    #[test]
    fn test_sound_effect_v5_with_interrupt() {
        let map = test_map(5);
        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 4),
                operand(OperandType::SmallConstant, 2),
                operand(OperandType::LargeConstant, 0x1020),
                operand(OperandType::LargeConstant, 0x180),
            ],
            opcode(5, 21),
            0x405,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x405));
        assert!(zmachine.sound_interrupt().is_some_and(|x| x == 0x600));
        assert_eq!(play_sound(), (256, 0x20, 16));
    }

    #[test]
    fn test_read_char() {
        let map = test_map(4);
        let mut zmachine = mock_zmachine(map);
        input(&[' ']);
        let i = mock_store_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 1)],
            opcode(4, 22),
            0x403,
            store(0x402, 0x80),
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x403));
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0x20));
    }

    #[test]
    fn test_read_char_timeout() {
        let mut map = test_map(4);
        mock_routine(&mut map, 0x600, &[]);
        let mut zmachine = mock_zmachine(map);
        input(&[' ']);
        set_input_timeout();

        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 1),
                operand(OperandType::LargeConstant, 1),
                operand(OperandType::LargeConstant, 0x180),
            ],
            opcode(4, 22),
            0x406,
            store(0x402, 0x80),
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x601));
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0));
    }

    #[test]
    fn test_read_char_timeout_continue() {
        let mut map = test_map(4);
        mock_routine(&mut map, 0x600, &[]);
        let mut zmachine = mock_zmachine(map);
        zmachine.set_read_interrupt_pending();
        assert!(zmachine.call_read_interrupt(0x600, 0x400).is_ok());
        assert!(zmachine.return_routine(0).is_ok());

        input(&[' ']);

        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 1),
                operand(OperandType::LargeConstant, 1),
                operand(OperandType::LargeConstant, 0x180),
            ],
            opcode(4, 22),
            0x406,
            store(0x402, 0x80),
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x406));
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0x20));
    }

    #[test]
    fn test_read_char_timeout_stop() {
        let mut map = test_map(4);
        mock_routine(&mut map, 0x600, &[]);
        let mut zmachine = mock_zmachine(map);
        zmachine.set_read_interrupt_pending();
        assert!(zmachine.call_read_interrupt(0x600, 0x400).is_ok());
        assert!(zmachine.return_routine(1).is_ok());

        input(&[' ']);

        let i = mock_store_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 1),
                operand(OperandType::LargeConstant, 1),
                operand(OperandType::LargeConstant, 0x180),
            ],
            opcode(4, 22),
            0x406,
            store(0x402, 0x80),
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x406));
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0));
    }

    #[test]
    fn test_scan_table() {
        let mut map = test_map(4);
        map[0x300] = 0x11;
        map[0x301] = 0x22;
        map[0x302] = 0x33;
        map[0x303] = 0x44;
        map[0x304] = 0x55;
        map[0x305] = 0x66;
        map[0x306] = 0x77;
        map[0x307] = 0x88;
        map[0x308] = 0x55;
        map[0x309] = 0x66;

        let mut zmachine = mock_zmachine(map);
        let i = mock_branch_store_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x5566),
                operand(OperandType::LargeConstant, 0x300),
                operand(OperandType::SmallConstant, 5),
            ],
            opcode(4, 23),
            0x408,
            branch(0x406, true, 0x40a),
            store(0x407, 0x80),
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x40a));
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0x304))
    }

    #[test]
    fn test_scan_table_not_found() {
        let mut map = test_map(4);
        set_variable(&mut map, 0x80, 0xFF);
        map[0x300] = 0x11;
        map[0x301] = 0x22;
        map[0x302] = 0x33;
        map[0x303] = 0x44;
        map[0x304] = 0x55;
        map[0x305] = 0x66;
        map[0x306] = 0x77;
        map[0x307] = 0x88;
        map[0x308] = 0x55;
        map[0x309] = 0x66;

        let mut zmachine = mock_zmachine(map);
        let i = mock_branch_store_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x6677),
                operand(OperandType::LargeConstant, 0x300),
                operand(OperandType::SmallConstant, 5),
            ],
            opcode(4, 23),
            0x408,
            branch(0x406, true, 0x40a),
            store(0x407, 0x80),
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x408));
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0))
    }

    #[test]
    fn test_scan_table_field_byte() {
        let mut map = test_map(4);
        set_variable(&mut map, 0x80, 0xFF);
        map[0x300] = 0x11;
        map[0x301] = 0x22;
        map[0x302] = 0x33;
        map[0x303] = 0x44;
        map[0x304] = 0x55;
        map[0x305] = 0x66;
        map[0x306] = 0x77;
        map[0x307] = 0x88;
        map[0x308] = 0x55;
        map[0x309] = 0x66;

        let mut zmachine = mock_zmachine(map);
        let i = mock_branch_store_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x55),
                operand(OperandType::LargeConstant, 0x300),
                operand(OperandType::SmallConstant, 5),
                operand(OperandType::SmallConstant, 0x02),
            ],
            opcode(4, 23),
            0x408,
            branch(0x406, true, 0x40a),
            store(0x407, 0x80),
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x40a));
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0x304))
    }

    #[test]
    fn test_scan_table_field_byte_not_found() {
        let mut map = test_map(4);
        set_variable(&mut map, 0x80, 0xFF);
        map[0x300] = 0x11;
        map[0x301] = 0x22;
        map[0x302] = 0x33;
        map[0x303] = 0x44;
        map[0x304] = 0x55;
        map[0x305] = 0x66;
        map[0x306] = 0x77;
        map[0x307] = 0x88;
        map[0x308] = 0x55;
        map[0x309] = 0x66;

        let mut zmachine = mock_zmachine(map);
        let i = mock_branch_store_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x66),
                operand(OperandType::LargeConstant, 0x300),
                operand(OperandType::SmallConstant, 5),
                operand(OperandType::SmallConstant, 0x02),
            ],
            opcode(4, 23),
            0x408,
            branch(0x406, true, 0x40a),
            store(0x407, 0x80),
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x408));
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0))
    }

    #[test]
    fn test_scan_table_field_word() {
        let mut map = test_map(4);
        set_variable(&mut map, 0x80, 0xFF);
        map[0x300] = 0x11;
        map[0x301] = 0x22;
        map[0x302] = 0x33;
        map[0x303] = 0x44;
        map[0x304] = 0x55;
        map[0x305] = 0x66;
        map[0x306] = 0x77;
        map[0x307] = 0x88;
        map[0x308] = 0x55;
        map[0x309] = 0x66;
        map[0x30a] = 0x99;
        map[0x30b] = 0xAA;

        let mut zmachine = mock_zmachine(map);
        let i = mock_branch_store_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x5566),
                operand(OperandType::LargeConstant, 0x300),
                operand(OperandType::SmallConstant, 3),
                operand(OperandType::SmallConstant, 0x84),
            ],
            opcode(4, 23),
            0x408,
            branch(0x406, true, 0x40a),
            store(0x407, 0x80),
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x40a));
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0x304))
    }

    #[test]
    fn test_scan_table_field_word_not_found() {
        let mut map = test_map(4);
        set_variable(&mut map, 0x80, 0xFF);
        map[0x300] = 0x11;
        map[0x301] = 0x22;
        map[0x302] = 0x33;
        map[0x303] = 0x44;
        map[0x304] = 0x55;
        map[0x305] = 0x66;
        map[0x306] = 0x77;
        map[0x307] = 0x88;
        map[0x308] = 0x55;
        map[0x309] = 0x66;
        map[0x30a] = 0x99;
        map[0x30b] = 0xAA;

        let mut zmachine = mock_zmachine(map);
        let i = mock_branch_store_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x7788),
                operand(OperandType::LargeConstant, 0x300),
                operand(OperandType::SmallConstant, 3),
                operand(OperandType::SmallConstant, 0x84),
            ],
            opcode(4, 23),
            0x408,
            branch(0x406, true, 0x40a),
            store(0x407, 0x80),
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x408));
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0))
    }

    #[test]
    fn test_not() {
        let map = test_map(5);
        let mut zmachine = mock_zmachine(map);
        let i = mock_store_instruction(
            0x400,
            vec![operand(OperandType::LargeConstant, 0x1234)],
            opcode(5, 24),
            0x404,
            store(0x403, 0x80),
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x404));
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0xEDCB));
    }

    #[test]
    fn test_call_vn_v5() {
        let mut map = test_map(5);
        mock_routine(&mut map, 0x600, &[1, 2, 3, 4]);
        let mut zmachine = mock_zmachine(map);
        assert!(zmachine.push(1).is_ok());
        let i = mock_instruction(
            0x401,
            vec![
                operand(OperandType::LargeConstant, 0x180),
                operand(OperandType::SmallConstant, 0x12),
                operand(OperandType::LargeConstant, 0x3456),
                operand(OperandType::LargeConstant, 0xABCD),
            ],
            opcode(5, 25),
            0x409,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x601));
        assert!(zmachine.peek_variable(0).is_err());
        assert!(zmachine.variable(1).is_ok_and(|x| x == 0x12));
        assert!(zmachine.variable(2).is_ok_and(|x| x == 0x3456));
        assert!(zmachine.variable(3).is_ok_and(|x| x == 0xABCD));
        assert!(zmachine.variable(4).is_ok_and(|x| x == 0));
        assert!(zmachine.variable(5).is_err());
        assert!(zmachine.return_routine(0xF0AD).is_ok_and(|x| x == 0x409));
        assert!(zmachine.variable(0).is_ok_and(|x| x == 1));
    }

    #[test]
    fn test_call_vn_v8() {
        let mut map = test_map(8);
        mock_routine(&mut map, 0x600, &[1, 2, 3, 4]);
        let mut zmachine = mock_zmachine(map);
        assert!(zmachine.push(1).is_ok());
        let i = mock_instruction(
            0x401,
            vec![
                operand(OperandType::LargeConstant, 0xC0),
                operand(OperandType::SmallConstant, 0x12),
                operand(OperandType::LargeConstant, 0x3456),
                operand(OperandType::LargeConstant, 0xABCD),
            ],
            opcode(8, 25),
            0x409,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x601));
        assert!(zmachine.peek_variable(0).is_err());
        assert!(zmachine.variable(1).is_ok_and(|x| x == 0x12));
        assert!(zmachine.variable(2).is_ok_and(|x| x == 0x3456));
        assert!(zmachine.variable(3).is_ok_and(|x| x == 0xABCD));
        assert!(zmachine.variable(4).is_ok_and(|x| x == 0));
        assert!(zmachine.variable(5).is_err());
        assert!(zmachine.return_routine(0xF0AD).is_ok_and(|x| x == 0x409));
        assert!(zmachine.variable(0).is_ok_and(|x| x == 1));
    }

    #[test]
    fn test_call_vn2_v5() {
        let mut map = test_map(5);
        mock_routine(&mut map, 0x600, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        let mut zmachine = mock_zmachine(map);
        assert!(zmachine.push(0x1234).is_ok());
        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x180),
                operand(OperandType::LargeConstant, 0x1111),
                operand(OperandType::SmallConstant, 0x22),
                operand(OperandType::LargeConstant, 0x3333),
                operand(OperandType::SmallConstant, 0x44),
                operand(OperandType::LargeConstant, 0x5555),
                operand(OperandType::SmallConstant, 0x66),
                operand(OperandType::LargeConstant, 0x7777),
            ],
            opcode(5, 26),
            0x411,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x601));
        assert!(zmachine.peek_variable(0).is_err());
        assert!(zmachine.variable(1).is_ok_and(|x| x == 0x1111));
        assert!(zmachine.variable(2).is_ok_and(|x| x == 0x22));
        assert!(zmachine.variable(3).is_ok_and(|x| x == 0x3333));
        assert!(zmachine.variable(4).is_ok_and(|x| x == 0x44));
        assert!(zmachine.variable(5).is_ok_and(|x| x == 0x5555));
        assert!(zmachine.variable(6).is_ok_and(|x| x == 0x66));
        assert!(zmachine.variable(7).is_ok_and(|x| x == 0x7777));
        assert!(zmachine.variable(8).is_ok_and(|x| x == 0));
        assert!(zmachine.variable(9).is_ok_and(|x| x == 0));
        assert!(zmachine.variable(10).is_ok_and(|x| x == 0));
        assert!(zmachine.variable(11).is_err());
        assert!(zmachine.return_routine(0x5678).is_ok_and(|x| x == 0x411));
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0));
    }

    #[test]
    fn test_call_vn2_v8() {
        let mut map = test_map(8);
        mock_routine(&mut map, 0x600, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        let mut zmachine = mock_zmachine(map);
        assert!(zmachine.push(0x1234).is_ok());
        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0xC0),
                operand(OperandType::LargeConstant, 0x1111),
                operand(OperandType::SmallConstant, 0x22),
                operand(OperandType::LargeConstant, 0x3333),
                operand(OperandType::SmallConstant, 0x44),
                operand(OperandType::LargeConstant, 0x5555),
                operand(OperandType::SmallConstant, 0x66),
                operand(OperandType::LargeConstant, 0x7777),
            ],
            opcode(8, 26),
            0x411,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x601));
        assert!(zmachine.peek_variable(0).is_err());
        assert!(zmachine.variable(1).is_ok_and(|x| x == 0x1111));
        assert!(zmachine.variable(2).is_ok_and(|x| x == 0x22));
        assert!(zmachine.variable(3).is_ok_and(|x| x == 0x3333));
        assert!(zmachine.variable(4).is_ok_and(|x| x == 0x44));
        assert!(zmachine.variable(5).is_ok_and(|x| x == 0x5555));
        assert!(zmachine.variable(6).is_ok_and(|x| x == 0x66));
        assert!(zmachine.variable(7).is_ok_and(|x| x == 0x7777));
        assert!(zmachine.variable(8).is_ok_and(|x| x == 0));
        assert!(zmachine.variable(9).is_ok_and(|x| x == 0));
        assert!(zmachine.variable(10).is_ok_and(|x| x == 0));
        assert!(zmachine.variable(11).is_err());
        assert!(zmachine.return_routine(0x5678).is_ok_and(|x| x == 0x411));
        assert!(zmachine.variable(0x80).is_ok_and(|x| x == 0));
    }

    #[test]
    fn test_tokenise() {
        let mut map = test_map(5);
        mock_dictionary(&mut map);
        // text buffer
        map[0x380] = 16;
        map[0x381] = 11;
        map[0x382] = b's';
        map[0x383] = b'a';
        map[0x384] = b'i';
        map[0x385] = b'l';
        map[0x386] = b'o';
        map[0x387] = b'r';
        map[0x388] = b' ';
        map[0x389] = b'm';
        map[0x38A] = b'o';
        map[0x38B] = b'o';
        map[0x38C] = b'n';

        map[0x3A0] = 2;

        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x380),
                operand(OperandType::LargeConstant, 0x3A0),
            ],
            opcode(5, 27),
            0x405,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x405));
        assert!(zmachine.read_byte(0x3A1).is_ok_and(|x| x == 2));
        assert!(zmachine.read_word(0x3A2).is_ok_and(|x| x == 0x322));
        assert!(zmachine.read_byte(0x3A4).is_ok_and(|x| x == 6));
        assert!(zmachine.read_byte(0x3A5).is_ok_and(|x| x == 2));
        assert!(zmachine.read_word(0x3A6).is_ok_and(|x| x == 0));
        assert!(zmachine.read_byte(0x3A8).is_ok_and(|x| x == 4));
        assert!(zmachine.read_byte(0x3A9).is_ok_and(|x| x == 9));
    }

    #[test]
    fn test_tokenise_custom_dictionary() {
        let mut map = test_map(5);
        mock_dictionary(&mut map);
        mock_custom_dictionary(&mut map, 0x340);

        // text buffer
        map[0x380] = 16;
        map[0x381] = 11;
        map[0x382] = b's';
        map[0x383] = b'a';
        map[0x384] = b'i';
        map[0x385] = b'l';
        map[0x386] = b'o';
        map[0x387] = b'r';
        map[0x388] = b' ';
        map[0x389] = b'm';
        map[0x38A] = b'o';
        map[0x38B] = b'o';
        map[0x38C] = b'n';

        map[0x3A0] = 2;
        map[0x3A1] = 2;
        map[0x3A2] = 0x03;
        map[0x3A3] = 0x22;
        map[0x3A4] = 6;
        map[0x3A5] = 2;
        map[0x3A6] = 0;
        map[0x3A7] = 0;
        map[0x3A8] = 4;
        map[0x3A9] = 9;

        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x380),
                operand(OperandType::LargeConstant, 0x3A0),
                operand(OperandType::LargeConstant, 0x340),
            ],
            opcode(5, 27),
            0x408,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x408));
        assert!(zmachine.read_byte(0x3A1).is_ok_and(|x| x == 2));
        assert!(zmachine.read_word(0x3A2).is_ok_and(|x| x == 0));
        assert!(zmachine.read_byte(0x3A4).is_ok_and(|x| x == 6));
        assert!(zmachine.read_byte(0x3A5).is_ok_and(|x| x == 2));
        assert!(zmachine.read_word(0x3A6).is_ok_and(|x| x == 0x359));
        assert!(zmachine.read_byte(0x3A8).is_ok_and(|x| x == 4));
        assert!(zmachine.read_byte(0x3A9).is_ok_and(|x| x == 9));
    }

    #[test]
    fn test_tokenise_custom_dictionary_flag() {
        let mut map = test_map(5);
        mock_dictionary(&mut map);
        mock_custom_dictionary(&mut map, 0x340);

        // text buffer
        map[0x380] = 16;
        map[0x381] = 11;
        map[0x382] = b's';
        map[0x383] = b'a';
        map[0x384] = b'i';
        map[0x385] = b'l';
        map[0x386] = b'o';
        map[0x387] = b'r';
        map[0x388] = b' ';
        map[0x389] = b'm';
        map[0x38A] = b'o';
        map[0x38B] = b'o';
        map[0x38C] = b'n';

        map[0x3A0] = 2;
        map[0x3A1] = 2;
        map[0x3A2] = 0x03;
        map[0x3A3] = 0x22;
        map[0x3A4] = 6;
        map[0x3A5] = 2;
        map[0x3A6] = 0;
        map[0x3A7] = 0;
        map[0x3A8] = 4;
        map[0x3A9] = 9;

        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x380),
                operand(OperandType::LargeConstant, 0x3A0),
                operand(OperandType::LargeConstant, 0x340),
                operand(OperandType::SmallConstant, 1),
            ],
            opcode(5, 27),
            0x408,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x408));
        assert!(zmachine.read_byte(0x3A1).is_ok_and(|x| x == 2));
        assert!(zmachine.read_word(0x3A2).is_ok_and(|x| x == 0x322));
        assert!(zmachine.read_byte(0x3A4).is_ok_and(|x| x == 6));
        assert!(zmachine.read_byte(0x3A5).is_ok_and(|x| x == 2));
        assert!(zmachine.read_word(0x3A6).is_ok_and(|x| x == 0x359));
        assert!(zmachine.read_byte(0x3A8).is_ok_and(|x| x == 4));
        assert!(zmachine.read_byte(0x3A9).is_ok_and(|x| x == 9));
    }

    #[test]
    fn test_encode_text() {
        let mut map = test_map(5);
        map[0x308] = b'm';
        map[0x309] = b'o';
        map[0x30A] = b'o';
        map[0x30B] = b'n';
        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x300),
                operand(OperandType::SmallConstant, 4),
                operand(OperandType::SmallConstant, 8),
                operand(OperandType::LargeConstant, 0x320),
            ],
            opcode(5, 28),
            0x407,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x407));
        assert!(zmachine.read_word(0x320).is_ok_and(|x| x == 0x4A94));
        assert!(zmachine.read_word(0x322).is_ok_and(|x| x == 0x4CA5));
        assert!(zmachine.read_word(0x324).is_ok_and(|x| x == 0x94A5));
    }

    #[test]
    fn test_copy_table() {
        let mut map = test_map(5);
        for i in 0..0x20 {
            map[0x300 + i] = i as u8 + 1;
            map[0x320 + i] = 0x80 + (i as u8 + 1);
        }

        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x300),
                operand(OperandType::LargeConstant, 0x320),
                operand(OperandType::SmallConstant, 0x20),
            ],
            opcode(5, 29),
            0x406,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x406));
        for i in 0..0x20 {
            assert!(zmachine
                .read_byte(0x320 + i)
                .is_ok_and(|x| x == i as u8 + 1));
        }
    }

    #[test]
    fn test_copy_table_zero() {
        let mut map = test_map(5);
        for i in 0..0x20 {
            map[0x300 + i] = i as u8 + 1;
        }

        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x300),
                operand(OperandType::LargeConstant, 0),
                operand(OperandType::SmallConstant, 0x20),
            ],
            opcode(5, 29),
            0x406,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x406));
        for i in 0..0x20 {
            assert!(zmachine.read_byte(0x300 + i).is_ok_and(|x| x == 0));
        }
    }

    #[test]
    fn test_copy_table_overlap() {
        let mut map = test_map(5);
        for i in 0..0x20 {
            map[0x300 + i] = i as u8 + 1;
        }

        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x300),
                operand(OperandType::LargeConstant, 0x310),
                operand(OperandType::SmallConstant, 0x20),
            ],
            opcode(5, 29),
            0x406,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x406));
        for i in 0..0x20 {
            assert!(zmachine
                .read_byte(0x310 + i)
                .is_ok_and(|x| x == i as u8 + 1));
        }
    }

    #[test]
    fn test_copy_table_destructive() {
        let mut map = test_map(5);
        for i in 0..0x20 {
            map[0x300 + i] = i as u8 + 1;
        }

        let mut zmachine = mock_zmachine(map);
        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x300),
                operand(OperandType::LargeConstant, 0x310),
                operand(OperandType::SmallConstant, 0xFFD0),
            ],
            opcode(5, 29),
            0x406,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x406));
        for i in 0..0x10 {
            assert!(zmachine
                .read_byte(0x310 + i)
                .is_ok_and(|x| x == i as u8 + 1));
            assert!(zmachine
                .read_byte(0x320 + i)
                .is_ok_and(|x| x == i as u8 + 1));
        }
    }

    #[test]
    fn test_print_table() {
        let mut map = test_map(5);
        for i in 0..8 {
            for j in 0..8 {
                map[0x300 + (i * 8) + j] = b'a' + j as u8;
            }
        }

        let mut zmachine = mock_zmachine(map);
        assert!(zmachine.set_cursor(5, 8).is_ok());
        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x300),
                operand(OperandType::SmallConstant, 8),
                operand(OperandType::SmallConstant, 8),
            ],
            opcode(5, 30),
            0x405,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x405));
        assert_print("abcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefgh");
        assert!(zmachine.cursor().is_ok_and(|x| x == (12, 16)));
    }

    #[test]
    fn test_print_table_skip() {
        let mut map = test_map(5);
        for i in 0..8 {
            for j in 0..8 {
                map[0x300 + (i * 8) + j] = b'a' + j as u8;
            }
        }

        let mut zmachine = mock_zmachine(map);
        assert!(zmachine.set_cursor(5, 8).is_ok());
        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::LargeConstant, 0x300),
                operand(OperandType::SmallConstant, 4),
                operand(OperandType::SmallConstant, 8),
                operand(OperandType::SmallConstant, 4),
            ],
            opcode(5, 30),
            0x406,
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x406));
        assert_print("abcdabcdabcdabcdabcdabcdabcdabcd");
        println!("{:?}", zmachine.cursor().unwrap());
        assert!(zmachine.cursor().is_ok_and(|x| x == (12, 12)));
    }

    #[test]
    fn test_check_arg_count_true() {
        let mut map = test_map(5);
        mock_routine(&mut map, 0x600, &[1, 2, 3, 4]);
        let mut zmachine = mock_zmachine(map);
        assert!(zmachine
            .call_routine(0x600, &vec![0x1122, 0x2233], None, 0x400)
            .is_ok());
        let i = mock_branch_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 2)],
            opcode(5, 31),
            0x403,
            branch(0x402, true, 0x40a),
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x40a));
    }

    #[test]
    fn test_check_arg_count_false() {
        let mut map = test_map(5);
        mock_routine(&mut map, 0x600, &[1, 2, 3, 4]);
        let mut zmachine = mock_zmachine(map);
        assert!(zmachine
            .call_routine(0x600, &vec![0x1122, 0x2233], None, 0x400)
            .is_ok());
        let i = mock_branch_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 3)],
            opcode(5, 31),
            0x403,
            branch(0x402, true, 0x40a),
        );
        assert!(dispatch(&mut zmachine, &i).is_ok_and(|x| x == 0x403));
    }
}
