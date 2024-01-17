//! Infocom [Zmachine](https://inform-fiction.org/zmachine/standards/z1point1/index.html) implementation
use std::collections::VecDeque;

use crate::{
    config::Config,
    error::{ErrorCode, RuntimeError},
    fatal_error,
    instruction::{
        decoder::{self, decode_instruction},
        processor::{self, operand_values, processor_var},
        InstructionResult, NextAddress, StoreResult,
    },
    object::property,
    quetzal::{IFhd, Mem, Quetzal, Stk, Stks},
    recoverable_error, text,
};

use self::{
    frame::Frame,
    header::{Flags1v3, Flags1v4, Flags2, HeaderField},
    memory::Memory,
    rng::{chacha_rng::ChaChaRng, ZRng},
};

mod frame;
pub mod header;
mod memory;
mod rng;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Error handling behavior for recoverable errors
pub enum ErrorHandling {
    /// Warn every time the error occurs and continue running
    ContinueWarnAlways,
    /// Warn once per error type and continue running
    ContinueWarnOnce,
    /// Ignore all recoverable errors
    Ignore,
    /// Treat recoverable errors as fatal errors
    Abort,
}

#[derive(Debug)]
/// Stream 3 memory table
struct Stream3 {
    /// Table address to write to when the strema is closed
    address: usize,
    /// Stream buffer
    buffer: Vec<u16>,
}

impl Stream3 {
    /// Constructor
    ///
    /// # Arguments
    /// * `address` - Address of table to write buffered output to when the stream is closed
    pub fn new(address: usize) -> Stream3 {
        Stream3 {
            address,
            buffer: Vec::new(),
        }
    }

    /// Push a value to the stream buffer
    ///
    /// # Arguments
    /// * `c` - Character value to push
    pub fn push(&mut self, c: u16) {
        self.buffer.push(c);
    }
}

#[derive(Debug, Default, Eq, PartialEq)]
/// Interrupt type
pub enum Interrupt {
    #[default]
    /// READ or READ_CHAR interrupt
    ReadTimeout,
    /// SOUND_EFFECT end-of-playback interrupt
    Sound,
}

#[derive(Debug, Default, Eq, PartialEq)]
/// Interpreter input event
pub struct InputEvent {
    /// Z character from keyboard input
    zchar: Option<u16>,
    /// Row for mouse-click input
    row: Option<u16>,
    /// Column for mouse-click input
    column: Option<u16>,
    /// Interrupt type if input was interrupted
    interrupt: Option<Interrupt>,
}

impl InputEvent {
    /// Constructor for a no-input event
    pub fn no_input() -> InputEvent {
        InputEvent::default()
    }

    /// Constructor for a keypress input event
    ///
    /// # Arguments
    /// * `zchar` - Z character value for the key that was pressed
    pub fn from_char(zchar: u16) -> InputEvent {
        InputEvent {
            zchar: Some(zchar),
            ..Default::default()
        }
    }

    /// Constructor for a mouse-click input event
    ///
    /// # Arguments
    /// * `zchar` - Mouse click character - 253 = double-click, 254 = single-click
    /// * `row` - row position of the mouse pointer where the click occured
    /// * `column` - column position of the mouse pointer where the click occured
    pub fn from_mouse(zchar: u16, row: u16, column: u16) -> InputEvent {
        InputEvent {
            zchar: Some(zchar),
            row: Some(row),
            column: Some(column),
            ..Default::default()
        }
    }

    /// Constructor for an interrupted input event
    ///
    /// # Arguments
    /// * `interrupt` - [Interrupt] type
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

    // pub fn row(&self) -> Option<u16> {
    //     self.row
    // }

    // pub fn column(&self) -> Option<u16> {
    //     self.column
    // }
}

#[derive(Clone, Debug)]
/// Interpreter request type
pub enum RequestType {
    BufferMode,
    EraseLine,
    EraseWindow,
    GetCursor,
    InputStream,
    Message,
    NewLine,
    OutputStream,
    Print,
    PrintRet,
    PrintTable,
    Quit,
    Read,
    ReadAbort,
    ReadRedraw,
    ReadChar,
    Restart,
    Restore,
    RestoreComplete,
    Save,
    SetColour,
    SetCursor,
    SetFont,
    SetTextStyle,
    SetWindow,
    ShowStatus,
    SoundEffect,
    SplitWindow,
}

#[derive(Clone, Debug, Default)]
/// Interpreter request payload
pub struct RequestPayload {
    // Messaging
    message: String,

    // BufferMode
    buffer_mode: u16,

    // EraseWindow
    window_erase: i16,

    // InputStream/OutputStream
    stream: i16,

    // Print//PrintRet
    text: Vec<u16>,
    transcript: bool,

    // PrintTable
    table: Vec<u16>,
    width: u16,
    height: u16,
    skip: u16,

    // Read
    length: u8,
    terminators: Vec<u16>,
    input: Vec<u16>,

    // Read/ReadChar
    timeout: u16,

    // Restore/Save
    name: String,

    // Save
    save_data: Vec<u8>,

    // SetColour
    foreground: u16,
    background: u16,

    // SetCursor
    row: u16,
    column: u16,

    // SetFont
    font: u16,

    // SetTextStyle
    style: u16,

    // SetWindow
    window_set: u16,

    // ShowStatus
    status_left: Vec<u16>,
    status_right: Vec<u16>,

    // SoundEffect
    number: u16,
    effect: u16,
    volume: u8,
    repeats: u8,
    routine: usize,

    // SplitWindow
    split_lines: u16,
}

impl RequestPayload {
    /// Get the message to display
    ///
    /// # Returns
    /// Message string
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Get the window to erase
    ///
    /// # Returns
    /// Window to erase
    pub fn window_erase(&self) -> i16 {
        self.window_erase
    }

    /// Get the buffer mode setting
    ///
    /// # Return
    /// Buffer mode setting
    pub fn buffer_mode(&self) -> u16 {
        self.buffer_mode
    }

    /// Get the text to print
    ///
    /// # Returns
    /// Text to print
    pub fn text(&self) -> &Vec<u16> {
        &self.text
    }

    /// Get the transcript flag
    ///
    /// # Returns
    /// Transcript flag
    pub fn transcript(&self) -> bool {
        self.transcript
    }

    /// Get the table data to print
    ///
    /// # Returns
    /// Table of text to print
    pub fn table(&self) -> &Vec<u16> {
        &self.table
    }

    /// Get the table width
    ///
    /// # Returns
    /// Table width in bytes
    pub fn width(&self) -> u16 {
        self.width
    }

    /// Get the table height
    ///
    /// # Returns
    /// Table height in lines
    pub fn height(&self) -> u16 {
        self.height
    }

    /// Get the number of characters to skip between table lines
    ///
    /// # Returns
    /// Characters to skip between lines
    pub fn skip(&self) -> u16 {
        self.skip
    }

    /// Get the READ maximum input length
    ///
    /// # Returns
    /// Maximum characters to READ, including the terminator
    pub fn length(&self) -> u8 {
        self.length
    }

    /// Get the READ input terminators
    ///
    /// # Returns
    /// Vector of input terminator characters
    pub fn terminators(&self) -> &Vec<u16> {
        &self.terminators
    }

    /// Get the READ/READ_CHAR timeout
    ///
    /// # Returns
    /// Read timeout, 0 for none
    pub fn timeout(&self) -> u16 {
        self.timeout
    }

    /// Get the existing input for a READ
    ///
    /// # Returns
    /// Existing input characters
    pub fn input(&self) -> &Vec<u16> {
        &self.input
    }

    /// Get the zcode base filename
    ///
    /// # Returns
    /// Base filename
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the data to save
    ///
    /// # Returns
    /// Vector of bytes to save
    pub fn save_data(&self) -> &Vec<u8> {
        &self.save_data
    }

    /// Get the foreground colour
    ///
    /// # Returns
    /// Foreground colour
    pub fn foreground(&self) -> u16 {
        self.foreground
    }

    /// Get the background colour
    ///
    /// # Returns
    /// Background colour
    pub fn background(&self) -> u16 {
        self.background
    }

    /// Gets the cursor row for SET_CURSOR
    ///
    /// # Returns
    /// Row value
    pub fn row(&self) -> u16 {
        self.row
    }

    /// Gets the cursor column for SET_CURSOR
    ///
    /// # Returns
    /// Column value
    pub fn column(&self) -> u16 {
        self.column
    }

    /// Gets the font to set
    ///
    /// # Returns
    /// Font
    pub fn font(&self) -> u16 {
        self.font
    }

    /// Gets the text style to set
    ///
    /// # Returns
    /// Text style
    pub fn style(&self) -> u16 {
        self.style
    }

    /// Gets the window to select
    ///
    /// # Return
    /// Window to select
    pub fn window_set(&self) -> u16 {
        self.window_set
    }

    /// Gets the left side of the status line
    ///
    /// # Returns
    /// Vector of text containing the left side of the status line
    pub fn status_left(&self) -> &Vec<u16> {
        &self.status_left
    }

    /// Gets the right side of the status line
    ///
    /// # Returns
    /// Vector of text containing the right side of the status line
    pub fn status_right(&self) -> &Vec<u16> {
        &self.status_right
    }

    /// Gets the SOUND_EFFECT number
    ///
    /// # Returns
    /// Number
    pub fn number(&self) -> u16 {
        self.number
    }

    /// Gets the SOUND_EFFECT effect
    ///
    /// # Returns
    /// Effect (sample number)
    pub fn effect(&self) -> u16 {
        self.effect
    }

    /// Gets the SOUND_EFFECT volumn
    ///
    /// # Returns
    /// Playback volume
    pub fn volume(&self) -> u8 {
        self.volume
    }

    /// Gets the SOUND_EFFECT repeat count
    ///
    /// # Returns
    /// Number of times to play the effect
    pub fn repeats(&self) -> u8 {
        self.repeats
    }

    /// Gets the SOUND_EFFECT end-of-playback routine address
    ///
    /// # Returns
    /// End-of-playback routine address or 0 if none
    pub fn routine(&self) -> usize {
        self.routine
    }

    /// Gets the line where a window split should occur
    ///
    /// # Returns
    /// Number of lines above the split
    pub fn split_lines(&self) -> u16 {
        self.split_lines
    }

    /// Gets the stream number
    ///
    /// # Returns
    /// Stream number
    pub fn stream(&self) -> i16 {
        self.stream
    }
}

