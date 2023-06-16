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
    let routine = if operands.len() > 2 { operands[3] } else { 0 };

    let mut input_buffer = Vec::new();

    if version < 4 {
        // V3 show status line before input
        state.status_line()?;
    } else if version > 4 {
        // text buffer may contain existing input
        let existing_len = state.read_byte(text_buffer + 1)? as usize;
        for i in 0..existing_len {
            input_buffer.push(state.read_byte(text_buffer + 2 + i)? as u16);
        }
    }

    info!(target: "app::input", "READ initial input: {:?}", input_buffer);

    let terminators = terminators(state)?;
    loop {
        match state.read_key(timeout)? {
            Some(key) => {
                if terminators.contains(&key) {
                    input_buffer.push(key);
                    state.print(&vec![key])?;
                    break;
                } else {
                    if input_buffer.len() < len {
                        if key == 0x08 {
                            if input_buffer.len() > 0 {
                                input_buffer.pop();
                                state.backspace()?;
                            }
                        } else if key >= 0x1f && key <= 0x7f {
                            input_buffer.push(key);
                            state.print(&vec![key])?;
                        }
                    }
                }
            }
            None => break,
        }
    }

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

    // TODO: If terminator is None, then input timed out, so do the needful
    if let None = terminator {
        todo!("Implement input timeout");
    }

    let end = input_buffer.len()
        - match terminator {
            Some(_) => 1,
            None => 0,
        };

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
    if parse > 0 || state.version < 5 {
        let dictionary = header::field_word(state.memory(), HeaderField::Dictionary)? as usize;
        let separators = text::separators(state, dictionary)?;
        let mut word = Vec::new();
        let mut word_start: usize = 0;
        let mut word_count: usize = 0;
        let max_words = state.read_byte(parse)? as usize;

        let data = input_buffer[0..end].to_vec();
        for i in 0..data.len() {
            let c = ((data[i] as u8) as char).to_ascii_lowercase();
            if word_count > max_words {
                break;
            }

            if separators.contains(&c) {
                // Store the word
                if word.len() > 0 {
                    let entry = text::from_dictionary(state, dictionary, &word)?;
                    let parse_address = parse + 2 + (4 * word_count);
                    store_parsed_entry(state, &word, word_start + 1, parse_address, entry as u16)?;
                    // info!(target: "app::input", "READ: dictionary for {:?} => ${:04x} store to ${:04x}", word, entry, parse_address);

                    // state.write_word(parse_address, entry as u16)?;
                    // state.write_byte(parse_address + 2, word.len() as u8)?;
                    // state.write_byte(parse_address + 3, word_start as u8 + 2)?;
                    word_count = word_count + 1;
                }

                // Store the separator
                if word_count < max_words {
                    let sep = vec![c];
                    let entry = text::from_dictionary(state, dictionary, &sep)?;
                    let parse_address = parse + 2 + (4 * word_count);
                    store_parsed_entry(
                        state,
                        &sep,
                        word_start + word.len() + 1,
                        parse_address,
                        entry as u16,
                    )?;

                    // let entry = text::from_dictionary(state, dictionary, &vec![c])?;
                    // info!(target: "app::input", "READ: dictionary for {:?} => ${:04x}", vec![c], entry);
                    // state.write_word(parse + 2 + (4 * word_count), entry as u16)?;
                    // state.write_byte(parse + 4 + (4 * word_count), 1)?;
                    // state.write_byte(parse + 5 + (4 * word_count), i as u8 + 2)?;
                    word_count = word_count + 1;
                }
                word.clear();
                word_start = i + 1;
            } else if c == ' ' {
                // Store the word but not the space
                if word.len() > 0 {
                    let entry = text::from_dictionary(state, dictionary, &word)?;
                    let parse_address = parse + 2 + (4 * word_count);
                    store_parsed_entry(state, &word, word_start + 1, parse_address, entry as u16)?;

                    // let entry = text::from_dictionary(state, dictionary, &word)?;
                    // info!(target: "app::input", "READ: dictionary for {:?} => ${:04x}", word, entry);
                    // state.write_word(parse + 2 + (4 * word_count), entry as u16)?;
                    // state.write_byte(parse + 4 + (4 * word_count), word.len() as u8)?;
                    // state.write_byte(parse + 5 + (4 * word_count), word_start as u8 + 2)?;
                    word_count = word_count + 1;
                }
                word.clear();
                word_start = i + 1;
            } else {
                word.push(c)
            }
        }

        // End of input, parse anything collected
        if word.len() > 0 && word_count < max_words {
            let entry = text::from_dictionary(state, dictionary, &word)?;
            let parse_address = parse + 2 + (4 * word_count);
            store_parsed_entry(state, &word, word_start + 1, parse_address, entry as u16)?;

            // let entry = text::from_default_dictionary(state, &word)?;
            // info!(target: "app::input", "READ: dictionary for {:?} => ${:04x}", word, entry);
            // state.write_word(parse + 2 + (4 * word_count), entry as u16)?;
            // state.write_byte(parse + 4 + (4 * word_count), word.len() as u8)?;
            // state.write_byte(parse + 5 + (4 * word_count), word_start as u8 + 2)?;
            word_count = word_count + 1;
        }

        info!(target: "app::input", "READ: parsed {} words", word_count);
        state.write_byte(parse + 1, word_count as u8)?;
    }

    if version > 4 {
        if let Some(t) = terminator {
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
    state.print_num(operands[0] as i16)?;
    // context.print_string(format!("{}", operands[0] as i16));
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

pub fn erase_window(state: &mut State, instruction: &Instruction) -> Result<usize, RuntimeError> {
    let operands = operand_values(state, instruction)?;
    state.erase_window(operands[0] as i16)?;
    Ok(instruction.next_address())
}

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
