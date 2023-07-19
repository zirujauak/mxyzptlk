use crate::error::RuntimeError;

use super::State;

pub enum HeaderField {
    Version = 0x00,
    Flags1 = 0x01,
    Release = 0x02,
    HighMark = 0x04,
    InitialPC = 0x06,
    Dictionary = 0x08,
    ObjectTable = 0x0A,
    GlobalTable = 0x0C,
    StaticMark = 0x0E,
    Flags2 = 0x10,
    Serial = 0x12,
    AbbreviationsTable = 0x18,
    FileLength = 0x1A,
    Checksum = 0x1C,
    InterpreterNumber = 0x1E,
    InterpreterVersion = 0x1F,
    ScreenLines = 0x20,
    ScreenColumns = 0x21,
    ScreenWidth = 0x22,
    ScreenHeight = 0x24,
    FontWidth = 0x26,
    FontHeight = 0x27,
    RoutinesOffset = 0x28,
    StringsOffset = 0x2A,
    DefaultBackground = 0x2C,
    DefaultForeground = 0x2D,
    TerminatorTable = 0x2E,
    Revision = 0x32,
    AlphabetTable = 0x34,
    ExtensionTable = 0x36,
    InformVersion = 0x3C,
}

pub enum Flags1v3 {
    // V3 flags
    StatusLineType = 0x02,         // bit 1
    StatusLineNotAvailable = 0x10, // bit 4
    ScreenSplitAvailable = 0x20,   // bit 5
    VariablePitchDefault = 0x40,   // bit 6
}

pub enum Flags1v4 {
    // V4+ flags
    ColoursAvailable = 0x01,      // bit 0
    PicturesAvailable = 0x02,     // bit 1
    BoldfaceAvailable = 0x04,     // bit 2
    ItalicAvailable = 0x08,       // bit 3
    FixedSpaceAvailable = 0x10,   // bit 4
    SoundEffectsAvailable = 0x20, // bit 5
    TimedInputAvailable = 0x80,   // bit 7
}

#[derive(Debug)]
pub enum Flags2 {
    Transcripting = 0x0001,       // bit 0
    ForceFixedPitch = 0x0002,     // bit 1
    RequestPictures = 0x0008,     // bit 3
    RequestUndo = 0x0010,         // bit 4
    RequestMouse = 0x0020,        // bit 5
    RequestColours = 0x0040,      // bit 6
    RequestSoundEffects = 0x0080, // bit 7
}

pub fn field_byte(state: &State, field: HeaderField) -> Result<u8, RuntimeError> {
    state.read_byte(field as usize)
}

pub fn field_word(state: &State, field: HeaderField) -> Result<u16, RuntimeError> {
    state.read_word(field as usize)
}

pub fn set_byte(state: &mut State, field: HeaderField, value: u8) -> Result<(), RuntimeError> {
    state.write_byte(field as usize, value)
}

pub fn set_word(state: &mut State, field: HeaderField, value: u16) -> Result<(), RuntimeError> {
    state.write_word(field as usize, value)
}

pub fn flag1(state: &State, flag: u8) -> Result<u8, RuntimeError> {
    let flags = field_byte(state, HeaderField::Flags1)?;
    if flags & flag > 0 {
        Ok(1)
    } else {
        Ok(0)
    }
}

pub fn flag2(state: &State, flag: Flags2) -> Result<u8, RuntimeError> {
    let flags = field_word(state, HeaderField::Flags2)?;
    if flags & flag as u16 > 0 {
        Ok(1)
    } else {
        Ok(0)
    }
}

pub fn set_flag1(state: &mut State, flag: u8) -> Result<(), RuntimeError> {
    let flags = field_byte(state, HeaderField::Flags1)?;
    let new = flags | flag;
    debug!(target: "app::header", "Set FLAG1 {:08b}: {:08b} => {:08b}", flag, flags, new);
    state.write_byte(HeaderField::Flags1 as usize, new)
}