#[derive(Clone, Debug)]
/// Interpreter callback request
pub struct InterpreterRequest {
    /// Request type
    request_type: RequestType,
    /// Requesty payload
    request: RequestPayload,
}

impl InterpreterRequest {
    /// Gets the type of request
    ///
    /// # Returns
    /// Request type
    pub fn request_type(&self) -> &RequestType {
        &self.request_type
    }

    /// Gets the request payload
    ///
    /// # Returns
    /// Request payload
    pub fn request(&self) -> &RequestPayload {
        &self.request
    }

    /// Message constructor
    ///
    /// # Arguments
    /// * `message` - Message string
    pub fn message(message: &str) -> Option<InterpreterRequest> {
        Some(InterpreterRequest {
            request_type: RequestType::Message,
            request: RequestPayload {
                message: message.to_string(),
                ..Default::default()
            },
        })
    }

    /// BufferMode constructor
    ///
    /// # Arguments
    /// * `mode` - buffer mode
    pub fn buffer_mode(mode: u16) -> Option<InterpreterRequest> {
        Some(InterpreterRequest {
            request_type: RequestType::BufferMode,
            request: RequestPayload {
                buffer_mode: mode,
                ..Default::default()
            },
        })
    }

    /// EraseLine constructor
    pub fn erase_line() -> Option<InterpreterRequest> {
        Some(InterpreterRequest {
            request_type: RequestType::EraseLine,
            request: RequestPayload::default(),
        })
    }

    /// EraseWindow constructor
    ///
    /// # Arguments
    /// * `window` - window to erase
    pub fn erase_window(window: i16) -> Option<InterpreterRequest> {
        Some(InterpreterRequest {
            request_type: RequestType::EraseWindow,
            request: RequestPayload {
                window_erase: window,
                ..Default::default()
            },
        })
    }

    /// GetCursor constructor
    pub fn get_cursor() -> Option<InterpreterRequest> {
        Some(InterpreterRequest {
            request_type: RequestType::GetCursor,
            request: RequestPayload::default(),
        })
    }

    /// InputStream constructor
    ///
    /// # Arguments
    /// * `stream` - input stream number
    pub fn input_stream(stream: i16) -> Option<InterpreterRequest> {
        Some(InterpreterRequest {
            request_type: RequestType::InputStream,
            request: RequestPayload {
                stream,
                ..Default::default()
            },
        })
    }

    /// NewLine constructor
    ///
    /// # Arguments
    /// * `transcript` - transcripting flag
    pub fn new_line(transcript: bool) -> Option<InterpreterRequest> {
        Some(InterpreterRequest {
            request_type: RequestType::NewLine,
            request: RequestPayload {
                transcript,
                ..Default::default()
            },
        })
    }

    /// OutputStream constructor
    ///
    /// # Arguments
    /// * `stream` - output stream
    /// * `name` - Base filename
    pub fn output_stream(stream: i16, name: &str) -> Option<InterpreterRequest> {
        Some(InterpreterRequest {
            request_type: RequestType::OutputStream,
            request: RequestPayload {
                stream,
                name: name.to_string(),
                ..Default::default()
            },
        })
    }

    /// Print constructor
    ///
    /// # Arguments
    /// * `text` - Decoded text to print
    /// * `transcript` - If true, text should be echoed to the transcript file
    pub fn print(text: Vec<u16>, transcript: bool) -> Option<InterpreterRequest> {
        Some(InterpreterRequest {
            request_type: RequestType::Print,
            request: RequestPayload {
                text,
                transcript,
                ..Default::default()
            },
        })
    }

    /// PrintRet constructor
    ///
    /// # Arguments
    /// * `text` - Decoded text to print
    /// * `transcript` - If true, text should be echoed to the transcript file
    pub fn print_ret(text: Vec<u16>, transcript: bool) -> Option<InterpreterRequest> {
        Some(InterpreterRequest {
            request_type: RequestType::PrintRet,
            request: RequestPayload {
                text,
                transcript,
                ..Default::default()
            },
        })
    }

    /// PrintTable constructor
    ///
    /// # Arguments
    /// * `table` - Table of decoded text to print
    /// * `width` - Table width in characters
    /// * `height` - Table height in lines
    /// * `skip` - Number of characters to skip between lines
    /// * `transcript` - If true, text should be echoed to the transcript file
    pub fn print_table(
        table: Vec<u16>,
        width: u16,
        height: u16,
        skip: u16,
        transcript: bool,
    ) -> Option<InterpreterRequest> {
        Some(InterpreterRequest {
            request_type: RequestType::PrintTable,
            request: RequestPayload {
                table,
                width,
                height,
                skip,
                transcript,
                ..Default::default()
            },
        })
    }

    /// Quit constructor
    pub fn quit() -> Option<InterpreterRequest> {
        Some(InterpreterRequest {
            request_type: RequestType::Quit,
            request: RequestPayload::default(),
        })
    }

    /// Read constructor
    ///
    /// # Arguments
    /// * `length` - Maxmium characters of input, including terminator
    /// * `terminators` - Vector of input terminator characters
    /// * `timeout` - Timeout, 0 if none
    /// * `input` - Existing input
    /// * `transcript` - transcripting flag
    pub fn read(
        length: u8,
        terminators: Vec<u16>,
        timeout: u16,
        input: Vec<u16>,
        transcript: bool,
    ) -> Option<InterpreterRequest> {
        Some(InterpreterRequest {
            request_type: RequestType::Read,
            request: RequestPayload {
                length,
                terminators,
                timeout,
                input,
                transcript,
                ..Default::default()
            },
        })
    }

    /// ReadRedraw constructor
    ///
    /// # Arguments
    /// * `input` - Input text to redraw
    pub fn read_redraw(input: Vec<u16>) -> Option<InterpreterRequest> {
        Some(InterpreterRequest {
            request_type: RequestType::ReadRedraw,
            request: RequestPayload {
                input,
                ..Default::default()
            },
        })
    }

    /// ReadChar constructor
    ///
    /// # Arguments
    /// * `timeout` - Timeout, 0 if none
    pub fn read_char(timeout: u16) -> Option<InterpreterRequest> {
        Some(InterpreterRequest {
            request_type: RequestType::ReadChar,
            request: RequestPayload {
                timeout,
                ..Default::default()
            },
        })
    }

    /// Restart constructor
    pub fn restart() -> Option<InterpreterRequest> {
        Some(InterpreterRequest {
            request_type: RequestType::Restart,
            request: RequestPayload::default(),
        })
    }

    /// Restore constructor
    ///
    /// # Arguments
    /// * `name` - ZCode base filename
    pub fn restore(name: &str) -> Option<InterpreterRequest> {
        Some(InterpreterRequest {
            request_type: RequestType::Restore,
            request: RequestPayload {
                name: name.to_string(),
                ..Default::default()
            },
        })
    }

    /// Save constructor
    ///
    /// # Arguments
    /// * `name` - ZCode base filename
    /// * `save_data` - Data to save
    pub fn save(name: &str, save_data: Vec<u8>) -> Option<InterpreterRequest> {
        Some(InterpreterRequest {
            request_type: RequestType::Save,
            request: RequestPayload {
                name: name.to_string(),
                save_data,
                ..Default::default()
            },
        })
    }

    /// SetColour constructor
    ///
    /// # Arguments
    /// * `foreground` - Foreground colour
    /// * `background` - Background colour
    pub fn set_colour(foreground: u16, background: u16) -> Option<InterpreterRequest> {
        Some(InterpreterRequest {
            request_type: RequestType::SetColour,
            request: RequestPayload {
                foreground,
                background,
                ..Default::default()
            },
        })
    }

    /// SetCursor constructor
    ///
    /// # Arguments
    /// * `row` - Cursor row
    /// * `column` - Cursor column
    pub fn set_cursor(row: u16, column: u16) -> Option<InterpreterRequest> {
        Some(InterpreterRequest {
            request_type: RequestType::SetCursor,
            request: RequestPayload {
                row,
                column,
                ..Default::default()
            },
        })
    }

    /// SetFont constructor
    ///
    /// # Arguments
    /// * `font` - Font to set
    pub fn set_font(font: u16) -> Option<InterpreterRequest> {
        Some(InterpreterRequest {
            request_type: RequestType::SetFont,
            request: RequestPayload {
                font,
                ..Default::default()
            },
        })
    }

    /// SetTextStyle constructor
    ///
    /// # Arguments
    /// * `style` - Text style to set
    pub fn set_text_style(style: u16) -> Option<InterpreterRequest> {
        Some(InterpreterRequest {
            request_type: RequestType::SetTextStyle,
            request: RequestPayload {
                style,
                ..Default::default()
            },
        })
    }

    /// SetWindow constructor
    ///
    /// # Arguments
    /// * `window` - Window to select
    pub fn set_window(window: u16) -> Option<InterpreterRequest> {
        Some(InterpreterRequest {
            request_type: RequestType::SetWindow,
            request: RequestPayload {
                window_set: window,
                ..Default::default()
            },
        })
    }

    /// ShowStatus constructor
    ///
    /// # Arguments
    /// * `left` - Decoded text for the left side of the status line
    /// * `right` - Decoded text for the right side of the status line
    pub fn show_status(left: Vec<u16>, right: Vec<u16>) -> Option<InterpreterRequest> {
        Some(InterpreterRequest {
            request_type: RequestType::ShowStatus,
            request: RequestPayload {
                status_left: left,
                status_right: right,
                ..Default::default()
            },
        })
    }

    /// SoundEffect constructor
    ///
    /// # Arguments
    /// * `number` - SOUND_EFFECT number
    /// * `effect` - Sample number
    /// * `volume` - Playback volume
    /// * `repeats` - Number of times to play the sample
    /// * `routine` - End-of-playback interrupt routine, or 0 for none
    pub fn sound_effect(
        number: u16,
        effect: u16,
        volume: u8,
        repeats: u8,
        routine: usize,
    ) -> Option<InterpreterRequest> {
        Some(InterpreterRequest {
            request_type: RequestType::SoundEffect,
            request: RequestPayload {
                number,
                effect,
                volume,
                repeats,
                routine,
                ..Default::default()
            },
        })
    }

