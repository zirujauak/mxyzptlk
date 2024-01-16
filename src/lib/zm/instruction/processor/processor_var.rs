//! [VAR](https://inform-fiction.org/zmachine/standards/z1point1/sect14.html#VAR)
//! instructions: Variable form instructions.

use crate::{
    error::{ErrorCode, RuntimeError},
    fatal_error,
    instruction::{processor::store_result, Instruction, InstructionResult, NextAddress::Address},
    object::property,
    recoverable_error, text,
    zmachine::{header::HeaderField, RequestType, ZMachine},
};

use super::{branch, call_routine, operand_values};

/// [CALL/CALL_VS](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#call): calls the
/// routine at the packed address in operand 0 with any additional operands as arguments.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing an [InstructionResult] or a [RuntimeError]
pub fn call_vs(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let address = zmachine.packed_routine_address(operands[0])?;
    let arguments = &operands[1..].to_vec();

    InstructionResult::new(call_routine(
        zmachine,
        address,
        instruction.next_address,
        arguments,
        instruction.store,
    )?)
}

/// [STOREW](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#storew): writes the
/// word value in operand 2 to the array at byte address in operand 0 indexed by operand 1.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing an [InstructionResult] or a [RuntimeError]
pub fn storew(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let address = operands[0] as isize + (operands[1] as i16 * 2) as isize;
    zmachine.write_word(address as usize, operands[2])?;
    // If storing to Flags2, bit 0 is used to enable/disable transcripting
    if address == 0x10 {
        if operands[2] & 1 == 1 {
            zmachine.output_stream(2, None)?;
            InstructionResult::output_stream(Address(instruction.next_address), 2, zmachine.name())
        } else {
            zmachine.output_stream(-2, None)?;
            InstructionResult::output_stream(Address(instruction.next_address), -2, zmachine.name())
        }
    } else {
        InstructionResult::new(Address(instruction.next_address))
    }
}

/// [STOREB](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#storeb): writes the
/// byte value in operand 2 to the array at byte address in operand 0 indexed by operand 1.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing an [InstructionResult] or a [RuntimeError]
pub fn storeb(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let address = operands[0] as isize + (operands[1] as i16) as isize;
    zmachine.write_byte(address as usize, operands[2] as u8)?;
    // If storing to the low byte of Flags2, bit 0 is used to enable/disable transcripting
    if address == 0x11 {
        if operands[2] & 1 == 1 {
            zmachine.output_stream(2, None)?;
            InstructionResult::output_stream(Address(instruction.next_address), 2, zmachine.name())
        } else {
            zmachine.output_stream(-2, None)?;
            InstructionResult::output_stream(Address(instruction.next_address), -2, zmachine.name())
        }
    } else {
        InstructionResult::new(Address(instruction.next_address))
    }
}

/// [PUT_PROP](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#put_prop): sets the 1-
/// or 2-byte property in operand 1 on the object in operand 0 to the value in operand 2.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing an [InstructionResult] or a [RuntimeError]
pub fn put_prop(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;

    property::set_property(
        zmachine,
        operands[0] as usize,
        operands[1] as u8,
        operands[2],
    )?;
    InstructionResult::new(Address(instruction.next_address))
}

/// Returns an array of input terminator characters.  For versions 5 and greater,
/// the optional terminator table as specified in the header is read.
///
/// # Arguments
/// * `zmachine` - reference to the zmachine
///
/// # Returns
/// Vector of terminator characters, consistent of carriage return and
/// any characters in the terminator table.
fn terminators(zmachine: &ZMachine) -> Result<Vec<u16>, RuntimeError> {
    let mut terminators = vec!['\r' as u16];

    if zmachine.version() > 4 {
        let mut table_addr = zmachine.header_word(HeaderField::TerminatorTable)? as usize;
        if table_addr > 0 {
            loop {
                let b = zmachine.read_byte(table_addr)?;
                if b == 0 {
                    break;
                } else if (129..155).contains(&b) || b >= 252 {
                    terminators.push(b as u16);
                }
                table_addr += 1;
            }
        }
    }

    Ok(terminators)
}

/// Returns the lower-case variant of an alpha ASCII character.
///
/// # Arguments
/// * `c` - character to cast to lower-case
///
/// # Returns
/// The lower-case variant of the character if `c` is alpha, else the
/// original character.
fn to_lower_case(c: u16) -> u8 {
    // Uppercase ASCII is 0x41 - 0x5A
    if c > 0x40 && c < 0x5b {
        // Lowercase ASCII is 0x61 - 0x7A, so OR 0x20 to convert
        (c | 0x20) as u8
    } else {
        c as u8
    }
}

/// [AREAD/SREAD](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#read): prepares
/// a Read interpreter request.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing an [InstructionResult] with a [RequestType::Read] interpreter request
/// or a [RuntimeError]
pub fn read_pre(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    // TBD handle return from timeout interrupt ... may need to pass existing input
    let operands = operand_values(zmachine, instruction)?;
    let text_buffer = operands[0] as usize;
    let length = if zmachine.version() < 5 {
        zmachine.read_byte(text_buffer)? - 1
    } else {
        zmachine.read_byte(text_buffer)?
    };

    let timeout = if operands.len() > 2 { operands[2] } else { 0 };
    let terminators = terminators(zmachine)?;

    // For V4+, the text buffer may already contain input
    let mut preload = Vec::new();
    match zmachine.version() {
        4 => {
            let mut i = 1;
            loop {
                let b = zmachine.read_byte(text_buffer + i)? as u16;
                if b == 0 {
                    break;
                }
                preload.push(b);
                i += 1;
            }
        }
        5..=8 => {
            let existing_len = zmachine.read_byte(text_buffer + 1)? as usize;
            for i in 0..existing_len {
                preload.push(zmachine.read_byte(text_buffer + 2 + i)? as u16)
            }
        }
        _ => {}
    }

    debug!(target: "app::screen", "Preload input: {:?}", preload);

    // Pass max input length and timeout, if any, to the interpreter
    InstructionResult::read(
        Address(instruction.next_address),
        length,
        terminators,
        timeout,
        preload,
        zmachine.is_stream_enabled(2),
    )
}

/// [AREAD/SREAD](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#read): processes
/// the input returned by the interpreter
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing an [InstructionResult] or a [RuntimeError]
pub fn read_post(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
    input_buffer: Vec<u16>,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let text_buffer = operands[0] as usize;
    let parse = if operands.len() > 1 {
        operands[1] as usize
    } else {
        0
    };

    let terminator = if let Some(t) = input_buffer.last() {
        t
    } else {
        return fatal_error!(ErrorCode::InvalidInput, "READ returned no input");
    };

    let end = input_buffer.len() - 1;

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
        store_result(zmachine, instruction, *terminator)?;
    }

    InstructionResult::new(Address(instruction.next_address))
}

/// [PRINT_CHAR](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#print_char): prints
/// a character.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing an [InstructionResult] with a [RequestType::Print] interpreter request
/// or a [RuntimeError]
pub fn print_char(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    zmachine.output(
        &[operands[0]],
        Address(instruction.next_address),
        RequestType::Print,
    )
}

/// [PRINT_NUM](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#print_num): prints
/// a number.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing an [InstructionResult] with a [RequestType::PRINT] interpreter request
/// or a [RuntimeError]
pub fn print_num(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let s = format!("{}", operands[0] as i16);
    let mut text = Vec::new();
    for c in s.chars() {
        text.push(c as u16);
    }
    zmachine.output(&text, Address(instruction.next_address), RequestType::Print)
}

