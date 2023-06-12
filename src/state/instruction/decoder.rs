use crate::error::*;
use crate::state::instruction::*;
use crate::state::memory;
use crate::state::memory::*;
use crate::state::header;
use crate::state::header::*;

fn opcode_name(opcode: u8, ext_opcode: Option<u8>, version: u8) -> String {
    match opcode {
        0x01 | 0x21 | 0x41 | 0x61 | 0xc1 => "je",
        0x02 | 0x22 | 0x42 | 0x62 | 0xc2 => "jl",
        0x03 | 0x23 | 0x43 | 0x63 | 0xc3 => "jg",
        0x04 | 0x24 | 0x44 | 0x64 | 0xc4 => "dec_chk",
        0x05 | 0x25 | 0x45 | 0x65 | 0xc5 => "inc_chk",
        0x06 | 0x26 | 0x46 | 0x66 | 0xc6 => "jin",
        0x07 | 0x27 | 0x47 | 0x67 | 0xc7 => "test",
        0x08 | 0x28 | 0x48 | 0x68 | 0xc8 => "or",
        0x09 | 0x29 | 0x49 | 0x69 | 0xc9 => "and",
        0x0a | 0x2a | 0x4a | 0x6a | 0xca => "test_attr",
        0x0b | 0x2b | 0x4b | 0x6b | 0xcb => "set_attr",
        0x0c | 0x2c | 0x4c | 0x6c | 0xcc => "clear_attr",
        0x0d | 0x2d | 0x4d | 0x6d | 0xcd => "store",
        0x0e | 0x2e | 0x4e | 0x6e | 0xce => "insert_obj",
        0x0f | 0x2f | 0x4f | 0x6f | 0xcf => "loadw",
        0x10 | 0x30 | 0x50 | 0x70 | 0xd0 => "loadb",
        0x11 | 0x31 | 0x51 | 0x71 | 0xd1 => "get_prop",
        0x12 | 0x32 | 0x52 | 0x72 | 0xd2 => "get_prop_addr",
        0x13 | 0x33 | 0x53 | 0x73 | 0xd3 => "get_next_prop",
        0x14 | 0x34 | 0x54 | 0x74 | 0xd4 => "add",
        0x15 | 0x35 | 0x55 | 0x75 | 0xd5 => "sub",
        0x16 | 0x36 | 0x56 | 0x76 | 0xd6 => "mul",
        0x17 | 0x37 | 0x57 | 0x77 | 0xd7 => "div",
        0x18 | 0x38 | 0x58 | 0x78 | 0xd8 => "mod",
        0x19 | 0x39 | 0x59 | 0x79 | 0xd9 => "call_2s",
        0x1a | 0x3a | 0x5a | 0x7a | 0xda => "call_2n",
        0x1b | 0x3b | 0x5b | 0x7b | 0xdb => "set_colour",
        0x1c | 0x3c | 0x5c | 0x7c | 0xdc => "throw",
        0x80 | 0x90 | 0xa0 => "jz",
        0x81 | 0x91 | 0xa1 => "get_sibling",
        0x82 | 0x92 | 0xa2 => "get_child",
        0x83 | 0x93 | 0xa3 => "get_parent",
        0x84 | 0x94 | 0xa4 => "get_prop_len",
        0x85 | 0x95 | 0xa5 => "inc",
        0x86 | 0x96 | 0xa6 => "dec",
        0x87 | 0x97 | 0xa7 => "print_addr",
        0x88 | 0x98 | 0xa8 => "call_1s",
        0x89 | 0x99 | 0xa9 => "remove_obj",
        0x8a | 0x9a | 0xaa => "print_obj",
        0x8b | 0x9b | 0xab => "ret",
        0x8c | 0x9c | 0xac => "jump",
        0x8d | 0x9d | 0xad => "print_paddr",
        0x8e | 0x9e | 0xae => "load",
        0x8f | 0x9f | 0xaf => {
            if version < 5 {
                return "not".to_string();
            } else {
                return "call_1n".to_string();
            }
        }
        0xb0 => "rtrue",
        0xb1 => "rfalse",
        0xb2 => "print",
        0xb3 => "print_ret",
        0xb4 => "nop",
        0xb5 => "save",
        0xb6 => "restore",
        0xb7 => "restart",
        0xb8 => "ret_popped",
        0xb9 => {
            if version < 5 {
                return "pop".to_string();
            } else {
                return "catch".to_string();
            }
        }
        0xba => "quit",
        0xbb => "new_line",
        0xbc => "show_status",
        0xbd => "verify",
        0xbe => match ext_opcode {
            Some(0x00) => "save",
            Some(0x01) => "restore",
            Some(0x02) => "log_shift",
            Some(0x03) => "art_shift",
            Some(0x04) => "set_font",
            Some(0x05) => "draw_picture",
            Some(0x06) => "picture_data",
            Some(0x07) => "erase_picture",
            Some(0x08) => "set_margins",
            Some(0x09) => "save_undo",
            Some(0x0a) => "restore_undo",
            Some(0x0b) => "print_unicode",
            Some(0x0c) => "check_unicode",
            Some(0x0d) => "set_true_colour",
            Some(0x10) => "move_window",
            Some(0x11) => "window_size",
            Some(0x12) => "window_style",
            Some(0x13) => "get_wind_prop",
            Some(0x14) => "scroll_window",
            Some(0x15) => "pop_stack",
            Some(0x16) => "read_mouse",
            Some(0x17) => "mouse_window",
            Some(0x18) => "push_stack",
            Some(0x19) => "put_wind_prop",
            Some(0x1a) => "print_form",
            Some(0x1b) => "make_menu",
            Some(0x1c) => "picture_table",
            Some(0x1d) => "buffer_screen",
            _ => "unknown",
        },
        0xbf => "piracy",
        0xe0 => {
            if version < 4 {
                return "call_routine".to_string();
            } else {
                return "call_vs".to_string();
            }
        }
        0xe1 => "storew",
        0xe2 => "storeb",
        0xe3 => "put_prop",
        0xe4 => {
            if version < 5 {
                return "sread".to_string();
            } else {
                return "aread".to_string();
            }
        }
        0xe5 => "print_char",
        0xe6 => "print_num",
        0xe7 => "random",
        0xe8 => "push",
        0xe9 => {
            if version != 6 {
                return "pull".to_string();
            } else {
                return "pull_stack".to_string();
            }
        }
        0xea => "split_window",
        0xeb => "set_window",
        0xec => "call_v2s",
        0xed => "erase_window",
        0xee => "erase_line",
        0xef => "set_cursor",
        0xf0 => "get_cursor",
        0xf1 => "set_text_style",
        0xf2 => "buffer_mode",
        0xf3 => "output_stream",
        0xf4 => "input_stream",
        0xf5 => "sound_effect",
        0xf6 => "read_char",
        0xf7 => "scan_table",
        0xf8 => "not",
        0xf9 => "call_vn",
        0xfa => "call_vn2",
        0xfb => "tokenise",
        0xfc => "encode_text",
        0xfd => "copy_table",
        0xfe => "print_table",
        0xff => "check_arg",
        _ => "unknown",
    }
    .to_string()
}