    /// SplitWindow constructor
    ///
    /// # Arguments
    /// * `lines` - Number of lines for the upper window
    pub fn split_window(lines: u16) -> Option<InterpreterRequest> {
        Some(InterpreterRequest {
            request_type: RequestType::SplitWindow,
            request: RequestPayload {
                split_lines: lines,
                ..Default::default()
            },
        })
    }
}

#[derive(Default)]
/// Interpreter callback response payload
pub struct ResponsePayload {
    // GET_CURSOR
    row: u16,
    column: u16,

    // READ
    input: Vec<u16>,
    terminator: InputEvent,

    // ...interrupted
    interrupt: Interrupt,

    // READ_CHAR
    key: InputEvent,

    // RESTORE
    save_data: Vec<u8>,

    // SAVE
    success: bool,

    // SET_FONT
    font: u16,

    // SOUND_EFFECT finished routine
    routine: usize,
}

// impl ResponsePayload {
//     // pub fn key(&self) -> &InputEvent {
//     //     &self.key
//     // }

//     // pub fn input(&self) -> &Vec<u16> {
//     //     &self.input
//     // }
// }

/// Interpeter callback response type
pub enum ResponseType {
    GetCursor,
    ReadComplete,
    ReadInterrupted,
    ReadCharComplete,
    ReadCharInterrupted,
    RestoreComplete,
    SaveComplete,
    SetFont,
    SoundInterrupt,
}

/// Interpreter callback response
pub struct InterpreterResponse {
    /// Callback response type
    response_type: ResponseType,
    /// Response payload
    response: ResponsePayload,
}

impl InterpreterResponse {
    /// GetCursor constructor
    ///
    /// # Arguments
    /// * `row` - Cursor position row
    /// * `column` - Cursor position column
    pub fn get_cursor(row: u16, column: u16) -> Option<InterpreterResponse> {
        Some(InterpreterResponse {
            response_type: ResponseType::GetCursor,
            response: ResponsePayload {
                row,
                column,
                ..Default::default()
            },
        })
    }

    /// ReadComplete constructor
    ///
    /// # Arguments
    /// * `input` - Interpreter input
    pub fn read_complete(input: Vec<u16>, terminator: InputEvent) -> Option<InterpreterResponse> {
        Some(InterpreterResponse {
            response_type: ResponseType::ReadComplete,
            response: ResponsePayload {
                input,
                terminator,
                ..Default::default()
            },
        })
    }

    /// ReadCharComplete constructor
    ///
    /// # Arguments
    /// * `key` - Input event
    pub fn read_char_complete(key: InputEvent) -> Option<InterpreterResponse> {
        Some(InterpreterResponse {
            response_type: ResponseType::ReadCharComplete,
            response: ResponsePayload {
                key,
                ..Default::default()
            },
        })
    }

    /// ReadInterrupted constructor
    ///
    /// # Arguments
    /// * `input` - Input buffer at the time of the interrupt
    /// * `interrupt` - Interrupt type
    /// * `routine` - For [Interrupt::Sound], the address of the routine to call.  Ignored for [Interrupt::ReadTimeout]
    pub fn read_interrupted(
        input: Vec<u16>,
        interrupt: Interrupt,
        routine: usize,
    ) -> Option<InterpreterResponse> {
        Some(InterpreterResponse {
            response_type: ResponseType::ReadInterrupted,
            response: ResponsePayload {
                input,
                interrupt,
                routine,
                ..Default::default()
            },
        })
    }

    /// ReadCharInterrupted constructor
    ///
    /// # Arguments
    /// * `interrupt` - Interrupt type
    pub fn read_char_interrupted(interrupt: Interrupt) -> Option<InterpreterResponse> {
        Some(InterpreterResponse {
            response_type: ResponseType::ReadCharInterrupted,
            response: ResponsePayload {
                interrupt,
                ..Default::default()
            },
        })
    }

    /// Restore constructor
    ///
    /// # Arguments
    /// * `save_data` - Saved data
    pub fn restore(save_data: Vec<u8>) -> Option<InterpreterResponse> {
        Some(InterpreterResponse {
            response_type: ResponseType::RestoreComplete,
            response: ResponsePayload {
                save_data,
                ..Default::default()
            },
        })
    }

    /// Save constructor
    ///
    /// # Arguments
    /// * `success` - `true` if the save succeeded, `false` if not
    pub fn save(success: bool) -> Option<InterpreterResponse> {
        Some(InterpreterResponse {
            response_type: ResponseType::SaveComplete,
            response: ResponsePayload {
                success,
                ..Default::default()
            },
        })
    }

    /// SetFont constructor
    ///
    /// # Arguments
    /// * `font` - Previous font number
    pub fn set_font(font: u16) -> Option<InterpreterResponse> {
        Some(InterpreterResponse {
            response_type: ResponseType::SetFont,
            response: ResponsePayload {
                font,
                ..Default::default()
            },
        })
    }

    /// SoundInterrupt constructor
    ///
    /// # Arguments
    /// * `routine` - Address of the sound end-of-playback routine to execute
    pub fn sound_interrupt(routine: usize) -> Option<InterpreterResponse> {
        Some(InterpreterResponse {
            response_type: ResponseType::SoundInterrupt,
            response: ResponsePayload {
                routine,
                ..Default::default()
            },
        })
    }
}

/// The Z-Machine!
pub struct ZMachine {
    ///Base ZCode filename - the filename minus any extension
    name: String,
    /// ZCode version
    version: u8,
    /// Memory map
    memory: Memory,
    /// RNG
    rng: Box<dyn ZRng>,
    /// Frame stack
    frames: Vec<Frame>,
    /// Undo stack
    undo_stack: VecDeque<Quetzal>,
    /// Output stream bitmask
    output_streams: u8,
    /// Stream 3 stack
    stream_3: Vec<Stream3>,
}

impl TryFrom<(&ZMachine, usize)> for Quetzal {
    type Error = RuntimeError;

    fn try_from((value, pc): (&ZMachine, usize)) -> Result<Self, Self::Error> {
        let ifhd = IFhd::try_from((value, pc))?;
        let mem = Mem::try_from(value)?;
        let stks = Stks::try_from(value)?;

        let quetzal = Quetzal::new(ifhd, mem, stks);
        Ok(quetzal)
    }
}

impl TryFrom<&ZMachine> for Mem {
    type Error = RuntimeError;

    fn try_from(value: &ZMachine) -> Result<Self, Self::Error> {
        let compressed_memory = value.memory.compress();
        debug!(target: "app::state", "Compressed dynamic memory: {:04x} bytes", compressed_memory.len());
        let mem = Mem::new(true, compressed_memory);
        Ok(mem)
    }
}

impl TryFrom<(&ZMachine, usize)> for IFhd {
    type Error = RuntimeError;

    fn try_from((value, pc): (&ZMachine, usize)) -> Result<Self, Self::Error> {
        let release_number = header::field_word(&value.memory, HeaderField::Release)?;
        let mut serial_number = Vec::new();
        for i in 0..6 {
            serial_number.push(value.read_byte(HeaderField::Serial as usize + i)?);
        }
        let checksum = header::field_word(&value.memory, HeaderField::Checksum)?;

        let ifhd = IFhd::new(
            release_number,
            &serial_number,
            checksum,
            (pc as u32) & 0xFFFFFF,
        );
        debug!(target: "app::state", "State derived IFhd: {}", ifhd);
        Ok(ifhd)
    }
}

impl TryFrom<&ZMachine> for Stks {
    type Error = RuntimeError;

    fn try_from(value: &ZMachine) -> Result<Self, Self::Error> {
        let mut frames = Vec::new();
        // TBD: Stk::trY_from(Frame)
        for f in &value.frames {
            // Flags: 0b000rvvvv
            //  r = 1 if the frame routine does not store a result
            //  vvvv = the number of local variables (0 - 15)
            let flags = match f.result() {
                Some(_) => 0x00,
                None => 0x10,
            } | f.local_variables().len();

            // Arguments: 0b87654321
            //  bits are set for each argument
            let mut arguments = 0;
            for _ in 0..f.argument_count() {
                arguments = (arguments << 1) | 0x01;
            }

            // Store result, or 0 if the routine doesn't store a result.
            // Note that "0" is also the stack if bit 4 of flags is set
            let result_variable = match f.result() {
                Some(r) => r.variable(),
                None => 0,
            };

            let frame = Stk::new(
                f.return_address() as u32,
                flags as u8,
                result_variable,
                arguments,
                &f.local_variables().clone(),
                &f.stack().clone(),
            );
            // debug!(target: "app::quetzal", "Frame: {}", frame);
            frames.push(frame);
        }

        let stks = Stks::new(frames);
        debug!(target: "app::state", "Runtime stack data: {} frames", stks.stks().len());
        Ok(stks)
    }
}

impl ZMachine {
    /// Constructor
    ///
    /// # Arguments
    /// * `zcode` - ZCode program to execute
    /// * `config` - Runtime configuration
    /// * `name` - Base filename
    /// * `rows` - Screen rows
    /// * `columns` - Screen columns
    pub fn new(
        zcode: Vec<u8>,
        config: &Config,
        name: &str,
        rows: u8,
        columns: u8,
    ) -> Result<ZMachine, RuntimeError> {
        let memory = Memory::new(zcode);
        let version = header::field_byte(&memory, HeaderField::Version)?;
        let rng = ChaChaRng::new();
        let mut zm = ZMachine {
            name: name.to_string(),
            version,
            memory,
            rng: Box::new(rng),
            frames: Vec::new(),
            undo_stack: VecDeque::new(),
            output_streams: 0x1,
            stream_3: Vec::new(),
        };

        zm.initialize(
            rows,
            columns,
            (config.foreground(), config.background()),
            false,
        )?;
        Ok(zm)
    }

    /// Get the Zcode version
    ///
    /// # Returns
    /// Zcode version
    pub fn version(&self) -> u8 {
        self.version
    }

