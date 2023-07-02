use std::fmt;

use crate::{
    error::{ErrorCode, RuntimeError},
    iff::quetzal::{
        cmem::CMem,
        ifhd::IFhd,
        stks::{StackFrame, Stks},
        Quetzal,
    },
};

use self::{
    frame::Frame,
    header::{Flags1v3, Flags1v4, Flags2, HeaderField},
    memory::Memory,
};

use crate::instruction::StoreResult;

mod frame;
pub mod header;
pub mod memory;

pub struct State {
    version: u8,
    memory: Memory,
    static_mark: usize,
    frames: Vec<Frame>,
    undo_stack: Vec<Quetzal>,
    // read_interrupt_pending is set when the READ starts, read_interrupt_result is set when the interrupt routine returns
    read_interrupt_pending: bool,
    read_interrupt_result: Option<u16>,
    // sound_interrupt containts the address of the interrupt routine and is stored when SOUND_EFFECT is run
    sound_interrupt: Option<usize>,
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "State: version: {}, address space: {:06x}, dynamic space: {:04x}, execution frames: {}", self. version, self.memory.len(), self.static_mark - 1, self.frames.len())
    }
}

impl TryFrom<(&State, usize)> for Quetzal {
    type Error = RuntimeError;

    fn try_from((state, pc): (&State, usize)) -> Result<Self, Self::Error> {
        let ifhd = IFhd::try_from((state, pc))?;
        let cmem = CMem::try_from(state)?;
        let stks = Stks::try_from(state)?;

        let quetzal = Quetzal::new(ifhd, None, Some(cmem), stks);
        Ok(quetzal)
    }
}

impl TryFrom<&State> for CMem {
    type Error = RuntimeError;

    fn try_from(value: &State) -> Result<Self, Self::Error> {
        debug!(target: "app::quetzal", "Building CMem chunk from state");
        let compressed_memory = value.memory().compress();
        let cmem = CMem::new(&compressed_memory);
        debug!(target: "app::quetzal", "CMem: {}", cmem);
        Ok(cmem)
    }
}

impl TryFrom<(&State, usize)> for IFhd {
    type Error = RuntimeError;

    fn try_from((state, pc): (&State, usize)) -> Result<Self, Self::Error> {
        debug!(target: "app::quetzal", "Building IFhd chunk from state");

        let release_number = header::field_word(state, HeaderField::Release)?;
        let mut serial_number = Vec::new();
        for i in 0..6 {
            serial_number.push(state.read_byte(HeaderField::Serial as usize + i)?);
        }
        let checksum = header::field_word(state, HeaderField::Checksum)?;

        let ifhd = IFhd::new(
            release_number,
            &serial_number,
            checksum,
            (pc as u32) & 0xFFFFFF,
        );
        debug!(target: "app::quetzal", "IFhd: {}", ifhd);
        Ok(ifhd)
    }
}

impl TryFrom<&State> for Stks {
    type Error = RuntimeError;

    fn try_from(value: &State) -> Result<Self, Self::Error> {
        debug!(target: "app::quetzal", "Building Stks chunk from state");
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

            let frame = StackFrame::new(
                f.return_address() as u32,
                flags as u8,
                result_variable,
                arguments,
                &f.local_variables().clone(),
                &f.stack().clone(),
            );
            debug!(target: "app::quetzal", "Frame: {}", frame);
            frames.push(frame);
        }

        let stks = Stks::new(frames);
        Ok(stks)
    }
}

impl State {
    pub fn new(memory: Memory) -> Result<State, RuntimeError> {
        let version = memory.read_byte(0)?;
        let static_mark = memory.read_word(HeaderField::StaticMark as usize)? as usize;
        Ok(State {
            version,
            memory,
            static_mark,
            frames: Vec::new(),
            undo_stack: Vec::new(),
            read_interrupt_pending: false,
            read_interrupt_result: None,
            sound_interrupt: None,
        })
    }

    pub fn version(&self) -> u8 {
        self.version
    }

    pub fn memory(&self) -> &Memory {
        &self.memory
    }

    // pub fn frames(&self) -> &Vec<Frame> {
    //     &self.frames
    // }

    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    fn current_frame(&self) -> Result<&Frame, RuntimeError> {
        if let Some(frame) = self.frames.last() {
            Ok(frame)
        } else {
            Err(RuntimeError::new(
                ErrorCode::StackUnderflow,
                "No runtime frame".to_string(),
            ))
        }
    }

    fn current_frame_mut(&mut self) -> Result<&mut Frame, RuntimeError> {
        if let Some(frame) = self.frames.last_mut() {
            Ok(frame)
        } else {
            Err(RuntimeError::new(
                ErrorCode::StackUnderflow,
                "No runtime frame".to_string(),
            ))
        }
    }