fn operand_type(type_byte: u8, operand_index: u8) -> OperandType {
    // Types are packed in the byte: 00112233
    // To get type 1 (index 0), shift left 6 bits
    // To get type 2 (index 1), shift left 4 bits
    // ... to get type n, shift left 6 - (n * 2) bits
    let t = (type_byte >> (6 - (operand_index * 2))) & 3;
    match t {
        0 => OperandType::LargeConstant,
        1 => OperandType::SmallConstant,
        2 => OperandType::Variable,
        _ => OperandType::Omitted,
    }
}

fn long_operand_type(opcode: u8, index: u8) -> OperandType {
    if opcode >> (6 - index) & 1 == 1 {
        OperandType::Variable
    } else {
        OperandType::SmallConstant
    }
}

fn operand_types(
    memory: &Memory,
    opcode: u8,
    form: &OpcodeForm,
    mut address: usize,
) -> Result<(usize, Vec<OperandType>), RuntimeError> {
    let mut types = Vec::new();
    match form {
        OpcodeForm::Short => {
            let t = operand_type(opcode, 1);
            types.push(t);
        }
        OpcodeForm::Long => {
            types.push(long_operand_type(opcode, 0));
            types.push(long_operand_type(opcode, 1));
        }
        OpcodeForm::Var => {
            let b = memory.read_byte(address)?;
            address = address + 1;
            for i in 0..4 {
                types.push(operand_type(b, i));
            }
            // 2VAR opcodes have another byte of operand types
            if opcode == 0xEC || opcode == 0xFA {
                let b = memory.read_byte(address)?;
                address = address + 1;
                for i in 0..4 {
                    types.push(operand_type(b, i));
                }
            }
        }
        OpcodeForm::Ext => {}
    }

    Ok((address, types))
}

