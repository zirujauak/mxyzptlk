use std::{fmt, str::FromStr};

pub enum OpcodeForm {
    Short,
    Long,
    Var,
    DoubleVar,
    Ext,
}

#[derive(Debug)]
pub enum OperandType {
    LargeConstant,
    SmallConstant,
    Variable,
    Omitted,
}

struct Branch {
    condition: bool,
    dest: usize,
}

pub struct Instruction {
    address: usize,
    bytes: Vec<u8>,
    opcode: u8,
    ext_opcode: Option<u8>,
    opcode_form: OpcodeForm,
    opcode_name: String,
    operand_types: Vec<OperandType>,
    operands: Vec<u16>,
    store: Option<u8>,
    branch: Option<Branch>,
    pub next_pc: usize,
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "${:05x}:", self.address)?;
        for b in &self.bytes {
            write!(f, " {:02x}", b)?;
        }
        for i in 0..8-self.bytes.len() {
            write!(f, "   ")?;
        }
        write!(f, "{:15} ", self.opcode_name.to_uppercase())?;
        for i in 0..self.operand_types.len() {
            match self.operand_types[i] {
                OperandType::LargeConstant => write!(f, " ${:04x}", self.operands[i])?,
                OperandType::SmallConstant => write!(f, " ${:02x}", self.operands[i])?,
                OperandType::Variable => {
                    if self.operands[i] == 0 {
                        write!(f, " (SP)+")?
                    } else if self.operands[i] < 16 {
                        write!(f, " L{:02x}", self.operands[i] - 1)?
                    } else {
                        write!(f, " G{:02x}", self.operands[i] - 16)?
                    }
                }
                OperandType::Omitted => {}
            }
        }
        // for ot in &self.operand_types {
        //     match ot {
        //         OperandType::Omitted => {}
        //         _ => write!(f, " {:?}", ot)?,
        //     }
        // }

        match &self.store {
            Some(s) => {
                write!(f, " -> ")?;
                if *s == 0 {
                    write!(f, "-(SP)")?
                } else if *s < 16 {
                    write!(f, "L{:02x}", *s - 1)?
                } else {
                    write!(f, "G{:02x}", *s - 16)?
                }
            }
            None => {}
        }
        match &self.branch {
            Some(b) => write!(f, " [{}] ${:05x}", b.condition.to_string().to_uppercase(), b.dest)?,
            None => {}
        }

        write!(f, "")
    }
}

fn map_opcode(version: u8, opcode: u8, ext_opcode: Option<u8>) -> String {
    String::from_str(match opcode {
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
        0xbe => {
            match ext_opcode {
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
                _ => "unknown"
            }
        }
        0xbf => "piracy",
        0xe0 => {
            if version < 4 {
                return "call_routine".to_string();
            } else {
                return "call_vs".to_string();
            }
        },
        0xe1 => "storew",
        0xe2 => "storeb",
        0xe3 => "put_prop",
        0xe4 => {
            if version < 5 {
                return "sread".to_string();
            } else {
                return "aread".to_string();
            }
        },
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
        },
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
    })
    .unwrap()
}

fn decode_operand_type(b: u8, n: u8) -> OperandType {
    match (b >> (6 - (n * 2))) & 3 {
        0 => OperandType::LargeConstant,
        1 => OperandType::SmallConstant,
        2 => OperandType::Variable,
        _ => OperandType::Omitted,
    }
}

fn word_value(v: &Vec<u8>, a: usize) -> u16 {
    let hb: u16 = (((v[a] as u16) << 8) as u16 & 0xFF00) as u16;
    let lb: u16 = (v[a + 1] & 0xFF) as u16;
    hb + lb
}

fn decode_operands(
    map: &Vec<u8>,
    mut address: usize,
    operand_types: &Vec<OperandType>,
) -> (usize, Vec<u16>) {
    let mut o = Vec::new();

    for optype in operand_types {
        match optype {
            OperandType::LargeConstant => {
                o.push(word_value(map, address));
                address = address + 2
            }
            OperandType::SmallConstant => {
                o.push(map[address] as u16);
                address = address + 1
            }
            OperandType::Variable => {
                o.push(map[address] as u16);
                address = address + 1
            }
            OperandType::Omitted => {}
        }
    }

    (address, o)
}