    /// Get the Zcode base filename
    ///
    /// # Returns
    /// Base name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Initialize (or re-initialize) the Z-Machine state
    ///
    /// # Arguments
    /// * `rows` - Screen rows
    /// * `columns` - Screen columns,
    /// * `default_colours` - Default (foreground, background) colours
    /// * `sound` - Is sound playback enabled?
    ///
    /// # Returns
    /// Empty [Result] or a [RuntimeError]
    pub fn initialize(
        &mut self,
        rows: u8,
        columns: u8,
        default_colors: (u8, u8),
        sound: bool,
    ) -> Result<(), RuntimeError> {
        // Set V3 flags
        if self.version < 4 {
            header::clear_flag1(&mut self.memory, Flags1v3::StatusLineNotAvailable as u8)?;
            header::set_flag1(&mut self.memory, Flags1v3::ScreenSplitAvailable as u8)?;
            header::clear_flag1(&mut self.memory, Flags1v3::VariablePitchDefault as u8)?;
        }

        // Set V4+ flags and header fields
        if self.version > 3 {
            if sound {
                header::set_flag1(&mut self.memory, Flags1v4::SoundEffectsAvailable as u8)?;
            }

            header::set_byte(
                &mut self.memory,
                HeaderField::DefaultBackground,
                default_colors.1,
            )?;
            header::set_byte(
                &mut self.memory,
                HeaderField::DefaultForeground,
                default_colors.0,
            )?;
            header::set_byte(&mut self.memory, HeaderField::ScreenLines, rows)?;
            header::set_byte(&mut self.memory, HeaderField::ScreenColumns, columns)?;
        }

        // Set V5+ flags and header fields
        if self.version > 4 {
            header::clear_flag1(&mut self.memory, Flags1v4::PicturesAvailable as u8)?;
            header::set_flag1(&mut self.memory, Flags1v4::ColoursAvailable as u8)?;
            header::set_flag1(&mut self.memory, Flags1v4::BoldfaceAvailable as u8)?;
            header::set_flag1(&mut self.memory, Flags1v4::ItalicAvailable as u8)?;
            header::set_flag1(&mut self.memory, Flags1v4::FixedSpaceAvailable as u8)?;
            header::set_flag1(&mut self.memory, Flags1v4::TimedInputAvailable as u8)?;
            //header::clear_flag2(&mut self.memory, Flags2::RequestMouse)?;
            // Graphics font 3 support is crap atm
            header::clear_flag2(&mut self.memory, Flags2::RequestPictures)?;
            // If sounds weren't loaded
            if !sound {
                header::clear_flag2(&mut self.memory, Flags2::RequestSoundEffects)?;
            }

            header::set_word(&mut self.memory, HeaderField::ScreenHeight, rows as u16)?;
            header::set_word(&mut self.memory, HeaderField::ScreenWidth, columns as u16)?;
            header::set_byte(&mut self.memory, HeaderField::FontWidth, 1)?;
            header::set_byte(&mut self.memory, HeaderField::FontHeight, 1)?;
        }

        // Interpreter # and version
        header::set_byte(&mut self.memory, HeaderField::InterpreterNumber, 6)?;
        header::set_byte(&mut self.memory, HeaderField::InterpreterVersion, b'Z')?;
        // self.memory.write_byte(0x1E, 6)?;
        // self.memory.write_byte(0x1F, b'Z')?;

        // Z-Machine standard compliance
        header::set_word(&mut self.memory, HeaderField::Revision, 0x0100)?;
        // self.write_byte(0x32, 1)?;
        // self.write_byte(0x33, 0)?;

        // Initializing after a restore will already have stack frames,
        // so check before pushing a dummy frame
        if self.frames.is_empty() {
            let pc = header::field_word(&self.memory, HeaderField::InitialPC)? as usize;
            let f = Frame::new(pc, pc, &[], 0, &[], None, 0);
            self.frames.clear();
            self.frames.push(f);
        }

        Ok(())
    }

    // Managed memory access (read/write dynamic, read static, no access to high)
    /// Read a byte from the memory map.
    ///
    /// Access is limited to dynamic and static memory.  Accessing high memory with
    /// this function will result in a [RuntimeError].
    ///
    /// # Arguments
    /// * `address` - Address to read from
    ///
    /// # Returns
    /// [Result] containing the byte value or a [RuntimeError]
    pub fn read_byte(&self, address: usize) -> Result<u8, RuntimeError> {
        if address < 0x10000 {
            self.memory.read_byte(address)
        } else {
            fatal_error!(
                ErrorCode::IllegalMemoryAccess,
                "Read from byte address in high memory: {:#06x}",
                address
            )
        }
    }

    /// Read a word from the memory map.
    ///
    /// Access is limited to dynamic and static memory.  Accessing high memory with
    /// this function will result in a [RuntimeError].
    ///
    /// # Arguments
    /// * `address` - Address to read from
    ///
    /// # Returns
    /// [Result] containing the word value or a [RuntimeError]
    pub fn read_word(&self, address: usize) -> Result<u16, RuntimeError> {
        if address < 0xFFFF {
            self.memory.read_word(address)
        } else {
            fatal_error!(
                ErrorCode::IllegalMemoryAccess,
                "Read from word address in hight memory: {:#06x}",
                address
            )
        }
    }

    // TODO: handle flipping the transcript bit

    /// Write a byte to the memory map
    ///
    /// Access is limited to dynamic memory.  Attempting to write to static or high memory
    /// will result in a [RuntimeError]
    ///
    /// Writes to [HeaderField::Flags2] that change the [Flags2::Transcripting] bit will toggle
    /// transcripting.
    ///
    /// # Arguments
    /// * `address` - address to write to
    /// * `value` - byte value to write
    ///
    /// # Returns
    /// Empty [Result] or a [RuntimeError]
    pub fn write_byte(&mut self, address: usize, value: u8) -> Result<(), RuntimeError> {
        if address < self.memory.static_mark() {
            self.memory.write_byte(address, value)
        } else {
            fatal_error!(
                ErrorCode::IllegalMemoryAccess,
                "Write to byte address above dynamic memory {:04x}: {:04x}",
                self.memory.static_mark() - 1,
                address,
            )
        }
    }

    /// Write a word to the memory map
    ///
    /// Access is limited to dynamic memory.  Attempting to write to static or high memory
    /// will result in a [RuntimeError]
    ///
    /// Writes to [HeaderField::Flags2] that change the [Flags2::Transcripting] bit will toggle
    /// transcripting.
    ///
    /// # Arguments
    /// * `address` - address to write to
    /// * `value` - word value to write
    ///
    /// # Returns
    /// Empty [Result] or a [RuntimeError]
    pub fn write_word(&mut self, address: usize, value: u16) -> Result<(), RuntimeError> {
        if address < self.memory.static_mark() - 1 {
            self.memory.write_word(address, value)?;
            Ok(())
        } else {
            fatal_error!(
                ErrorCode::IllegalMemoryAccess,
                "Write to word address above dynamic memory {:04x}: {:04x}",
                self.memory.static_mark() - 1,
                address,
            )
        }
    }

    /// Calculate the checksum of the Zcode file
    ///
    /// # Returns
    /// [Result] containing the checksum value or a [RuntimeError]
    pub fn checksum(&self) -> Result<u16, RuntimeError> {
        self.memory.checksum()
    }

    // Save/restore
    /// Save the current game state to the undo stack
    ///
    /// # Arguments
    /// * `pc` - address of the SAVE_UNDO instruction store location byte
    ///
    /// # Returns
    /// Empty [Result] or a [RuntimeError]
    pub fn save_undo(&mut self, pc: usize) -> Result<(), RuntimeError> {
        let quetzal = Quetzal::try_from((&*self, pc))?;
        debug!(target: "app::state", "Storing undo state");
        self.undo_stack.push_back(quetzal);
        while self.undo_stack.len() > 10 {
            // Remove the first (oldest) entries
            self.undo_stack.pop_front();
        }
        Ok(())
    }

    /// Restore game state from the undo stack
    ///
    /// # Returns
    /// [Result] containing an [Option] with the address to resume execution if the restore succeeds, else [None]
    /// or a [RuntimeError]
    pub fn restore_undo(&mut self) -> Result<Option<usize>, RuntimeError> {
        if let Some(quetzal) = self.undo_stack.pop_back() {
            debug!(target: "app::state", "Restoring undo state");
            self.restore_state(quetzal)
        } else {
            warn!(target: "app::state", "No saved state for undo");
            recoverable_error!(ErrorCode::UndoNoState, "Undo stack is empty")
        }
    }

    /// Restart game execution
    ///
    /// # Returns
    /// [Result] containing the address of the initial instruction to execute or a [RuntimeError]
    pub fn restart(&mut self) -> Result<usize, RuntimeError> {
        // Reset the RNG
        self.rng.seed(0);

        // Header fields that should be preserved:
        // Flags2
        let flags2 = self.read_word(0x10)?;
        debug!(target: "app::instruction", "Flags2: {:016b}", flags2);
        debug!(target: "app::instruction", "Streams: {:04b}", self.output_streams);

        // Default foreground
        let fg = header::field_byte(&self.memory, HeaderField::DefaultForeground)?;
        // Default background
        let bg = header::field_byte(&self.memory, HeaderField::DefaultBackground)?;
        // Screen rows
        let rows = header::field_byte(&self.memory, HeaderField::ScreenLines)?;
        // Screen columns
        let columns = header::field_byte(&self.memory, HeaderField::ScreenColumns)?;

        // Reset the memory map
        self.memory.reset();
        // Empty the frame stack
        self.frames.clear();

        // Re-initialize
        self.initialize(rows, columns, (fg, bg), false)?;
        // Put the Flags2 value back and reset the output streams
        self.write_word(HeaderField::Flags2 as usize, flags2)?;
        self.output_streams &= 0x3;
        self.stream_3.clear();

        debug!(target: "app::instruction", "Flags2: {:016b}", self.read_word(0x10)?);
        debug!(target: "app::instruction", "Streams: {:04b}", self.output_streams);

        Ok(self.current_frame()?.pc())
    }

    // Unmanaged memory access: string literals, routines

    /// Get a string literal from an address that may reside in high memory that is normally
    /// off limits.
    ///
    /// # Arguments
    /// * `address` - Address of the literal
    ///
    /// # Returns
    /// Encoded ztext string data
    pub fn string_literal(&self, address: usize) -> Result<Vec<u16>, RuntimeError> {
        let mut d = Vec::new();
        // Read until bit 15 of the word is set
        loop {
            let w = self.memory.read_word(address + (d.len() * 2))?;
            d.push(w);
            if w & 0x8000 == 0x8000 {
                return Ok(d);
            }
        }
    }

