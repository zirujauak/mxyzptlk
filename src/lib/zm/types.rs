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
pub enum Interrupt {
    ReadTimeout,
    Sound,
}

#[derive(Debug, Default, Eq, PartialEq)]
pub struct InputEvent {
    zchar: Option<u16>,
    row: Option<u16>,
    column: Option<u16>,
    interrupt: Option<Interrupt>
}

impl InputEvent {
    pub fn no_input() -> InputEvent {
        InputEvent::default()
    }

    pub fn from_char(zchar: u16) -> InputEvent {
        InputEvent {
            zchar: Some(zchar),
            ..Default::default()
        }
    }

    pub fn from_mouse(zchar: u16, row: u16, column: u16) -> InputEvent {
        InputEvent {
            zchar: Some(zchar),
            row: Some(row),
            column: Some(column),
            ..Default::default()
        }
    }

    pub fn from_interrupt(interrupt: Interrupt) -> InputEvent {
        InputEvent {
            interrupt: Some(interrupt),
            ..Default::default()
        }
    }

    pub fn zchar(&self) -> Option<u16> {
        self.zchar
    }

    pub fn interrupt(&self) -> Option<&Interrupt> {
        self.interrupt.as_ref()
    }

    pub fn row(&self) -> Option<u16> {
        self.row
    }

    pub fn column(&self) -> Option<u16> {
        self.column
    }
}

#[derive(Debug, Default)]
pub struct InstructionResult {
    directive: Option<Directive>,
    request: DirectiveRequest,
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
            request,
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
            request: DirectiveRequest::message(message),
            next_instruction,
        }
    }

    pub fn directive(&self) -> Option<&Directive> {
        self.directive.as_ref()
    }

    pub fn request(&self) -> &DirectiveRequest {
        &self.request
    }

    pub fn next_instruction(&self) -> usize {
        self.next_instruction
    }
}

#[derive(Debug, PartialEq)]
pub enum Directive {
    BufferMode,
    EraseLine,
    EraseWindow,
    GetCursor,
    Message,
    NewLine,
    Read,
    ReadChar,
    ReadCharInterruptReturn,
    ReadInterruptReturn,
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

#[derive(Debug, Default)]
pub struct DirectiveRequest {
    // Message
    message: String,

    // BUFFER_MODE
    mode: u16,

    // ERASE_WINDOW
    window_erase: i16,

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
    redraw_input: bool,

    // READ, READ_CHAR
    timeout: u16,
    read_instruction: usize,
    read_next_instruction: usize,
    read_int_routine: usize,
    read_int_result: u16,
    

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
    window_set: u16,

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
            mode,
            ..Default::default()
        }
    }

    pub fn erase_window(window: i16) -> DirectiveRequest {
        DirectiveRequest {
            window_erase: window,
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

    pub fn read(length: u8, terminators: &[u16], timeout: u16, read_int_routine: usize, read_instruction: usize, read_next_instruction: usize, preload: &[u16]) -> DirectiveRequest {
        DirectiveRequest {
            length,
            terminators: terminators.to_vec(),
            timeout,
            read_instruction,
            read_next_instruction,
            read_int_routine,
            preload: preload.to_vec(),
            ..Default::default()
        }
    }

    pub fn read_char(timeout: u16, read_instruction: usize) -> DirectiveRequest {
        DirectiveRequest {
            timeout,
            read_instruction,
            ..Default::default()
        }
    }

    pub fn read_interrupt_return(result: u16, redraw_input: bool) -> DirectiveRequest {
        DirectiveRequest {
            read_int_result: result,
            redraw_input,
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
            window_set: window,
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

    pub fn mode(&self) -> u16 {
        self.mode
    }
    pub fn text(&self) -> &Vec<u16> {
        &self.text
    }

    pub fn length(&self) -> u8 {
        self.length
    }

    pub fn preload(&self) -> &[u16] {
        &self.preload
    }

    pub fn terminators(&self) -> &[u16] {
        &self.terminators
    }

    pub fn timeout(&self) -> u16 {
        self.timeout
    }

    pub fn redraw_input(&self) -> bool {
        self.redraw_input
    }

    pub fn read_instruction(&self) -> usize {
        self.read_instruction
    }

    pub fn read_next_instruction(&self) -> usize {
        self.read_next_instruction
    }

    pub fn read_int_routine(&self) -> usize {
        self.read_int_routine
    }

    pub fn read_int_result(&self) -> u16 {
        self.read_int_result
    }

    pub fn window_erase(&self) -> i16 {
        self.window_erase
    }

    pub fn style(&self) -> u16 {
        self.style
    }

    pub fn split(&self) -> u16 {
        self.split
    }

    pub fn window_set(&self) -> u16 {
        self.window_set
    }

    pub fn row(&self) -> u16 {
        self.row
    }

    pub fn column(&self) -> u16 {
        self.column
    }

    pub fn foreground(&self) -> u16 {
        self.foreground
    }

    pub fn background(&self) -> u16 {
        self.background
    }

    pub fn font(&self) -> u16 {
        self.font
    }
}