fn decode_operand_types(
    map: &Vec<u8>,
    mut address: usize,
    opcode: u8,
    form: &OpcodeForm,
) -> (usize, Vec<OperandType>) {
    let mut ot = Vec::new();
    match form {
        OpcodeForm::Short => {
            let t = decode_operand_type(opcode, 1);
            match t {
                OperandType::Omitted => {}
                _ => ot.push(decode_operand_type(opcode, 1)),
            }
        }
        OpcodeForm::Long => {
            if opcode & 0x40 == 0x40 {
                ot.push(OperandType::Variable)
            } else {
                ot.push(OperandType::SmallConstant)
            }
            if opcode & 0x20 == 0x20 {
                ot.push(OperandType::Variable)
            } else {
                ot.push(OperandType::SmallConstant)
            }
        }
        OpcodeForm::Var => {
            let b = map[address];
            address = address + 1;
            for i in 0..4 {
                ot.push(decode_operand_type(b, i))
            }
        }
        OpcodeForm::DoubleVar => {
            let b = map[address];
            for i in 0..4 {
                ot.push(decode_operand_type(b, i))
            }
            let c = map[address + 1];
            for i in 0..4 {
                ot.push(decode_operand_type(c, i))
            }
            address = address + 2
        }
        OpcodeForm::Ext => {}
    }

    (address, ot)
}

fn map_opcode_form(o: u8) -> OpcodeForm {
    match o {
        0xBE => OpcodeForm::Ext,
        0xFA | 0xEC => OpcodeForm::DoubleVar,
        _ => match (o >> 6) & 3 {
            3 => OpcodeForm::Var,
            2 => OpcodeForm::Short,
            _ => OpcodeForm::Long,
        },
    }
}

fn decode_store(map: &Vec<u8>, version: u8, address: usize, opcode: u8) -> (usize, Option<u8>) {
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
        0xf8 => (address + 1, Some(map[address])),
        // Version < 5
        0xbf => {
            if version < 5 {
                return (address + 1, Some(map[address]));
            } else {
                return (address, None);
            }
        }
        // Version 4
        0xb5 | 0xb6 => {
            if version == 3 {
                return (address + 1, Some(map[address]));
            } else {
                return (address, None);
            }
        }
        // Version > 4
        0xb9 | 0xe4 | 0xf8 => {
            if version > 4 {
                return (address + 1, Some(map[address]));
            } else {
                return (address, None);
            }
        }
        _ => (address, None),
    }
}

fn decode_branch(map: &Vec<u8>, address: usize, opcode: u8) -> (usize, Option<Branch>) {
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
        0x8c | 0x9c | 0xac | 
        /* b5 for < version 4 */
        /* b6 for < version 4 */
        0xbd |
        /* be for ext_opcode = 06, 18, 1b */
        0xbf => {
            let b = map[address];
            let condition = b & 0x80 == 0x80;
            if b & 0x40 == 0x40 {
                let offset = (b & 0x3f) as usize;
                return (
                    address + 1,
                    Some(Branch {
                        condition,
                        dest: match offset {
                            0 => 0,
                            1 => 1,
                            _ => address + offset - 1,
                        },
                    }),
                );
            } else {
                let mut offset = (((b & 0x3f) as u16) << 8) | map[address + 1] as u16;
                if offset & 0x2000 == 0x2000 {
                    offset = offset | 0xC000;
                }
                return (
                    address + 2,
                    Some(Branch {
                        condition,
                        dest: match offset {
                            0 => 0,
                            1 => 1,
                            _ => ((address as isize) + (offset as i16) as isize) as usize
                            },
                    }),
                );
            }
        }
        _ => (address, None),
    }
}

fn decode_opcode(map: &Vec<u8>, addr: usize) -> (usize, u8, Option<u8>) {
    let opcode = map[addr];
    if opcode == 190 {
        return (addr + 2, opcode, Some(map[addr + 1]));
    } else {
        return (addr + 1, opcode, None);
    }
}

pub fn decode_instruction(map: &Vec<u8>, version: u8, address: usize) -> Instruction {
    let (offset, opcode, ext_opcode) = decode_opcode(map, address);
    let opcode_name = map_opcode(version, opcode, ext_opcode);
    let opcode_form = map_opcode_form(opcode);

    let (offset, operand_types) = decode_operand_types(map, offset, opcode, &opcode_form);

    // let operand_types = decode_operand_types(map, version, address, opcode, &opcode_form);
    // let opers = match opcode_form {
    //     OpcodeForm::Var => address + 2,
    //     OpcodeForm::DoubleVar => address + 3,
    //     OpcodeForm::Ext => address + 3,
    //     _ => address + 1
    // };

    let (offset, operands) = decode_operands(map, offset, &operand_types);
    let (offset, store) = decode_store(map, version, offset, opcode);
    let (offset, branch) = decode_branch(map, offset, opcode);

    Instruction {
        address,
        bytes: map[address..offset].to_vec(),
        opcode,
        ext_opcode,
        opcode_form,
        opcode_name,
        operand_types,
        operands,
        store,
        branch,
        next_pc: offset,
    }
}