fn operands(
    memory: &Memory,
    operand_types: &Vec<OperandType>,
    mut address: usize,
) -> Result<(usize, Vec<u16>), RuntimeError> {
    let mut operands = Vec::new();

    for optype in operand_types {
        match optype {
            OperandType::LargeConstant => {
                operands.push(memory.read_word(address)?);
                address = address + 2;
            }
            OperandType::SmallConstant | OperandType::Variable => {
                operands.push(memory.read_byte(address)? as u16);
                address = address + 1;
            }
            OperandType::Omitted => break,
        }
    }

    Ok((address, operands))
}

fn form(opcode: u8) -> OpcodeForm {
    match opcode {
        0xBE => OpcodeForm::Ext,
        _ => match (opcode >> 6) & 3 {
            3 => OpcodeForm::Var,
            2 => OpcodeForm::Short,
            _ => OpcodeForm::Long,
        },
    }
}

fn result_variable(
    memory: &Memory,
    opcode: u8,
    version: u8,
    address: usize,
) -> Result<(usize, Option<u8>), RuntimeError> {
    match opcode {
        // Always store, regardless of version
        0x08 | 0x28 | 0x48 | 0x68 | 0xc8 | 
        0x09 | 0x29 | 0x49 | 0x69 | 0xc9 | 
        0x0F | 0x2F | 0x4F | 0x6F | 0xcf | 
        0x10 | 0x30 | 0x50 | 0x70 | 0xd0 | 
        0x11 | 0x31 | 0x51 | 0x71 | 0xd1 | 
        0x12 | 0x32 | 0x52 | 0x72 | 0xd2 | 
        0x13 | 0x33 | 0x53 | 0x73 | 0xd3 | 
        0x14 | 0x34 | 0x54 | 0x74 | 0xd4 | 
        0x15 | 0x35 | 0x55 | 0x75 | 0xd5 | 
        0x16 | 0x36 | 0x56 | 0x76 | 0xd6 | 
        0x17 | 0x37 | 0x57 | 0x77 | 0xd7 | 
        0x18 | 0x38 | 0x58 | 0x78 | 0xd8 | 
        0x19 | 0x39 | 0x59 | 0x79 | 0xd9 | 
        0x81 | 0x91 | 0xa1 | 
        0x82 | 0x92 | 0xa2 | 
        0x83 | 0x93 | 0xa3 | 
        0x84 | 0x94 | 0xa4 | 
        0x88 | 0x98 | 0xa8 | 
        0x8e | 0x9e | 0xae | 
        0xe0 | 
        0xe7 | 
        0xeC | 
        0xf6 | 
        0xf7 | 
        0xf8 => Ok((address + 1, Some(memory.read_byte(address)?))),
        // Version < 5
        0xbf => {
            if version < 5 {
                return Ok((address + 1, Some(memory.read_byte(address)?)));
            } else {
                return Ok((address, None));
            }
        }
        // Version 4
        0xb5 | 0xb6 => {
            if version == 3 {
                return Ok((address + 1, Some(memory.read_byte(address)?)));
            } else {
                return Ok((address, None));
            }
        }
        // Version > 4
        0xb9 | 0xe4 | 0xf8 => {
            if version > 4 {
                return Ok((address + 1, Some(memory.read_byte(address)?)));
            } else {
                return Ok((address, None));
            }
        }
        _ => Ok((address, None)),
    }

}

