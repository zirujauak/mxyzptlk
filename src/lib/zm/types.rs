use core::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StoreResult {
    address: usize,
    variable: u8,
}

impl fmt::Display for StoreResult {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.variable == 0 {
            write!(f, "-(SP)")
        } else if self.variable < 16 {
            write!(f, "L{:02x}", self.variable - 1)
        } else {
            write!(f, "G{:02x}", self.variable - 16)
        }
    }
}

impl StoreResult {
    pub fn new(address: usize, variable: u8) -> StoreResult {
        StoreResult { address, variable }
    }

    pub fn address(&self) -> usize {
        self.address
    }

    pub fn variable(&self) -> u8 {
        self.variable
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct InputEvent {
    zchar: Option<u16>,
    row: Option<u16>,
    column: Option<u16>,
}

impl InputEvent {
    pub fn zchar(&self) -> Option<u16> {
        self.zchar
    }

    pub fn row(&self) -> Option<u16> {
        self.row
    }

    pub fn column(&self) -> Option<u16> {
        self.column
    }
}

#[derive(Default)]
pub struct InstructionResult {
    directive: Option<Directive>,
    request: Option<DirectiveRequest>,
    next_instruction: usize,
}

impl InstructionResult {
    pub fn new(
        directive: Directive,
        request: DirectiveRequest,
        next_instruction: usize,
    ) -> InstructionResult {
        InstructionResult {
            directive: Some(directive),
            request: Some(request),
            next_instruction,
        }
    }

    pub fn empty(directive: Directive, next_instruction: usize) -> InstructionResult {
        InstructionResult {
            directive: Some(directive),
            next_instruction,
            ..Default::default()
        }
    }

    pub fn none(next_instruction: usize) -> InstructionResult {
        InstructionResult {
            next_instruction,
            ..Default::default()
        }
    }

    pub fn message(message: String, next_instruction: usize) -> InstructionResult {
        InstructionResult {
            directive: Some(Directive::Message),
            request: Some(DirectiveRequest::message(message)),
            next_instruction,
        }
    }
}
pub enum Directive {
    BufferMode,
    EraseLine,
    EraseWindow,
    GetCursor,
    Message,
    NewLine,
    Read,
    ReadChar,
    Print,
    PrintRet,
    PrintTable,
    Quit,
    Restart,
    Restore,
    SetColour,
    SetCursor,
    SetFont,
    SetTextStyle,
    SetWindow,
    ShowStatus,
    SoundEffect,
    SplitWindow,
}

#[derive(Default)]
pub struct DirectiveRequest {
    // Message
    message: String,

    // BUFFER_MODE
    buffer_mode: u16,

    // ERASE_WINDOW
    erase_window: i16,

    // PRINT, PRINT_RET
    text: Vec<u16>,

    // PRINT_TABLE
    table: Vec<u16>,
    width: u16,
    height: u16,
    skip: u16,

    // READ
    length: u8,
    terminators: Vec<u16>,
    preload: Vec<u16>,

    // READ, READ_CHAR
    timeout: u16,

    // SET_COLOUR
    foreground: u16,
    background: u16,

    // SET_CURSOR
    row: u16,
    column: u16,

    // SET_FONT
    font: u16,

    // SET_TEXT_STYLE
    style: u16,

    // SET_WINDOW
    set_window: u16,

    // SHOW_STATUS
    left: Vec<u16>,
    right: Vec<u16>,

    // SOUND_EFFECT
    number: u16,
    effect: u16,
    volume: u8,
    repeats: u8,

    // SPLIT_WINDOW
    split: u16,
}

impl DirectiveRequest {
    pub fn buffer_mode(mode: u16) -> DirectiveRequest {
        DirectiveRequest {
            buffer_mode: mode,
            ..Default::default()
        }
    }

    pub fn erase_window(window: i16) -> DirectiveRequest {
        DirectiveRequest {
            erase_window: window,
            ..Default::default()
        }
    }

    pub fn message(message: String) -> DirectiveRequest {
        DirectiveRequest {
            message,
            ..Default::default()
        }
    }

    pub fn print(text: &[u16]) -> DirectiveRequest {
        DirectiveRequest {
            text: text.to_vec(),
            ..Default::default()
        }
    }

    pub fn print_table(table: &[u16], width: u16, height: u16, skip: u16) -> DirectiveRequest {
        DirectiveRequest {
            table: table.to_vec(),
            width,
            height, 
            skip,
            ..Default::default()
        }
    }

    pub fn read(length: u8, terminators: &[u16], timeout: u16, preload: &[u16]) -> DirectiveRequest {
        DirectiveRequest {
            length,
            terminators: terminators.to_vec(),
            timeout,
            preload: preload.to_vec(),
            ..Default::default()
        }
    }

    pub fn read_char(timeout: u16) -> DirectiveRequest {
        DirectiveRequest {
            timeout,
            ..Default::default()
        }
    }

    pub fn set_colour(foreground: u16, background: u16) -> DirectiveRequest {
        DirectiveRequest {
            foreground,
            background,
            ..Default::default()
        }
    }

    pub fn set_cursor(row: u16, column: u16) -> DirectiveRequest {
        DirectiveRequest {
            row,
            column,
            ..Default::default()
        }
    }

    pub fn set_font(font: u16) -> DirectiveRequest {
        DirectiveRequest {
            font,
            ..Default::default()
        }
    }

    pub fn set_text_style(style: u16) -> DirectiveRequest {
        DirectiveRequest {
            style,
            ..Default::default()
        }
    }

    pub fn set_window(window: u16) -> DirectiveRequest {
        DirectiveRequest {
            set_window: window,
            ..Default::default()
        }
    }

    pub fn show_status(left: &[u16], right: &[u16]) -> DirectiveRequest {
        DirectiveRequest {
            left: left.to_vec(),
            right: right.to_vec(),
            ..Default::default()
        }
    }

    pub fn sound_effect(number: u16, effect: u16, volume: u8, repeats: u8) -> DirectiveRequest {
        DirectiveRequest {
            number,
            effect,
            volume,
            repeats,
            ..Default::default()
        }
    }
    
    pub fn split_window(split: u16) -> DirectiveRequest {
        DirectiveRequest {
            split,
            ..Default::default()
        }
    }
}