    /// Get instruction bytes that may reside in high memory
    ///
    /// # Arguments
    /// * `address` - Address of the instruction
    ///
    /// # Returns
    /// A vector containing 23 bytes from the `address`, which is the longest instruction possible.
    pub fn instruction(&self, address: usize) -> Vec<u8> {
        // An instruction may be up to 23 bytes long, excluding literal strings
        // Opcode: up to 2 bytes
        // Operand types: up to 8 (2 bytes)
        // Operands: up to 8 (16 bytes)
        // Store variable: up to 1 byte
        // Branch offset: up to 2 bytes
        self.memory.slice(address, 23)
    }

    /// Decode a routine header from an address that may reside in high memory
    ///
    /// # Arguments
    /// * `address` - Address of the routine header
    ///
    /// # Returns
    /// [Result] containing a tuple of (instruction address, local variables) for the routine or a [RuntimeError]
    fn routine_header(&self, address: usize) -> Result<(usize, Vec<u16>), RuntimeError> {
        let variable_count = self.memory.read_byte(address)? as usize;
        if variable_count > 15 {
            fatal_error!(
                ErrorCode::InvalidRoutine,
                "Routines can have at most 15 local variables: {}",
                variable_count
            )
        } else {
            let (initial_pc, local_variables) = if self.version < 5 {
                let mut l = Vec::new();
                for i in 0..variable_count {
                    let a = address + 1 + (i * 2);
                    l.push(self.memory.read_word(a)?);
                }

                (address + 1 + (variable_count * 2), l)
            } else {
                (address + 1, vec![0; variable_count])
            };

            Ok((initial_pc, local_variables))
        }
    }

    // Packed addresses

    /// Unpack a routine address
    ///
    /// # Arguments
    /// * `address` - Packed address
    ///
    /// # Returns
    /// [Result] with the unpacked byte address of the routine header or a [RuntimeError]
    pub fn packed_routine_address(&self, address: u16) -> Result<usize, RuntimeError> {
        match self.version {
            3 => Ok(address as usize * 2),
            4..=5 => Ok(address as usize * 4),
            7 => Ok((address as usize * 4)
                + (self
                    .memory
                    .read_word(HeaderField::RoutinesOffset as usize)? as usize
                    * 8)),
            8 => Ok(address as usize * 8),
            _ => fatal_error!(
                ErrorCode::UnsupportedVersion,
                "Unsupported version: {}",
                self.version
            ),
        }
    }

    /// Unpack a string address
    ///
    /// # Arguments
    /// * `address` - Packed address
    ///
    /// # Returns
    /// [Result] with the unpacked byte address of the string data or a [RuntimeError]
    pub fn packed_string_address(&self, address: u16) -> Result<usize, RuntimeError> {
        match self.version {
            1..=3 => Ok(address as usize * 2),
            4..=5 => Ok(address as usize * 4),
            7 => Ok((address as usize * 4)
                + (self.memory.read_word(HeaderField::StringsOffset as usize)? as usize * 8)),
            8 => Ok(address as usize * 8),
            _ => fatal_error!(
                ErrorCode::UnsupportedVersion,
                "Unsupported version: {}",
                self.version
            ),
        }
    }

    // Header
    /// Reads a byte field from the header
    ///
    /// # Arguments
    /// * `field` - Field to read
    ///
    /// # Returns
    /// [Result] with the byte value from the header or a [RuntimeError]
    pub fn header_byte(&self, field: HeaderField) -> Result<u8, RuntimeError> {
        header::field_byte(&self.memory, field)
    }

    /// Reads a word field from the header
    ///
    /// # Arguments
    /// * `field` - Field to read
    ///
    /// # Returns
    /// [Result] with the word value from the header or a [RuntimeError]
    pub fn header_word(&self, field: HeaderField) -> Result<u16, RuntimeError> {
        header::field_word(&self.memory, field)
    }

    // Frame stack
    /// Get the frame pointer
    ///
    /// # Returns
    /// Frame pointer (which is the current depth of the frame stack)
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// Get a reference to the current frame
    ///
    /// # Returns
    /// [Result] with a reference to the current frame or a [RuntimeError]
    fn current_frame(&self) -> Result<&Frame, RuntimeError> {
        if let Some(frame) = self.frames.last() {
            Ok(frame)
        } else {
            fatal_error!(ErrorCode::NoFrame, "No runtime frame")
        }
    }

    /// Get a mutable reference to the current frame
    ///
    /// # Returns
    /// [Result] with a mutable reference to the current frame or a [RuntimeError]
    fn current_frame_mut(&mut self) -> Result<&mut Frame, RuntimeError> {
        if let Some(frame) = self.frames.last_mut() {
            Ok(frame)
        } else {
            fatal_error!(ErrorCode::NoFrame, "No runtime frame")
        }
    }

    // Routines
    /// Call a routine
    ///
    /// # Arguments
    /// * `address` - (unpacked) address of the routine header
    /// * `arguments` - vector of any arguments passed to the routine
    /// * `result` - store location for the return value of the routine
    /// * `return_address` - address to resume execution at when the routine returns
    ///
    /// # Returns
    /// [Result] with the address of the first instruction of the routine or a [RuntimeError]
    pub fn call_routine(
        &mut self,
        address: usize,
        arguments: &Vec<u16>,
        result: Option<StoreResult>,
        return_address: usize,
    ) -> Result<NextAddress, RuntimeError> {
        // Call to address 0 results in FALSE
        if address == 0 {
            if let Some(r) = result {
                self.set_variable(r.variable(), 0)?;
            }
            Ok(NextAddress::Address(return_address))
        } else {
            let (initial_pc, local_variables) = self.routine_header(address)?;
            let frame = Frame::call_routine(
                address,
                initial_pc,
                arguments,
                local_variables,
                result,
                return_address,
            )?;
            self.frames.push(frame);

            Ok(NextAddress::Address(initial_pc))
        }
    }

    /// Call a READ interrupt routine
    ///
    /// # Arguments
    /// * `address` - (unpacked) address of the routine header
    /// * `arguments` - vector of any arguments passed to the routine
    /// * `return_address` - address to resume execution at when the routine returns
    ///
    /// # Returns
    /// [Result] with the address of the first instruction of the routine or a [RuntimeError]
    pub fn call_read_interrupt(
        &mut self,
        address: usize,
        arguments: &Vec<u16>,
        return_address: usize,
    ) -> Result<NextAddress, RuntimeError> {
        // Call to address 0 results in FALSE
        if address == 0 {
            Ok(NextAddress::Address(return_address))
        } else {
            let (initial_pc, local_variables) = self.routine_header(address)?;
            let mut frame = Frame::call_routine(
                address,
                initial_pc,
                arguments,
                local_variables,
                None,
                return_address,
            )?;
            frame.set_read_interrupt();
            self.frames.push(frame);

            Ok(NextAddress::Address(initial_pc))
        }
    }

    /// Call a READ_CHAR interrupt routine
    ///
    /// # Arguments
    /// * `address` - (unpacked) address of the routine header
    /// * `arguments` - vector of any arguments passed to the routine
    /// * `return_address` - address to resume execution at when the routine returns
    ///
    /// # Returns
    /// [Result] with the address of the first instruction of the routine or a [RuntimeError]
    pub fn call_read_char_interrupt(
        &mut self,
        address: usize,
        arguments: &Vec<u16>,
        return_address: usize,
    ) -> Result<NextAddress, RuntimeError> {
        // Call to address 0 results in FALSE
        if address == 0 {
            Ok(NextAddress::Address(return_address))
        } else {
            let (initial_pc, local_variables) = self.routine_header(address)?;
            let mut frame = Frame::call_routine(
                address,
                initial_pc,
                arguments,
                local_variables,
                None,
                return_address,
            )?;
            frame.set_read_char_interrupt();
            self.frames.push(frame);

            Ok(NextAddress::Address(initial_pc))
        }
    }

    /// Is the current frame a READ interrupt?
    ///
    /// # Returns
    /// `true` if the current frame is a READ interrupt, false if not
    pub fn is_read_interrupt(&self) -> Result<bool, RuntimeError> {
        Ok(self.current_frame()?.read_interrupt())
    }

    // /// Is the current frame a READ_CHAR interrupt?
    // ///
    // /// # Returns
    // /// `true` if the current frame is a READ_CAR interrupt, false if not
    // pub fn is_read_char_interrupt(&self) -> Result<bool, RuntimeError> {
    //     Ok(self.current_frame()?.read_char_interrupt())
    // }

    /// Set the redraw input flag on the current frame.
    ///
    /// This is called when a READ interrupt routine prints text to the screen.  If the READ is
    /// resumed, it will need to print any input the player had entered prior to the interrupt
    ///
    /// # Returns
    /// Empty [Result] or a [RuntimeError]
    pub fn set_redraw_input(&mut self) -> Result<(), RuntimeError> {
        self.current_frame_mut()?.set_redraw_input();
        Ok(())
    }

    /// Return from a routine
    ///
    /// # Arguments
    /// * `value` - Return value, which may or may not be stored
    ///
    /// # Returns
    /// [Result] with the return address to resume execution at or a [RuntimeError]
    pub fn return_routine(&mut self, value: u16) -> Result<NextAddress, RuntimeError> {
        if let Some(f) = self.frames.pop() {
            debug!(target: "app::state", "Return {:04x} => {:?} to ${:06x}, redraw: {}", value, f.result(), f.return_address(), f.redraw_input());
            if let Some(r) = f.result() {
                self.set_variable(r.variable(), value)?;
            }

            let n = self.current_frame_mut()?;
            n.set_next_pc(f.return_address());

            // If this was a READ interrupt, include the return value (which will not be stored) and redraw-input values
            if f.read_interrupt() {
                debug!(target: "app::screen", "Return from READ interrupt");
                Ok(NextAddress::ReadInterrupt(
                    f.return_address(),
                    value,
                    f.redraw_input(),
                ))
            // If this was a READ_CHAR interrupt, include the return value (which will not be stored)
            } else if f.read_char_interrupt() {
                debug!(target: "app::screen", "Return from READ_CHAR interrupt");
                Ok(NextAddress::ReadCharInterrupt(f.return_address(), value))
            } else {
                Ok(NextAddress::Address(f.return_address()))
            }
        } else {
            fatal_error!(
                ErrorCode::ReturnNoCaller,
                "Return from routine with nowhere to return to"
            )
        }
    }

    /// Gets the count of arguments passed to the executing routine
    ///
    /// # Returns
    /// Count of arguments to the current routine
    pub fn argument_count(&self) -> Result<u8, RuntimeError> {
        Ok(self.current_frame()?.argument_count())
    }