/// [RANDOM](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#random): generates a random number
/// or seeds the RNG.
///
/// If operand 0 is:
/// * ..=-1000 - seeds the RNG with the absolute value, storing 0
/// * -999..=0 - sets the RNG into predictable mode, returning 1..=operand[0] in sequence, storing 0
/// * 1.. -  generates a random number from 1..=operand[0], storing the result
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing an [InstructionResult] or a [RuntimeError]
pub fn random(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
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

    InstructionResult::new(Address(instruction.next_address))
}

/// [PUSH](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#push): push operand 0
/// onto the stack.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing an [InstructionResult] or a [RuntimeError]
pub fn push(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    zmachine.push(operands[0])?;
    InstructionResult::new(Address(instruction.next_address))
}

/// [PULL](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#pull): pulls the value on
/// top of the stack and stores it.
///
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing an [InstructionResult] or a [RuntimeError]
pub fn pull(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let value = zmachine.variable(0)?;

    // If pulling to the stack, need to remove what was underneath the
    // value pulled before pushing it back.  This effectively discards
    // the second value in the stack.
    if operands[0] == 0 {
        zmachine.variable(0)?;
    }

    zmachine.set_variable(operands[0] as u8, value)?;
    InstructionResult::new(Address(instruction.next_address))
}

/// [SPLIT_WINDOW](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#split_window): prepares
/// a split window interpreter request
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing an [InstructionResult] with a [RequestType::SplitWindow] interpreter request
/// or a [RuntimeError]
pub fn split_window(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    InstructionResult::split_window(Address(instruction.next_address), operands[0])
}

/// [SET_WINDOW](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#set_window): prepares
/// a set window interpreter request
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing an [InstructionResult] with a [RequestType::SetWindow] interpreter request
/// or a [RuntimeError]
pub fn set_window(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    InstructionResult::set_window(Address(instruction.next_address), operands[0])
}

/// [CALL_VS2](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#call_vs2): calls the routing
/// at the packed address in operand 0 with any remaining operands as arguments.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing an [InstructionResult] or a [RuntimeError]
pub fn call_vs2(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let address = zmachine.packed_routine_address(operands[0])?;
    let arguments = operands[1..operands.len()].to_vec();

    InstructionResult::new(call_routine(
        zmachine,
        address,
        instruction.next_address,
        &arguments,
        instruction.store,
    )?)
}

/// [ERASE_WINDOW](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#erase_window): prepares
/// an erase window interpreter request
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing an [InstructionResult] with a [RequestType::EraseWindow] interpreter request
/// or a [RuntimeError]
pub fn erase_window(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    InstructionResult::erase_window(Address(instruction.next_address), operands[0] as i16)
}

/// [ERASE_LINE](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#erase_line): prepares
/// a erase line interpreter request
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing an [InstructionResult] with a [RequestType::EraseLine] interpreter request
/// or a [RuntimeError]
pub fn erase_line(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    if operands[0] == 1 {
        InstructionResult::erase_line(Address(instruction.next_address))
    } else {
        InstructionResult::new(Address(instruction.next_address))
    }
}

/// [SET_CURSOR](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#set_cursor): prepares
/// a set cursor interpreter request
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing an [InstructionResult] with a [RequestType::SetCursor] interpreter request
/// or a [RuntimeError]
pub fn set_cursor(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    InstructionResult::set_cursor(Address(instruction.next_address), operands[0], operands[1])
}

/// [GET_CURSOR](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#get_cursor): prepares
/// a get cursor interpreter request
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing an [InstructionResult] with a [RequestType::GetCursor] interpreter request
/// or a [RuntimeError]
pub fn get_cursor_pre(
    _zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    InstructionResult::get_cursor(Address(instruction.next_address))
}

/// [SET_TEXT_STYLE](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#set_text_style): prepares
/// a set text style interpreter request
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing an [InstructionResult] with a [RequestType::SetTextStyle] interpreter request
/// or a [RuntimeError]
pub fn set_text_style(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    InstructionResult::set_text_style(Address(instruction.next_address), operands[0])
}

/// [BUFFER_MODE](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#buffer_mode): prepares
/// a buffer mode interpreter request
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing an [InstructionResult] with a [RequestType::BufferMode] interpreter request
/// or a [RuntimeError]
pub fn buffer_mode(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    InstructionResult::buffer_mode(Address(instruction.next_address), operands[0])
}

/// [SPLIT_WINDOW](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#pull): enables or disables
/// the output stream in operand 0.  Stream 3 must include a byte address in operand 1 when enabled.
///
/// If operand 0 is positive, the stream is enabled, otherwise the stream is disabled.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing an [InstructionResult] with a [RequestType::OutputStream] interpreter request
/// or a [RuntimeError]
pub fn output_stream(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let stream = operands[0] as i16;
    let table = if stream == 3 {
        Some(operands[1] as usize)
    } else {
        None
    };

    zmachine.output_stream(stream, table)?;
    InstructionResult::output_stream(Address(instruction.next_address), stream, zmachine.name())
}

/// [INPUT_STREAM](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#input_stream): enable or
/// disable an input stream.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing an [InstructionResult] with a [RequestType::InputStream] interpreter request
/// or a [RuntimeError]
pub fn input_stream(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    InstructionResult::input_stream(Address(instruction.next_address), operands[0] as i16)
}

/// [SOUND_EFFECT](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#sound_effect): prepares
/// a sound effect interpreter request
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing an [InstructionResult] with a [RequestType::SoundEffect] interpreter request
/// or a [RuntimeError]
pub fn sound_effect_pre(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands: Vec<u16> = operand_values(zmachine, instruction)?;
    let number = operands[0];
    match number {
        1 | 2 => {
            InstructionResult::sound_effect(Address(instruction.next_address), number, 0, 0, 0, 0)
        }
        _ => {
            let effect = operands[1];
            match effect {
                // Prepare, Stop, Unload
                1 | 3 | 4 => InstructionResult::sound_effect(
                    Address(instruction.next_address),
                    number,
                    effect,
                    0,
                    0,
                    0,
                ),
                // Play
                2 => {
                    let (volume, repeats) = if operands.len() > 2 {
                        (
                            (operands[2] & 0xFF) as u8,
                            ((operands[2] >> 8) & 0xFF) as u8,
                        )
                    } else {
                        (255, 1)
                    };
                    let routine = if operands.len() > 3 {
                        zmachine.packed_routine_address(operands[3])?
                    } else {
                        0
                    };

                    InstructionResult::sound_effect(
                        Address(instruction.next_address),
                        number,
                        effect,
                        volume,
                        repeats,
                        routine,
                    )
                }
                _ => {
                    // TBD: Beep here?
                    recoverable_error!(
                        ErrorCode::InvalidSoundEffect,
                        "Invalid sound effect {}",
                        effect,
                    )
                }
            }
        }
    }
}

/// [READ_CHAR](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#read_char): prepares
/// a read char interpreter request
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing an [InstructionResult] with a [RequestType::ReadChar] interpreter request
/// or a [RuntimeError]
pub fn read_char_pre(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    if !operands.is_empty() && operands[0] != 1 {
        return fatal_error!(
            ErrorCode::InvalidInstruction,
            "READ_CHAR first argument must be 1, was {}",
            operands[0]
        );
    }

    let timeout = if operands.len() > 1 { operands[1] } else { 0 };
    InstructionResult::read_char(Address(instruction.next_address), timeout)
}

