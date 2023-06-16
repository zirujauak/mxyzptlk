use crate::state::memory::Memory;
use crate::error::*;

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
    ColoursAvailable = 0x01,    // bit 0
    BoldfaceAvailable = 0x04,   // bit 2
    ItalicAvailable = 0x08,     // bit 3
    FixedSpaceAvailable = 0x10, // bit 4
    //SoundEffectsAvailable = 0x20,   // bit 5
    TimedInputAvailable = 0x80, // bit 7
}

pub enum Flags2 {
    Transcripting = 0x0001,       // bit 0
    ForceFixedPitch = 0x0002,     // bit 1
    RequestPictures = 0x0008,     // bit 3
    RequestUndo = 0x0010,         // bit 4
    RequestMouse = 0x0020,        // bit 5
    RequestColours = 0x0040,      // bit 6
    RequestSoundEffects = 0x0080, // bit 7
}

pub fn field_byte(memory: &Memory, field: HeaderField) -> Result<u8, RuntimeError> {
    memory.read_byte(field as usize)
}

pub fn field_word(memory: &Memory, field: HeaderField) -> Result<u16, RuntimeError> {
    memory.read_word(field as usize)
}

pub fn set_byte(
    memory: &mut Memory,
    field: HeaderField,
    value: u8,
) -> Result<(), RuntimeError> {
    memory.write_byte(field as usize, value)
}

pub fn set_word(
    memory: &mut Memory,
    field: HeaderField,
    value: u16,
) -> Result<(), RuntimeError> {
    memory.write_word(field as usize, value)
}

pub fn flag1(memory: &Memory, flag: u8) -> Result<u8, RuntimeError> {
    let flags = field_byte(memory, HeaderField::Flags1)?;
    if flags & flag as u8 > 0 {
        Ok(1)
    } else {
        Ok(0)
    }
}

pub fn flag2(memory: &Memory, flag: Flags2) -> Result<u8, RuntimeError> {
    let flags = field_word(memory, HeaderField::Flags2)?;
    if flags & flag as u16 > 0 {
        Ok(1)
    } else {
        Ok(0)
    }
}

pub fn set_flag1(memory: &mut Memory, flag: u8) -> Result<(), RuntimeError> {
    let mut flags = field_byte(memory, HeaderField::Flags1)?;
    flags = flags | flag;
    memory.write_byte(HeaderField::Flags1 as usize, flags)
}

pub fn set_flag2(memory: &mut Memory, flag: Flags2) -> Result<(), RuntimeError> {
    let mut flags = field_word(memory, HeaderField::Flags2)?;
    flags = flags | flag as u16;
    memory.write_word(HeaderField::Flags2 as usize, flags)
}

pub fn clear_flag1(memory: &mut Memory, flag: u8) -> Result<(), RuntimeError> {
    let mut flags = field_byte(memory, HeaderField::Flags1)?;
    flags = flags & !flag;
    memory.write_byte(HeaderField::Flags1 as usize, flags)
}

pub fn clear_flag2(memory: &mut Memory, flag: Flags2) -> Result<(), RuntimeError> {
    let mut flags = field_word(memory, HeaderField::Flags2)?;
    flags = flags & !(flag as u16);
    memory.write_word(HeaderField::Flags2 as usize, flags)
}