    /// Throws execution, returning to an arbitrary frame pointer and throwing away any intermediary frames
    ///
    /// # Arguments
    /// * `depth` - Frame pointer to throw to
    /// * `value` - Return value to the frame thrown to
    ///
    /// # Returns
    /// [Result] with the address of the next instruction to execute or a [RuntimeError]
    pub fn throw(&mut self, depth: u16, result: u16) -> Result<NextAddress, RuntimeError> {
        self.frames.truncate(depth as usize);
        self.return_routine(result)
    }

    // Variables
    /// Get the address of a [global variable](https://inform-fiction.org/zmachine/standards/z1point1/sect06.html#two)
    ///
    /// # Arguments
    /// * `varibale` - Global variable, which should be 16..=255
    ///
    /// # Returns
    /// [Result] with the address of the global variable in memory or a [RuntimeError]
    fn global_variable_address(&self, variable: u8) -> Result<usize, RuntimeError> {
        let table = header::field_word(&self.memory, HeaderField::GlobalTable)? as usize;
        let index = (variable as usize - 16) * 2;
        Ok(table + index)
    }

    /// Get the value of a variable.
    ///
    /// # Arguments
    /// * `variable` - Variable number
    ///
    /// # Returns
    /// [Result] with the variable value or a [RuntimeError]
    pub fn variable(&mut self, variable: u8) -> Result<u16, RuntimeError> {
        if variable < 16 {
            self.current_frame_mut()?.local_variable(variable)
        } else {
            let address = self.global_variable_address(variable)?;
            self.read_word(address)
        }
    }

    /// Peek at the value of a variable.
    ///
    /// If variable 0 is requested, this function does not change the stack.
    ///
    /// # Arguments
    /// * `variable` - Variable number
    ///
    /// # Returns
    /// [Result] with the variable value or a [RuntimeError]
    pub fn peek_variable(&mut self, variable: u8) -> Result<u16, RuntimeError> {
        if variable < 16 {
            self.current_frame()?.peek_local_variable(variable)
        } else {
            let address = self.global_variable_address(variable)?;
            self.read_word(address)
        }
    }

    /// Set the value of a variable
    ///
    /// If the `variable` number is 0, the `value` is pushed to the stack.
    ///
    /// # Arguments
    /// * `variable` - Variable number
    /// * `value` - Value to set
    ///
    /// # Returns
    /// Empty [Result] or a [RuntimeError]
    pub fn set_variable(&mut self, variable: u8, value: u16) -> Result<(), RuntimeError> {
        debug!(target: "app::state", "Set variable {:02x} to {:04x}", variable, value);
        if variable < 16 {
            self.current_frame_mut()?
                .set_local_variable(variable, value)
        } else {
            let address = self.global_variable_address(variable)?;
            self.write_word(address, value)
        }
    }

    /// Sets the value of a variable indirectly
    ///
    /// If the `variable` number is 0, the top of the stack is replaced with the `value`.
    ///
    /// # Arguments
    /// * `variable` - Variable number
    /// * `value` - Value to set
    ///
    /// # Returns
    /// Empty [Result] or a [RuntimeError]
    pub fn set_variable_indirect(&mut self, variable: u8, value: u16) -> Result<(), RuntimeError> {
        debug!(target: "app::state", "Set variable indirect {:02x} to {:04x}", variable, value);
        if variable < 16 {
            self.current_frame_mut()?
                .set_local_variable_indirect(variable, value)
        } else {
            let address = self.global_variable_address(variable)?;
            self.write_word(address, value)
        }
    }

    /// Push a value to the stack
    ///
    /// # Arguments
    /// * `value` - Value to push
    ///
    /// # Returns
    /// Empty [Result] or a [RuntimeError]
    pub fn push(&mut self, value: u16) -> Result<(), RuntimeError> {
        self.current_frame_mut()?.set_local_variable(0, value)
    }

    // Status line
    /// Get the left and right side of the status line.
    ///
    /// The left side is typically the short name of the current room object.  The right
    /// side is either the current score and turn, or a time, depending on [Flags1v3::StatusLineType].
    ///
    /// # Returns
    /// [Result] with a tuple (left, right) vectors of text or a [RuntimeError]
    pub fn status_line(&mut self) -> Result<(Vec<u16>, Vec<u16>), RuntimeError> {
        let status_type = header::flag1(&self.memory, Flags1v3::StatusLineType as u8)?;
        let object = self.variable(16)? as usize;
        let left = text::from_vec(self, &property::short_name(self, object)?, false)?;
        let right: Vec<u16> = if status_type == 0 {
            // Score is between -99 and 999 inclusive
            let score = i16::min(999, i16::max(-99, self.variable(17)? as i16));
            // Turns is between 0 and 9999 inclusive
            let turns = u16::min(9999, self.variable(18)?);
            // Combine score and turns, padding to 8 characters
            format!("{:<8}", format!("{:}/{:}", score, turns))
                .as_bytes()
                .iter()
                .map(|x| *x as u16)
                .collect()
        } else {
            // Hour is between 0 and 23, inclusive
            let hour = u16::min(23, self.variable(17)?);
            // Minute is between 0 and 59, inclusive
            let minute = u16::min(59, self.variable(18)?);
            let suffix = if hour > 11 { "PM" } else { "AM" };
            // 0-24 -> 1-12
            let h = if hour == 0 {
                12
            } else if hour > 12 {
                hour - 12
            } else {
                hour
            };

            format!("{:2}:{:02} {}", h, minute, suffix)
                .as_bytes()
                .iter()
                .map(|x| *x as u16)
                .collect()
        };

        Ok((left, right))
    }

    // RNG
    /// Get a random number
    ///
    /// # Arguments
    /// * `range` - Upper limit of the number to generate
    ///
    /// # Returns
    /// Random number in the range 1..=`range``
    pub fn random(&mut self, range: u16) -> u16 {
        self.rng.random(range)
    }

    /// Seed the RNG.
    ///
    /// Sets the RNG to random mode.
    ///
    /// # Arguments
    /// * `seed` value
    pub fn seed(&mut self, seed: u16) {
        self.rng.seed(seed)
    }

    /// Set the RNG to predictable mode
    ///
    /// # Arguments
    /// * `seed` - Upper limit of the predictable range
    pub fn predictable(&mut self, seed: u16) {
        self.rng.predictable(seed)
    }

    // Streams

    pub fn is_stream_enabled(&self, stream: u8) -> bool {
        let mask = (1 << (stream - 1)) & 0xF;
        self.output_streams & mask == mask
    }

    /// Enable an [output stream](https://inform-fiction.org/zmachine/standards/z1point1/sect07.html#one)
    ///
    /// Stream 3 can stack ... enabling stream 3 when already enabled creates a new buffer
    /// that is written to until closed, at which time output is directed to the previous stream 3
    /// buffer.
    ///
    /// # Arguments
    /// * `stream` - stream to enable
    /// * `table` - Optional table address, required if `stream` is 3
    ///
    /// # Returns
    /// Empty [Result] or a [RuntimeError]
    fn enable_output_stream(
        &mut self,
        stream: u8,
        table: Option<usize>,
    ) -> Result<(), RuntimeError> {
        if (1..4).contains(&stream) {
            let mask = (1 << (stream - 1)) & 0xF;
            self.output_streams |= mask;
            debug!(target: "app::stream", "Enable output stream {} => {:04b}", stream, self.output_streams);
            // self.screen.output_stream(self.output_streams, table);
        }
        match stream {
            1 | 2 => Ok(()),
            3 => {
                if let Some(address) = table {
                    self.stream_3.push(Stream3::new(address));
                    Ok(())
                } else {
                    fatal_error!(
                        ErrorCode::Stream3Table,
                        "Stream 3 enabled without a table to write to"
                    )
                }
            }
            4 => fatal_error!(
                ErrorCode::InvalidOutputStream,
                "Stream 4 is not implemented yet"
            ),
            _ => fatal_error!(
                ErrorCode::InvalidOutputStream,
                "Stream {} is not a valid stream [1..4]",
                stream
            ),
        }
    }

    /// Disable an output stream
    ///
    /// When stream 3 is disabled, the contents of the stream buffer are written to its table.
    ///
    /// # Arguments
    /// * `stream` - stream to enable
    ///
    /// # Returns
    /// Empty [Result] or a [RuntimeError]
    fn disable_output_stream(&mut self, stream: u8) -> Result<(), RuntimeError> {
        let mask = (1 << (stream - 1)) & 0xF;
        debug!(target: "app::stream", "Disable output stream {} => {:04b}", stream, self.output_streams);
        match stream {
            1 | 2 => {
                self.output_streams &= !mask;
                Ok(())
            }
            3 => {
                if let Some(s) = self.stream_3.pop() {
                    let len = s.buffer.len();
                    self.memory.write_word(s.address, len as u16)?;
                    for i in 0..len {
                        self.memory
                            .write_byte(s.address + 2 + i, s.buffer[i] as u8)?;
                    }
                    if self.stream_3.is_empty() {
                        self.output_streams &= !mask;
                    }
                    Ok(())
                } else {
                    Ok(())
                }
            }
            4 => fatal_error!(
                ErrorCode::InvalidOutputStream,
                "Stream 4 is not implemented yet"
            ),
            _ => fatal_error!(
                ErrorCode::InvalidOutputStream,
                "Stream {} is not a valid stream [1..4]",
                stream
            ),
        }
    }

    /// Enable or disable an output stream
    ///
    /// # Arguments
    /// * `stream` - the stream to enable or disable; if positive, the stream is enabled, if negative it is disabled
    /// * `table` - optional table address, required if stream 3 is being enabled
    ///
    /// # Returns
    /// Empty [Result] or a [RuntimeError]
    pub fn output_stream(&mut self, stream: i16, table: Option<usize>) -> Result<(), RuntimeError> {
        match stream {
            1..=4 => {
                debug!(target: "app::stream", "Enabling output stream {}", stream);
                if stream == 2 {
                    // Set the transcript bit
                    let f2 = self.read_word(0x10)?;
                    self.memory.write_word(0x10, f2 | 1)?;
                    self.enable_output_stream(stream as u8, table)
                } else {
                    self.enable_output_stream(stream as u8, table)
                }
            }
            -4..=-1 => {
                debug!(target: "app::stream", "Disabling output stream {}", i16::abs(stream));
                if stream == -2 {
                    // Unset the transcript bit
                    let f2 = self.read_word(0x10)?;
                    self.write_word(0x10, f2 & 0xFFFE)?;
                }
                self.disable_output_stream(i16::abs(stream) as u8)
            }
            _ => recoverable_error!(
                ErrorCode::InvalidOutputStream,
                "Output stream {} is not valid: [-4..4]",
                stream
            ),
        }
    }

