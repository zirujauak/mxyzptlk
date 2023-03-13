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
///
/// ```TODO: Refactor this to return an option to indicate an invalid Flag was used
/// instead of returning 0?```
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
        if size > index {
            state.set_word(table + ((index + 1) * 2), value);
        } else {
            // TODO: This should probably halt execution
            error!(
                "Attempt to set entry {} in header extension table with size {}",
                index, size
            );
        }
    }
}

#[cfg(test)]
mod test {
    use crate::interpreter::Interpreter;

    use super::*;

    fn memory_map() -> Vec<u8> {
        let mut memory = Vec::new();
        for i in 0..0x40 {
            memory.push(i as u8);
        }

        // Header extension table at 0x40
        memory[0x36] = 0x00;
        memory[0x37] = 0x40;

        memory.append(&mut vec![
            0x00 as u8, 0x03, 0x11, 0x11, 0x22, 0x22, 0x33, 0x33,
        ]);

        memory
    }

    struct DummyInterpreter;

    impl Interpreter for DummyInterpreter {
        fn buffer_mode(&mut self, mode: bool) {
            todo!()
        }

        fn erase_line(&mut self, value: u16) {
            todo!()
        }

        fn erase_window(&mut self, window: i16) {
            todo!()
        }

        fn get_cursor(&mut self) -> (u16, u16) {
            todo!()
        }

        fn input_stream(&mut self, stream: u16) {
            todo!()
        }

        fn new_line(&mut self) {
            todo!()
        }

        fn output_stream(&mut self, stream: i16, table: usize) {
            todo!()
        }

        fn print(&mut self, text: String) {
            todo!()
        }

        fn print_table(&mut self, data: Vec<u8>, width: u16, height: u16, skip: u16) {
            todo!()
        }

        fn read(
            &mut self,
            length: u8,
            time: u16,
            existing_input: &Vec<char>,
            redraw: bool,
            terminators: Vec<u8>,
        ) -> (Vec<char>, bool, crate::interpreter::Input) {
            todo!()
        }

        fn read_char(&mut self, time: u16) -> crate::interpreter::Input {
            todo!()
        }

        fn set_colour(&mut self, foreground: u16, background: u16) {
            todo!()
        }

        fn set_cursor(&mut self, line: u16, column: u16) {
            todo!()
        }

        fn set_font(&mut self, font: u16) -> u16 {
            todo!()
        }

        fn set_text_style(&mut self, style: u16) {
            todo!()
        }

        fn set_window(&mut self, window: u16) {
            todo!()
        }

        fn show_status(&mut self, location: &str, status: &str) {
            todo!()
        }

        fn sound_effect(&mut self, number: u16, effect: u16, volume: u8, repeats: u8, routine: Option<usize>) {
            todo!()
        }

        fn split_window(&mut self, lines: u16) {
            todo!()
        }

        fn save(&mut self, data: &Vec<u8>) {
            todo!()
        }

        fn restore(&mut self) -> Vec<u8> {
            todo!()
        }

        fn resources(&mut self, sounds: std::collections::HashMap<u8, crate::interpreter::Sound>) {
            todo!()
        }

        fn spec(&mut self, version: u8) -> crate::interpreter::Spec {
            todo!()
        }
    }

    #[test]
    fn version_test() {
        let state = State::new(&memory_map(), Box::new(DummyInterpreter {}));
        assert_eq!(0x00, version(&state));
    }

