use std::fmt;

#[derive(Debug, PartialEq)]
/// Status line type (V1 - V3)
pub enum StatusLineType {
    ScoreTurns,
    HoursMinutes,
}

/// Flag names
pub enum Flags {
    StatusLineType,
    StatusLineNotAvailable,
    ScreenSplittingAvailable,
    VariablePitchDefaultFont,
    TandyBit,
    Transcripting,
    ForceFixedPitch,
    UseMenus,
}

impl StatusLineType {
    /// Returns the status line type from the `flags1` header byte
    ///
    /// # Arguments
    ///
    /// * `b` - The flags1 byte value from the header
    pub fn from_byte(b: u8) -> Self {
        if b & 2 == 2 {
            StatusLineType::HoursMinutes
        } else {
            StatusLineType::ScoreTurns
        }
    }
}

/// Flag values from the `flags1` header field
pub struct Flags1 {
    status_line_type: u8,
    status_line_not_available: u8,
    screen_splitting_available: u8,
    variable_pitch_default_font: u8,
    tandy: u8,
}

impl fmt::Display for Flags1 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "\tStatus line type: {:?}", self.status_line_type)?;
        writeln!(
            f,
            "\tStatus line not available?: {}",
            self.status_line_not_available
        )?;
        writeln!(
            f,
            "\tScreen splitting available?: {}",
            self.screen_splitting_available
        )?;
        writeln!(
            f,
            "\tVariable pitch default font?: {}",
            self.variable_pitch_default_font
        )?;
        write!(f, "\tTandy bit: {}", self.tandy)
    }
}

impl Flags1 {
    /// Create a Flags1 structure from the `flags1` header field.  Flags that should
    /// be set by the interpreter are false by default.
    ///
    /// # Arguments
    ///
    /// * `b` - The `flags1` header byte value
    pub fn from_byte(b: u8) -> Self {
        Self {
            status_line_type: b & 0x1,
            status_line_not_available: 0,
            screen_splitting_available: 0,
            variable_pitch_default_font: 0,
            tandy: 0,
        }
    }

    /// Return the current value of a flag.
    ///
    /// # Arguments
    ///
    /// * `f` - `Flags` enum value of the flag to read.
    pub fn flag(&self, f: Flags) -> u8 {
        match f {
            Flags::StatusLineType => self.status_line_type,
            Flags::TandyBit => self.tandy,
            Flags::StatusLineNotAvailable => self.status_line_not_available,
            Flags::ScreenSplittingAvailable => self.screen_splitting_available,
            Flags::VariablePitchDefaultFont => self.variable_pitch_default_font,
            /* TODO: Error */
            _ => 0,
        }
    }

    /// Set a flag to true.  This method updates the appropriate flag value in the memory map.
    ///
    /// # Arguments
    ///
    /// * `v` - memory map vector.
    /// * `f` - `Flags` enum value to set
    pub fn set_flag(&mut self, v: &mut Vec<u8>, f: Flags) {
        let mut b = v[1];
        match f {
            Flags::TandyBit => {
                self.tandy = 1;
                b = b | 0x8
            }
            Flags::StatusLineNotAvailable => {
                self.status_line_not_available = 1;
                b = b | 0x10
            }
            Flags::ScreenSplittingAvailable => {
                self.screen_splitting_available = 1;
                b = b | 0x20
            }
            Flags::VariablePitchDefaultFont => {
                self.variable_pitch_default_font = 1;
                b = b | 0x40
            }
            /* TODO: Error */
            _ => {}
        }
        v[1] = b;
    }

    /// Set a flag to false.  This method updates the appropriate flag value in the memory map.
    ///
    /// # Arguments
    ///
    /// * `v` - memory map vector.
    /// * `f` - `Flags` enum value to clear
    pub fn clear_flag(&mut self, v: &mut Vec<u8>, f: Flags) {
        let mut b = v[1];
        match f {
            Flags::TandyBit => {
                self.tandy = 0;
                b = b | 0xF7
            }
            Flags::StatusLineNotAvailable => {
                self.status_line_not_available = 0;
                b = b | 0xEF
            }
            Flags::ScreenSplittingAvailable => {
                self.screen_splitting_available = 0;
                b = b | 0xDF
            }
            Flags::VariablePitchDefaultFont => {
                self.variable_pitch_default_font = 0;
                b = b | 0xBF
            }
            _ => {}
        }
        v[1] = b;
    }
}

/// Flag values for the `flags2` header field
pub struct Flags2 {
    transcripting: u8,
    force_fixed_pitch: u8,
    use_menus: u8
}

impl fmt::Display for Flags2 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "\tTranscipting on?: {}", self.transcripting)?;
        writeln!(f, "\tForce fixed-pitch?: {}", self.force_fixed_pitch)?;
        write!(f, "\tGame wants menus?: {}", self.use_menus)
    }
}

impl Flags2 {
    /// Create a Flags2 structure from the `flags2` header field.  Flags that should
    /// be set by the interpreter are false by default.
    ///
    /// # Arguments
    ///
    /// * `w` - The `flags2` header word value    
    pub fn from_word(_b: u16) -> Self {
        Self {
            transcripting: 0,
            force_fixed_pitch: 0,
            use_menus: 0
        }
    }

    /// Return the current value of a flag.
    ///
    /// # Arguments
    ///
    /// * `f` - `Flags` enum value of the flag to read.
    pub fn flag(&self, f: Flags) -> u8 {
        match f {
            Flags::Transcripting => self.transcripting,
            Flags::ForceFixedPitch => self.force_fixed_pitch,
            Flags::UseMenus => self.use_menus,
            /* TODO: Error */
            _ => 0,
        }
    }

    /// Set a flag to true.  This method updates the appropriate flag value in the memory map.
    ///
    /// # Arguments
    ///
    /// * `v` - memory map vector.
    /// * `f` - `Flags` enum value to set
    pub fn set_flag(&mut self, v: &mut Vec<u8>, f: Flags) {
        let mut hb = v[16];
        let mut lb: u8 = v[17];
        match f {
            Flags::Transcripting => {
                self.transcripting = 1;
                lb = lb | 0x1
            }
            Flags::ForceFixedPitch => {
                self.force_fixed_pitch = 1;
                lb = lb | 0x2
            },
            Flags::UseMenus => {
                self.use_menus = 1;
                hb = hb | 0x1
            }
            /* TODO: Error */
            _ => {}
        }
        v[16] = hb;
        v[17] = lb;
    }

    /// Set a flag to false.  This method updates the appropriate flag value in the memory map.
    ///
    /// # Arguments
    ///
    /// * `v` - memory map vector.
    /// * `f` - `Flags` enum value to clear
    pub fn clear_flag(&mut self, v: &mut Vec<u8>, f: Flags) {
        let mut hb = v[16];
        let mut lb: u8 = v[17];
        match f {
            Flags::Transcripting => {
                self.transcripting = 0;
                lb = lb | 0xFE
            }
            Flags::StatusLineNotAvailable => {
                self.force_fixed_pitch = 0;
                lb = lb | 0xFD
            },
            Flags::UseMenus => {
                self.use_menus = 0;
                hb = hb | 0xFE
            }
            /* TODO: Error */
            _ => {}
        }
        v[16] = hb;
        v[17] = lb;
    }
}
