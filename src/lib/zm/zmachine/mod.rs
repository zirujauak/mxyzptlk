use std::{
    collections::{HashSet, VecDeque},
    fs::File,
};

use crate::{
    config::Config,
    error::{ErrorCode, RuntimeError},
    fatal_error,
    instruction::{processor, Instruction},
    object::property,
    quetzal::{IFhd, Mem, Quetzal, Stk, Stks},
    recoverable_error, text,
    types::{DirectiveRequest, InstructionResult, StoreResult, Directive},
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
        config: Config,
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

    fn restore_state(&mut self, quetzal: Quetzal) -> Result<Option<usize>, RuntimeError> {
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

    pub fn restore(&mut self) -> Result<Option<usize>, RuntimeError> {
        Err(RuntimeError::fatal(
            ErrorCode::UnimplementedInstruction,
            "Restore TBD".to_string(),
        ))
        // match self.prompt_and_read("Restore from: ", "ifzs") {
        //     Ok(save_data) => {
        //         let quetzal = Quetzal::try_from(save_data)?;
        //         debug!(target: "app::state", "Restoring game state");
        //         // trace!(target: "app::quetzal", "{}", quetzal);
        //         // &*self is an immutable ref, necessary for try_from
        //         let ifhd = IFhd::try_from((&*self, 0))?;
        //         if &ifhd != quetzal.ifhd() {
        //             error!(target: "app::state", "Restore state was created from a different story file");
        //             recoverable_error!(
        //                 ErrorCode::Restore,
        //                 "Save file was created from a different story file"
        //             )
        //         } else {
        //             self.restore_state(quetzal)
        //         }
        //             },
        //     Err(e) => {
        //         error!(target: "app::state", "Error restoring state: {}", e);
        //         Err(e)
        //     }
        // }
    }

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
    ) -> Result<usize, RuntimeError> {
        // Call to address 0 results in FALSE
        if address == 0 {
            if let Some(r) = result {
                self.set_variable(r.variable(), 0)?;
            }
            Ok(return_address)
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

            Ok(initial_pc)
        }
    }

    pub fn call_read_interrupt(
        &mut self,
        address: usize,
        arguments: &Vec<u16>,
        result: Option<StoreResult>,
        return_address: usize,
    ) -> Result<usize, RuntimeError> {
        // Call to address 0 results in FALSE
        if address == 0 {
            if let Some(r) = result {
                self.set_variable(r.variable(), 0)?;
            }
            Ok(return_address)
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
            frame.set_input_interrupt(true);
            self.frames.push(frame);

            Ok(initial_pc)
        }
    }

    pub fn is_input_interrupt(&self) -> Result<bool, RuntimeError> {
        Ok(self.current_frame()?.input_interrupt())
    }

    pub fn set_redraw_input(&mut self) -> Result<(), RuntimeError> {
        self.current_frame_mut()?.set_redraw_input(true);
        Ok(())
    }

    pub fn return_routine(&mut self, value: u16) -> Result<InstructionResult, RuntimeError> {
        if let Some(f) = self.frames.pop() {
            let n = self.current_frame_mut()?;
            n.set_pc(f.return_address());
            debug!(target: "app::state", "Return {:04x} => {:?} to ${:06x}", value, f.result(), f.return_address());
            if let Some(r) = f.result() {
                self.set_variable(r.variable(), value)?;
            }

            if f.input_interrupt() {
                debug!(target: "app::screen", "Return from input interrupt");
                Ok(InstructionResult::new(
                    Directive::ReadInterruptReturn,
                    DirectiveRequest::read_interrupt_return(value, f.redraw_input()),
                    f.return_address(),
                ))
            } else {
                // TODO: Interrupts
                // if f.input_interrupt() {
                //     if self.read_interrupt_pending {
                //         self.read_interrupt_result = Some(value);
                //     }
                // } else if let Some(r) = f.result() {
                //     self.set_variable(r.variable(), value)?
                // }

                Ok(InstructionResult::none(self.current_frame()?.pc()))
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

    pub fn throw(&mut self, depth: u16, result: u16) -> Result<InstructionResult, RuntimeError> {
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

    // Runtime
    pub fn execute(
        &mut self,
        instruction: &Instruction,
    ) -> Result<InstructionResult, RuntimeError> {
        processor::dispatch(self, &instruction)
    }

    pub fn pc(&self) -> Result<usize, RuntimeError> {
        Ok(self.current_frame()?.pc())
    }

    pub fn set_pc(&mut self, pc: usize) -> Result<(), RuntimeError> {
        self.current_frame_mut()?.set_pc(pc);
        Ok(())
    }
}