    #[test]
    fn flag_bit_test() {
        // V1
        assert_eq!(1, flag_bit(1, &Flag::StatusLineType));
        assert_eq!(3, flag_bit(1, &Flag::TandyBit));
        assert_eq!(4, flag_bit(1, &Flag::StatusLineNotAvailable));
        assert_eq!(5, flag_bit(1, &Flag::ScreenSplittingAvailable));
        assert_eq!(6, flag_bit(1, &Flag::VariablePitchDefaultFont));
        assert_eq!(0, flag_bit(1, &Flag::Transcripting));
        // TODO: See refactor note in code, this test may need to change
        assert_eq!(0, flag_bit(1, &Flag::PicturesAvailable));

        // V2
        assert_eq!(1, flag_bit(2, &Flag::StatusLineType));
        assert_eq!(3, flag_bit(2, &Flag::TandyBit));
        assert_eq!(4, flag_bit(2, &Flag::StatusLineNotAvailable));
        assert_eq!(5, flag_bit(2, &Flag::ScreenSplittingAvailable));
        assert_eq!(6, flag_bit(2, &Flag::VariablePitchDefaultFont));
        assert_eq!(0, flag_bit(2, &Flag::Transcripting));
        assert_eq!(0, flag_bit(2, &Flag::PicturesAvailable));

        // V3
        assert_eq!(1, flag_bit(3, &Flag::StatusLineType));
        assert_eq!(3, flag_bit(3, &Flag::TandyBit));
        assert_eq!(4, flag_bit(3, &Flag::StatusLineNotAvailable));
        assert_eq!(5, flag_bit(3, &Flag::ScreenSplittingAvailable));
        assert_eq!(6, flag_bit(3, &Flag::VariablePitchDefaultFont));
        assert_eq!(0, flag_bit(3, &Flag::Transcripting));
        assert_eq!(1, flag_bit(3, &Flag::ForceFixedPitch));
        assert_eq!(0, flag_bit(3, &Flag::PicturesAvailable));

        // V4
        assert_eq!(2, flag_bit(4, &Flag::BoldfaceAvailable));
        assert_eq!(3, flag_bit(4, &Flag::ItalicAvailable));
        assert_eq!(4, flag_bit(4, &Flag::FixedSpaceAvailable));
        assert_eq!(7, flag_bit(4, &Flag::TimedInputAvailable));
        assert_eq!(0, flag_bit(4, &Flag::Transcripting));
        assert_eq!(1, flag_bit(4, &Flag::ForceFixedPitch));
        assert_eq!(0, flag_bit(4, &Flag::StatusLineNotAvailable));

        // V5
        assert_eq!(0, flag_bit(5, &Flag::ColoursAvailable));
        assert_eq!(1, flag_bit(5, &Flag::PicturesAvailable));
        assert_eq!(2, flag_bit(5, &Flag::BoldfaceAvailable));
        assert_eq!(3, flag_bit(5, &Flag::ItalicAvailable));
        assert_eq!(4, flag_bit(5, &Flag::FixedSpaceAvailable));
        assert_eq!(5, flag_bit(5, &Flag::SoundEffectsAvailable));
        assert_eq!(7, flag_bit(5, &Flag::TimedInputAvailable));
        assert_eq!(0, flag_bit(5, &Flag::Transcripting));
        assert_eq!(1, flag_bit(5, &Flag::ForceFixedPitch));
        assert_eq!(3, flag_bit(5, &Flag::GameWantsPictures));
        assert_eq!(4, flag_bit(5, &Flag::GameWantsUndo));
        assert_eq!(5, flag_bit(5, &Flag::GameWantsMouse));
        assert_eq!(6, flag_bit(5, &Flag::GameWantsColour));
        assert_eq!(7, flag_bit(5, &Flag::GameWantsSoundEffects));
        assert_eq!(0, flag_bit(5, &Flag::StatusLineNotAvailable));

        // V6
        assert_eq!(0, flag_bit(6, &Flag::ColoursAvailable));
        assert_eq!(1, flag_bit(6, &Flag::PicturesAvailable));
        assert_eq!(2, flag_bit(6, &Flag::BoldfaceAvailable));
        assert_eq!(3, flag_bit(6, &Flag::ItalicAvailable));
        assert_eq!(4, flag_bit(6, &Flag::FixedSpaceAvailable));
        assert_eq!(5, flag_bit(6, &Flag::SoundEffectsAvailable));
        assert_eq!(7, flag_bit(6, &Flag::TimedInputAvailable));
        assert_eq!(0, flag_bit(6, &Flag::Transcripting));
        assert_eq!(1, flag_bit(6, &Flag::ForceFixedPitch));
        assert_eq!(2, flag_bit(6, &Flag::RequestRedraw));
        assert_eq!(3, flag_bit(6, &Flag::GameWantsPictures));
        assert_eq!(4, flag_bit(6, &Flag::GameWantsUndo));
        assert_eq!(5, flag_bit(6, &Flag::GameWantsMouse));
        assert_eq!(6, flag_bit(6, &Flag::GameWantsColour));
        assert_eq!(7, flag_bit(6, &Flag::GameWantsSoundEffects));
        assert_eq!(8, flag_bit(6, &Flag::GameWantsMenus));
        assert_eq!(0, flag_bit(6, &Flag::StatusLineNotAvailable));

        // V7
        assert_eq!(0, flag_bit(7, &Flag::ColoursAvailable));
        assert_eq!(1, flag_bit(7, &Flag::PicturesAvailable));
        assert_eq!(2, flag_bit(7, &Flag::BoldfaceAvailable));
        assert_eq!(3, flag_bit(7, &Flag::ItalicAvailable));
        assert_eq!(4, flag_bit(7, &Flag::FixedSpaceAvailable));
        assert_eq!(5, flag_bit(7, &Flag::SoundEffectsAvailable));
        assert_eq!(7, flag_bit(7, &Flag::TimedInputAvailable));
        assert_eq!(0, flag_bit(7, &Flag::Transcripting));
        assert_eq!(1, flag_bit(7, &Flag::ForceFixedPitch));
        assert_eq!(3, flag_bit(7, &Flag::GameWantsPictures));
        assert_eq!(4, flag_bit(7, &Flag::GameWantsUndo));
        assert_eq!(5, flag_bit(7, &Flag::GameWantsMouse));
        assert_eq!(6, flag_bit(7, &Flag::GameWantsColour));
        assert_eq!(7, flag_bit(7, &Flag::GameWantsSoundEffects));
        assert_eq!(0, flag_bit(7, &Flag::StatusLineNotAvailable));

        // V8
        assert_eq!(0, flag_bit(8, &Flag::ColoursAvailable));
        assert_eq!(1, flag_bit(8, &Flag::PicturesAvailable));
        assert_eq!(2, flag_bit(8, &Flag::BoldfaceAvailable));
        assert_eq!(3, flag_bit(8, &Flag::ItalicAvailable));
        assert_eq!(4, flag_bit(8, &Flag::FixedSpaceAvailable));
        assert_eq!(5, flag_bit(8, &Flag::SoundEffectsAvailable));
        assert_eq!(7, flag_bit(8, &Flag::TimedInputAvailable));
        assert_eq!(0, flag_bit(8, &Flag::Transcripting));
        assert_eq!(1, flag_bit(8, &Flag::ForceFixedPitch));
        assert_eq!(3, flag_bit(8, &Flag::GameWantsPictures));
        assert_eq!(4, flag_bit(8, &Flag::GameWantsUndo));
        assert_eq!(5, flag_bit(8, &Flag::GameWantsMouse));
        assert_eq!(6, flag_bit(8, &Flag::GameWantsColour));
        assert_eq!(7, flag_bit(8, &Flag::GameWantsSoundEffects));
        assert_eq!(0, flag_bit(8, &Flag::StatusLineNotAvailable));

        // Invalid version
        assert_eq!(0, flag_bit(9, &Flag::Transcripting));
    }