/// [SCAN_TABLE](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#scan_table): scans the
/// table at the byte address in operand 1, which is operand 2 fields long, for the value in
/// operand 0.  
///
/// If a fourth operands is present, bit 7 is set for words or clear for bytes. The remaining 7
/// bits indicate the size of each table entry in bytes. Only the first byte or word of
/// each entry is scanned.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing an [InstructionResult] or a [RuntimeError]
pub fn scan_table(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
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

    InstructionResult::new(branch(zmachine, instruction, condition)?)
}

/// [NOT](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#not): stores the bitwise
/// not of operand 0.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing an [InstructionResult] or a [RuntimeError]
pub fn not(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    store_result(zmachine, instruction, !operands[0])?;
    InstructionResult::new(Address(instruction.next_address))
}

/// [CALL_VN](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#call_vn): calls the routine
/// at the packed address in operand 0 with any additional operands as arguments without string a result.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing an [InstructionResult] or a [RuntimeError]
pub fn call_vn(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let address = zmachine.packed_routine_address(operands[0])?;
    let arguments = &operands[1..].to_vec();

    InstructionResult::new(call_routine(
        zmachine,
        address,
        instruction.next_address,
        arguments,
        instruction.store,
    )?)
}

/// [CALL_VN2](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#call_vn2): calls the routine
/// at the packed address in operand 0 with any additional operands as arugments without storing a result
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing an [InstructionResult] or a [RuntimeError]
pub fn call_vn2(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let address = zmachine.packed_routine_address(operands[0])?;
    let arguments = &operands[1..].to_vec();

    InstructionResult::new(call_routine(
        zmachine,
        address,
        instruction.next_address,
        arguments,
        instruction.store,
    )?)
}

/// [TOKENISE](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#tokenise): performs lexical
/// analysis of the text at the byte address in operand 0, with the parse buffer at the byte address in
/// operand 1, an optional dictioary at byte address in operand 2, and an optional flag in operand 3
/// that indicates unrecognized words should not be written to the parse buffer.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing an [InstructionResult] or a [RuntimeError]
pub fn tokenise(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
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
    InstructionResult::new(Address(instruction.next_address))
}

/// [ENCODE_TEXT](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#encode_text): encodes
/// the text at the byte address in operand 0 with length in operand 1 and starting index in operand 2, storing
/// the result to the byte address in operand 3.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing an [InstructionResult] or a [RuntimeError]
pub fn encode_text(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
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

    InstructionResult::new(Address(instruction.next_address))
}

/// [COPY_TABLE](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#copy_table): copies operand 2 bytes
/// from the table at the byte address in operand 0 to the byte address in operand 1.
///
/// If operand 1 is 0, then operand 2 bytes of the table at operand 0 are zeroed out.
/// If operand 2 is postive, then the copy is performed forwards or backwards to handle any
/// overlap between the source and destination.
/// If operand 2 is negative, then the copy if performed "forwards", regardless of any overlap.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing an [InstructionResult] or a [RuntimeError]
pub fn copy_table(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
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

    InstructionResult::new(Address(instruction.next_address))
}

/// [PRINT_TABLE](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#print_table): prepares
/// a print table interpreter request.
///
/// Prints from the table of zscii text at the byte address in operand 0, which has
/// width in operand 1.  Height is specified by operand 2, if present, or defaults to 1.
/// Operand 3, if present, specifies the number of characters to skip between lines,.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing an [InstructionResult] with a [RequestType::PrintTable] interpreter request
/// or a [RuntimeError]
pub fn print_table(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;
    let table = operands[0] as usize;
    let width = operands[1] as usize;
    let height = if operands.len() > 2 { operands[2] } else { 1 };
    let skip = if operands.len() > 3 { operands[3] } else { 0 } as usize;

    let mut data = Vec::new();
    for i in 0..height as usize {
        let offset = i * (width + skip);
        for j in 0..(width + skip) {
            data.push(zmachine.read_byte(table + offset + j)? as u16);
        }
    }

    InstructionResult::print_table(
        Address(instruction.next_address),
        data,
        width as u16,
        height,
        skip as u16,
        zmachine.is_stream_enabled(2),
    )
}