pub fn set_flag2(state: &mut State, flag: Flags2) -> Result<(), RuntimeError> {
    let f = format!("{:?}", flag);
    let flags = field_word(state, HeaderField::Flags2)?;
    let new = flags | flag as u16;
    debug!(target: "app::header", "Set FLAG2 {}: {:010b} => {:010b}", f, flags, new);
    state.memory.write_word(HeaderField::Flags2 as usize, new)
}

pub fn clear_flag1(state: &mut State, flag: u8) -> Result<(), RuntimeError> {
    let flags = field_byte(state, HeaderField::Flags1)?;
    let new = flags & !flag;
    debug!(target: "app::header", "Clear FLAG1 {:08b}: {:08b} => {:08b}", flag, flags, new);
    state.write_byte(HeaderField::Flags1 as usize, new)
}

pub fn clear_flag2(state: &mut State, flag: Flags2) -> Result<(), RuntimeError> {
    let f = format!("{:?}", flag);
    let flags = field_word(state, HeaderField::Flags2)?;
    let new = flags & !(flag as u16);
    debug!(target: "app::header", "Clear FLAG2 {}: {:010b} => {:010b}", f, flags, new);
    state.memory.write_word(HeaderField::Flags2 as usize, new)
}

pub fn set_extension(state: &mut State, index: usize, value: u16) -> Result<(), RuntimeError> {
    let extension_table_address = field_word(state, HeaderField::ExtensionTable)? as usize;
    if extension_table_address > 0 {
        debug!(target: "app::header", "Set extension table word {} to {:04x}", index, value);
        let table_size = state.read_word(extension_table_address)? as usize;
        if table_size >= index {
            state.write_word(extension_table_address + (index * 2), value)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        assert_ok_eq,
        test_util::{mock_state, test_map},
        zmachine::state::header::{self, Flags1v3, Flags1v4, Flags2, HeaderField},
    };

    #[test]
    fn test_fields() {
        let mut map = test_map(3);
        for (i, b) in (0..0x40).enumerate() {
            map[i] = b + 1;
        }
        map[0x0e] = 0x4;

        let state = mock_state(map);
        assert_ok_eq!(header::field_byte(&state, HeaderField::Version), 0x1);
        assert_ok_eq!(header::field_byte(&state, HeaderField::Flags1), 0x2);
        assert_ok_eq!(header::field_word(&state, HeaderField::Release), 0x304);
        assert_ok_eq!(header::field_word(&state, HeaderField::HighMark), 0x506);
        assert_ok_eq!(header::field_word(&state, HeaderField::InitialPC), 0x708);
        assert_ok_eq!(header::field_word(&state, HeaderField::Dictionary), 0x90a);
        assert_ok_eq!(header::field_word(&state, HeaderField::ObjectTable), 0xb0c);
        assert_ok_eq!(header::field_word(&state, HeaderField::GlobalTable), 0xd0e);
        assert_ok_eq!(header::field_word(&state, HeaderField::StaticMark), 0x410);
        assert_ok_eq!(header::field_word(&state, HeaderField::Flags2), 0x1112);
        assert_ok_eq!(header::field_word(&state, HeaderField::Serial), 0x1314);
        assert_ok_eq!(
            header::field_word(&state, HeaderField::AbbreviationsTable),
            0x191a
        );
        assert_ok_eq!(header::field_word(&state, HeaderField::FileLength), 0x1b1c);
        assert_ok_eq!(header::field_word(&state, HeaderField::Checksum), 0x1d1e);
        assert_ok_eq!(
            header::field_byte(&state, HeaderField::InterpreterNumber),
            0x1f
        );
        assert_ok_eq!(
            header::field_byte(&state, HeaderField::InterpreterVersion),
            0x20
        );
        assert_ok_eq!(header::field_byte(&state, HeaderField::ScreenLines), 0x21);
        assert_ok_eq!(header::field_byte(&state, HeaderField::ScreenColumns), 0x22);
        assert_ok_eq!(header::field_word(&state, HeaderField::ScreenWidth), 0x2324);
        assert_ok_eq!(
            header::field_word(&state, HeaderField::ScreenHeight),
            0x2526
        );
        assert_ok_eq!(header::field_byte(&state, HeaderField::FontWidth), 0x27);
        assert_ok_eq!(header::field_byte(&state, HeaderField::FontHeight), 0x28);
        assert_ok_eq!(
            header::field_word(&state, HeaderField::RoutinesOffset),
            0x292a
        );
        assert_ok_eq!(
            header::field_word(&state, HeaderField::StringsOffset),
            0x2b2c
        );
        assert_ok_eq!(
            header::field_byte(&state, HeaderField::DefaultBackground),
            0x2d
        );
        assert_ok_eq!(
            header::field_byte(&state, HeaderField::DefaultForeground),
            0x2e
        );
        assert_ok_eq!(
            header::field_word(&state, HeaderField::TerminatorTable),
            0x2f30
        );
        assert_ok_eq!(header::field_word(&state, HeaderField::Revision), 0x3334);
        assert_ok_eq!(
            header::field_word(&state, HeaderField::AlphabetTable),
            0x3536
        );
        assert_ok_eq!(
            header::field_word(&state, HeaderField::ExtensionTable),
            0x3738
        );
        assert_ok_eq!(
            header::field_word(&state, HeaderField::InformVersion),
            0x3d3e
        );
    }
    #[test]
    fn test_field_byte() {
        let mut map = test_map(3);
        map[0x1D] = 0xf0;
        map[0x1E] = 0x12;
        map[0x1F] = 0x34;
        let state = mock_state(map);
        assert_ok_eq!(
            header::field_byte(&state, HeaderField::InterpreterNumber),
            0x12
        );
    }

    #[test]
    fn test_field_word() {
        let mut map = test_map(3);
        map[0x0D] = 0x12;
        map[0x0E] = 0x04;
        map[0x0F] = 0x00;
        map[0x10] = 0x78;
        let state = mock_state(map);
        assert_ok_eq!(header::field_word(&state, HeaderField::StaticMark), 0x400);
    }

    #[test]
    fn test_set_byte() {
        let mut map = test_map(3);
        map[0x1D] = 0xf0;
        map[0x1E] = 0x12;
        map[0x1F] = 0x34;
        let mut state = mock_state(map);
        assert!(header::set_byte(&mut state, HeaderField::InterpreterNumber, 0xFF).is_ok());
        assert_ok_eq!(state.read_byte(0x1E), 0xFF);
    }

    #[test]
    fn test_set_word() {
        let mut map = test_map(3);
        map[0x0D] = 0x12;
        map[0x0E] = 0x04;
        map[0x0F] = 0x00;
        map[0x10] = 0x78;
        let mut state = mock_state(map);
        assert!(header::set_word(&mut state, HeaderField::StaticMark, 0x3456).is_ok());
        assert_ok_eq!(state.read_word(0x0E), 0x3456);
    }

    #[test]
    fn test_flag1_v3() {
        let map = test_map(3);
        let mut state = mock_state(map);

        assert_ok_eq!(
            header::flag1(&state, Flags1v3::ScreenSplitAvailable as u8),
            0
        );
        assert_ok_eq!(
            header::flag1(&state, Flags1v3::StatusLineNotAvailable as u8),
            0
        );
        assert_ok_eq!(header::flag1(&state, Flags1v3::StatusLineType as u8), 0);
        assert_ok_eq!(
            header::flag1(&state, Flags1v3::VariablePitchDefault as u8),
            0
        );
        assert!(header::set_flag1(&mut state, Flags1v3::ScreenSplitAvailable as u8).is_ok());
        assert_ok_eq!(
            header::flag1(&state, Flags1v3::ScreenSplitAvailable as u8),
            1
        );
        assert_ok_eq!(
            header::flag1(&state, Flags1v3::StatusLineNotAvailable as u8),
            0
        );
        assert_ok_eq!(header::flag1(&state, Flags1v3::StatusLineType as u8), 0);
        assert_ok_eq!(
            header::flag1(&state, Flags1v3::VariablePitchDefault as u8),
            0
        );
        assert!(header::set_flag1(&mut state, Flags1v3::StatusLineNotAvailable as u8).is_ok());
        assert_ok_eq!(
            header::flag1(&state, Flags1v3::ScreenSplitAvailable as u8),
            1
        );
        assert_ok_eq!(
            header::flag1(&state, Flags1v3::StatusLineNotAvailable as u8),
            1
        );
        assert_ok_eq!(header::flag1(&state, Flags1v3::StatusLineType as u8), 0);
        assert_ok_eq!(
            header::flag1(&state, Flags1v3::VariablePitchDefault as u8),
            0
        );
        assert!(header::set_flag1(&mut state, Flags1v3::StatusLineType as u8).is_ok());
        assert_ok_eq!(
            header::flag1(&state, Flags1v3::ScreenSplitAvailable as u8),
            1
        );
        assert_ok_eq!(
            header::flag1(&state, Flags1v3::StatusLineNotAvailable as u8),
            1
        );
        assert_ok_eq!(header::flag1(&state, Flags1v3::StatusLineType as u8), 1);
        assert_ok_eq!(
            header::flag1(&state, Flags1v3::VariablePitchDefault as u8),
            0
        );
        assert!(header::set_flag1(&mut state, Flags1v3::VariablePitchDefault as u8).is_ok());
        assert_ok_eq!(
            header::flag1(&state, Flags1v3::ScreenSplitAvailable as u8),
            1
        );
        assert_ok_eq!(
            header::flag1(&state, Flags1v3::StatusLineNotAvailable as u8),
            1
        );
        assert_ok_eq!(header::flag1(&state, Flags1v3::StatusLineType as u8), 1);
        assert_ok_eq!(
            header::flag1(&state, Flags1v3::VariablePitchDefault as u8),
            1
        );
        assert!(header::clear_flag1(&mut state, Flags1v3::StatusLineNotAvailable as u8).is_ok());
        assert_ok_eq!(
            header::flag1(&state, Flags1v3::ScreenSplitAvailable as u8),
            1
        );
        assert_ok_eq!(
            header::flag1(&state, Flags1v3::StatusLineNotAvailable as u8),
            0
        );
        assert_ok_eq!(header::flag1(&state, Flags1v3::StatusLineType as u8), 1);
        assert_ok_eq!(
            header::flag1(&state, Flags1v3::VariablePitchDefault as u8),
            1
        );
        assert!(header::clear_flag1(&mut state, Flags1v3::VariablePitchDefault as u8).is_ok());
        assert_ok_eq!(
            header::flag1(&state, Flags1v3::ScreenSplitAvailable as u8),
            1
        );
        assert_ok_eq!(
            header::flag1(&state, Flags1v3::StatusLineNotAvailable as u8),
            0
        );
        assert_ok_eq!(header::flag1(&state, Flags1v3::StatusLineType as u8), 1);
        assert_ok_eq!(
            header::flag1(&state, Flags1v3::VariablePitchDefault as u8),
            0
        );
        assert!(header::clear_flag1(&mut state, Flags1v3::StatusLineType as u8).is_ok());
        assert_ok_eq!(
            header::flag1(&state, Flags1v3::ScreenSplitAvailable as u8),
            1
        );
        assert_ok_eq!(
            header::flag1(&state, Flags1v3::StatusLineNotAvailable as u8),
            0
        );
        assert_ok_eq!(header::flag1(&state, Flags1v3::StatusLineType as u8), 0);
        assert_ok_eq!(
            header::flag1(&state, Flags1v3::VariablePitchDefault as u8),
            0
        );
        assert!(header::clear_flag1(&mut state, Flags1v3::ScreenSplitAvailable as u8).is_ok());
        assert_ok_eq!(
            header::flag1(&state, Flags1v3::ScreenSplitAvailable as u8),
            0
        );
        assert_ok_eq!(
            header::flag1(&state, Flags1v3::StatusLineNotAvailable as u8), 0
        );
        assert_ok_eq!(header::flag1(&state, Flags1v3::StatusLineType as u8), 0);
        assert_ok_eq!(
            header::flag1(&state, Flags1v3::VariablePitchDefault as u8),
            0
        );
    }

    #[test]
    fn test_flag1_v4() {
        let map = test_map(4);
        let mut state = mock_state(map);

        assert_ok_eq!(header::flag1(&state, Flags1v4::BoldfaceAvailable as u8), 0);
        assert_ok_eq!(header::flag1(&state, Flags1v4::ColoursAvailable as u8), 0);
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::FixedSpaceAvailable as u8),
            0
        );
        assert_ok_eq!(header::flag1(&state, Flags1v4::ItalicAvailable as u8), 0);
        assert_ok_eq!(header::flag1(&state, Flags1v4::PicturesAvailable as u8), 0);
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::SoundEffectsAvailable as u8),
            0
        );
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::TimedInputAvailable as u8),
            0
        );
        assert!(header::set_flag1(&mut state, Flags1v4::BoldfaceAvailable as u8).is_ok());
        assert_ok_eq!(header::flag1(&state, Flags1v4::BoldfaceAvailable as u8), 1);
        assert_ok_eq!(header::flag1(&state, Flags1v4::ColoursAvailable as u8), 0);
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::FixedSpaceAvailable as u8),
            0
        );
        assert_ok_eq!(header::flag1(&state, Flags1v4::ItalicAvailable as u8), 0);
        assert_ok_eq!(header::flag1(&state, Flags1v4::PicturesAvailable as u8), 0);
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::SoundEffectsAvailable as u8),
            0
        );
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::TimedInputAvailable as u8),
            0
        );
        assert!(header::set_flag1(&mut state, Flags1v4::ColoursAvailable as u8).is_ok());
        assert_ok_eq!(header::flag1(&state, Flags1v4::BoldfaceAvailable as u8), 1);
        assert_ok_eq!(header::flag1(&state, Flags1v4::ColoursAvailable as u8), 1);
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::FixedSpaceAvailable as u8),
            0
        );
        assert_ok_eq!(header::flag1(&state, Flags1v4::ItalicAvailable as u8), 0);
        assert_ok_eq!(header::flag1(&state, Flags1v4::PicturesAvailable as u8), 0);
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::SoundEffectsAvailable as u8),
            0
        );
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::TimedInputAvailable as u8),
            0
        );
        assert!(header::set_flag1(&mut state, Flags1v4::FixedSpaceAvailable as u8).is_ok());
        assert_ok_eq!(header::flag1(&state, Flags1v4::BoldfaceAvailable as u8), 1);
        assert_ok_eq!(header::flag1(&state, Flags1v4::ColoursAvailable as u8), 1);
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::FixedSpaceAvailable as u8),
            1
        );
        assert_ok_eq!(header::flag1(&state, Flags1v4::ItalicAvailable as u8), 0);
        assert_ok_eq!(header::flag1(&state, Flags1v4::PicturesAvailable as u8), 0);
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::SoundEffectsAvailable as u8),
            0
        );
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::TimedInputAvailable as u8),
            0
        );
        assert!(header::set_flag1(&mut state, Flags1v4::ItalicAvailable as u8).is_ok());
        assert_ok_eq!(header::flag1(&state, Flags1v4::BoldfaceAvailable as u8), 1);
        assert_ok_eq!(header::flag1(&state, Flags1v4::ColoursAvailable as u8), 1);
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::FixedSpaceAvailable as u8),
            1
        );
        assert_ok_eq!(header::flag1(&state, Flags1v4::ItalicAvailable as u8), 1);
        assert_ok_eq!(header::flag1(&state, Flags1v4::PicturesAvailable as u8), 0);
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::SoundEffectsAvailable as u8),
            0
        );
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::TimedInputAvailable as u8),
            0
        );
        assert!(header::set_flag1(&mut state, Flags1v4::PicturesAvailable as u8).is_ok());
        assert_ok_eq!(header::flag1(&state, Flags1v4::BoldfaceAvailable as u8), 1);
        assert_ok_eq!(header::flag1(&state, Flags1v4::ColoursAvailable as u8), 1);
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::FixedSpaceAvailable as u8),
            1
        );
        assert_ok_eq!(header::flag1(&state, Flags1v4::ItalicAvailable as u8), 1);
        assert_ok_eq!(header::flag1(&state, Flags1v4::PicturesAvailable as u8), 1);
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::SoundEffectsAvailable as u8),
            0
        );
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::TimedInputAvailable as u8),
            0
        );
        assert!(header::set_flag1(&mut state, Flags1v4::SoundEffectsAvailable as u8).is_ok());
        assert_ok_eq!(header::flag1(&state, Flags1v4::BoldfaceAvailable as u8), 1);
        assert_ok_eq!(header::flag1(&state, Flags1v4::ColoursAvailable as u8), 1);
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::FixedSpaceAvailable as u8),
            1
        );
        assert_ok_eq!(header::flag1(&state, Flags1v4::ItalicAvailable as u8), 1);
        assert_ok_eq!(header::flag1(&state, Flags1v4::PicturesAvailable as u8), 1);
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::SoundEffectsAvailable as u8),
            1
        );
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::TimedInputAvailable as u8),
            0
        );
        assert!(header::set_flag1(&mut state, Flags1v4::TimedInputAvailable as u8).is_ok());
        assert_ok_eq!(header::flag1(&state, Flags1v4::BoldfaceAvailable as u8), 1);
        assert_ok_eq!(header::flag1(&state, Flags1v4::ColoursAvailable as u8), 1);
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::FixedSpaceAvailable as u8),
            1
        );
        assert_ok_eq!(header::flag1(&state, Flags1v4::ItalicAvailable as u8), 1);
        assert_ok_eq!(header::flag1(&state, Flags1v4::PicturesAvailable as u8), 1);
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::SoundEffectsAvailable as u8),
            1
        );
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::TimedInputAvailable as u8),
            1
        );
        assert!(header::clear_flag1(&mut state, Flags1v4::ColoursAvailable as u8).is_ok());
        assert_ok_eq!(header::flag1(&state, Flags1v4::BoldfaceAvailable as u8), 1);
        assert_ok_eq!(header::flag1(&state, Flags1v4::ColoursAvailable as u8), 0);
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::FixedSpaceAvailable as u8),
            1
        );
        assert_ok_eq!(header::flag1(&state, Flags1v4::ItalicAvailable as u8), 1);
        assert_ok_eq!(header::flag1(&state, Flags1v4::PicturesAvailable as u8), 1);
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::SoundEffectsAvailable as u8),
            1
        );
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::TimedInputAvailable as u8),
            1
        );
        assert!(header::clear_flag1(&mut state, Flags1v4::ItalicAvailable as u8).is_ok());
        assert_ok_eq!(header::flag1(&state, Flags1v4::BoldfaceAvailable as u8), 1);
        assert_ok_eq!(header::flag1(&state, Flags1v4::ColoursAvailable as u8), 0);
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::FixedSpaceAvailable as u8),
            1
        );
        assert_ok_eq!(header::flag1(&state, Flags1v4::ItalicAvailable as u8), 0);
        assert_ok_eq!(header::flag1(&state, Flags1v4::PicturesAvailable as u8), 1);
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::SoundEffectsAvailable as u8),
            1
        );
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::TimedInputAvailable as u8),
            1
        );
        assert!(header::clear_flag1(&mut state, Flags1v4::SoundEffectsAvailable as u8).is_ok());
        assert_ok_eq!(header::flag1(&state, Flags1v4::BoldfaceAvailable as u8), 1);
        assert_ok_eq!(header::flag1(&state, Flags1v4::ColoursAvailable as u8), 0);
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::FixedSpaceAvailable as u8),
            1
        );
        assert_ok_eq!(header::flag1(&state, Flags1v4::ItalicAvailable as u8), 0);
        assert_ok_eq!(header::flag1(&state, Flags1v4::PicturesAvailable as u8), 1);
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::SoundEffectsAvailable as u8),
            0
        );
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::TimedInputAvailable as u8),
            1
        );
        assert!(header::clear_flag1(&mut state, Flags1v4::FixedSpaceAvailable as u8).is_ok());
        assert_ok_eq!(header::flag1(&state, Flags1v4::BoldfaceAvailable as u8), 1);
        assert_ok_eq!(header::flag1(&state, Flags1v4::ColoursAvailable as u8), 0);
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::FixedSpaceAvailable as u8),
            0
        );
        assert_ok_eq!(header::flag1(&state, Flags1v4::ItalicAvailable as u8), 0);
        assert_ok_eq!(header::flag1(&state, Flags1v4::PicturesAvailable as u8), 1);
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::SoundEffectsAvailable as u8),
            0
        );
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::TimedInputAvailable as u8),
            1
        );
        assert!(header::clear_flag1(&mut state, Flags1v4::PicturesAvailable as u8).is_ok());
        assert_ok_eq!(header::flag1(&state, Flags1v4::BoldfaceAvailable as u8), 1);
        assert_ok_eq!(header::flag1(&state, Flags1v4::ColoursAvailable as u8), 0);
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::FixedSpaceAvailable as u8),
            0
        );
        assert_ok_eq!(header::flag1(&state, Flags1v4::ItalicAvailable as u8), 0);
        assert_ok_eq!(header::flag1(&state, Flags1v4::PicturesAvailable as u8), 0);
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::SoundEffectsAvailable as u8),
            0
        );
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::TimedInputAvailable as u8),
            1
        );
        assert!(header::clear_flag1(&mut state, Flags1v4::TimedInputAvailable as u8).is_ok());
        assert_ok_eq!(header::flag1(&state, Flags1v4::BoldfaceAvailable as u8), 1);
        assert_ok_eq!(header::flag1(&state, Flags1v4::ColoursAvailable as u8), 0);
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::FixedSpaceAvailable as u8),
            0
        );
        assert_ok_eq!(header::flag1(&state, Flags1v4::ItalicAvailable as u8), 0);
        assert_ok_eq!(header::flag1(&state, Flags1v4::PicturesAvailable as u8), 0);
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::SoundEffectsAvailable as u8),
            0
        );
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::TimedInputAvailable as u8),
            0
        );
        assert!(header::clear_flag1(&mut state, Flags1v4::BoldfaceAvailable as u8).is_ok());
        assert_ok_eq!(header::flag1(&state, Flags1v4::BoldfaceAvailable as u8), 0);
        assert_ok_eq!(header::flag1(&state, Flags1v4::ColoursAvailable as u8), 0);
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::FixedSpaceAvailable as u8),
            0
        );
        assert_ok_eq!(header::flag1(&state, Flags1v4::ItalicAvailable as u8), 0);
        assert_ok_eq!(header::flag1(&state, Flags1v4::PicturesAvailable as u8), 0);
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::SoundEffectsAvailable as u8),
            0
        );
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::TimedInputAvailable as u8),
            0
        );
    }

    #[test]
    fn test_flag2() {
        let map = test_map(4);
        let mut state = mock_state(map);

        assert_ok_eq!(header::flag2(&state, Flags2::ForceFixedPitch), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestColours), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestMouse), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestPictures), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestSoundEffects), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestUndo), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::Transcripting), 0);
        assert!(header::set_flag2(&mut state, Flags2::ForceFixedPitch).is_ok());
        assert_ok_eq!(header::flag2(&state, Flags2::ForceFixedPitch), 1);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestColours), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestMouse), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestPictures), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestSoundEffects), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestUndo), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::Transcripting), 0);
        assert!(header::set_flag2(&mut state, Flags2::RequestColours).is_ok());
        assert_ok_eq!(header::flag2(&state, Flags2::ForceFixedPitch), 1);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestColours), 1);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestMouse), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestPictures), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestSoundEffects), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestUndo), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::Transcripting), 0);
        assert!(header::set_flag2(&mut state, Flags2::RequestMouse).is_ok());
        assert_ok_eq!(header::flag2(&state, Flags2::ForceFixedPitch), 1);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestColours), 1);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestMouse), 1);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestPictures), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestSoundEffects), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestUndo), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::Transcripting), 0);
        assert!(header::set_flag2(&mut state, Flags2::RequestPictures).is_ok());
        assert_ok_eq!(header::flag2(&state, Flags2::ForceFixedPitch), 1);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestColours), 1);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestMouse), 1);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestPictures), 1);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestSoundEffects), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestUndo), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::Transcripting), 0);
        assert!(header::set_flag2(&mut state, Flags2::RequestSoundEffects).is_ok());
        assert_ok_eq!(header::flag2(&state, Flags2::ForceFixedPitch), 1);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestColours), 1);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestMouse), 1);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestPictures), 1);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestSoundEffects), 1);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestUndo), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::Transcripting), 0);
        assert!(header::set_flag2(&mut state, Flags2::RequestUndo).is_ok());
        assert_ok_eq!(header::flag2(&state, Flags2::ForceFixedPitch), 1);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestColours), 1);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestMouse), 1);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestPictures), 1);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestSoundEffects), 1);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestUndo), 1);
        assert_ok_eq!(header::flag2(&state, Flags2::Transcripting), 0);
        assert!(header::set_flag2(&mut state, Flags2::Transcripting).is_ok());
        assert_ok_eq!(header::flag2(&state, Flags2::ForceFixedPitch), 1);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestColours), 1);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestMouse), 1);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestPictures), 1);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestSoundEffects), 1);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestUndo), 1);
        assert_ok_eq!(header::flag2(&state, Flags2::Transcripting), 1);
        assert!(header::clear_flag2(&mut state, Flags2::RequestUndo).is_ok());
        assert_ok_eq!(header::flag2(&state, Flags2::ForceFixedPitch), 1);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestColours), 1);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestMouse), 1);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestPictures), 1);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestSoundEffects), 1);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestUndo), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::Transcripting), 1);
        assert!(header::clear_flag2(&mut state, Flags2::RequestColours).is_ok());
        assert_ok_eq!(header::flag2(&state, Flags2::ForceFixedPitch), 1);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestColours), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestMouse), 1);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestPictures), 1);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestSoundEffects), 1);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestUndo), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::Transcripting), 1);
        assert!(header::clear_flag2(&mut state, Flags2::RequestSoundEffects).is_ok());
        assert_ok_eq!(header::flag2(&state, Flags2::ForceFixedPitch), 1);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestColours), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestMouse), 1);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestPictures), 1);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestSoundEffects), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestUndo), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::Transcripting), 1);
        assert!(header::clear_flag2(&mut state, Flags2::RequestMouse).is_ok());
        assert_ok_eq!(header::flag2(&state, Flags2::ForceFixedPitch), 1);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestColours), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestMouse), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestPictures), 1);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestSoundEffects), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestUndo), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::Transcripting), 1);
        assert!(header::clear_flag2(&mut state, Flags2::RequestPictures).is_ok());
        assert_ok_eq!(header::flag2(&state, Flags2::ForceFixedPitch), 1);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestColours), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestMouse), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestPictures), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestSoundEffects), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestUndo), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::Transcripting), 1);
        assert!(header::clear_flag2(&mut state, Flags2::ForceFixedPitch).is_ok());
        assert_ok_eq!(header::flag2(&state, Flags2::ForceFixedPitch), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestColours), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestMouse), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestPictures), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestSoundEffects), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestUndo), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::Transcripting), 1);
        assert!(header::clear_flag2(&mut state, Flags2::Transcripting).is_ok());
        assert_ok_eq!(header::flag2(&state, Flags2::ForceFixedPitch), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestColours), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestMouse), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestPictures), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestSoundEffects), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestUndo), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::Transcripting), 0);
    }
}
