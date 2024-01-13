use std::{
    collections::{HashSet, VecDeque},
    fs::File,
};

use pancurses::Input;

use crate::{
    config::Config,
    error::{ErrorCode, RuntimeError},
    fatal_error,
    instruction::{
        decoder::{self, decode_instruction},
        processor::{self, operand_values, processor_0op, processor_ext, processor_var},
        Instruction, InstructionResult, NextAddress, Operand, StoreResult,
    },
    object::property,
    quetzal::{IFhd, Mem, Quetzal, Stk, Stks},
    recoverable_error, text, zmachine,
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
pub enum ErrorHandling {
    ContinueWarnAlways,
    ContinueWarnOnce,
    Ignore,
    Abort,
}

#[derive(Debug)]
struct Stream3 {
    address: usize,
    buffer: Vec<u16>,
}

impl Stream3 {
    pub fn new(address: usize) -> Stream3 {
        Stream3 {
            address,
            buffer: Vec::new(),
        }
    }

    pub fn address(&self) -> usize {
        self.address
    }

    pub fn buffer(&self) -> &Vec<u16> {
        &self.buffer
    }

    pub fn push(&mut self, c: u16) {
        self.buffer.push(c);
    }
}

#[derive(Debug, Default, Eq, PartialEq)]
pub enum Interrupt {
    #[default]
    ReadTimeout,
    Sound,
}

#[derive(Debug, Default, Eq, PartialEq)]
pub struct InputEvent {
    zchar: Option<u16>,
    row: Option<u16>,
    column: Option<u16>,
    interrupt: Option<Interrupt>,
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

#[derive(Clone, Debug)]
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
pub struct RequestPayload {
    // Messaging
    message: String,

    // BufferMode
    buffer_mode: u16,

    // EraseWindow
    window_erase: i16,

    // OutputStream
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

    // ...interrupted
    next_instruction_address: usize,
    instruction_address: usize,

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
    pub fn message(&self) -> &str {
        &self.message
    }

    // EraseWindow
    pub fn window_erase(&self) -> i16 {
        self.window_erase
    }

    // BufferMode
    pub fn buffer_mode(&self) -> u16 {
        self.buffer_mode
    }

    // Print/PrintRet
    pub fn text(&self) -> &Vec<u16> {
        &self.text
    }

    // PrintTable
    pub fn table(&self) -> &Vec<u16> {
        &self.table
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }

    pub fn skip(&self) -> u16 {
        self.skip
    }

    // Read
    pub fn length(&self) -> u8 {
        self.length
    }

    pub fn terminators(&self) -> &Vec<u16> {
        &self.terminators
    }

    pub fn timeout(&self) -> u16 {
        self.timeout
    }

    pub fn input(&self) -> &Vec<u16> {
        &self.input
    }

    pub fn instruction_address(&self) -> usize {
        self.instruction_address
    }

    pub fn next_instruction_address(&self) -> usize {
        self.next_instruction_address
    }

    // Save
    pub fn save_data(&self) -> &Vec<u8> {
        &self.save_data
    }

    // SetColour
    pub fn foreground(&self) -> u16 {
        self.foreground
    }

    pub fn background(&self) -> u16 {
        self.background
    }

    // SetCursor
    pub fn row(&self) -> u16 {
        self.row
    }

    // SetFont
    pub fn font(&self) -> u16 {
        self.font
    }

    pub fn column(&self) -> u16 {
        self.column
    }

    // SetTextStyle
    pub fn style(&self) -> u16 {
        self.style
    }

    // SetWindow
    pub fn window_set(&self) -> u16 {
        self.window_set
    }

    // ShowStatus
    pub fn status_left(&self) -> &Vec<u16> {
        &self.status_left
    }

    pub fn status_right(&self) -> &Vec<u16> {
        &self.status_right
    }

    // SoundEffect
    pub fn number(&self) -> u16 {
        self.number
    }

    pub fn effect(&self) -> u16 {
        self.effect
    }

    pub fn volume(&self) -> u8 {
        self.volume
    }

    pub fn repeats(&self) -> u8 {
        self.repeats
    }

    pub fn routine(&self) -> usize {
        self.routine
    }

    // SplitWindow
    pub fn split_lines(&self) -> u16 {
        self.split_lines
    }
}

/// Request for the interpreter (screen, sound) to do something
#[derive(Clone, Debug)]
pub struct InterpreterRequest {
    request_type: RequestType,
    request: RequestPayload,
}

impl InterpreterRequest {
    pub fn request_type(&self) -> &RequestType {
        &self.request_type
    }

    pub fn request(&self) -> &RequestPayload {
        &self.request
    }

    pub fn message(message: &str) -> Option<InterpreterRequest> {
        Some(InterpreterRequest {
            request_type: RequestType::Message,
            request: RequestPayload {
                message: message.to_string(),
                ..Default::default()
            },
        })
    }

    pub fn buffer_mode(mode: u16) -> Option<InterpreterRequest> {
        Some(InterpreterRequest {
            request_type: RequestType::BufferMode,
            request: RequestPayload {
                buffer_mode: mode,
                ..Default::default()
            },
        })
    }

    pub fn erase_line() -> Option<InterpreterRequest> {
        Some(InterpreterRequest {
            request_type: RequestType::EraseLine,
            request: RequestPayload::default(),
        })
    }

    pub fn erase_window(window: i16) -> Option<InterpreterRequest> {
        Some(InterpreterRequest {
            request_type: RequestType::EraseWindow,
            request: RequestPayload {
                window_erase: window,
                ..Default::default()
            },
        })
    }

    pub fn get_cursor() -> Option<InterpreterRequest> {
        Some(InterpreterRequest {
            request_type: RequestType::GetCursor,
            request: RequestPayload::default(),
        })
    }

    pub fn new_line() -> Option<InterpreterRequest> {
        Some(InterpreterRequest {
            request_type: RequestType::NewLine,
            request: RequestPayload {
                ..Default::default()
            },
        })
    }

    pub fn output_stream(stream: i16) -> Option<InterpreterRequest> {
        Some(InterpreterRequest {
            request_type: RequestType::OutputStream,
            request: RequestPayload {
                stream,
                ..Default::default()
            },
        })
    }

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

    pub fn print_table(
        table: Vec<u16>,
        width: u16,
        height: u16,
        skip: u16,
    ) -> Option<InterpreterRequest> {
        Some(InterpreterRequest {
            request_type: RequestType::PrintTable,
            request: RequestPayload {
                table,
                width,
                height,
                skip,
                ..Default::default()
            },
        })
    }

    pub fn quit() -> Option<InterpreterRequest> {
        Some(InterpreterRequest {
            request_type: RequestType::Quit,
            request: RequestPayload::default(),
        })
    }

    pub fn read(
        length: u8,
        terminators: Vec<u16>,
        timeout: u16,
        input: Vec<u16>,
        redraw: bool,
    ) -> Option<InterpreterRequest> {
        Some(InterpreterRequest {
            request_type: RequestType::Read,
            request: RequestPayload {
                length,
                terminators,
                timeout,
                input,
                ..Default::default()
            },
        })
    }

    pub fn read_abort(address: usize, next_address: usize) -> Option<InterpreterRequest> {
        Some(InterpreterRequest {
            request_type: RequestType::ReadAbort,
            request: RequestPayload {
                instruction_address: address,
                next_instruction_address: next_address,
                ..Default::default()
            },
        })
    }

    pub fn read_redraw(address: usize, input: Vec<u16>) -> Option<InterpreterRequest> {
        Some(InterpreterRequest {
            request_type: RequestType::ReadRedraw,
            request: RequestPayload {
                instruction_address: address,
                input,
                ..Default::default()
            },
        })
    }

    pub fn read_char(timeout: u16) -> Option<InterpreterRequest> {
        Some(InterpreterRequest {
            request_type: RequestType::ReadChar,
            request: RequestPayload {
                timeout,
                ..Default::default()
            },
        })
    }

    pub fn restore(name: &str) -> Option<InterpreterRequest> {
        Some(InterpreterRequest {
            request_type: RequestType::Restore,
            request: RequestPayload {
                name: name.to_string(),
                ..Default::default()
            },
        })
    }

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

    pub fn set_font(font: u16) -> Option<InterpreterRequest> {
        Some(InterpreterRequest {
            request_type: RequestType::SetFont,
            request: RequestPayload {
                font,
                ..Default::default()
            },
        })
    }

    pub fn set_text_style(style: u16) -> Option<InterpreterRequest> {
        Some(InterpreterRequest {
            request_type: RequestType::SetTextStyle,
            request: RequestPayload {
                style,
                ..Default::default()
            },
        })
    }

    pub fn set_window(window: u16) -> Option<InterpreterRequest> {
        Some(InterpreterRequest {
            request_type: RequestType::SetWindow,
            request: RequestPayload {
                window_set: window,
                ..Default::default()
            },
        })
    }

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
pub struct ResponsePayload {
    // GET_CURSOR
    row: u16,
    column: u16,

    // READ
    input: Vec<u16>,
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

impl ResponsePayload {
    pub fn key(&self) -> &InputEvent {
        &self.key
    }

    pub fn input(&self) -> &Vec<u16> {
        &self.input
    }
}

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

// Response from the interpreter to an InterpreterRequest
pub struct InterpreterResponse {
    response_type: ResponseType,
    response: ResponsePayload,
}

impl InterpreterResponse {
    pub fn response_type(&self) -> &ResponseType {
        &self.response_type
    }

    pub fn response(&self) -> &ResponsePayload {
        &self.response
    }

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

    pub fn read_complete(input: Vec<u16>) -> Option<InterpreterResponse> {
        Some(InterpreterResponse {
            response_type: ResponseType::ReadComplete,
            response: ResponsePayload {
                input,
                ..Default::default()
            },
        })
    }

    pub fn read_char_complete(key: InputEvent) -> Option<InterpreterResponse> {
        Some(InterpreterResponse {
            response_type: ResponseType::ReadCharComplete,
            response: ResponsePayload {
                key,
                ..Default::default()
            },
        })
    }

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

    pub fn read_char_interrupted(interrupt: Interrupt) -> Option<InterpreterResponse> {
        Some(InterpreterResponse {
            response_type: ResponseType::ReadCharInterrupted,
            response: ResponsePayload {
                interrupt,
                ..Default::default()
            },
        })
    }

    pub fn restore(save_data: Vec<u8>) -> Option<InterpreterResponse> {
        Some(InterpreterResponse {
            response_type: ResponseType::RestoreComplete,
            response: ResponsePayload {
                save_data,
                ..Default::default()
            },
        })
    }

    pub fn save(success: bool) -> Option<InterpreterResponse> {
        Some(InterpreterResponse {
            response_type: ResponseType::SaveComplete,
            response: ResponsePayload {
                success,
                ..Default::default()
            },
        })
    }

    pub fn set_font(font: u16) -> Option<InterpreterResponse> {
        Some(InterpreterResponse {
            response_type: ResponseType::SetFont,
            response: ResponsePayload {
                font,
                ..Default::default()
            },
        })
    }

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

pub struct ZMachine {
    name: String,
    version: u8,
    memory: Memory,
    rng: Box<dyn ZRng>,
    frames: Vec<Frame>,
    undo_stack: VecDeque<Quetzal>,
    errors: HashSet<ErrorCode>,
    error_handling: ErrorHandling,
    output_streams: u8,
    stream_2: Option<File>,
    stream_3: Vec<Stream3>,
    instruction_count: usize,
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
        let error_handling = config.error_handling();
        let mut zm = ZMachine {
            name: name.to_string(),
            version,
            memory,
            rng: Box::new(rng),
            frames: Vec::new(),
            undo_stack: VecDeque::new(),
            errors: HashSet::new(),
            error_handling,
            output_streams: 0x1,
            stream_2: None,
            stream_3: Vec::new(),
            instruction_count: 0,
        };

        zm.initialize(
            rows,
            columns,
            (config.foreground(), config.background()),
            false,
        )?;
        Ok(zm)
    }

    pub fn version(&self) -> u8 {
        self.version
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn initialize(
        &mut self,
        rows: u8,
        columns: u8,
        default_colors: (u8, u8),
        sound: bool,
    ) -> Result<(), RuntimeError> {
        // Clear any pending interrupt
        // self.read_interrupt_pending = false;
        // self.read_interrupt_result = None;
        // self.sound_interrupt = None;

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
    pub fn read_byte(&self, address: usize) -> Result<u8, RuntimeError> {
        self.memory.read_byte(address)
    }

    pub fn read_word(&self, address: usize) -> Result<u16, RuntimeError> {
        self.memory.read_word(address)
    }

    pub fn write_byte(&mut self, address: usize, value: u8) -> Result<(), RuntimeError> {
        self.memory.write_byte(address, value)
    }

    pub fn write_word(&mut self, address: usize, value: u16) -> Result<(), RuntimeError> {
        self.memory.write_word(address, value)
    }

    pub fn checksum(&self) -> Result<u16, RuntimeError> {
        self.memory.checksum()
    }

    // Save/restore
    pub fn save(&self, pc: usize) -> Result<(), RuntimeError> {
        let quetzal = Quetzal::try_from((self, pc))?;
        debug!(target: "app::state", "Game state encoded");
        Err(RuntimeError::fatal(
            ErrorCode::UnimplementedInstruction,
            "Save TBD".to_string(),
        ))
        // Ok(())
        //self.prompt_and_write("Save to: ", "ifzs", &Vec::from(quetzal), false)

        // Ok(Vec::from(quetzal))
    }

    // fn restore_state(&mut self, quetzal: Quetzal) -> Result<Option<usize>, RuntimeError> {
    //     // Capture flags 2, default colors, rows, and columns from header
    //     let flags2 = header::field_word(&self.memory, HeaderField::Flags2)?;
    //     let fg = header::field_byte(&self.memory, HeaderField::DefaultForeground)?;
    //     let bg = header::field_byte(&self.memory, HeaderField::DefaultBackground)?;
    //     let rows = header::field_byte(&self.memory, HeaderField::ScreenLines)?;
    //     let columns = header::field_byte(&self.memory, HeaderField::ScreenColumns)?;

    //     // Overwrite dynamic memory
    //     if quetzal.mem().compressed() {
    //         self.memory.restore_compressed(quetzal.mem().memory())?
    //     } else {
    //         self.memory.restore(quetzal.mem().memory())?
    //     }

    //     // Reset the frame stack
    //     self.frames = Vec::from(quetzal.stks());

    //     // Re-initialize the state, which will set the default colors, rows, and columns
    //     // Ignore sound (for now), since it's in Flags2
    //     self.initialize(rows, columns, (fg, bg), false)?;

    //     // Restore flags 2
    //     self.write_word(HeaderField::Flags2 as usize, flags2)?;

    //     Ok(Some(quetzal.ifhd().pc() as usize))
    // }

    // pub fn restore(&mut self) -> Result<Option<usize>, RuntimeError> {
    //     Err(RuntimeError::fatal(
    //         ErrorCode::UnimplementedInstruction,
    //         "Restore TBD".to_string(),
    //     ))
    //     // match self.prompt_and_read("Restore from: ", "ifzs") {
    //     //     Ok(save_data) => {
    //     //         let quetzal = Quetzal::try_from(save_data)?;
    //     //         debug!(target: "app::state", "Restoring game state");
    //     //         // trace!(target: "app::quetzal", "{}", quetzal);
    //     //         // &*self is an immutable ref, necessary for try_from
    //     //         let ifhd = IFhd::try_from((&*self, 0))?;
    //     //         if &ifhd != quetzal.ifhd() {
    //     //             error!(target: "app::state", "Restore state was created from a different story file");
    //     //             recoverable_error!(
    //     //                 ErrorCode::Restore,
    //     //                 "Save file was created from a different story file"
    //     //             )
    //     //         } else {
    //     //             self.restore_state(quetzal)
    //     //         }
    //     //             },
    //     //     Err(e) => {
    //     //         error!(target: "app::state", "Error restoring state: {}", e);
    //     //         Err(e)
    //     //     }
    //     // }
    // }

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

    pub fn restore_undo(&mut self) -> Result<Option<usize>, RuntimeError> {
        if let Some(quetzal) = self.undo_stack.pop_back() {
            debug!(target: "app::state", "Restoring undo state");
            self.restore_state(quetzal)
        } else {
            warn!(target: "app::state", "No saved state for undo");
            recoverable_error!(ErrorCode::UndoNoState, "Undo stack is empty")
        }
    }

    pub fn restart(&mut self) -> Result<usize, RuntimeError> {
        self.rng.seed(0);

        let flags2 = header::field_word(&self.memory, HeaderField::Flags2)?;
        let fg = header::field_byte(&self.memory, HeaderField::DefaultForeground)?;
        let bg = header::field_byte(&self.memory, HeaderField::DefaultBackground)?;
        let rows = header::field_byte(&self.memory, HeaderField::ScreenLines)?;
        let columns = header::field_byte(&self.memory, HeaderField::ScreenColumns)?;

        self.memory.reset();
        self.frames.clear();

        self.initialize(rows, columns, (fg, bg), false)?;
        self.write_word(HeaderField::Flags2 as usize, flags2)?;

        Ok(self.current_frame()?.pc())
    }

    // Unmanaged memory access: string literals, routines
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

    pub fn instruction(&self, address: usize) -> Vec<u8> {
        // An instruction may be up to 23 bytes long, excluding literal strings
        // Opcode: up to 2 bytes
        // Operand types: up to 8 (2 bytes)
        // Operands: up to 8 (16 bytes)
        // Store variable: up to 1 byte
        // Branch offset: up to 2 bytes
        self.memory.slice(address, 23)
    }

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
    pub fn packed_routine_address(&self, address: u16) -> Result<usize, RuntimeError> {
        match self.version {
            3 => Ok(address as usize * 2),
            4 | 5 => Ok(address as usize * 4),
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

    pub fn packed_string_address(&self, address: u16) -> Result<usize, RuntimeError> {
        match self.version {
            1 | 2 | 3 => Ok(address as usize * 2),
            4 | 5 => Ok(address as usize * 4),
            7 => Ok((address as usize * 4)
                + (self.memory.read_word(HeaderField::StringsOffset as usize)? as usize * 8)),
            8 => Ok(address as usize * 8),
            // TODO: error
            _ => fatal_error!(
                ErrorCode::UnsupportedVersion,
                "Unsupported version: {}",
                self.version
            ),
        }
    }

    // Header
    pub fn header_byte(&self, field: HeaderField) -> Result<u8, RuntimeError> {
        header::field_byte(&self.memory, field)
    }

    pub fn header_word(&self, field: HeaderField) -> Result<u16, RuntimeError> {
        header::field_word(&self.memory, field)
    }

    // Frame stack
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    fn current_frame(&self) -> Result<&Frame, RuntimeError> {
        if let Some(frame) = self.frames.last() {
            Ok(frame)
        } else {
            fatal_error!(ErrorCode::NoFrame, "No runtime frame")
        }
    }

    fn current_frame_mut(&mut self) -> Result<&mut Frame, RuntimeError> {
        if let Some(frame) = self.frames.last_mut() {
            Ok(frame)
        } else {
            fatal_error!(ErrorCode::NoFrame, "No runtime frame")
        }
    }

    // Routines
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

    pub fn call_read_interrupt(
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
            let mut frame = Frame::call_routine(
                address,
                initial_pc,
                arguments,
                local_variables,
                result,
                return_address,
            )?;
            frame.set_read_interrupt(true);
            self.frames.push(frame);

            Ok(NextAddress::Address(initial_pc))
        }
    }

    pub fn call_read_char_interrupt(
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
            let mut frame = Frame::call_routine(
                address,
                initial_pc,
                arguments,
                local_variables,
                result,
                return_address,
            )?;
            frame.set_read_char_interrupt(true);
            self.frames.push(frame);

            Ok(NextAddress::Address(initial_pc))
        }
    }

    pub fn is_read_interrupt(&self) -> Result<bool, RuntimeError> {
        Ok(self.current_frame()?.read_interrupt())
    }

    pub fn is_read_char_interrupt(&self) -> Result<bool, RuntimeError> {
        Ok(self.current_frame()?.read_char_interrupt())
    }

    pub fn set_redraw_input(&mut self) -> Result<(), RuntimeError> {
        self.current_frame_mut()?.set_redraw_input(true);
        Ok(())
    }

    pub fn return_routine(&mut self, value: u16) -> Result<NextAddress, RuntimeError> {
        if let Some(f) = self.frames.pop() {
            debug!(target: "app::state", "Return {:04x} => {:?} to ${:06x}, redraw: {}", value, f.result(), f.return_address(), f.redraw_input());
            if let Some(r) = f.result() {
                self.set_variable(r.variable(), value)?;
            }

            let n = self.current_frame_mut()?;
            n.set_next_pc(f.return_address());

            if f.read_interrupt() {
                debug!(target: "app::screen", "Return from READ interrupt");
                Ok(NextAddress::ReadInterrupt(
                    f.return_address(),
                    value,
                    f.redraw_input(),
                ))
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

    pub fn argument_count(&self) -> Result<u8, RuntimeError> {
        Ok(self.current_frame()?.argument_count())
    }

    pub fn throw(&mut self, depth: u16, result: u16) -> Result<NextAddress, RuntimeError> {
        self.frames.truncate(depth as usize);
        self.return_routine(result)
    }

    // Variables
    fn global_variable_address(&self, variable: u8) -> Result<usize, RuntimeError> {
        let table = header::field_word(&self.memory, HeaderField::GlobalTable)? as usize;
        let index = (variable as usize - 16) * 2;
        Ok(table + index)
    }

    pub fn variable(&mut self, variable: u8) -> Result<u16, RuntimeError> {
        if variable < 16 {
            self.current_frame_mut()?.local_variable(variable)
        } else {
            let address = self.global_variable_address(variable)?;
            self.read_word(address)
        }
    }

    pub fn peek_variable(&mut self, variable: u8) -> Result<u16, RuntimeError> {
        if variable < 16 {
            self.current_frame()?.peek_local_variable(variable)
        } else {
            let address = self.global_variable_address(variable)?;
            self.read_word(address)
        }
    }

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

    pub fn push(&mut self, value: u16) -> Result<(), RuntimeError> {
        self.current_frame_mut()?.set_local_variable(0, value)
    }

    // Status line
    pub fn status_line(&mut self) -> Result<(Vec<u16>, Vec<u16>), RuntimeError> {
        let status_type = header::flag1(&self.memory, Flags1v3::StatusLineType as u8)?;
        let object = self.variable(16)? as usize;
        let mut left = text::from_vec(self, &property::short_name(self, object)?, false)?;
        let mut right: Vec<u16> = if status_type == 0 {
            // Score is between -99 and 999 inclusive
            let score = i16::min(999, i16::max(-99, self.variable(17)? as i16));
            // Turns is between 0 and 9999 inclusive
            let turns = u16::min(9999, self.variable(18)?);
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
        // self.io.status_line(&mut left, &mut right)
    }

    // RNG
    pub fn random(&mut self, range: u16) -> u16 {
        self.rng.random(range)
    }

    pub fn seed(&mut self, seed: u16) {
        self.rng.seed(seed)
    }

    pub fn predictable(&mut self, seed: u16) {
        self.rng.predictable(seed)
    }

    // Streams
    fn is_stream_2_open(&self) -> bool {
        self.stream_2.is_some()
    }

    fn set_stream_2(&mut self, file: File) {
        self.stream_2 = Some(file)
    }

    pub fn is_stream_enabled(&self, stream: u8) -> bool {
        let mask = (1 << (stream - 1)) & 0xF;
        self.output_streams & mask == mask
    }

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
                    self.memory.write_word(s.address(), len as u16)?;
                    for i in 0..len {
                        self.memory
                            .write_byte(s.address + 2 + i, s.buffer()[i] as u8)?;
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

    fn start_stream_2(&mut self) -> Result<(), RuntimeError> {
        Err(RuntimeError::recoverable(
            ErrorCode::UnimplementedInstruction,
            "Stream 2 not implemented yet".to_string(),
        ))
        // let file = self.prompt_and_create("Transcript file name: ", "txt", false)?;
        // self.io.set_stream_2(file);
    }

    pub fn output_stream(&mut self, stream: i16, table: Option<usize>) -> Result<(), RuntimeError> {
        match stream {
            1..=4 => {
                debug!(target: "app::stream", "Enabling output stream {}", stream);
                if stream == 2 {
                    if !self.is_stream_2_open() {
                        if let Err(e) = self.start_stream_2() {
                            error!(target: "app::stream", "Error starting stream 2: {}", e);
                            return recoverable_error!(
                                ErrorCode::Transcript,
                                "Error creating transcript file: {}",
                                e
                            );
                        }
                    }
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

    pub fn output(
        &mut self,
        text: &[u16],
        next_address: NextAddress,
        print_ret: bool,
    ) -> Result<InstructionResult, RuntimeError> {
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
            if self.is_read_interrupt()? {
                self.set_redraw_input()?;
            }

            if print_ret {
                InstructionResult::print_ret(next_address, text.to_vec(), self.is_stream_enabled(2))
            } else {
                InstructionResult::print(next_address, text.to_vec(), self.is_stream_enabled(2))
            }
        } else {
            InstructionResult::new(next_address)
        }
    }

    // Save/Restore
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

    pub fn restore_post(
        &mut self,
        instruction: &Instruction,
        data: Vec<u8>,
    ) -> Result<InstructionResult, RuntimeError> {
        if self.version < 5 {
            processor_0op::restore_post(self, instruction, data)
        } else {
            processor_ext::restore_post(self, instruction, data)
        }
    }

    pub fn save_state(&self, pc: usize) -> Result<Vec<u8>, RuntimeError> {
        let quetzal = Quetzal::try_from((self, pc))?;
        Ok(Vec::from(quetzal))
    }

    pub fn save_post(
        &mut self,
        instruction: &Instruction,
        success: bool,
    ) -> Result<InstructionResult, RuntimeError> {
        if self.version < 5 {
            processor_0op::save_post(self, instruction, success)
        } else {
            processor_ext::save_post(self, instruction, success)
        }
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
                                                Ok(InterpreterRequest::read_redraw(*address, input))
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
                                    _ => Ok(Some(InterpreterRequest::quit().unwrap())),
                                }
                            }
                        }
                    }
                    Err(e) => Err(e),
                }
            }
            Some(res) => match res.response_type() {
                ResponseType::GetCursor => {
                    let i = decode_instruction(self, self.pc()?)?;
                    let operands = processor::operand_values(self, &i)?;
                    self.write_word(operands[0] as usize, res.response().row)?;
                    self.write_word(operands[0] as usize + 2, res.response().column)?;
                    self.set_next_pc(i.next_address())?;
                    Ok(None)
                }
                ResponseType::ReadComplete => {
                    let i = decoder::decode_instruction(self, self.pc()?)?;
                    let r = processor_var::read_post(self, &i, res.response().input().clone())?;
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
                    let routine = if res.response().routine > 0 {
                        res.response().routine
                    } else {
                        self.packed_routine_address(operands[3])?
                    };

                    if self.version < 5 {
                        for (i, c) in res.response().input.iter().enumerate() {
                            self.write_byte(text_buffer + 1 + i, *c as u8)?
                        }
                    } else {
                        self.write_byte(text_buffer + 1, res.response().input.len() as u8)?;
                        for (i, c) in res.response().input.iter().enumerate() {
                            self.write_byte(text_buffer + 2 + i, *c as u8)?
                        }
                    }
                    match res.response().interrupt {
                        Interrupt::ReadTimeout => {
                            if let NextAddress::Address(a) =
                                self.call_read_interrupt(routine, &Vec::new(), None, self.pc()?)?
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
                            if let NextAddress::Address(a) = self.call_routine(
                                routine,
                                &vec![],
                                None,
                                self.pc()?,
                            )? {
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
                        res.response()
                            .key()
                            .zchar()
                            .expect("Completed READ_CHAR should return a zchar"),
                    )?;
                    Ok(None)
                }
                ResponseType::ReadCharInterrupted => {
                    let i = decoder::decode_instruction(self, self.pc()?)?;
                    let operands = processor::operand_values(self, &i)?;
                    let routine = self.packed_routine_address(operands[2])? as usize;
                    if let NextAddress::Address(a) =
                        self.call_read_interrupt(routine, &Vec::new(), None, self.pc()?)?
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
                    let quetzal = Quetzal::try_from(res.response().save_data.clone())?;
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
                                    self.set_next_pc(i.next_address());
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
                                4..=8 => match i.store() {
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
                                        self.set_next_pc(i.next_address());
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
                        3 => match processor::branch(self, &i, res.response().success)? {
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
                                    if res.response().success { 1 } else { 0 },
                                )?;
                                self.set_next_pc(i.next_address());
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
                        res.response().font,
                    )?;
                    Ok(None)
                }
                ResponseType::SoundInterrupt => {
                    let r = self.call_routine(res.response().routine, &vec![], None, self.pc()?)?;
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

    // Store cursor position
    pub fn get_cursor_post(
        &mut self,
        instruction: &Instruction,
        row: u16,
        column: u16,
    ) -> Result<InstructionResult, RuntimeError> {
        processor_var::get_cursor_post(self, instruction, row, column)
    }

    // Process input
    pub fn read_post(
        &mut self,
        instruction: &Instruction,
        input: Vec<u16>,
    ) -> Result<InstructionResult, RuntimeError> {
        processor_var::read_post(self, instruction, input)
    }

    // Read timed out
    pub fn read_interrupted(
        &mut self,
        instruction: &Instruction,
        input: &[u16],
    ) -> Result<InstructionResult, RuntimeError> {
        processor_var::read_interrupted(self, instruction, input)
    }

    // Read aborted after interrupt
    pub fn read_abort(
        &mut self,
        instruction: &Instruction,
    ) -> Result<InstructionResult, RuntimeError> {
        processor_var::read_abort(self, instruction)
    }

    pub fn read_char_post(
        &mut self,
        instruction: &Instruction,
        key: InputEvent,
    ) -> Result<InstructionResult, RuntimeError> {
        processor_var::read_char_post(self, instruction, key)
    }

    pub fn read_char_interrupted(
        &mut self,
        instruction: &Instruction,
    ) -> Result<InstructionResult, RuntimeError> {
        processor_var::read_char_interrupted(self, instruction)
    }

    pub fn read_char_abort(
        &mut self,
        instruction: &Instruction,
    ) -> Result<InstructionResult, RuntimeError> {
        processor_var::read_char_abort(self, instruction)
    }

    pub fn set_font_post(
        &mut self,
        instruction: &Instruction,
        old_font: u8,
    ) -> Result<InstructionResult, RuntimeError> {
        processor_ext::set_font_post(self, instruction, old_font)
    }

    pub fn pc(&self) -> Result<usize, RuntimeError> {
        Ok(self.current_frame()?.pc())
    }

    pub fn set_pc(&mut self, pc: usize) -> Result<(), RuntimeError> {
        self.current_frame_mut()?.set_pc(pc);
        Ok(())
    }

    pub fn next_pc(&self) -> Result<usize, RuntimeError> {
        Ok(self.current_frame()?.next_pc())
    }

    pub fn set_next_pc(&mut self, next_pc: usize) -> Result<(), RuntimeError> {
        self.current_frame_mut()?.set_next_pc(next_pc);
        Ok(())
    }
}