/// [CHECK_ARG_COUNT](https://inform-fiction.org/zmachine/standards/z1point1/sect15.html#check_arg_count): branches if the
/// current frame has at least operand 0 arguments.
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `instruction` - Reference to the instruction
///
/// # Returns
/// Result containing an [InstructionResult] or a [RuntimeError]
pub fn check_arg_count(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<InstructionResult, RuntimeError> {
    let operands = operand_values(zmachine, instruction)?;

    InstructionResult::new(branch(
        zmachine,
        instruction,
        zmachine.argument_count()? >= operands[0] as u8,
    )?)
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use crate::{
        assert_ok_eq, assert_print,
        instruction::{processor::dispatch, Opcode, OpcodeForm, OperandCount, OperandType},
        object::property,
        test_util::*,
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x61f);
        assert!(zmachine.peek_variable(0).is_err());
        assert_ok_eq!(zmachine.variable(1), 0x12);
        assert_ok_eq!(zmachine.variable(2), 0x3456);
        assert_ok_eq!(zmachine.variable(3), 0xABCD);
        assert_ok_eq!(zmachine.variable(4), 4);
        assert_ok_eq!(zmachine.variable(5), 5);
        assert_ok_eq!(zmachine.variable(6), 6);
        assert_ok_eq!(zmachine.variable(7), 7);
        assert_ok_eq!(zmachine.variable(8), 8);
        assert_ok_eq!(zmachine.variable(9), 9);
        assert_ok_eq!(zmachine.variable(10), 10);
        assert_ok_eq!(zmachine.variable(11), 11);
        assert_ok_eq!(zmachine.variable(12), 12);
        assert_ok_eq!(zmachine.variable(13), 13);
        assert_ok_eq!(zmachine.variable(14), 14);
        assert_ok_eq!(zmachine.variable(15), 15);
        assert_ok_eq!(zmachine.return_routine(0xF0AD), 0x409);
        assert_ok_eq!(zmachine.variable(0), 0xF0AD);
        assert_ok_eq!(zmachine.variable(0), 1);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x61f);
        assert!(zmachine.peek_variable(0).is_err());
        assert_ok_eq!(zmachine.variable(1), 0x12);
        assert_ok_eq!(zmachine.variable(2), 0x3456);
        assert_ok_eq!(zmachine.variable(3), 0xABCD);
        assert_ok_eq!(zmachine.variable(4), 4);
        assert_ok_eq!(zmachine.variable(5), 5);
        assert_ok_eq!(zmachine.variable(6), 6);
        assert_ok_eq!(zmachine.variable(7), 7);
        assert_ok_eq!(zmachine.variable(8), 8);
        assert_ok_eq!(zmachine.variable(9), 9);
        assert_ok_eq!(zmachine.variable(10), 10);
        assert_ok_eq!(zmachine.variable(11), 11);
        assert_ok_eq!(zmachine.variable(12), 12);
        assert_ok_eq!(zmachine.variable(13), 13);
        assert_ok_eq!(zmachine.variable(14), 14);
        assert_ok_eq!(zmachine.variable(15), 15);
        assert_ok_eq!(zmachine.return_routine(0xF0AD), 0x409);
        assert_ok_eq!(zmachine.variable(0), 0xF0AD);
        assert_ok_eq!(zmachine.variable(0), 1);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x601);
        assert!(zmachine.peek_variable(0).is_err());
        assert_ok_eq!(zmachine.variable(1), 0x12);
        assert_ok_eq!(zmachine.variable(2), 0x3456);
        assert_ok_eq!(zmachine.variable(3), 0xABCD);
        assert_ok_eq!(zmachine.variable(4), 0);
        assert_ok_eq!(zmachine.variable(5), 0);
        assert_ok_eq!(zmachine.variable(6), 0);
        assert_ok_eq!(zmachine.variable(7), 0);
        assert_ok_eq!(zmachine.variable(8), 0);
        assert_ok_eq!(zmachine.variable(9), 0);
        assert_ok_eq!(zmachine.variable(10), 0);
        assert_ok_eq!(zmachine.variable(11), 0);
        assert_ok_eq!(zmachine.variable(12), 0);
        assert_ok_eq!(zmachine.variable(13), 0);
        assert_ok_eq!(zmachine.variable(14), 0);
        assert_ok_eq!(zmachine.variable(15), 0);
        assert_ok_eq!(zmachine.return_routine(0xF0AD), 0x409);
        assert_ok_eq!(zmachine.variable(0), 0xF0AD);
        assert_ok_eq!(zmachine.variable(0), 1);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x601);
        assert!(zmachine.peek_variable(0).is_err());
        assert_ok_eq!(zmachine.variable(1), 0x12);
        assert_ok_eq!(zmachine.variable(2), 0x3456);
        assert_ok_eq!(zmachine.variable(3), 0xABCD);
        assert_ok_eq!(zmachine.variable(4), 0);
        assert_ok_eq!(zmachine.variable(5), 0);
        assert_ok_eq!(zmachine.variable(6), 0);
        assert_ok_eq!(zmachine.variable(7), 0);
        assert_ok_eq!(zmachine.variable(8), 0);
        assert_ok_eq!(zmachine.variable(9), 0);
        assert_ok_eq!(zmachine.variable(10), 0);
        assert_ok_eq!(zmachine.variable(11), 0);
        assert_ok_eq!(zmachine.variable(12), 0);
        assert_ok_eq!(zmachine.variable(13), 0);
        assert_ok_eq!(zmachine.variable(14), 0);
        assert_ok_eq!(zmachine.variable(15), 0);
        assert_ok_eq!(zmachine.return_routine(0xF0AD), 0x409);
        assert_ok_eq!(zmachine.variable(0), 0xF0AD);
        assert_ok_eq!(zmachine.variable(0), 1);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x406);
        assert_ok_eq!(zmachine.read_word(0x388), 0x1234);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x405);
        assert_ok_eq!(zmachine.read_byte(0x384), 0x56);
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
        assert_ok_eq!(property::property(&zmachine, 1, 15), 0x56);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x404);
        assert_ok_eq!(property::property(&zmachine, 1, 15), 0xFE);
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
        assert_ok_eq!(property::property(&zmachine, 1, 20), 0x1234);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x405);
        assert_ok_eq!(property::property(&zmachine, 1, 20), 0xFEDC);
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
        assert_ok_eq!(property::property(&zmachine, 1, 20), 0x1234);
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
        assert_ok_eq!(property::property(&zmachine, 1, 35), 0x56);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x404);
        assert_ok_eq!(property::property(&zmachine, 1, 35), 0xFE);
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
        assert_ok_eq!(property::property(&zmachine, 1, 40), 0x1234);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x405);
        assert_ok_eq!(property::property(&zmachine, 1, 40), 0xFEDC);
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

        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x405);
        // Text buffer
        assert_ok_eq!(zmachine.read_byte(0x381), b'i');
        assert_ok_eq!(zmachine.read_byte(0x382), b'n');
        assert_ok_eq!(zmachine.read_byte(0x383), b'v');
        assert_ok_eq!(zmachine.read_byte(0x384), b'e');
        assert_ok_eq!(zmachine.read_byte(0x385), b'n');
        assert_ok_eq!(zmachine.read_byte(0x386), b't');
        assert_ok_eq!(zmachine.read_byte(0x387), b'o');
        assert_ok_eq!(zmachine.read_byte(0x388), b'r');
        assert_ok_eq!(zmachine.read_byte(0x389), b'y');
        assert_ok_eq!(zmachine.read_byte(0x38a), 0);
        // Parse buffer
        assert_ok_eq!(zmachine.read_byte(0x3A1), 1);
        assert_ok_eq!(zmachine.read_word(0x3A2), 0x310);
        assert_ok_eq!(zmachine.read_byte(0x3A4), 9);
        assert_ok_eq!(zmachine.read_byte(0x3A5), 1);
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

        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x405);
        // Text buffer
        assert_ok_eq!(zmachine.read_byte(0x381), b'i');
        assert_ok_eq!(zmachine.read_byte(0x382), b'n');
        assert_ok_eq!(zmachine.read_byte(0x383), b'v');
        assert_ok_eq!(zmachine.read_byte(0x384), b'e');
        assert_ok_eq!(zmachine.read_byte(0x385), b'n');
        assert_ok_eq!(zmachine.read_byte(0x386), b't');
        assert_ok_eq!(zmachine.read_byte(0x387), b'o');
        assert_ok_eq!(zmachine.read_byte(0x388), b'r');
        assert_ok_eq!(zmachine.read_byte(0x389), b'y');
        assert_ok_eq!(zmachine.read_byte(0x38a), 0);
        // Parse buffer
        assert_ok_eq!(zmachine.read_byte(0x3A1), 1);
        assert_ok_eq!(zmachine.read_word(0x3A2), 0x310);
        assert_ok_eq!(zmachine.read_byte(0x3A4), 9);
        assert_ok_eq!(zmachine.read_byte(0x3A5), 1);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x605);
        // Text buffer
        assert_ok_eq!(zmachine.read_byte(0x381), b'I');
        assert_ok_eq!(zmachine.read_byte(0x382), b'n');
        assert_ok_eq!(zmachine.read_byte(0x383), b'v');
        assert_ok_eq!(zmachine.read_byte(0x384), b'e');
        assert_ok_eq!(zmachine.read_byte(0x385), b'n');
        assert_ok_eq!(zmachine.read_byte(0x386), b't');
        assert_ok_eq!(zmachine.read_byte(0x387), 0);
        // Parse buffer
        assert_ok_eq!(zmachine.read_byte(0x3A1), 0);
    }

    // #[test]
    // fn test_sread_v4_interrupt_continue() {
    //     let mut map = test_map(4);
    //     mock_dictionary(&mut map);
    //     mock_routine(&mut map, 0x600, &[0x1234, 0x5678]);
    //     // Text buffer from previous READ
    //     map[0x381] = b'I';
    //     map[0x382] = b'n';
    //     map[0x383] = b'v';
    //     map[0x384] = b'e';
    //     map[0x385] = b'n';
    //     map[0x386] = b't';
    //     map[0x387] = 0;

    //     let mut zmachine = mock_zmachine(map);
    //     zmachine.set_read_interrupt_pending();
    //     assert!(zmachine.call_read_interrupt(0x600, 0x400).is_ok());
    //     assert!(zmachine.return_routine(0).is_ok());

    //     // Read with a 3 second timeout
    //     let i = mock_instruction(
    //         0x400,
    //         vec![
    //             operand(OperandType::LargeConstant, 0x380),
    //             operand(OperandType::LargeConstant, 0x3A0),
    //             operand(OperandType::LargeConstant, 30),
    //             operand(OperandType::LargeConstant, 0x180),
    //         ],
    //         opcode(4, 4),
    //         0x405,
    //     );

    //     input(&['o', 'r', 'y']);

    //     // Input was interrupted
    //     assert_ok_eq!(dispatch(&mut zmachine, &i), 0x405);
    //     // Text buffer
    //     assert_ok_eq!(zmachine.read_byte(0x381), b'i');
    //     assert_ok_eq!(zmachine.read_byte(0x382), b'n');
    //     assert_ok_eq!(zmachine.read_byte(0x383), b'v');
    //     assert_ok_eq!(zmachine.read_byte(0x384), b'e');
    //     assert_ok_eq!(zmachine.read_byte(0x385), b'n');
    //     assert_ok_eq!(zmachine.read_byte(0x386), b't');
    //     assert_ok_eq!(zmachine.read_byte(0x387), b'o');
    //     assert_ok_eq!(zmachine.read_byte(0x388), b'r');
    //     assert_ok_eq!(zmachine.read_byte(0x389), b'y');
    //     assert_ok_eq!(zmachine.read_byte(0x38a), 0);
    //     // Parse buffer
    //     assert_ok_eq!(zmachine.read_byte(0x3A1), 1);
    //     assert_ok_eq!(zmachine.read_word(0x3A2), 0x310);
    //     assert_ok_eq!(zmachine.read_byte(0x3A4), 9);
    //     assert_ok_eq!(zmachine.read_byte(0x3A5), 1);
    // }

    // #[test]
    // fn test_sread_v4_interrupt_stop() {
    //     let mut map = test_map(4);
    //     mock_dictionary(&mut map);
    //     mock_routine(&mut map, 0x600, &[0x1234, 0x5678]);
    //     // Text buffer from previous READ
    //     map[0x381] = b'I';
    //     map[0x382] = b'n';
    //     map[0x383] = b'v';
    //     map[0x384] = b'e';
    //     map[0x385] = b'n';
    //     map[0x386] = b't';
    //     map[0x387] = 0;

    //     let mut zmachine = mock_zmachine(map);
    //     zmachine.set_read_interrupt_pending();
    //     assert!(zmachine.call_read_interrupt(0x600, 0x400).is_ok());
    //     assert!(zmachine.return_routine(1).is_ok());

    //     // Read with a 3 second timeout
    //     let i = mock_instruction(
    //         0x400,
    //         vec![
    //             operand(OperandType::LargeConstant, 0x380),
    //             operand(OperandType::LargeConstant, 0x3A0),
    //             operand(OperandType::LargeConstant, 30),
    //             operand(OperandType::LargeConstant, 0x180),
    //         ],
    //         opcode(4, 4),
    //         0x405,
    //     );

    //     // Input was interrupted
    //     assert_ok_eq!(dispatch(&mut zmachine, &i), 0x405);
    //     // Text buffer
    //     assert_ok_eq!(zmachine.read_byte(0x381), 0);
    //     assert_ok_eq!(zmachine.read_byte(0x382), 0);
    //     assert_ok_eq!(zmachine.read_byte(0x383), 0);
    //     assert_ok_eq!(zmachine.read_byte(0x384), 0);
    //     assert_ok_eq!(zmachine.read_byte(0x385), 0);
    //     assert_ok_eq!(zmachine.read_byte(0x386), 0);
    //     assert_ok_eq!(zmachine.read_byte(0x387), 0);
    //     // Parse buffer
    //     assert_ok_eq!(zmachine.read_byte(0x3A1), 0);
    // }

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

        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x406);
        assert_ok_eq!(zmachine.variable(0x80), b'\r' as u16);
        // Text buffer
        assert_ok_eq!(zmachine.read_byte(0x381), 9);
        assert_ok_eq!(zmachine.read_byte(0x382), b'i');
        assert_ok_eq!(zmachine.read_byte(0x383), b'n');
        assert_ok_eq!(zmachine.read_byte(0x384), b'v');
        assert_ok_eq!(zmachine.read_byte(0x385), b'e');
        assert_ok_eq!(zmachine.read_byte(0x386), b'n');
        assert_ok_eq!(zmachine.read_byte(0x387), b't');
        assert_ok_eq!(zmachine.read_byte(0x388), b'o');
        assert_ok_eq!(zmachine.read_byte(0x389), b'r');
        assert_ok_eq!(zmachine.read_byte(0x38a), b'y');
        // Parse buffer
        assert_ok_eq!(zmachine.read_byte(0x3A1), 1);
        assert_ok_eq!(zmachine.read_word(0x3A2), 0x310);
        assert_ok_eq!(zmachine.read_byte(0x3A4), 9);
        assert_ok_eq!(zmachine.read_byte(0x3A5), 2);
    }

    #[test]
    fn test_aread_v5_terminator_table() {
        let mut map = test_map(5);
        // Set up a terminating character table at 0x200
        map[0x2E] = 0x2;
        map[0x200] = b'n'; // This one is invalid
        map[0x201] = 0xFE;
        map[0x202] = 0;

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

        // Terminate input with 0xFE
        input(&['I', 'n', 'v', 'e', 'n', 't', 'o', 'r', 'y', 0xFE as char]);

        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x406);
        assert_ok_eq!(zmachine.variable(0x80), 0xFE);
        // Text buffer
        assert_ok_eq!(zmachine.read_byte(0x381), 9);
        assert_ok_eq!(zmachine.read_byte(0x382), b'i');
        assert_ok_eq!(zmachine.read_byte(0x383), b'n');
        assert_ok_eq!(zmachine.read_byte(0x384), b'v');
        assert_ok_eq!(zmachine.read_byte(0x385), b'e');
        assert_ok_eq!(zmachine.read_byte(0x386), b'n');
        assert_ok_eq!(zmachine.read_byte(0x387), b't');
        assert_ok_eq!(zmachine.read_byte(0x388), b'o');
        assert_ok_eq!(zmachine.read_byte(0x389), b'r');
        assert_ok_eq!(zmachine.read_byte(0x38a), b'y');
        // Parse buffer
        assert_ok_eq!(zmachine.read_byte(0x3A1), 1);
        assert_ok_eq!(zmachine.read_word(0x3A2), 0x310);
        assert_ok_eq!(zmachine.read_byte(0x3A4), 9);
        assert_ok_eq!(zmachine.read_byte(0x3A5), 2);
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

        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x406);
        assert_ok_eq!(zmachine.variable(0x80), b'\r' as u16);
        // Text buffer
        assert_ok_eq!(zmachine.read_byte(0x381), 9);
        assert_ok_eq!(zmachine.read_byte(0x382), b'i');
        assert_ok_eq!(zmachine.read_byte(0x383), b'n');
        assert_ok_eq!(zmachine.read_byte(0x384), b'v');
        assert_ok_eq!(zmachine.read_byte(0x385), b'e');
        assert_ok_eq!(zmachine.read_byte(0x386), b'n');
        assert_ok_eq!(zmachine.read_byte(0x387), b't');
        assert_ok_eq!(zmachine.read_byte(0x388), b'o');
        assert_ok_eq!(zmachine.read_byte(0x389), b'r');
        assert_ok_eq!(zmachine.read_byte(0x38a), b'y');
        // Parse buffer
        assert_ok_eq!(zmachine.read_byte(0x3A1), 0);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x601);
        assert_ok_eq!(zmachine.variable(0x80), 0);
        // Text buffer
        assert_ok_eq!(zmachine.read_byte(0x381), 6);
        assert_ok_eq!(zmachine.read_byte(0x382), b'I');
        assert_ok_eq!(zmachine.read_byte(0x383), b'n');
        assert_ok_eq!(zmachine.read_byte(0x384), b'v');
        assert_ok_eq!(zmachine.read_byte(0x385), b'e');
        assert_ok_eq!(zmachine.read_byte(0x386), b'n');
        assert_ok_eq!(zmachine.read_byte(0x387), b't');
        // Parse buffer
        assert_ok_eq!(zmachine.read_byte(0x3A1), 0);
    }

    // #[test]
    // fn test_aread_v5_interrupt_continue() {
    //     let mut map = test_map(5);
    //     mock_dictionary(&mut map);
    //     mock_routine(&mut map, 0x600, &[0x1234, 0x5678]);
    //     // Text buffer from previous READ
    //     map[0x381] = 6;
    //     map[0x382] = b'i';
    //     map[0x383] = b'n';
    //     map[0x384] = b'v';
    //     map[0x385] = b'e';
    //     map[0x386] = b'n';
    //     map[0x387] = b't';

    //     let mut zmachine = mock_zmachine(map);
    //     zmachine.set_read_interrupt_pending();
    //     assert!(zmachine.call_read_interrupt(0x600, 0x400).is_ok());
    //     assert!(zmachine.return_routine(0).is_ok());

    //     // Read with a 3 second timeout
    //     let i = mock_store_instruction(
    //         0x400,
    //         vec![
    //             operand(OperandType::LargeConstant, 0x380),
    //             operand(OperandType::LargeConstant, 0x3A0),
    //             operand(OperandType::LargeConstant, 30),
    //             operand(OperandType::LargeConstant, 0x180),
    //         ],
    //         opcode(5, 4),
    //         0x409,
    //         store(0x408, 0x80),
    //     );

    //     input(&['o', 'r', 'y']);

    //     // Input was interrupted
    //     assert_ok_eq!(dispatch(&mut zmachine, &i), 0x409);
    //     assert_ok_eq!(zmachine.variable(0x80), b'\r' as u16);
    //     // Text buffer
    //     assert_ok_eq!(zmachine.read_byte(0x381), 9);
    //     assert_ok_eq!(zmachine.read_byte(0x382), b'i');
    //     assert_ok_eq!(zmachine.read_byte(0x383), b'n');
    //     assert_ok_eq!(zmachine.read_byte(0x384), b'v');
    //     assert_ok_eq!(zmachine.read_byte(0x385), b'e');
    //     assert_ok_eq!(zmachine.read_byte(0x386), b'n');
    //     assert_ok_eq!(zmachine.read_byte(0x387), b't');
    //     assert_ok_eq!(zmachine.read_byte(0x388), b'o');
    //     assert_ok_eq!(zmachine.read_byte(0x389), b'r');
    //     assert_ok_eq!(zmachine.read_byte(0x38a), b'y');
    //     // Parse buffer
    //     assert_ok_eq!(zmachine.read_byte(0x3A1), 1);
    //     assert_ok_eq!(zmachine.read_word(0x3A2), 0x310);
    //     assert_ok_eq!(zmachine.read_byte(0x3A4), 9);
    //     assert_ok_eq!(zmachine.read_byte(0x3A5), 2);
    // }

    // #[test]
    // fn test_aread_v5_interrupt_stop() {
    //     let mut map = test_map(5);
    //     set_variable(&mut map, 0x80, 0xFF);
    //     mock_dictionary(&mut map);
    //     mock_routine(&mut map, 0x600, &[0x1234, 0x5678]);
    //     // Text buffer from previous READ
    //     map[0x381] = 6;
    //     map[0x382] = b'i';
    //     map[0x383] = b'n';
    //     map[0x384] = b'v';
    //     map[0x385] = b'e';
    //     map[0x386] = b'n';
    //     map[0x387] = b't';

    //     let mut zmachine = mock_zmachine(map);
    //     zmachine.set_read_interrupt_pending();
    //     assert!(zmachine.call_read_interrupt(0x600, 0x400).is_ok());
    //     assert!(zmachine.return_routine(1).is_ok());

    //     // Read with a 3 second timeout
    //     let i = mock_store_instruction(
    //         0x400,
    //         vec![
    //             operand(OperandType::LargeConstant, 0x380),
    //             operand(OperandType::LargeConstant, 0x3A0),
    //             operand(OperandType::LargeConstant, 30),
    //             operand(OperandType::LargeConstant, 0x180),
    //         ],
    //         opcode(5, 4),
    //         0x409,
    //         store(0x408, 0x80),
    //     );

    //     // Input was interrupted
    //     assert_ok_eq!(dispatch(&mut zmachine, &i), 0x409);
    //     assert_ok_eq!(zmachine.variable(0x80), 0);
    //     // Text buffer
    //     assert_ok_eq!(zmachine.read_byte(0x381), 0);
    //     // Parse buffer
    //     assert_ok_eq!(zmachine.read_byte(0x3A1), 0);
    // }

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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x403);
        assert_print!("@");
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x403);
        assert_print!("32767");
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x403);
        assert_print!("-32768");
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x404);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x404);
        assert_ok_eq!(zmachine.variable(0x80), 0);

        let i = mock_store_instruction(
            0x400,
            vec![operand(OperandType::LargeConstant, 0x7FFF)],
            opcode(3, 7),
            0x404,
            store(0x403, 0x080),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x404);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x404);
        assert_ok_eq!(zmachine.variable(0x80), 0);
        let i = mock_store_instruction(
            0x400,
            vec![operand(OperandType::LargeConstant, 7)],
            opcode(3, 7),
            0x404,
            store(0x403, 0x080),
        );
        // 1, 2, 3, 4, 5, 6, 7, 8, 1
        // But the range is 7, so that becomes
        // 1, 2, 3, 4, 5, 6, 7, 8 % 7, 1
        for r in 1..8 {
            assert_ok_eq!(dispatch(&mut zmachine, &i), 0x404);
            assert_ok_eq!(zmachine.variable(0x80), r % 8);
        }
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x404);
        assert_ok_eq!(zmachine.variable(0x80), 8 % 7);
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x404);
        assert_ok_eq!(zmachine.variable(0x80), 1);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x404);
        assert_ok_eq!(zmachine.variable(0x80), 0);
        let i = mock_store_instruction(
            0x400,
            vec![operand(OperandType::LargeConstant, 32767)],
            opcode(3, 7),
            0x404,
            store(0x403, 0x080),
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x404);
        assert_ok_eq!(zmachine.variable(0x80), 0x4DD5);
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x404);
        assert_ok_eq!(zmachine.variable(0x80), 0x0AD5);
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x404);
        assert_ok_eq!(zmachine.variable(0x80), 0x3D5E);
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x404);
        assert_ok_eq!(zmachine.variable(0x80), 0x0F57);
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x404);
        assert_ok_eq!(zmachine.variable(0x80), 0x12E1);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x403);
        assert_ok_eq!(zmachine.peek_variable(0), 0x1234);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x403);
        assert_ok_eq!(zmachine.variable(0x80), 0x5678);
        assert_ok_eq!(zmachine.peek_variable(0), 0x1234);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x403);
        assert_ok_eq!(zmachine.variable(0), 0x5678);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x402);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x402);
        let i = mock_instruction(
            0x400,
            vec![operand(OperandType::SmallConstant, 0x1)],
            opcode(3, 11),
            0x402,
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x402);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x615);
        assert!(zmachine.peek_variable(0).is_err());
        assert_ok_eq!(zmachine.variable(1), 0x1111);
        assert_ok_eq!(zmachine.variable(2), 0x22);
        assert_ok_eq!(zmachine.variable(3), 0x3333);
        assert_ok_eq!(zmachine.variable(4), 0x44);
        assert_ok_eq!(zmachine.variable(5), 0x5555);
        assert_ok_eq!(zmachine.variable(6), 0x66);
        assert_ok_eq!(zmachine.variable(7), 0x7777);
        assert_ok_eq!(zmachine.variable(8), 0x8);
        assert_ok_eq!(zmachine.variable(9), 0x9);
        assert_ok_eq!(zmachine.variable(10), 0xA);
        assert!(zmachine.variable(11).is_err());
        assert_ok_eq!(zmachine.return_routine(0x5678), 0x411);
        assert_ok_eq!(zmachine.variable(0x80), 0x5678);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x601);
        assert!(zmachine.peek_variable(0).is_err());
        assert_ok_eq!(zmachine.variable(1), 0x1111);
        assert_ok_eq!(zmachine.variable(2), 0x22);
        assert_ok_eq!(zmachine.variable(3), 0x3333);
        assert_ok_eq!(zmachine.variable(4), 0x44);
        assert_ok_eq!(zmachine.variable(5), 0x5555);
        assert_ok_eq!(zmachine.variable(6), 0x66);
        assert_ok_eq!(zmachine.variable(7), 0x7777);
        assert_ok_eq!(zmachine.variable(8), 0);
        assert_ok_eq!(zmachine.variable(9), 0);
        assert_ok_eq!(zmachine.variable(10), 0);
        assert!(zmachine.variable(11).is_err());
        assert_ok_eq!(zmachine.return_routine(0x5678), 0x411);
        assert_ok_eq!(zmachine.variable(0x80), 0x5678);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x601);
        assert!(zmachine.peek_variable(0).is_err());
        assert_ok_eq!(zmachine.variable(1), 0x1111);
        assert_ok_eq!(zmachine.variable(2), 0x22);
        assert_ok_eq!(zmachine.variable(3), 0x3333);
        assert_ok_eq!(zmachine.variable(4), 0x44);
        assert_ok_eq!(zmachine.variable(5), 0x5555);
        assert_ok_eq!(zmachine.variable(6), 0x66);
        assert_ok_eq!(zmachine.variable(7), 0x7777);
        assert_ok_eq!(zmachine.variable(8), 0);
        assert_ok_eq!(zmachine.variable(9), 0);
        assert_ok_eq!(zmachine.variable(10), 0);
        assert!(zmachine.variable(11).is_err());
        assert_ok_eq!(zmachine.return_routine(0x5678), 0x411);
        assert_ok_eq!(zmachine.variable(0x80), 0x5678);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x402);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x402);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x402);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x402);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x402);
        assert_eq!(split(), 0);
        assert!(erase_line());
    }

    // #[test]
    // fn test_set_cursor() {
    //     let map = test_map(4);
    //     let mut zmachine = mock_zmachine(map);
    //     let mut c = (0, 0);
    //     assert!(zmachine.cursor().is_ok_and(|x| {
    //         c = x;
    //         true
    //     }));

    //     let i = mock_instruction(
    //         0x400,
    //         vec![
    //             operand(OperandType::SmallConstant, c.0 - 1),
    //             operand(OperandType::SmallConstant, c.1 + 1),
    //         ],
    //         opcode(4, 15),
    //         0x403,
    //     );
    //     assert_ok_eq!(dispatch(&mut zmachine, &i), 0x403);
    //     assert!(zmachine
    //         .cursor()
    //         .is_ok_and(|x| x.0 == c.0 - 1 && x.1 == c.1 + 1));
    // }

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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x403);
        assert_ok_eq!(zmachine.read_word(0x300), 24);
        assert_ok_eq!(zmachine.read_word(0x302), 1);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x402);
        assert_eq!(split(), 0);
        assert_eq!(style(), 1);
    }

    // #[test]
    // fn test_set_text_additive() {
    //     let map = test_map(4);
    //     let mut zmachine = mock_zmachine(map);
    //     assert!(zmachine.set_text_style(2).is_ok());
    //     let i = mock_instruction(
    //         0x400,
    //         vec![operand(OperandType::SmallConstant, 1)],
    //         opcode(4, 17),
    //         0x402,
    //     );
    //     assert_ok_eq!(dispatch(&mut zmachine, &i), 0x402);
    //     assert_eq!(split(), 0);
    //     assert_eq!(style(), 3);
    // }

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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x402);
        assert_eq!(split(), 0);
        assert_eq!(style(), 3);
    }

    // #[test]
    // fn test_set_text_style_roman() {
    //     let map = test_map(4);
    //     let mut zmachine = mock_zmachine(map);
    //     assert!(zmachine.set_text_style(0xF).is_ok());
    //     let i = mock_instruction(
    //         0x400,
    //         vec![operand(OperandType::SmallConstant, 0)],
    //         opcode(4, 17),
    //         0x402,
    //     );
    //     assert_ok_eq!(dispatch(&mut zmachine, &i), 0x402);
    //     assert_eq!(split(), 0);
    //     assert_eq!(style(), 0);
    // }

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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x402);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x402);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x402);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x403);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x402);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x402);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x402);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x405);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x405);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x405);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x405);
        let i = mock_instruction(
            0x400,
            vec![
                operand(OperandType::SmallConstant, 3),
                operand(OperandType::SmallConstant, 3),
            ],
            opcode(4, 21),
            0x405,
        );
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x405);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x405);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x405);
        assert_eq!(play_sound(), (256, 0x20, 5));
    }

    // #[test]
    // fn test_sound_effect_v5_with_interrupt() {
    //     let map = test_map(5);
    //     let mut zmachine = mock_zmachine(map);
    //     let i = mock_instruction(
    //         0x400,
    //         vec![
    //             operand(OperandType::SmallConstant, 4),
    //             operand(OperandType::SmallConstant, 2),
    //             operand(OperandType::LargeConstant, 0x1020),
    //             operand(OperandType::LargeConstant, 0x180),
    //         ],
    //         opcode(5, 21),
    //         0x405,
    //     );
    //     assert_ok_eq!(dispatch(&mut zmachine, &i), 0x405);
    //     assert_some_eq!(zmachine.sound_interrupt(), 0x600);
    //     assert_eq!(play_sound(), (256, 0x20, 16));
    // }

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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x403);
        assert_ok_eq!(zmachine.variable(0x80), 0x20);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x601);
        assert_ok_eq!(zmachine.variable(0x80), 0);
    }

    // #[test]
    // fn test_read_char_timeout_continue() {
    //     let mut map = test_map(4);
    //     mock_routine(&mut map, 0x600, &[]);
    //     let mut zmachine = mock_zmachine(map);
    //     zmachine.set_read_interrupt_pending();
    //     assert!(zmachine.call_read_interrupt(0x600, 0x400).is_ok());
    //     assert!(zmachine.return_routine(0).is_ok());

    //     input(&[' ']);

    //     let i = mock_store_instruction(
    //         0x400,
    //         vec![
    //             operand(OperandType::SmallConstant, 1),
    //             operand(OperandType::LargeConstant, 1),
    //             operand(OperandType::LargeConstant, 0x180),
    //         ],
    //         opcode(4, 22),
    //         0x406,
    //         store(0x402, 0x80),
    //     );
    //     assert_ok_eq!(dispatch(&mut zmachine, &i), 0x406);
    //     assert_ok_eq!(zmachine.variable(0x80), 0x20);
    // }

    // #[test]
    // fn test_read_char_timeout_stop() {
    //     let mut map = test_map(4);
    //     mock_routine(&mut map, 0x600, &[]);
    //     let mut zmachine = mock_zmachine(map);
    //     zmachine.set_read_interrupt_pending();
    //     assert!(zmachine.call_read_interrupt(0x600, 0x400).is_ok());
    //     assert!(zmachine.return_routine(1).is_ok());

    //     input(&[' ']);

    //     let i = mock_store_instruction(
    //         0x400,
    //         vec![
    //             operand(OperandType::SmallConstant, 1),
    //             operand(OperandType::LargeConstant, 1),
    //             operand(OperandType::LargeConstant, 0x180),
    //         ],
    //         opcode(4, 22),
    //         0x406,
    //         store(0x402, 0x80),
    //     );
    //     assert_ok_eq!(dispatch(&mut zmachine, &i), 0x406);
    //     assert_ok_eq!(zmachine.variable(0x80), 0);
    // }

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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x40a);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x408);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x40a);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x408);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x40a);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x408);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x404);
        assert_ok_eq!(zmachine.variable(0x80), 0xEDCB);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x601);
        assert!(zmachine.peek_variable(0).is_err());
        assert_ok_eq!(zmachine.variable(1), 0x12);
        assert_ok_eq!(zmachine.variable(2), 0x3456);
        assert_ok_eq!(zmachine.variable(3), 0xABCD);
        assert_ok_eq!(zmachine.variable(4), 0);
        assert!(zmachine.variable(5).is_err());
        assert_ok_eq!(zmachine.return_routine(0xF0AD), 0x409);
        assert_ok_eq!(zmachine.variable(0), 1);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x601);
        assert!(zmachine.peek_variable(0).is_err());
        assert_ok_eq!(zmachine.variable(1), 0x12);
        assert_ok_eq!(zmachine.variable(2), 0x3456);
        assert_ok_eq!(zmachine.variable(3), 0xABCD);
        assert_ok_eq!(zmachine.variable(4), 0);
        assert!(zmachine.variable(5).is_err());
        assert_ok_eq!(zmachine.return_routine(0xF0AD), 0x409);
        assert_ok_eq!(zmachine.variable(0), 1);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x601);
        assert!(zmachine.peek_variable(0).is_err());
        assert_ok_eq!(zmachine.variable(1), 0x1111);
        assert_ok_eq!(zmachine.variable(2), 0x22);
        assert_ok_eq!(zmachine.variable(3), 0x3333);
        assert_ok_eq!(zmachine.variable(4), 0x44);
        assert_ok_eq!(zmachine.variable(5), 0x5555);
        assert_ok_eq!(zmachine.variable(6), 0x66);
        assert_ok_eq!(zmachine.variable(7), 0x7777);
        assert_ok_eq!(zmachine.variable(8), 0);
        assert_ok_eq!(zmachine.variable(9), 0);
        assert_ok_eq!(zmachine.variable(10), 0);
        assert!(zmachine.variable(11).is_err());
        assert_ok_eq!(zmachine.return_routine(0x5678), 0x411);
        assert_ok_eq!(zmachine.variable(0x80), 0);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x601);
        assert!(zmachine.peek_variable(0).is_err());
        assert_ok_eq!(zmachine.variable(1), 0x1111);
        assert_ok_eq!(zmachine.variable(2), 0x22);
        assert_ok_eq!(zmachine.variable(3), 0x3333);
        assert_ok_eq!(zmachine.variable(4), 0x44);
        assert_ok_eq!(zmachine.variable(5), 0x5555);
        assert_ok_eq!(zmachine.variable(6), 0x66);
        assert_ok_eq!(zmachine.variable(7), 0x7777);
        assert_ok_eq!(zmachine.variable(8), 0);
        assert_ok_eq!(zmachine.variable(9), 0);
        assert_ok_eq!(zmachine.variable(10), 0);
        assert!(zmachine.variable(11).is_err());
        assert_ok_eq!(zmachine.return_routine(0x5678), 0x411);
        assert_ok_eq!(zmachine.variable(0x80), 0);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x405);
        assert_ok_eq!(zmachine.read_byte(0x3A1), 2);
        assert_ok_eq!(zmachine.read_word(0x3A2), 0x322);
        assert_ok_eq!(zmachine.read_byte(0x3A4), 6);
        assert_ok_eq!(zmachine.read_byte(0x3A5), 2);
        assert_ok_eq!(zmachine.read_word(0x3A6), 0);
        assert_ok_eq!(zmachine.read_byte(0x3A8), 4);
        assert_ok_eq!(zmachine.read_byte(0x3A9), 9);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x408);
        assert_ok_eq!(zmachine.read_byte(0x3A1), 2);
        assert_ok_eq!(zmachine.read_word(0x3A2), 0);
        assert_ok_eq!(zmachine.read_byte(0x3A4), 6);
        assert_ok_eq!(zmachine.read_byte(0x3A5), 2);
        assert_ok_eq!(zmachine.read_word(0x3A6), 0x359);
        assert_ok_eq!(zmachine.read_byte(0x3A8), 4);
        assert_ok_eq!(zmachine.read_byte(0x3A9), 9);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x408);
        assert_ok_eq!(zmachine.read_byte(0x3A1), 2);
        assert_ok_eq!(zmachine.read_word(0x3A2), 0x322);
        assert_ok_eq!(zmachine.read_byte(0x3A4), 6);
        assert_ok_eq!(zmachine.read_byte(0x3A5), 2);
        assert_ok_eq!(zmachine.read_word(0x3A6), 0x359);
        assert_ok_eq!(zmachine.read_byte(0x3A8), 4);
        assert_ok_eq!(zmachine.read_byte(0x3A9), 9);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x407);
        assert_ok_eq!(zmachine.read_word(0x320), 0x4A94);
        assert_ok_eq!(zmachine.read_word(0x322), 0x4CA5);
        assert_ok_eq!(zmachine.read_word(0x324), 0x94A5);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x406);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x406);
        for i in 0..0x20 {
            assert_ok_eq!(zmachine.read_byte(0x300 + i), 0);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x406);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x406);
        for i in 0..0x10 {
            assert!(zmachine
                .read_byte(0x310 + i)
                .is_ok_and(|x| x == i as u8 + 1));
            assert!(zmachine
                .read_byte(0x320 + i)
                .is_ok_and(|x| x == i as u8 + 1));
        }
    }

    // #[test]
    // fn test_print_table() {
    //     let mut map = test_map(5);
    //     for i in 0..8 {
    //         for j in 0..8 {
    //             map[0x300 + (i * 8) + j] = b'a' + j as u8;
    //         }
    //     }

    //     let mut zmachine = mock_zmachine(map);
    //     assert!(zmachine.set_cursor(5, 8).is_ok());
    //     let i = mock_instruction(
    //         0x400,
    //         vec![
    //             operand(OperandType::LargeConstant, 0x300),
    //             operand(OperandType::SmallConstant, 8),
    //             operand(OperandType::SmallConstant, 8),
    //         ],
    //         opcode(5, 30),
    //         0x405,
    //     );
    //     assert_ok_eq!(dispatch(&mut zmachine, &i), 0x405);
    //     assert_print!("abcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefgh");
    //     assert_ok_eq!(zmachine.cursor(), (12, 16));
    // }

    // #[test]
    // fn test_print_table_skip() {
    //     let mut map = test_map(5);
    //     for i in 0..8 {
    //         for j in 0..8 {
    //             map[0x300 + (i * 8) + j] = b'a' + j as u8;
    //         }
    //     }

    //     let mut zmachine = mock_zmachine(map);
    //     assert!(zmachine.set_cursor(5, 8).is_ok());
    //     let i = mock_instruction(
    //         0x400,
    //         vec![
    //             operand(OperandType::LargeConstant, 0x300),
    //             operand(OperandType::SmallConstant, 4),
    //             operand(OperandType::SmallConstant, 8),
    //             operand(OperandType::SmallConstant, 4),
    //         ],
    //         opcode(5, 30),
    //         0x406,
    //     );
    //     assert_ok_eq!(dispatch(&mut zmachine, &i), 0x406);
    //     assert_print!("abcdabcdabcdabcdabcdabcdabcdabcd");
    //     assert_ok_eq!(zmachine.cursor(), (12, 12));
    // }

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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x40a);
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
        assert_ok_eq!(dispatch(&mut zmachine, &i), 0x403);
    }
}