    /// Helper function to output text to output streams to be called by [processor] instructions.
    ///
    /// If stream 3 is enabled, output to other streams is halted.
    ///
    /// # Arguments
    /// * `text` - decoded text array to output
    /// * `next_address` - address of the next instruction to execute
    /// * `request_type` - the interpreter request type for the print operation
    ///
    /// # Returns
    /// [Result] with an instruction result that will contain an interpreter callback if output is to be sent to streams 1, 2, or 4, or a [RuntimeError]
    pub fn output(
        &mut self,
        text: &[u16],
        next_address: NextAddress,
        request_type: RequestType,
    ) -> Result<InstructionResult, RuntimeError> {
        // Stream 3 halts output to any other stream
        if self.is_stream_enabled(3) {
            if let Some(s) = self.stream_3.last_mut() {
                for c in text {
                    match *c {
                        0 => {}
                        0xa => s.push(0xd),
                        _ => s.push(*c),
                    }
                }
                InstructionResult::new(next_address)
            } else {
                fatal_error!(
                    ErrorCode::Stream3Table,
                    "Stream 3 enabled, but no table to write to"
                )
            }
        } else if self.is_stream_enabled(1) {
            // If this is a READ interrupt routine, set the redraw flag
            if self.is_read_interrupt()? {
                self.set_redraw_input()?;
            }

            debug!(target: "app::screen", "output(): {}", self.is_stream_enabled(2));

            match request_type {
                RequestType::PrintRet => InstructionResult::print_ret(
                    next_address,
                    text.to_vec(),
                    self.is_stream_enabled(2),
                ),
                RequestType::Print => {
                    InstructionResult::print(next_address, text.to_vec(), self.is_stream_enabled(2))
                }
                _ => InstructionResult::new(next_address),
            }
        } else {
            InstructionResult::new(next_address)
        }
    }

    pub fn new_line(
        &mut self,
        next_address: NextAddress,
    ) -> Result<InstructionResult, RuntimeError> {
        // Stream 3 halts output to any other stream
        if self.is_stream_enabled(3) {
            if let Some(s) = self.stream_3.last_mut() {
                s.push(0xd);
                InstructionResult::new(next_address)
            } else {
                fatal_error!(
                    ErrorCode::Stream3Table,
                    "Stream 3 enabled, but no table to write to"
                )
            }
        } else if self.is_stream_enabled(1) {
            // If this is a READ interrupt routine, set the redraw flag
            if self.is_read_interrupt()? {
                self.set_redraw_input()?;
            }

            InstructionResult::new_line(next_address, self.is_stream_enabled(2))
        } else {
            InstructionResult::new(next_address)
        }
    }

    // Save/Restore
    /// Restore game state from a [Quetzal](http://inform-fiction.org/zmachine/standards/quetzal/index.html) state
    ///
    /// # Arguments
    /// * `quetzal` - Quetzal state
    ///
    /// # Returns
    /// [Result] with an [Option] containing the address to resume execution at when the restore succeeds, [None] if it fails, or a [RuntimeError]
    pub fn restore_state(&mut self, quetzal: Quetzal) -> Result<Option<usize>, RuntimeError> {
        // Capture flags 2, default colors, rows, and columns from header
        let flags2 = header::field_word(&self.memory, HeaderField::Flags2)?;
        let fg = header::field_byte(&self.memory, HeaderField::DefaultForeground)?;
        let bg = header::field_byte(&self.memory, HeaderField::DefaultBackground)?;
        let rows = header::field_byte(&self.memory, HeaderField::ScreenLines)?;
        let columns = header::field_byte(&self.memory, HeaderField::ScreenColumns)?;

        // Overwrite dynamic memory
        if quetzal.mem().compressed() {
            self.memory.restore_compressed(quetzal.mem().memory())?
        } else {
            self.memory.restore(quetzal.mem().memory())?
        }

        // Reset the frame stack
        self.frames = Vec::from(quetzal.stks());

        // Re-initialize the state, which will set the default colors, rows, and columns
        // Ignore sound (for now), since it's in Flags2
        self.initialize(rows, columns, (fg, bg), false)?;

        // Restore flags 2
        self.write_word(HeaderField::Flags2 as usize, flags2)?;

        Ok(Some(quetzal.ifhd().pc() as usize))
    }

    /// Return a byte array containing the current game state in Quetzal format
    ///
    /// # Argument
    /// * `pc` - address of the branch or store result descriptor for the SAVE instruction
    ///
    /// # Returns
    /// [Result] containing the save data or a [RuntimeError]
    pub fn save_state(&self, pc: usize) -> Result<Vec<u8>, RuntimeError> {
        let quetzal = Quetzal::try_from((self, pc))?;
        Ok(Vec::from(quetzal))
    }

    // Runtime