    pub fn initialize(
        &mut self,
        rows: u8,
        columns: u8,
        default_colors: (u8, u8),
        sound: bool,
    ) -> Result<(), RuntimeError> {
        // Clear any pending interrupt
        self.read_interrupt_pending = false;
        self.read_interrupt_result = None;
        self.sound_interrupt = None;

        // Set V3 Flags 1
        if self.version < 4 {
            header::clear_flag1(self, Flags1v3::StatusLineNotAvailable as u8)?;
            header::set_flag1(self, Flags1v3::ScreenSplitAvailable as u8)?;
            header::clear_flag1(self, Flags1v3::VariablePitchDefault as u8)?;
        }

        // Set V4+ Flags 1
        if self.version > 3 {
            header::set_byte(self, HeaderField::DefaultBackground, default_colors.1)?;
            header::set_byte(self, HeaderField::DefaultForeground, default_colors.0)?;
            header::set_byte(self, HeaderField::ScreenLines, rows)?;
            header::set_byte(self, HeaderField::ScreenColumns, columns)?;

            header::set_flag1(self, Flags1v4::SoundEffectsAvailable as u8)?;
        }

        // Set V5+ Flags 1
        if self.version > 4 {
            header::set_word(self, HeaderField::ScreenHeight, rows as u16)?;
            header::set_word(self, HeaderField::ScreenWidth, columns as u16)?;
            header::set_byte(self, HeaderField::FontWidth, 1)?;
            header::set_byte(self, HeaderField::FontHeight, 1)?;
            header::clear_flag1(self, Flags1v4::PicturesAvailable as u8)?;
            header::set_flag1(self, Flags1v4::ColoursAvailable as u8)?;
            header::set_flag1(self, Flags1v4::BoldfaceAvailable as u8)?;
            header::set_flag1(self, Flags1v4::ItalicAvailable as u8)?;
            header::set_flag1(self, Flags1v4::FixedSpaceAvailable as u8)?;
            header::set_flag1(self, Flags1v4::TimedInputAvailable as u8)?;
            //header::clear_flag2(&mut self.memory, Flags2::RequestMouse)?;
            // Graphics font 3 support is crap atm
            header::clear_flag2(self, Flags2::RequestPictures)?;
            // If sounds weren't loaded
            if !sound {
                header::clear_flag2(self, Flags2::RequestSoundEffects)?;
            }
        }

        // Interpreter # and version
        self.write_byte(0x1E, 6)?;
        self.write_byte(0x1F, b'Z')?;

        // Z-Machine standard compliance
        self.write_byte(0x32, 1)?;
        self.write_byte(0x33, 0)?;

        if self.frames.is_empty() {
            let pc = header::field_word(self, HeaderField::InitialPC)? as usize;
            let f = Frame::new(pc, pc, &vec![], 0, &vec![], None, 0);
            self.frames.clear();
            self.frames.push(f);
        }

        Ok(())
    }

    // MMU - read up to address $FFFF, write to dynamic memory only
    pub fn read_byte(&self, address: usize) -> Result<u8, RuntimeError> {
        if address < 0x10000 {
            self.memory.read_byte(address)
        } else {
            Err(RuntimeError::new(
                ErrorCode::IllegalAccess,
                format!("Byte address {:#06x} is in high memory", address),
            ))
        }
    }

    pub fn read_word(&self, address: usize) -> Result<u16, RuntimeError> {
        if address < 0xFFFF {
            self.memory.read_word(address)
        } else {
            Err(RuntimeError::new(
                ErrorCode::IllegalAccess,
                format!("Word address {:#06x} is in high memory", address),
            ))
        }
    }

    pub fn write_byte(&mut self, address: usize, value: u8) -> Result<(), RuntimeError> {
        if address < self.static_mark {
            self.memory.write_byte(address, value)
        } else {
            Err(RuntimeError::new(
                ErrorCode::IllegalAccess,
                format!(
                    "Byte address {:#04x} is above the end of dynamic memory ({:#04x})",
                    address, self.static_mark
                ),
            ))
        }
    }

    pub fn write_word(&mut self, address: usize, value: u16) -> Result<(), RuntimeError> {
        if address < self.static_mark - 1 {
            self.memory.write_word(address, value)?;
            Ok(())
        } else {
            Err(RuntimeError::new(
                ErrorCode::IllegalAccess,
                format!(
                    "Word address {:#04x} is above the end of dynamic memory ({:#04x})",
                    address, self.static_mark
                ),
            ))
        }
    }

    pub fn checksum(&self) -> Result<u16, RuntimeError> {
        self.memory.checksum()
    }

