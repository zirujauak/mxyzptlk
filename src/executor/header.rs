use super::state::State;

/// Header flags, version specific
#[derive(Debug)]
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

/// Returns the ZMachine version (1-5, 7-8 are supported) stored in the header at offset $00
pub fn version(state: &State) -> u8 {
    state.byte_value(0x00)
}

/// Identifies the bit in a Flags structure that corresponds to a specific flag
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
                // Flags 2
                Flag::Transcripting => 0,
                Flag::ForceFixedPitch => 1,
                // TODO: This is an error
                _ => 0,
            }
        }
        4 => {
            match flag {
                // Flags 1
                Flag::BoldfaceAvailable => 2,
                Flag::ItalicAvailable => 3,
                Flag::FixedSpaceAvailable => 4,
                Flag::TimedInputAvailable => 7,
                // Flags 2
                Flag::Transcripting => 0,
                Flag::ForceFixedPitch => 1,
                // TODO: This is an error
                _ => 0,
            }
        }
        5 | 7 | 8 => {
            match flag {
                // Flags 1
                Flag::ColoursAvailable => 0,
                Flag::PicturesAvailable => 1,
                Flag::BoldfaceAvailable => 2,
                Flag::ItalicAvailable => 3,
                Flag::FixedSpaceAvailable => 4,
                Flag::SoundEffectsAvailable => 5,
                Flag::TimedInputAvailable => 7,
                // Flags 2
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
                // Flags 1
                Flag::ColoursAvailable => 0,
                Flag::PicturesAvailable => 1,
                Flag::BoldfaceAvailable => 2,
                Flag::ItalicAvailable => 3,
                Flag::FixedSpaceAvailable => 4,
                Flag::SoundEffectsAvailable => 5,
                Flag::TimedInputAvailable => 7,
                // Flags 2
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

/// Tests where a Flag is a member of the Flags1 structure.  If the result is false,
/// then the flag must be part of Flags2.
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

/// Returns the current value of a flag, `0` for off, `1` for on.
pub fn flag(state: &State, flag: Flag) -> u16 {
    let v = version(state);
    let bit = flag_bit(v, &flag);

    if is_flag1(&flag) {
        (state.byte_value(0x01) >> bit) as u16 & 1
    } else {
        (state.word_value(0x0a) >> bit) & 1
    }
}

/// Sets a flag to `1`
pub fn set_flag(state: &mut State, flag: Flag) {
    let v = version(state);
    let bit = flag_bit(v, &flag);

    if is_flag1(&flag) {
        let mask = ((1 as u8) << bit) & 0xFF;
        state.set_byte(0x01, state.byte_value(1) | mask);
    } else {
        let mask = ((1 as u16) << bit) & 0xFFFF;
        state.set_word(0x10, state.word_value(0x10) | mask);
    }
}

/// Clears a flag to `0`
pub fn clear_flag(state: &mut State, flag: Flag) {
    let v = state.version;
    let bit = flag_bit(v, &flag);

    if is_flag1(&flag) {
        let mask = !(((1 as u8) << bit) & 0xFF);
        state.set_byte(0x01, state.byte_value(1) & mask);
    } else {
        let mask = !(((1 as u16) << bit) & 0xFFFF);
        state.set_word(0x10, state.word_value(0x10) & mask);
    }
}

/// Returns the release number from the header stored at offset $02
pub fn release_number(state: &State) -> u16 {
    state.word_value(0x02)
}

/// Returns a vector containing the 6-byte serial number from the header stored
/// at offset $12
pub fn serial_number(state: &State) -> Vec<u8> {
    state.memory_map()[0x12..0x18].to_vec()
}

/// Returns the initial program counter from the header stored at offset $06
pub fn initial_pc(state: &State) -> u16 {
    state.word_value(0x06)
}

/// Returns the routine offset (V6,7) from the header stored at offset $28
pub fn routine_offset(state: &State) -> u16 {
    state.word_value(0x28)
}

/// Returns the string offset (V6,7) from the header stored at offset $2A
pub fn strings_offset(state: &State) -> u16 {
    state.word_value(0x2a)
}

/// Returns the object table byte address from the header stored at offset $0A
pub fn object_table(state: &State) -> usize {
    state.word_value(0x0a) as usize
}

/// Returns the global variable table byte address from the header stored at offset $0C
pub fn global_variable_table(state: &State) -> u16 {
    state.word_value(0x0c)
}

/// Returns the dictionary table byte address from the header stored at offset $08
pub fn dictionary_table(state: &State) -> u16 {
    state.word_value(0x08)
}

/// Returns the base of static memory from the header stored at offset $0E
pub fn static_memory_base(state: &State) -> u16 {
    state.word_value(0x0e)
}

/// Returns the (packed) length word from the header stored at offset $1A
pub fn length(state: &State) -> u16 {
    state.word_value(0x1a)
}

/// Returns the checksum word from the header stored at offset $1C
pub fn checksum(state: &State) -> u16 {
    state.word_value(0x1c)
}

/// Returns the terminating character table byte address from the header stored
/// at offset $2E
pub fn terminating_character_table(state: &State) -> u16 {
    state.word_value(0x2e)
}

/// Sets a word in the header extension table.
/// 
/// # Arguments
/// * `index`: 0-based index in the table of the word to set
/// * `value`: word to set
pub fn set_extension_word(state: &mut State, index: usize, value: u16) {
    let table = state.word_value(0x36) as usize;
    if table > 0 {
        let size = state.word_value(table) as usize;
        if size >= index {
            state.set_word(table + ((index + 1) * 2), value);
        }
    }
}