    /// Decodes a single instruction at the current program counter address,
    /// executes it and returns control to the interpreter.
    ///
    /// If the instruction result contains an interpreter directive, that data
    /// is passed back to the interpreter, but the program counter is left
    /// as is and will be updated after the interpreter responds to the directive.
    ///
    /// If no directive is returned, the program counter is updated as the
    /// interpreter will generally just turn around and run the next instruction
    ///
    /// # Arguments
    /// * `response` - [Option] with the interpreter's response to a callback
    ///
    /// # Returns
    /// [Result] with an [Option] containing any interpreter callback or [None] or a [RuntimeError]
    pub fn execute(
        &mut self,
        response: Option<&InterpreterResponse>,
    ) -> Result<Option<InterpreterRequest>, RuntimeError> {
        debug!(target: "app::instruction", "PC: {:05x}, next PC: {:05x}", self.pc()?, self.next_pc()?);
        match response {
            None => {
                // No interpreter callback, advance to next instruction
                self.set_pc(self.next_pc()?)?;
                let instruction = decoder::decode_instruction(self, self.pc()?)?;
                match processor::dispatch(self, &instruction) {
                    Ok(result) => {
                        debug!(target: "app::instruction", "Instruction result: {:?}", result);
                        match result.interpreter_request() {
                            Some(req) => {
                                debug!(target: "app::instruction", "Interpreter callback: {:?}", req);
                                match result.next_address() {
                                    NextAddress::Address(a) => self.set_next_pc(*a)?,
                                    _ => {
                                        return fatal_error!(
                                            ErrorCode::InvalidInstruction,
                                            "InterpreterRequest next_address: {:?}",
                                            result.next_address()
                                        )
                                    }
                                }
                                Ok(Some(req.clone()))
                            }
                            None => {
                                match result.next_address() {
                                    // Instruction provides next address
                                    NextAddress::Address(a) => {
                                        self.set_next_pc(*a)?;
                                        Ok(None)
                                    }
                                    // QUIT
                                    NextAddress::Quit => Ok(InterpreterRequest::quit()),
                                    // READ_CHAR interrupt routine return
                                    // address is the READ_CHAR instruction
                                    NextAddress::ReadCharInterrupt(address, value) => {
                                        if *value == 0 {
                                            // READ_CHAR again
                                            self.set_next_pc(*address)?;
                                            Ok(None)
                                        } else {
                                            // Abort READ_CHAR
                                            let i = decoder::decode_instruction(self, *address)?;
                                            match i.store() {
                                                Some(v) => self.set_variable(v.variable(), 0)?,
                                                None => {
                                                    return fatal_error!(
                                                        ErrorCode::InvalidInstruction,
                                                        "READ_CHAR should have a store location"
                                                    )
                                                }
                                            }
                                            self.set_next_pc(i.next_address())?;
                                            Ok(None)
                                        }
                                    }
                                    NextAddress::ReadInterrupt(address, value, redraw) => {
                                        let i = decoder::decode_instruction(self, *address)?;
                                        let text_buffer = operand_values(self, &i)?[0] as usize;
                                        if *value == 0 {
                                            // Redraw existing input, if necessary
                                            if *redraw {
                                                let mut input = Vec::new();
                                                if self.version < 5 {
                                                    let mut i = 1;
                                                    loop {
                                                        let b =
                                                            self.read_byte(text_buffer + i)? as u16;
                                                        if b == 0 {
                                                            break;
                                                        }
                                                        input.push(b);
                                                        i += 1;
                                                    }
                                                } else {
                                                    let l =
                                                        self.read_byte(text_buffer + 1)? as usize;
                                                    for i in 0..l {
                                                        input.push(
                                                            self.read_byte(text_buffer + 2 + i)?
                                                                as u16,
                                                        );
                                                    }
                                                }
                                                Ok(InterpreterRequest::read_redraw(input))
                                            } else {
                                                self.set_next_pc(*address)?;
                                                Ok(None)
                                            }
                                        } else {
                                            if self.version < 5 {
                                                // Clear the input buffer
                                                let len = self.read_byte(text_buffer)? as usize - 1;
                                                for i in 0..len {
                                                    self.write_byte(text_buffer + i + 1, 0)?;
                                                }
                                            } else {
                                                // Set text buffer size to 0
                                                self.write_byte(text_buffer + 1, 0)?;
                                                // Store terminator 0
                                                match i.store() {
                                                    Some(v) => {
                                                        self.set_variable(v.variable(), 0)?
                                                    }
                                                    None => {
                                                        return fatal_error!(
                                                            ErrorCode::InvalidInstruction,
                                                            "READ should have a store location"
                                                        )
                                                    }
                                                }
                                            }
                                            self.set_next_pc(i.next_address())?;
                                            Ok(None)
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(mut e) => {
                        if e.is_recoverable() {
                            e.set_next_address(NextAddress::Address(instruction.next_address()));
                        }

                        Err(e)
                    }
                }
            }
            Some(res) => match res.response_type {
                ResponseType::GetCursor => {
                    let i = decode_instruction(self, self.pc()?)?;
                    let operands = processor::operand_values(self, &i)?;
                    self.write_word(operands[0] as usize, res.response.row)?;
                    self.write_word(operands[0] as usize + 2, res.response.column)?;
                    self.set_next_pc(i.next_address())?;
                    Ok(None)
                }
                ResponseType::ReadComplete => {
                    let i = decoder::decode_instruction(self, self.pc()?)?;
                    if res.response.terminator.zchar.unwrap() == 253
                        || res.response.terminator.zchar.unwrap() == 254
                    {
                        header::set_extension(
                            &mut self.memory,
                            1,
                            res.response.terminator.column.unwrap(),
                        )?;
                        header::set_extension(
                            &mut self.memory,
                            2,
                            res.response.terminator.row.unwrap(),
                        )?;
                    }
                    let r = processor_var::read_post(self, &i, res.response.input.clone())?;
                    if let NextAddress::Address(a) = r.next_address() {
                        self.set_next_pc(*a)?;
                        Ok(None)
                    } else {
                        fatal_error!(
                            ErrorCode::InvalidInstruction,
                            "READ should return an address"
                        )
                    }
                }
                ResponseType::ReadInterrupted => {
                    let i = decoder::decode_instruction(self, self.pc()?)?;
                    let operands = processor::operand_values(self, &i)?;
                    let text_buffer = operands[0] as usize;
                    let routine = if res.response.routine > 0 {
                        res.response.routine
                    } else {
                        self.packed_routine_address(operands[3])?
                    };

                    if self.version < 5 {
                        for (i, c) in res.response.input.iter().enumerate() {
                            self.write_byte(text_buffer + 1 + i, *c as u8)?
                        }
                    } else {
                        self.write_byte(text_buffer + 1, res.response.input.len() as u8)?;
                        for (i, c) in res.response.input.iter().enumerate() {
                            self.write_byte(text_buffer + 2 + i, *c as u8)?
                        }
                    }
                    match res.response.interrupt {
                        Interrupt::ReadTimeout => {
                            if let NextAddress::Address(a) =
                                self.call_read_interrupt(routine, &Vec::new(), self.pc()?)?
                            {
                                self.set_next_pc(a)?;
                                Ok(None)
                            } else {
                                fatal_error!(
                                    ErrorCode::InvalidInstruction,
                                    "calling routine should return address"
                                )
                            }
                        }
                        Interrupt::Sound => {
                            if let NextAddress::Address(a) =
                                self.call_routine(routine, &vec![], None, self.pc()?)?
                            {
                                self.set_next_pc(a)?;
                                Ok(None)
                            } else {
                                fatal_error!(
                                    ErrorCode::InvalidInstruction,
                                    "calling routine should return address"
                                )
                            }
                        }
                    }
                }
                ResponseType::ReadCharComplete => {
                    let i = decoder::decode_instruction(self, self.pc()?)?;
                    self.set_variable(
                        i.store()
                            .expect("READ_CHAR should have a store location")
                            .variable(),
                        res.response
                            .key
                            .zchar
                            .expect("Completed READ_CHAR should return a zchar"),
                    )?;
                    if res.response.key.zchar.unwrap() == 253
                        || res.response.key.zchar.unwrap() == 254
                    {
                        header::set_extension(
                            &mut self.memory,
                            1,
                            res.response.key.column.unwrap(),
                        )?;
                        header::set_extension(&mut self.memory, 2, res.response.key.row.unwrap())?;
                    }
                    Ok(None)
                }
                ResponseType::ReadCharInterrupted => {
                    let i = decoder::decode_instruction(self, self.pc()?)?;
                    let operands = processor::operand_values(self, &i)?;
                    let routine = self.packed_routine_address(operands[2])? as usize;
                    if let NextAddress::Address(a) =
                        self.call_read_char_interrupt(routine, &Vec::new(), self.pc()?)?
                    {
                        self.set_next_pc(a)?;
                        Ok(None)
                    } else {
                        fatal_error!(
                            ErrorCode::InvalidInstruction,
                            "calling routine should return address"
                        )
                    }
                }
                ResponseType::RestoreComplete => {
                    let i = decoder::decode_instruction(self, self.pc()?)?;
                    let quetzal = Quetzal::try_from(res.response.save_data.clone())?;
                    let ifhd = IFhd::try_from((&*self, 0))?;
                    if &ifhd != quetzal.ifhd() {
                        error!(target: "app::state", "Restore state was created from a different zcode program");
                        match self.version {
                            3 => match processor::branch(self, &i, false)? {
                                NextAddress::Address(a) => self.set_next_pc(a)?,
                                _ => {
                                    return fatal_error!(
                                        ErrorCode::InvalidInstruction,
                                        "RESTORE branch should return an address"
                                    )
                                }
                            },
                            4..=8 => match i.store() {
                                Some(v) => {
                                    self.set_variable(v.variable(), 0)?;
                                    self.set_next_pc(i.next_address())?;
                                }
                                None => {
                                    return fatal_error!(
                                        ErrorCode::InvalidInstruction,
                                        "RESTORE should have a store location"
                                    )
                                }
                            },
                            _ => {
                                return fatal_error!(
                                    ErrorCode::UnsupportedVersion,
                                    "Version {} is not supported",
                                    self.version
                                )
                            }
                        }
                        Ok(InterpreterRequest::message(
                            "Restore failed: file belongs to a different zcode program",
                        ))
                    } else {
                        let a = self.restore_state(quetzal)?;
                        debug!(target: "app::instruction", "Restore: {:?}", a);
                        if let Some(address) = a {
                            let inst_a = match self.version {
                                3 | 4 => address - 1,
                                5..=8 => address - 3,
                                _ => {
                                    return fatal_error!(
                                        ErrorCode::UnsupportedVersion,
                                        "Version {} is not supported",
                                        self.version
                                    )
                                }
                            };
                            let inst = decoder::decode_instruction(self, inst_a)?;
                            match self.version {
                                3 => match processor::branch(self, &inst, true)? {
                                    NextAddress::Address(a) => {
                                        self.set_next_pc(a)?;
                                    }
                                    _ => {
                                        return fatal_error!(
                                            ErrorCode::InvalidInstruction,
                                            "RESTORE branch should return an address"
                                        )
                                    }
                                },
                                4..=8 => match inst.store() {
                                    Some(v) => {
                                        self.set_variable(v.variable(), 2)?;
                                        self.set_next_pc(inst.next_address())?;
                                    }
                                    None => {
                                        return fatal_error!(
                                            ErrorCode::InvalidInstruction,
                                            "RESTORE should have a store location"
                                        )
                                    }
                                },
                                _ => {
                                    return fatal_error!(
                                        ErrorCode::UnsupportedVersion,
                                        "Version {} is not supported",
                                        self.version
                                    )
                                }
                            }
                            Ok(None)
                        } else {
                            match self.version() {
                                3 => match processor::branch(self, &i, false)? {
                                    NextAddress::Address(a) => self.set_next_pc(a)?,
                                    _ => {
                                        return fatal_error!(
                                            ErrorCode::InvalidInstruction,
                                            "RESTORE branch should return an address"
                                        )
                                    }
                                },
                                4..=8 => match i.store() {
                                    Some(v) => {
                                        self.set_variable(v.variable(), 0)?;
                                        self.set_next_pc(i.next_address())?;
                                    }
                                    None => {
                                        return fatal_error!(
                                            ErrorCode::InvalidInstruction,
                                            "RESTORE should have a store location"
                                        )
                                    }
                                },
                                _ => {
                                    return fatal_error!(
                                        ErrorCode::UnsupportedVersion,
                                        "Version {} is not supported",
                                        self.version
                                    )
                                }
                            }
                            Ok(None)
                        }
                    }
                }
                ResponseType::SaveComplete => {
                    let i = decoder::decode_instruction(self, self.pc()?)?;
                    match self.version {
                        3 => match processor::branch(self, &i, res.response.success)? {
                            NextAddress::Address(a) => self.set_next_pc(a)?,
                            _ => {
                                return fatal_error!(
                                    ErrorCode::InvalidInstruction,
                                    "SAVE branch should return an address"
                                )
                            }
                        },
                        4..=8 => match i.store() {
                            Some(v) => {
                                self.set_variable(
                                    v.variable(),
                                    if res.response.success { 1 } else { 0 },
                                )?;
                                self.set_next_pc(i.next_address())?;
                            }
                            None => {
                                return fatal_error!(
                                    ErrorCode::InvalidInstruction,
                                    "SAVE should have a store location"
                                )
                            }
                        },
                        _ => {
                            return fatal_error!(
                                ErrorCode::UnsupportedVersion,
                                "Version {} not supported",
                                self.version
                            );
                        }
                    }
                    Ok(None)
                }
                ResponseType::SetFont => {
                    let i = decoder::decode_instruction(self, self.pc()?)?;
                    self.set_variable(
                        i.store()
                            .expect("SET_FONT should have a store location")
                            .variable(),
                        res.response.font,
                    )?;
                    Ok(None)
                }
                ResponseType::SoundInterrupt => {
                    let r = self.call_routine(res.response.routine, &vec![], None, self.pc()?)?;
                    match r {
                        NextAddress::Address(a) => self.set_next_pc(a)?,
                        _ => {
                            return fatal_error!(
                                ErrorCode::InvalidInstruction,
                                "Call routine should return an Address"
                            )
                        }
                    }
                    Ok(None)
                }
            },
        }
    }

    /// Get the address of the currently executing instruction
    ///
    /// # Returns
    /// [Result] with the current pc or a [RuntimeError]
    pub fn pc(&self) -> Result<usize, RuntimeError> {
        Ok(self.current_frame()?.pc())
    }

    /// Set the address of the currently executing instruction
    ///
    /// # Arguments
    /// * `pc` - Address of the currently executing instruction
    ///
    /// # Returns
    /// Empty [Result] or a [RuntimeError]
    pub fn set_pc(&mut self, pc: usize) -> Result<(), RuntimeError> {
        self.current_frame_mut()?.set_pc(pc);
        Ok(())
    }

    /// Get the address of the next instruction to execute
    ///
    /// # Returns
    /// [Result] with the next pc or a [RuntimeError]
    pub fn next_pc(&self) -> Result<usize, RuntimeError> {
        Ok(self.current_frame()?.next_pc())
    }

    /// Set the address of the next instruction to execute
    ///
    /// # Arguments
    /// * `pc` - Address of the next instruction to execute
    ///
    /// # Returns
    /// Empty [Result] or a [RuntimeError]
    pub fn set_next_pc(&mut self, next_pc: usize) -> Result<(), RuntimeError> {
        self.current_frame_mut()?.set_next_pc(next_pc);
        Ok(())
    }
}
