pub mod flags;

use self::flags::*;
use std::fmt;

pub struct Header {
    version: u8,
    flags1: Flags1,
    release_number: u16,
    pub base_high_memory: u16,
    initial_pc: u16,
    dictionary_address: u16,
    pub object_table_address: u16,
    global_variable_table_address: u16,
    pub base_static_memory: u16,
    flags2: Flags2,
    serial_number: String,
    abbreviations_table_address: u16,
    file_length: usize,
    file_checksum: u16,
    interpreter_number: u8,
    interpreter_version: u8,
}

impl fmt::Display for Header {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Version: {}", self.version)?;
        writeln!(f, "Flags 1:\n{}", self.flags1)?;
        writeln!(f, "Release number: {}", self.release_number)?;
        writeln!(f, "Base of high memory: ${:x}", self.base_high_memory)?;
        writeln!(f, "Initial PC address: ${:x}", self.initial_pc)?;
        writeln!(f, "Dictionary address: ${:x}", self.dictionary_address)?;
        writeln!(f, "Object table address: ${:x}", self.object_table_address)?;
        writeln!(
            f,
            "Global variable table address: ${:x}",
            self.global_variable_table_address
        )?;
        writeln!(f, "Base of static memory: ${:x}", self.base_static_memory)?;
        writeln!(f, "Flags 2: \n{}", self.flags2)?;
        writeln!(f, "Serial number: {}", self.serial_number)?;
        writeln!(
            f,
            "Abbreviations table address: ${:x}",
            self.abbreviations_table_address
        )?;
        writeln!(f, "File length: ${:x}", self.file_length)?;
        writeln!(f, "File checksum: ${:x}", self.file_checksum)?;
        writeln!(f, "Interpreter number: {}", self.interpreter_number)?;
        write!(f, "Interpreter version: {}", self.interpreter_version)
    }
}

fn word_value(v: &Vec<u8>, a: usize) -> u16 {
    let hb: u16 = (((v[a] as u16) << 8) as u16 & 0xFF00) as u16;
    let lb: u16 = (v[a + 1] & 0xFF) as u16;
    hb + lb
}

fn string_value(v: &Vec<u8>, a: usize, l: usize) -> String {
    let mut s = String::new();
    for i in 0..l {
        s.push(v[a + i] as char);
    }
    s
}

fn file_length(v: &Vec<u8>) -> usize {
    let s: usize = word_value(v, 26) as usize;
    let v: u8 = v[0];
    match v {
        1 | 2 | 3 => s * 2,
        4 | 5 => s * 4,
        6 | 7 | 8 => s * 8,
        _ => s,
    }
}

impl Header {
    pub fn from_vec(v: &Vec<u8>) -> Self {
        Self {
            version: v[0],
            flags1: Flags1::from_byte(v[1]),
            release_number: word_value(v, 2),
            base_high_memory: word_value(v, 4),
            initial_pc: word_value(v, 6),
            dictionary_address: word_value(v, 8),
            object_table_address: word_value(v, 10),
            global_variable_table_address: word_value(v, 12),
            base_static_memory: word_value(v, 14),
            flags2: Flags2::from_word(word_value(v, 16)),
            serial_number: string_value(v, 18, 6),
            abbreviations_table_address: word_value(v, 24),
            file_length: file_length(v),
            file_checksum: word_value(v, 28),
            interpreter_number: 6,
            interpreter_version: 0xFF,
        }
    }

    pub fn flag(&mut self, f: Flags) -> u8 {
        match f {
            Flags::StatusLineType
            | Flags::StatusLineNotAvailable
            | Flags::ScreenSplittingAvailable
            | Flags::VariablePitchDefaultFont
            | Flags::TandyBit => self.flags1.flag(f),
            Flags::Transcripting | Flags::ForceFixedPitch | Flags::UseMenus => self.flags2.flag(f),
        }
    }

    pub fn set_flag(&mut self, v: &mut Vec<u8>, f: Flags) {
        match f {
            Flags::StatusLineNotAvailable
            | Flags::ScreenSplittingAvailable
            | Flags::VariablePitchDefaultFont
            | Flags::TandyBit => {
                self.flags1.set_flag(v, f);
            }
            Flags::Transcripting | Flags::ForceFixedPitch | Flags::UseMenus => self.flags2.set_flag(v, f),
            /* TODO: Error */
            _ => {}
        }
    }

    pub fn clear_flag(&mut self, v: &mut Vec<u8>, f: Flags) {
        match f {
            Flags::StatusLineNotAvailable
            | Flags::ScreenSplittingAvailable
            | Flags::VariablePitchDefaultFont
            | Flags::TandyBit => {
                self.flags1.clear_flag(v, f);
            }
            Flags::Transcripting | Flags::ForceFixedPitch | Flags::UseMenus => self.flags2.clear_flag(v, f),
            /* TODO: Error */
            _ => {}
        }
    }
}