fn branch_condition(memory: &Memory, opcode: u8, version: u8, address: usize) -> Result<(usize, Option<Branch>), RuntimeError> {
    match opcode {
        0x01 | 0x21 | 0x41 | 0x61 | 0xc1 |
        0x02 | 0x22 | 0x42 | 0x62 | 0xc2 |
        0x03 | 0x23 | 0x43 | 0x63 | 0xc3 |
        0x04 | 0x24 | 0x44 | 0x64 | 0xc4 |
        0x05 | 0x25 | 0x45 | 0x65 | 0xc5 |
        0x06 | 0x26 | 0x46 | 0x66 | 0xc6 | 
        0x07 | 0x27 | 0x47 | 0x67 | 0xc7 |
        0x0a | 0x2a | 0x4a | 0x6a | 0xca | 
        0x80 | 0x90 | 0xa0 |
        0x81 | 0x91 | 0xa1 |
        0x82 | 0x92 | 0xa2 |
        /* b5 for < version 4 */
        /* b6 for < version 4 */
        0xbd |
        /* be for ext_opcode = 06, 18, 1b */
        0xbf => {
            let b = memory.read_byte(address)?;
            let condition = b & 0x80 == 0x80;
            if b & 0x40 == 0x40 {
                let offset = (b & 0x3f) as usize;
                return Ok((
                    address + 1,
                    Some(Branch {
                        condition,
                        dest: match offset {
                            0 => 0,
                            1 => 1,
                            _ => address + offset - 1,
                        },
                    }),
                ));
            } else {
                let lb = memory.read_byte(address + 1)?;
                let mut offset = memory::word_value(b & 0x3f, lb);
                // let mut offset = (((b & 0x3f) as u16) << 8) | map.read_byte(address + 1)? as u16;
                if offset & 0x2000 == 0x2000 {
                    offset = offset | 0xC000;
                }
                return Ok((
                    address + 2,
                    Some(Branch {
                        condition,
                        dest: match offset {
                            0 => 0,
                            1 => 1,
                            _ => ((address as isize) + (offset as i16) as isize) as usize
                            },
                    }),
                ));
            }
        }
        _ => Ok((address, None)),
    }
}

fn opcode(memory: &Memory, address: usize) -> Result<(usize, u8, Option<u8>),RuntimeError> {
    let opcode = memory.read_byte(address)?;
    if opcode == 0xBE {
        Ok((address + 2, opcode, Some(memory.read_byte(address + 1)?)))
    } else {
        Ok((address + 1, opcode, None))
    }
}

pub fn decode_instruction(memory: &Memory, address: usize) -> Result<Instruction,RuntimeError> {
    let version = header::field_byte(memory, HeaderField::Version)?;
    let (offset, opcode, ext_opcode) = opcode(memory, address)?;
    let opcode_name = opcode_name(opcode, ext_opcode, version);
    let opcode_form = form(opcode);

    let (offset, operand_types) = operand_types(memory, opcode, &opcode_form, offset)?;
    let (offset, operands) = operands(memory, &operand_types, offset)?;
    let (offset, store) = result_variable(memory, opcode, version, offset)?;
    let (offset, branch) = branch_condition(memory, opcode, version, offset)?;

    let mut bytes = Vec::new();
    for i in address..offset {
        bytes.push(memory.read_byte(i)?)
    };

    Ok(Instruction {
        address,
        bytes,
        opcode,
        ext_opcode,
        opcode_form,
        opcode_name,
        operand_types,
        operands,
        store,
        branch,
        next_pc: offset,
    })
}