    #[test]
    fn is_flag1_test() {
        assert_eq!(true, is_flag1(&Flag::StatusLineType));
        assert_eq!(true, is_flag1(&Flag::StatusLineNotAvailable));
        assert_eq!(true, is_flag1(&Flag::TandyBit));
        assert_eq!(true, is_flag1(&Flag::ScreenSplittingAvailable));
        assert_eq!(true, is_flag1(&Flag::VariablePitchDefaultFont));
        assert_eq!(true, is_flag1(&Flag::ColoursAvailable));
        assert_eq!(true, is_flag1(&Flag::PicturesAvailable));
        assert_eq!(true, is_flag1(&Flag::BoldfaceAvailable));
        assert_eq!(true, is_flag1(&Flag::ItalicAvailable));
        assert_eq!(true, is_flag1(&Flag::FixedSpaceAvailable));
        assert_eq!(true, is_flag1(&Flag::SoundEffectsAvailable));
        assert_eq!(true, is_flag1(&Flag::TimedInputAvailable));
        assert_eq!(false, is_flag1(&Flag::Transcripting));
        assert_eq!(false, is_flag1(&Flag::ForceFixedPitch));
        assert_eq!(false, is_flag1(&Flag::RequestRedraw));
        assert_eq!(false, is_flag1(&Flag::GameWantsPictures));
        assert_eq!(false, is_flag1(&Flag::GameWantsUndo));
        assert_eq!(false, is_flag1(&Flag::GameWantsMouse));
        assert_eq!(false, is_flag1(&Flag::GameWantsColour));
        assert_eq!(false, is_flag1(&Flag::GameWantsSoundEffects));
        assert_eq!(false, is_flag1(&Flag::GameWantsMenus));
    }

    fn flag_test() {
        let mut memory = memory_map();
        memory[0x01] = 0;
        memory[0x10] = 0;
        memory[0x11] = 0;
        let mut state = State::new(&memory, Box::new(DummyInterpreter{}));

    }
    #[test]
    fn static_memory_base_test() {
        let state = State::new(&memory_map(), Box::new(DummyInterpreter {}));
        assert_eq!(0x0E0F, static_memory_base(&state));
    }

    #[test]
    fn length_text() {
        let state = State::new(&memory_map(), Box::new(DummyInterpreter {}));
        assert_eq!(0x1A1B, length(&state));
    }

    #[test]
    fn checksum_test() {
        let state = State::new(&memory_map(), Box::new(DummyInterpreter {}));
        assert_eq!(0x1C1D, checksum(&state));
    }

    #[test]
    fn terminating_character_table_test() {
        let state = State::new(&memory_map(), Box::new(DummyInterpreter {}));
        assert_eq!(0x2E2F, terminating_character_table(&state));
    }

    #[test]
    fn set_extension_word_test() {
        let mut state = State::new(&memory_map(), Box::new(DummyInterpreter {}));
        // Initial state
        assert_eq!(0x1111, state.word_value(0x42));
        assert_eq!(0x2222, state.word_value(0x44));
        assert_eq!(0x3333, state.word_value(0x46));

        // Set table entry 1 (index 0)
        set_extension_word(&mut state, 0, 0xABCD);
        assert_eq!(0xABCD, state.word_value(0x42));
        assert_eq!(0x2222, state.word_value(0x44));
        assert_eq!(0x3333, state.word_value(0x46));

        // Set table entry 3 (index 2)
        set_extension_word(&mut state, 2, 0xFEDC);
        assert_eq!(0xABCD, state.word_value(0x42));
        assert_eq!(0x2222, state.word_value(0x44));
        assert_eq!(0xFEDC, state.word_value(0x46));

        // Set table entry 4 (index 3), which is out of range
        set_extension_word(&mut state, 3, 0x1234);
        assert_eq!(0xABCD, state.word_value(0x42));
        assert_eq!(0x2222, state.word_value(0x44));
        assert_eq!(0xFEDC, state.word_value(0x46));
    }
}
