use crate::executor::util::*;

pub enum Flag {
    // Flags 1, v1 - 3
    StatusLineType,           // bit 1
    TandyBit,                 // bit 3
    StatusLineNotAvailable,   // bit 4
    ScreenSplittingAvailable, // bit 5
    VariablePitchDefaultFont, // bit 6
    // Flags 1, v4+
    ColoursAvailable,      // bit 0
    PicturesAvailable,     // bit 1
    BoldfaceAvailable,     // bit 2
    ItalicAvailable,       // bit 3
    FixedSpaceAvailable,   // bit 4
    SoundEffectsAvailable, // bit 5
    TimedInputAvailable,   // bit 7
    // Flags 2
    Transcripting,         // bit 0
    ForceFixedPitch,       // bit 1
    RequestRedraw,         // bit 2
    GameWantsPictures,     // bit 3
    GameWantsUndo,         // bit 4
    GameWantsMouse,        // bit 5
    GameWantsColour,       // bit 6
    GameWantsSoundEffects, // bit 7
    GameWantsMenus,        // bit 8
}

pub fn version(memory_map: &Vec<u8>) -> u8 {
    byte_value(memory_map, 0x00)
}

fn flag_bit(version: u8, flag: &Flag) -> u8 {
    match version {
        1 | 2 => {
            match flag {
                // Flags1
                Flag::StatusLineType => 1,
                Flag::TandyBit => 3,
                Flag::StatusLineNotAvailable => 4,
                Flag::ScreenSplittingAvailable => 5,
                Flag::VariablePitchDefaultFont => 6,
                Flag::Transcripting => 0,
                // TODO: This is an error
                _ => 0,
            }
        }
        3 => {
            match flag {
                // Flags1
                Flag::StatusLineType => 1,
                Flag::TandyBit => 3,
                Flag::StatusLineNotAvailable => 4,
                Flag::ScreenSplittingAvailable => 5,
                Flag::VariablePitchDefaultFont => 6,
                Flag::Transcripting => 0,
                Flag::ForceFixedPitch => 1,
                // TODO: This is an error
                _ => 0,
            }
        }
        4 => {
            match flag {
                Flag::BoldfaceAvailable => 2,
                Flag::ItalicAvailable => 3,
                Flag::FixedSpaceAvailable => 4,
                Flag::TimedInputAvailable => 7,
                Flag::Transcripting => 0,
                Flag::ForceFixedPitch => 1,
                // TODO: This is an error
                _ => 0,
            }
        }
        5 | 7 | 8 => {
            match flag {
                Flag::ColoursAvailable => 0,
                Flag::BoldfaceAvailable => 2,
                Flag::ItalicAvailable => 3,
                Flag::FixedSpaceAvailable => 4,
                Flag::TimedInputAvailable => 7,
                Flag::Transcripting => 0,
                Flag::ForceFixedPitch => 1,
                Flag::GameWantsPictures => 3,
                Flag::GameWantsUndo => 4,
                Flag::GameWantsMouse => 5,
                Flag::GameWantsColour => 6,
                Flag::GameWantsSoundEffects => 7,
                // TODO: This is an error
                _ => 0,
            }
        }
        6 => {
            match flag {
                Flag::ColoursAvailable => 0,
                Flag::PicturesAvailable => 1,
                Flag::BoldfaceAvailable => 2,
                Flag::ItalicAvailable => 3,
                Flag::FixedSpaceAvailable => 4,
                Flag::SoundEffectsAvailable => 5,
                Flag::TimedInputAvailable => 7,
                Flag::Transcripting => 0,
                Flag::ForceFixedPitch => 1,
                Flag::RequestRedraw => 2,
                Flag::GameWantsPictures => 3,
                Flag::GameWantsUndo => 4,
                Flag::GameWantsMouse => 5,
                Flag::GameWantsColour => 6,
                Flag::GameWantsSoundEffects => 7,
                Flag::GameWantsMenus => 8,
                // TODO: This is an error
                _ => 0,
            }
        }
        // TODO: This is an error
        _ => 0,
    }
}

fn is_flag1(flag: &Flag) -> bool {
    match flag {
        Flag::StatusLineType
        | Flag::StatusLineNotAvailable
        | Flag::TandyBit
        | Flag::ScreenSplittingAvailable
        | Flag::VariablePitchDefaultFont
        | Flag::ColoursAvailable
        | Flag::PicturesAvailable
        | Flag::BoldfaceAvailable
        | Flag::ItalicAvailable
        | Flag::FixedSpaceAvailable
        | Flag::SoundEffectsAvailable
        | Flag::TimedInputAvailable => true,
        _ => false,
    }
}
pub fn flag(memory_map: &Vec<u8>, flag: Flag) -> u16 {
    let v = version(memory_map);
    let bit = flag_bit(v, &flag);

    if is_flag1(&flag) {
        (byte_value(memory_map, 0x01) >> bit) as u16 & 1
    } else {
        (word_value(memory_map, 0x0a) >> bit) & 1
    }
}

pub fn set_flag(memory_map: &mut Vec<u8>, flag: Flag) {
    let v = version(memory_map);
    let bit = flag_bit(v, &flag);

    if is_flag1(&flag) {
        let mask = ((1 as u8) << bit) & 0xFF;
        set_byte(memory_map, 0x01, byte_value(memory_map, 1) | mask);
    } else {
        let mask = ((1 as u16) << bit) & 0xFFFF;
        set_word(memory_map, 0x0a, word_value(memory_map, 10) | mask);
    }
}

pub fn clear_flag(memory_map: &mut Vec<u8>, flag: Flag) {
    let v = version(memory_map);
    let bit = flag_bit(v, &flag);

    if is_flag1(&flag) {
        let mask = !(((1 as u8) << bit) & 0xFF);
        set_byte(memory_map, 0x01, byte_value(memory_map, 1) & mask);
    } else {
        let mask = !(((1 as u16) << bit) & 0xFFFF);
        set_word(memory_map, 0x0a, word_value(memory_map, 10) & mask);
    }
}

pub fn initial_pc(memory_map: &Vec<u8>) -> u16 {
    word_value(memory_map, 0x06)
}

pub fn routine_offset(memory_map: &Vec<u8>) -> u16 {
    word_value(memory_map, 0x28)
}

pub fn strings_offset(memory_map: &Vec<u8>) -> u16 {
    word_value(memory_map, 0x2a)
}

pub fn object_table(memory_map: &Vec<u8>) -> usize {
    word_value(memory_map, 0x0a) as usize
}

pub fn global_variable_table(memory_map: &Vec<u8>) -> u16 {
    word_value(memory_map, 0x0c)
}