    // Variables
    fn global_variable_address(&self, variable: u8) -> Result<usize, RuntimeError> {
        let table = header::field_word(self, HeaderField::GlobalTable)? as usize;
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
        if variable < 16 {
            self.current_frame_mut()?
                .set_local_variable(variable, value)
        } else {
            let address = self.global_variable_address(variable)?;
            self.write_word(address, value)
        }
    }

    pub fn set_variable_indirect(&mut self, variable: u8, value: u16) -> Result<(), RuntimeError> {
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

    // Helper functions to read potentially "unreadable" memory
    pub fn instruction(&self, address: usize) -> Vec<u8> {
        // An instruction may be up to 23 bytes long, excluding literal strings
        // Opcode: up to 2 bytes
        // Operand types: up to 2 bytes
        // Operands: up to 16 bytes
        // Store variable: up to 1 byte
        // Branch offset: up to 2 bytes
        self.memory().slice(address, 23)
    }

    fn routine_header(&self, address: usize) -> Result<(usize, Vec<u16>), RuntimeError> {
        let variable_count = self.memory.read_byte(address)? as usize;
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

    pub fn packed_routine_address(&self, address: u16) -> Result<usize, RuntimeError> {
        match self.version {
            1 | 2 | 3 => Ok(address as usize * 2),
            4 | 5 => Ok(address as usize * 4),
            7 => Ok((address as usize * 4)
                + (self
                    .memory
                    .read_word(HeaderField::RoutinesOffset as usize)? as usize
                    * 8)),
            8 => Ok(address as usize * 8),
            _ => Err(RuntimeError::new(
                ErrorCode::UnsupportedVersion,
                format!("Unsupported version: {}", self.version),
            )),
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
            _ => Err(RuntimeError::new(
                ErrorCode::UnsupportedVersion,
                format!("Unsupported version: {}", self.version),
            )),
        }
    }

    // Routines/Interrupts
    pub fn is_input_interrupt(&self) -> bool {
        self.read_interrupt_result.is_some()
    }

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
        return_address: usize,
    ) -> Result<usize, RuntimeError> {
        if self.read_interrupt_pending {
            debug!(target: "app::frame", "Read interrupt routine firing @ ${:06x}", address);
            self.clear_read_interrupt();
            self.read_interrupt_result = Some(0);
            let (initial_pc, local_variables) = self.routine_header(address)?;
            let frame =
                Frame::call_input_interrupt(address, initial_pc, local_variables, return_address)?;

            self.frames.push(frame);
            Ok(initial_pc)
        } else {
            Err(RuntimeError::new(
                ErrorCode::System,
                "No read interrupt pending".to_string(),
            ))
        }
    }

    pub fn read_interrupt_pending(&self) -> bool {
        self.read_interrupt_pending
    }

    pub fn set_read_interrupt(&mut self) {
        self.read_interrupt_pending = true;
    }

    pub fn read_interrupt_result(&self) -> Option<u16> {
        self.read_interrupt_result
    }

    pub fn clear_read_interrupt(&mut self) {
        self.read_interrupt_pending = false;
        self.read_interrupt_result = None;
    }

    pub fn sound_interrupt(&self) -> Option<usize> {
        self.sound_interrupt
    }

    pub fn set_sound_interrupt(&mut self, address: usize) {
        self.sound_interrupt = Some(address);
    }

    pub fn clear_sound_interrupt(&mut self) {
        self.sound_interrupt = None;
    }

    pub fn call_sound_interrupt(&mut self, return_address: usize) -> Result<usize, RuntimeError> {
        if let Some(address) = self.sound_interrupt {
            let (initial_pc, local_variables) = self.routine_header(address)?;
            let frame = Frame::call_routine(
                address,
                initial_pc,
                &vec![],
                local_variables,
                None,
                return_address,
            )?;
            self.frames.push(frame);
            self.clear_sound_interrupt();
            Ok(initial_pc)
        } else {
            Err(RuntimeError::new(
                ErrorCode::System,
                "No pending interrupt".to_string(),
            ))
        }
    }

    pub fn return_routine(&mut self, value: u16) -> Result<usize, RuntimeError> {
        if let Some(f) = self.frames.pop() {
            let n = self.current_frame_mut()?;
            n.set_pc(f.return_address());
            debug!(target: "app::frame", "Return to ${:06x} -> {:?}", f.return_address(), f.result());
            if f.input_interrupt() {
                if self.read_interrupt_pending {
                    self.read_interrupt_result = Some(value);
                }
            } else {
                match f.result() {
                    Some(r) => self.set_variable(r.variable(), value)?,
                    None => (),
                }
            }

            Ok(self.current_frame()?.pc())
        } else {
            Err(RuntimeError::new(
                ErrorCode::System,
                "No frame to return to".to_string(),
            ))
        }
    }

    pub fn throw(&mut self, depth: u16, result: u16) -> Result<usize, RuntimeError> {
        self.frames.truncate(depth as usize);
        self.return_routine(result)
    }

    pub fn pc(&self) -> Result<usize, RuntimeError> {
        Ok(self.current_frame()?.pc())
    }

    pub fn set_pc(&mut self, pc: usize) -> Result<(), RuntimeError> {
        self.current_frame_mut()?.set_pc(pc);
        Ok(())
    }

    pub fn argument_count(&self) -> Result<u8, RuntimeError> {
        Ok(self.current_frame()?.argument_count())
    }

    // Save/Restore
    pub fn save(&self, pc: usize) -> Result<Vec<u8>, RuntimeError> {
        let quetzal = Quetzal::try_from((self, pc))?;
        debug!(target: "app::quetzal", "Saving game state");
        // trace!(target: "app::quetzal", "{}", quetzal);
        Ok(Vec::from(quetzal))
    }

    fn restore_state(&mut self, quetzal: Quetzal) -> Result<Option<usize>, RuntimeError> {
        // Reset the frame stack
        self.frames = Vec::from(quetzal.stks());

        // Capture flags 2, default colors, rows, and columns from header
        let flags2 = header::field_word(self, HeaderField::Flags2)?;
        let fg = header::field_byte(self, HeaderField::DefaultForeground)?;
        let bg = header::field_byte(self, HeaderField::DefaultBackground)?;
        let rows = header::field_byte(self, HeaderField::ScreenLines)?;
        let columns = header::field_byte(self, HeaderField::ScreenColumns)?;

        // Overwrite dynamic memory
        if let Some(umem) = quetzal.umem() {
            self.memory.restore(umem.data())?
        } else if let Some(cmem) = quetzal.cmem() {
            self.memory.restore_compressed(cmem.data())?
        } else {
            error!(target: "app::quetzal", "No CMem/Umem chunk found in save state");
            return Err(RuntimeError::new(
                ErrorCode::Restore,
                "No CMem/UMem chunk in save file".to_string(),
            ));
        }

        // Re-initialize the state, which will set the default colors, rows, and columns
        // Ignore sound (for now), since it's in Flags2
        self.initialize(rows, columns, (fg, bg), false)?;

        // Restore flags 2
        self.write_word(HeaderField::Flags2 as usize, flags2)?;

        Ok(Some(quetzal.ifhd().pc() as usize))
    }

    pub fn restore(&mut self, data: Vec<u8>) -> Result<Option<usize>, RuntimeError> {
        let quetzal = Quetzal::try_from(data)?;
        debug!(target: "app::quetzal", "Restoring game state");
        // trace!(target: "app::quetzal", "{}", quetzal);
        // &*self is an immutable ref, necessary for try_from
        let ifhd = IFhd::try_from((&*self, 0))?;
        if &ifhd != quetzal.ifhd() {
            error!(target: "app::quetzal", "Save file was created from a different story file");
            Err(RuntimeError::new(
                ErrorCode::Restore,
                "Save file was created from a different story file".to_string(),
            ))
        } else {
            self.restore_state(quetzal)
        }
    }

    pub fn save_undo(&mut self, pc: usize) -> Result<(), RuntimeError> {
        let quetzal = Quetzal::try_from((&*self, pc))?;
        debug!(target: "app::quetzal", "Storing undo state");
        self.undo_stack.push(quetzal);
        self.undo_stack.truncate(10);
        Ok(())
    }

    pub fn restore_undo(&mut self) -> Result<Option<usize>, RuntimeError> {
        if let Some(quetzal) = self.undo_stack.pop() {
            debug!(target: "app::quetzal", "Restoring undo state");
            self.restore_state(quetzal)
        } else {
            warn!(target: "app::quetzal", "No saved state for undo");
            Err(RuntimeError::new(
                ErrorCode::Restore,
                "Undo stack is empty".to_string(),
            ))
        }
    }

    pub fn restart(&mut self) -> Result<usize, RuntimeError> {
        // Capture flags 2, default colors, rows, and columns from header
        let flags2 = header::field_word(self, HeaderField::Flags2)?;
        let fg = header::field_byte(self, HeaderField::DefaultForeground)?;
        let bg = header::field_byte(self, HeaderField::DefaultBackground)?;
        let rows = header::field_byte(self, HeaderField::ScreenLines)?;
        let columns = header::field_byte(self, HeaderField::ScreenColumns)?;

        self.memory.reset();
        self.frames.clear();

        self.initialize(rows, columns, (fg, bg), false)?;
        self.write_word(HeaderField::Flags2 as usize, flags2)?;

        Ok(self.current_frame()?.pc())
    }
}
