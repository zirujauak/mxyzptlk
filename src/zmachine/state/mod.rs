use std::{collections::VecDeque, fmt};

use crate::{
    error::{ErrorCode, RuntimeError},
    fatal_error,
    quetzal::{IFhd, Mem, Quetzal, Stk, Stks},
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

#[derive(Debug)]
pub struct State {
    version: u8,
    memory: Memory,
    static_mark: usize,
    frames: Vec<Frame>,
    undo_stack: VecDeque<Quetzal>,
    // read_interrupt_pending is set when the READ starts, read_interrupt_result is set when the interrupt routine returns
    read_interrupt_pending: bool,
    read_interrupt_result: Option<u16>,
    // sound_interrupt containts the address of the interrupt routine and is stored when SOUND_EFFECT is run
    sound_interrupt: Option<usize>,
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "State: version: {}, address space: {:06x}, dynamic space: {:04x}, execution frames: {}", self. version, self.memory.size(), self.static_mark - 1, self.frames.len())
    }
}

impl TryFrom<(&State, usize)> for Quetzal {
    type Error = RuntimeError;

    fn try_from((state, pc): (&State, usize)) -> Result<Self, Self::Error> {
        let ifhd = IFhd::try_from((state, pc))?;
        let mem = Mem::try_from(state)?;
        let stks = Stks::try_from(state)?;

        let quetzal = Quetzal::new(ifhd, mem, stks);
        Ok(quetzal)
    }
}

impl TryFrom<&State> for Mem {
    type Error = RuntimeError;

    fn try_from(value: &State) -> Result<Self, Self::Error> {
        debug!(target: "app::quetzal", "Building CMem chunk from state");
        let compressed_memory = value.memory().compress();
        let mem = Mem::new(true, compressed_memory);
        // debug!(target: "app::quetzal", "CMem: {:?}", mem);
        Ok(mem)
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
            undo_stack: VecDeque::new(),
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

    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    fn current_frame(&self) -> Result<&Frame, RuntimeError> {
        if let Some(frame) = self.frames.last() {
            Ok(frame)
        } else {
            fatal_error!(ErrorCode::StackUnderflow, "No runtime frame")
        }
    }

    fn current_frame_mut(&mut self) -> Result<&mut Frame, RuntimeError> {
        if let Some(frame) = self.frames.last_mut() {
            Ok(frame)
        } else {
            fatal_error!(ErrorCode::StackUnderflow, "No runtime frame")
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

        // Set V3 flags
        if self.version < 4 {
            header::clear_flag1(self, Flags1v3::StatusLineNotAvailable as u8)?;
            header::set_flag1(self, Flags1v3::ScreenSplitAvailable as u8)?;
            header::clear_flag1(self, Flags1v3::VariablePitchDefault as u8)?;
        }

        // Set V4+ flags and header fields
        if self.version > 3 {
            if sound {
                header::set_flag1(self, Flags1v4::SoundEffectsAvailable as u8)?;
            }

            header::set_byte(self, HeaderField::DefaultBackground, default_colors.1)?;
            header::set_byte(self, HeaderField::DefaultForeground, default_colors.0)?;
            header::set_byte(self, HeaderField::ScreenLines, rows)?;
            header::set_byte(self, HeaderField::ScreenColumns, columns)?;
        }

        // Set V5+ flags and header fields
        if self.version > 4 {
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

            header::set_word(self, HeaderField::ScreenHeight, rows as u16)?;
            header::set_word(self, HeaderField::ScreenWidth, columns as u16)?;
            header::set_byte(self, HeaderField::FontWidth, 1)?;
            header::set_byte(self, HeaderField::FontHeight, 1)?;
        }

        // Interpreter # and version
        self.write_byte(0x1E, 6)?;
        self.write_byte(0x1F, b'Z')?;

        // Z-Machine standard compliance
        self.write_byte(0x32, 1)?;
        self.write_byte(0x33, 0)?;

        // Initializing after a restore will already have stack frames,
        // so check before pushing a dummy frame
        if self.frames.is_empty() {
            let pc = header::field_word(self, HeaderField::InitialPC)? as usize;
            let f = Frame::new(pc, pc, &[], 0, &[], None, 0);
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
            fatal_error!(
                ErrorCode::IllegalAccess,
                "Byte address {:#06x} is in high memory",
                address
            )
        }
    }

    pub fn read_word(&self, address: usize) -> Result<u16, RuntimeError> {
        if address < 0xFFFF {
            self.memory.read_word(address)
        } else {
            fatal_error!(
                ErrorCode::IllegalAccess,
                "Word address {:#06x} is in high memory",
                address
            )
        }
    }

    pub fn write_byte(&mut self, address: usize, value: u8) -> Result<(), RuntimeError> {
        if address < self.static_mark {
            self.memory.write_byte(address, value)
        } else {
            fatal_error!(
                ErrorCode::IllegalAccess,
                "Byte address {:#04x} is above the end of dynamic memory ({:#04x})",
                address,
                self.static_mark
            )
        }
    }

    pub fn write_word(&mut self, address: usize, value: u16) -> Result<(), RuntimeError> {
        if address < self.static_mark - 1 {
            self.memory.write_word(address, value)?;
            Ok(())
        } else {
            fatal_error!(
                ErrorCode::IllegalAccess,
                "Word address {:#04x} is above the end of dynamic memory ({:#04x})",
                address,
                self.static_mark
            )
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

    // Helper functions to read code that may lie in high memory - instruction,
    // routines, strings
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

    // Unpack addresses
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
            self.read_interrupt_result = Some(0);
            let initial_pc = self.call_routine(address, &vec![], None, return_address)?;
            self.current_frame_mut()?.set_input_interrupt(true);
            Ok(initial_pc)
        } else {
            fatal_error!(ErrorCode::System, "No read interrupt pending")
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
            let initial_pc = self.call_routine(address, &vec![], None, return_address)?;
            self.current_frame_mut()?.set_sound_interrupt(true);
            self.clear_sound_interrupt();
            Ok(initial_pc)
        } else {
            fatal_error!(ErrorCode::System, "No pending interrupt")
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
            } else if let Some(r) = f.result() {
                self.set_variable(r.variable(), value)?
            }

            Ok(self.current_frame()?.pc())
        } else {
            fatal_error!(ErrorCode::System, "No frame to return to")
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
        // Capture flags 2, default colors, rows, and columns from header
        let flags2 = header::field_word(self, HeaderField::Flags2)?;
        let fg = header::field_byte(self, HeaderField::DefaultForeground)?;
        let bg = header::field_byte(self, HeaderField::DefaultBackground)?;
        let rows = header::field_byte(self, HeaderField::ScreenLines)?;
        let columns = header::field_byte(self, HeaderField::ScreenColumns)?;

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

    pub fn restore(&mut self, data: Vec<u8>) -> Result<Option<usize>, RuntimeError> {
        let quetzal = Quetzal::try_from(data)?;
        debug!(target: "app::quetzal", "Restoring game state");
        // trace!(target: "app::quetzal", "{}", quetzal);
        // &*self is an immutable ref, necessary for try_from
        let ifhd = IFhd::try_from((&*self, 0))?;
        if &ifhd != quetzal.ifhd() {
            error!(target: "app::quetzal", "Save file was created from a different story file");
            fatal_error!(
                ErrorCode::Restore,
                "Save file was created from a different story file"
            )
        } else {
            self.restore_state(quetzal)
        }
    }

    pub fn save_undo(&mut self, pc: usize) -> Result<(), RuntimeError> {
        let quetzal = Quetzal::try_from((&*self, pc))?;
        debug!(target: "app::quetzal", "Storing undo state");
        self.undo_stack.push_back(quetzal);
        while self.undo_stack.len() > 10 {
            // Remove the first (oldest) entries
            self.undo_stack.pop_front();
        }
        Ok(())
    }

    pub fn restore_undo(&mut self) -> Result<Option<usize>, RuntimeError> {
        if let Some(quetzal) = self.undo_stack.pop_back() {
            debug!(target: "app::quetzal", "Restoring undo state");
            self.restore_state(quetzal)
        } else {
            warn!(target: "app::quetzal", "No saved state for undo");
            fatal_error!(ErrorCode::Restore, "Undo stack is empty")
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

#[cfg(test)]
mod tests {
    use crate::{assert_ok, assert_ok_eq, assert_some, assert_some_eq, test_util::test_map};

    use super::*;

    #[test]
    fn test_quetzal_try_from() {
        let mut map = test_map(3);
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        map[0x02] = 0x12;
        map[0x03] = 0x34;
        map[0x12] = b'2';
        map[0x13] = b'3';
        map[0x14] = b'0';
        map[0x15] = b'7';
        map[0x16] = b'1';
        map[0x17] = b'5';
        map[0x1C] = 0x56;
        map[0x1D] = 0x78;

        let m = Memory::new(map.clone());
        let mut state = assert_ok!(State::new(m));
        // See memory.rs tests ... change dynamic memory a little bit
        // so the compressed memory isn't just runs of 0s
        assert!(state.write_byte(0x200, 0xFC).is_ok());
        assert!(state.write_byte(0x280, 0x10).is_ok());
        assert!(state.write_byte(0x300, 0xFD).is_ok());

        state.frames.push(Frame::new(
            0x500,
            0x501,
            &[0x1122, 0x3344, 0x5566],
            2,
            &[0x1111, 0x2222],
            Some(StoreResult::new(0x402, 0x80)),
            0x48E,
        ));
        state.frames.push(Frame::new(
            0x480,
            0x48C,
            &[0x8899, 0xaabb],
            0,
            &[],
            None,
            0x623,
        ));

        let quetzal = assert_ok!(Quetzal::try_from((&state, 0x494)));
        // let cmem = assert_some!(quetzal.mem());
        assert_eq!(
            quetzal.mem().memory(),
            &vec![0x00, 0xFF, 0x00, 0xFF, 0xFC, 0x00, 0x7E, 0x90, 0x00, 0x7E, 0xFD, 0x00, 0xFE]
        );
        assert_eq!(quetzal.ifhd().release_number(), 0x1234);
        assert_eq!(
            quetzal.ifhd().serial_number(),
            &[b'2', b'3', b'0', b'7', b'1', b'5']
        );
        assert_eq!(quetzal.ifhd().checksum(), 0x5678);
        assert_eq!(quetzal.ifhd().pc(), 0x494);
        assert_eq!(quetzal.stks().stks().len(), 2);
        assert_eq!(quetzal.stks().stks()[0].return_address(), 0x48E);
        assert_eq!(quetzal.stks().stks()[0].flags(), 0x3);
        assert_eq!(quetzal.stks().stks()[0].result_variable(), 0x80);
        assert_eq!(quetzal.stks().stks()[0].arguments(), 0x3);
        assert_eq!(
            quetzal.stks().stks()[0].variables(),
            &[0x1122, 0x3344, 0x5566]
        );
        assert_eq!(quetzal.stks().stks()[0].stack(), &[0x1111, 0x2222]);
        assert_eq!(quetzal.stks().stks()[1].return_address(), 0x623);
        assert_eq!(quetzal.stks().stks()[1].flags(), 0x12);
        assert_eq!(quetzal.stks().stks()[1].result_variable(), 0);
        assert_eq!(quetzal.stks().stks()[1].arguments(), 0);
        assert_eq!(quetzal.stks().stks()[1].variables(), &[0x8899, 0xaabb]);
        assert!(quetzal.stks().stks()[1].stack().is_empty());
    }

    #[test]
    fn test_mem_try_from() {
        let mut map = test_map(3);
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        let m = Memory::new(map.clone());
        let mut state = assert_ok!(State::new(m));
        // See memory.rs tests ... change dynamic memory a little bit
        // so the compressed memory isn't just runs of 0s
        assert!(state.write_byte(0x200, 0xFC).is_ok());
        assert!(state.write_byte(0x280, 0x10).is_ok());
        assert!(state.write_byte(0x300, 0xFD).is_ok());

        let mem = assert_ok!(Mem::try_from(&state));
        assert_eq!(
            mem.memory(),
            &vec![0x00, 0xFF, 0x00, 0xFF, 0xFC, 0x00, 0x7E, 0x90, 0x00, 0x7E, 0xFD, 0x00, 0xFE]
        );
    }

    #[test]
    fn test_ifhd_try_from() {
        let mut map = test_map(3);
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        map[0x02] = 0x12;
        map[0x03] = 0x34;
        map[0x12] = b'2';
        map[0x13] = b'3';
        map[0x14] = b'0';
        map[0x15] = b'7';
        map[0x16] = b'1';
        map[0x17] = b'5';
        map[0x1C] = 0x56;
        map[0x1D] = 0x78;

        let m = Memory::new(map.clone());
        let state = assert_ok!(State::new(m));
        let ifhd = assert_ok!(IFhd::try_from((&state, 0x9abc)));
        assert_eq!(ifhd.release_number(), 0x1234);
        assert_eq!(ifhd.serial_number(), &[b'2', b'3', b'0', b'7', b'1', b'5']);
        assert_eq!(ifhd.checksum(), 0x5678);
        assert_eq!(ifhd.pc(), 0x9abc);
    }

    #[test]
    fn test_stks_try_from() {
        let mut map = test_map(3);
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        let m = Memory::new(map.clone());
        let mut state = assert_ok!(State::new(m));
        state.frames.push(Frame::new(
            0x500,
            0x501,
            &[0x1122, 0x3344, 0x5566],
            2,
            &[0x1111, 0x2222],
            Some(StoreResult::new(0x402, 0x80)),
            0x48E,
        ));
        state.frames.push(Frame::new(
            0x480,
            0x48C,
            &[0x8899, 0xaabb],
            0,
            &[],
            None,
            0x623,
        ));

        let stks = assert_ok!(Stks::try_from(&state));
        assert_eq!(stks.stks().len(), 2);
        assert_eq!(stks.stks()[0].return_address(), 0x48E);
        assert_eq!(stks.stks()[0].flags(), 0x3);
        assert_eq!(stks.stks()[0].result_variable(), 0x80);
        assert_eq!(stks.stks()[0].arguments(), 0x3);
        assert_eq!(stks.stks()[0].variables(), &[0x1122, 0x3344, 0x5566]);
        assert_eq!(stks.stks()[0].stack(), &[0x1111, 0x2222]);
        assert_eq!(stks.stks()[1].return_address(), 0x623);
        assert_eq!(stks.stks()[1].flags(), 0x12);
        assert_eq!(stks.stks()[1].result_variable(), 0);
        assert_eq!(stks.stks()[1].arguments(), 0);
        assert_eq!(stks.stks()[1].variables(), &[0x8899, 0xaabb]);
        assert!(stks.stks()[1].stack().is_empty());
    }

    #[test]
    fn test_constructor() {
        let map = test_map(3);
        let m = Memory::new(map.clone());
        let state = assert_ok!(State::new(m));
        assert_eq!(state.version(), 3);
        assert_eq!(state.memory().slice(0, 0x800), map);
        assert_eq!(state.static_mark, 0x400);
        assert!(state.frames.is_empty());
        assert_eq!(state.frame_count(), 0);
        assert!(state.undo_stack.is_empty());
        assert!(!state.read_interrupt_pending());
        assert!(state.read_interrupt_result().is_none());
        assert!(state.sound_interrupt().is_none());
    }

    #[test]
    fn test_current_frame() {
        let map = test_map(3);
        let m = Memory::new(map);
        let mut state = assert_ok!(State::new(m));
        state.frames.push(Frame::new(
            0x500,
            0x501,
            &[0x1122, 0x3344, 0x5566],
            2,
            &[0x1111, 0x2222],
            Some(StoreResult::new(0x402, 0x80)),
            0x48E,
        ));
        state.frames.push(Frame::new(
            0x480,
            0x48C,
            &[0x8899, 0xaabb],
            0,
            &[],
            None,
            0x623,
        ));
        let frame = assert_ok!(state.current_frame());
        assert_eq!(frame.address(), 0x480);
        assert_eq!(frame.pc(), 0x48C);
        assert_eq!(frame.local_variables(), &[0x8899, 0xaabb]);
        assert!(frame.stack().is_empty());
        assert!(frame.result().is_none());
        assert_eq!(frame.return_address(), 0x623);
        assert!(!frame.input_interrupt());
        assert!(!frame.sound_interrupt());
    }

    #[test]
    fn test_current_frame_err() {
        let map = test_map(3);
        let m = Memory::new(map);
        let state = assert_ok!(State::new(m));
        assert!(state.current_frame().is_err());
    }

    #[test]
    fn test_current_frame_mut() {
        let map = test_map(3);
        let m = Memory::new(map);
        let mut state = assert_ok!(State::new(m));
        state.frames.push(Frame::new(
            0x500,
            0x501,
            &[0x1122, 0x3344, 0x5566],
            2,
            &[0x1111, 0x2222],
            Some(StoreResult::new(0x402, 0x80)),
            0x48E,
        ));
        state.frames.push(Frame::new(
            0x480,
            0x48C,
            &[0x8899, 0xaabb],
            0,
            &[],
            None,
            0x623,
        ));
        let frame = assert_ok!(state.current_frame_mut());
        assert_eq!(frame.address(), 0x480);
        assert_eq!(frame.pc(), 0x48C);
        assert_eq!(frame.local_variables(), &[0x8899, 0xaabb]);
        assert!(frame.stack().is_empty());
        assert!(frame.result().is_none());
        assert_eq!(frame.return_address(), 0x623);
        assert!(!frame.input_interrupt());
        assert!(!frame.sound_interrupt());
        // Test mutability
        frame.push(0x1234);
        assert_eq!(frame.stack(), &[0x1234]);
    }

    #[test]
    fn test_current_frame_mut_err() {
        let map = test_map(3);
        let m = Memory::new(map);
        let mut state = assert_ok!(State::new(m));
        assert!(state.current_frame_mut().is_err());
    }

    #[test]
    fn test_initialize_v3() {
        let map = test_map(3);
        let m = Memory::new(map);
        let mut state = assert_ok!(State::new(m));
        assert!(state.initialize(24, 80, (9, 2), true).is_ok());
        assert_ok_eq!(
            header::flag1(&state, Flags1v3::StatusLineNotAvailable as u8),
            0
        );
        assert_ok_eq!(
            header::flag1(&state, Flags1v3::ScreenSplitAvailable as u8),
            1
        );
        assert_ok_eq!(
            header::flag1(&state, Flags1v3::VariablePitchDefault as u8),
            0
        );
        assert_ok_eq!(
            header::field_byte(&state, HeaderField::InterpreterNumber),
            6
        );
        assert_ok_eq!(
            header::field_byte(&state, HeaderField::InterpreterVersion),
            b'Z'
        );
        assert_ok_eq!(state.read_word(0x32), 0x0100);
        assert_eq!(state.frame_count(), 1);
        let frame = assert_ok!(state.current_frame());
        assert_eq!(frame.address(), 0x400);
        assert_eq!(frame.pc(), 0x400);
        assert!(frame.local_variables().is_empty());
        assert_eq!(frame.argument_count(), 0);
        assert!(frame.stack().is_empty());
        assert!(frame.result().is_none());
        assert_eq!(frame.return_address(), 0);
        assert!(!frame.input_interrupt());
        assert!(!frame.sound_interrupt());
    }

    #[test]
    fn test_initialize_v4() {
        let map = test_map(4);
        let m = Memory::new(map);
        let mut state = assert_ok!(State::new(m));
        assert!(state.initialize(24, 80, (9, 2), true).is_ok());
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::SoundEffectsAvailable as u8),
            1
        );
        assert_ok_eq!(
            header::field_byte(&state, HeaderField::DefaultBackground),
            2
        );
        assert_ok_eq!(
            header::field_byte(&state, HeaderField::DefaultForeground),
            9
        );
        assert_ok_eq!(header::field_byte(&state, HeaderField::ScreenLines), 24);
        assert_ok_eq!(header::field_byte(&state, HeaderField::ScreenColumns), 80);
        assert_ok_eq!(
            header::field_byte(&state, HeaderField::InterpreterNumber),
            6
        );
        assert_ok_eq!(
            header::field_byte(&state, HeaderField::InterpreterVersion),
            b'Z'
        );
        assert_ok_eq!(state.read_word(0x32), 0x0100);
        assert_eq!(state.frame_count(), 1);
        let frame = assert_ok!(state.current_frame());
        assert_eq!(frame.address(), 0x400);
        assert_eq!(frame.pc(), 0x400);
        assert!(frame.local_variables().is_empty());
        assert_eq!(frame.argument_count(), 0);
        assert!(frame.stack().is_empty());
        assert!(frame.result().is_none());
        assert_eq!(frame.return_address(), 0);
        assert!(!frame.input_interrupt());
        assert!(!frame.sound_interrupt());
    }

    #[test]
    fn test_initialize_v4_no_sounds() {
        let map = test_map(4);
        let m = Memory::new(map);
        let mut state = assert_ok!(State::new(m));
        assert!(state.initialize(24, 80, (9, 2), false).is_ok());
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::SoundEffectsAvailable as u8),
            0
        );
        assert_ok_eq!(
            header::field_byte(&state, HeaderField::DefaultBackground),
            2
        );
        assert_ok_eq!(
            header::field_byte(&state, HeaderField::DefaultForeground),
            9
        );
        assert_ok_eq!(header::field_byte(&state, HeaderField::ScreenLines), 24);
        assert_ok_eq!(header::field_byte(&state, HeaderField::ScreenColumns), 80);
        assert_ok_eq!(
            header::field_byte(&state, HeaderField::InterpreterNumber),
            6
        );
        assert_ok_eq!(
            header::field_byte(&state, HeaderField::InterpreterVersion),
            b'Z'
        );
        assert_ok_eq!(state.read_word(0x32), 0x0100);
        assert_eq!(state.frame_count(), 1);
        let frame = assert_ok!(state.current_frame());
        assert_eq!(frame.address(), 0x400);
        assert_eq!(frame.pc(), 0x400);
        assert!(frame.local_variables().is_empty());
        assert_eq!(frame.argument_count(), 0);
        assert!(frame.stack().is_empty());
        assert!(frame.result().is_none());
        assert_eq!(frame.return_address(), 0);
        assert!(!frame.input_interrupt());
        assert!(!frame.sound_interrupt());
    }

    #[test]
    fn test_initialize_v5() {
        let mut map = test_map(5);
        map[0x11] = 0xF8;
        let m = Memory::new(map);
        let mut state = assert_ok!(State::new(m));
        assert!(state.initialize(24, 80, (9, 2), true).is_ok());
        assert_ok_eq!(header::flag1(&state, Flags1v4::PicturesAvailable as u8), 0);
        assert_ok_eq!(header::flag1(&state, Flags1v4::ColoursAvailable as u8), 1);
        assert_ok_eq!(header::flag1(&state, Flags1v4::BoldfaceAvailable as u8), 1);
        assert_ok_eq!(header::flag1(&state, Flags1v4::ItalicAvailable as u8), 1);
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::FixedSpaceAvailable as u8),
            1
        );
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::SoundEffectsAvailable as u8),
            1
        );
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::TimedInputAvailable as u8),
            1
        );
        assert_ok_eq!(header::flag2(&state, Flags2::RequestPictures), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestUndo), 1);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestMouse), 1);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestColours), 1);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestSoundEffects), 1);
        assert_ok_eq!(
            header::field_byte(&state, HeaderField::DefaultBackground),
            2
        );
        assert_ok_eq!(
            header::field_byte(&state, HeaderField::DefaultForeground),
            9
        );
        assert_ok_eq!(header::field_byte(&state, HeaderField::ScreenLines), 24);
        assert_ok_eq!(header::field_byte(&state, HeaderField::ScreenColumns), 80);
        assert_ok_eq!(header::field_word(&state, HeaderField::ScreenHeight), 24);
        assert_ok_eq!(header::field_word(&state, HeaderField::ScreenWidth), 80);
        assert_ok_eq!(header::field_byte(&state, HeaderField::FontWidth), 1);
        assert_ok_eq!(header::field_byte(&state, HeaderField::FontHeight), 1);
        assert_ok_eq!(
            header::field_byte(&state, HeaderField::InterpreterNumber),
            6
        );
        assert_ok_eq!(
            header::field_byte(&state, HeaderField::InterpreterVersion),
            b'Z'
        );
        assert_ok_eq!(state.read_word(0x32), 0x0100);
        assert_eq!(state.frame_count(), 1);
        let frame = assert_ok!(state.current_frame());
        assert_eq!(frame.address(), 0x400);
        assert_eq!(frame.pc(), 0x400);
        assert!(frame.local_variables().is_empty());
        assert_eq!(frame.argument_count(), 0);
        assert!(frame.stack().is_empty());
        assert!(frame.result().is_none());
        assert_eq!(frame.return_address(), 0);
        assert!(!frame.input_interrupt());
        assert!(!frame.sound_interrupt());
    }

    #[test]
    fn test_initialize_v5_no_sounds() {
        let mut map = test_map(5);
        map[0x11] = 0xF8;
        let m = Memory::new(map);
        let mut state = assert_ok!(State::new(m));
        assert!(state.initialize(24, 80, (9, 2), false).is_ok());
        assert_ok_eq!(header::flag1(&state, Flags1v4::PicturesAvailable as u8), 0);
        assert_ok_eq!(header::flag1(&state, Flags1v4::ColoursAvailable as u8), 1);
        assert_ok_eq!(header::flag1(&state, Flags1v4::BoldfaceAvailable as u8), 1);
        assert_ok_eq!(header::flag1(&state, Flags1v4::ItalicAvailable as u8), 1);
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::FixedSpaceAvailable as u8),
            1
        );
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::SoundEffectsAvailable as u8),
            0
        );
        assert_ok_eq!(
            header::flag1(&state, Flags1v4::TimedInputAvailable as u8),
            1
        );
        assert_ok_eq!(header::flag2(&state, Flags2::RequestPictures), 0);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestUndo), 1);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestMouse), 1);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestColours), 1);
        assert_ok_eq!(header::flag2(&state, Flags2::RequestSoundEffects), 0);
        assert_ok_eq!(
            header::field_byte(&state, HeaderField::DefaultBackground),
            2
        );
        assert_ok_eq!(
            header::field_byte(&state, HeaderField::DefaultForeground),
            9
        );
        assert_ok_eq!(header::field_byte(&state, HeaderField::ScreenLines), 24);
        assert_ok_eq!(header::field_byte(&state, HeaderField::ScreenColumns), 80);
        assert_ok_eq!(header::field_word(&state, HeaderField::ScreenHeight), 24);
        assert_ok_eq!(header::field_word(&state, HeaderField::ScreenWidth), 80);
        assert_ok_eq!(header::field_byte(&state, HeaderField::FontWidth), 1);
        assert_ok_eq!(header::field_byte(&state, HeaderField::FontHeight), 1);
        assert_ok_eq!(
            header::field_byte(&state, HeaderField::InterpreterNumber),
            6
        );
        assert_ok_eq!(
            header::field_byte(&state, HeaderField::InterpreterVersion),
            b'Z'
        );
        assert_ok_eq!(state.read_word(0x32), 0x0100);
        assert_eq!(state.frame_count(), 1);
        let frame = assert_ok!(state.current_frame());
        assert_eq!(frame.address(), 0x400);
        assert_eq!(frame.pc(), 0x400);
        assert!(frame.local_variables().is_empty());
        assert_eq!(frame.argument_count(), 0);
        assert!(frame.stack().is_empty());
        assert!(frame.result().is_none());
        assert_eq!(frame.return_address(), 0);
        assert!(!frame.input_interrupt());
        assert!(!frame.sound_interrupt());
    }

    #[test]
    fn test_read_byte() {
        let mut map = vec![0; 0x10001];
        map[0] = 3;
        map[0x0E] = 0x4;

        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        let m = Memory::new(map);
        let state = assert_ok!(State::new(m));
        // Read dynamic memory
        assert_ok_eq!(state.read_byte(0x00), 0x03);
        // Read static memory
        assert_ok_eq!(state.read_byte(0x400), 0x00);
        // Read high memory
        assert!(state.read_byte(0x10000).is_err());
    }

    #[test]
    fn test_read_word() {
        let mut map = vec![0; 0x10001];
        map[0] = 3;
        map[0x0E] = 0x4;
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        let m = Memory::new(map);
        let state = assert_ok!(State::new(m));
        // Read dynamic memory
        assert_ok_eq!(state.read_word(0x00), 0x300);
        // Read static memory
        assert_ok_eq!(state.read_word(0x400), 0x1);
        // Read high memory
        assert!(state.read_word(0xFFFF).is_err());
    }

    #[test]
    fn test_write_byte() {
        let mut map = vec![0; 0x10001];
        map[0] = 3;
        map[0x0E] = 0x4;

        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        let m = Memory::new(map);
        let mut state = assert_ok!(State::new(m));
        // Write dynamic memory
        assert_ok_eq!(state.read_byte(0x300), 0);
        assert!(state.write_byte(0x300, 0x99).is_ok());
        assert_ok_eq!(state.read_byte(0x300), 0x99);
        // Read static memory
        assert!(state.write_byte(0x400, 0x99).is_err());
        // Read high memory
        assert!(state.write_byte(0x10000, 0x99).is_err());
    }

    #[test]
    fn test_write_word() {
        let mut map = vec![0; 0x10001];
        map[0] = 3;
        map[0x0E] = 0x4;

        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        let m = Memory::new(map);
        let mut state = assert_ok!(State::new(m));
        // Write dynamic memory
        assert_ok_eq!(state.read_word(0x300), 1);
        assert!(state.write_word(0x300, 0x99aa).is_ok());
        assert_ok_eq!(state.read_word(0x300), 0x99aa);
        // Read static memory
        assert!(state.write_word(0x3FF, 0x99).is_err());
        // Read high memory
        assert!(state.write_word(0x10000, 0x99).is_err());
    }

    #[test]
    fn test_checksum() {
        let mut map = test_map(3);
        map[0x1a] = 0x4;
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        let m = Memory::new(map);
        let state = assert_ok!(State::new(m));
        assert_ok_eq!(state.checksum(), 0xf420);
    }

    #[test]
    fn test_variable_global() {
        let mut map = test_map(3);
        // Variable 0x10 = G00
        map[0x100] = 0x11;
        map[0x101] = 0x22;
        // Variable 0x80 = G70
        map[0x1E0] = 0x33;
        map[0x1E1] = 0x44;
        // Variable 0xFF = GEF
        map[0x2DE] = 0x55;
        map[0x2DF] = 0x66;
        let m = Memory::new(map);
        let mut state = assert_ok!(State::new(m));
        assert_ok_eq!(state.variable(0x10), 0x1122);
        assert_ok_eq!(state.variable(0x80), 0x3344);
        assert_ok_eq!(state.variable(0xFF), 0x5566);
    }

    #[test]
    fn test_variable_local() {
        let mut map = test_map(3);
        // Variable 0x10 = G00
        map[0x100] = 0x11;
        map[0x101] = 0x22;
        // Variable 0x80 = G70
        map[0x1E0] = 0x33;
        map[0x1E1] = 0x44;
        // Variable 0xFF = GEF
        map[0x2DE] = 0x55;
        map[0x2DF] = 0x66;
        let m = Memory::new(map);
        let mut state = assert_ok!(State::new(m));
        state.frames.push(Frame::new(
            0x400,
            0x401,
            &[0x1234, 0x5678, 0x9abc, 0xdef0],
            2,
            &[],
            None,
            0x500,
        ));
        assert_ok_eq!(state.variable(0x1), 0x1234);
        assert_ok_eq!(state.variable(0x2), 0x5678);
        assert_ok_eq!(state.variable(0x3), 0x9ABC);
        assert_ok_eq!(state.variable(0x4), 0xDEF0);
        // Unset locals
        for i in 0x5..0x10 {
            assert!(state.variable(i).is_err());
        }
    }

    #[test]
    fn test_variable_stack() {
        let mut map = test_map(3);
        // Variable 0x10 = G00
        map[0x100] = 0x11;
        map[0x101] = 0x22;
        // Variable 0x80 = G70
        map[0x1E0] = 0x33;
        map[0x1E1] = 0x44;
        // Variable 0xFF = GEF
        map[0x2DE] = 0x55;
        map[0x2DF] = 0x66;
        let m = Memory::new(map);
        let mut state = assert_ok!(State::new(m));
        state.frames.push(Frame::new(
            0x400,
            0x401,
            &[0x1234, 0x5678, 0x9abc, 0xdef0],
            2,
            &[0x9876, 0x5432],
            None,
            0x500,
        ));
        assert_ok_eq!(state.variable(0), 0x5432);
        assert_ok_eq!(state.variable(0), 0x9876);
        assert!(state.variable(0).is_err());
    }

    #[test]
    fn test_peek_variable_global() {
        let mut map = test_map(3);
        // Variable 0x10 = G00
        map[0x100] = 0x11;
        map[0x101] = 0x22;
        // Variable 0x80 = G70
        map[0x1E0] = 0x33;
        map[0x1E1] = 0x44;
        // Variable 0xFF = GEF
        map[0x2DE] = 0x55;
        map[0x2DF] = 0x66;
        let m = Memory::new(map);
        let mut state = assert_ok!(State::new(m));
        assert_ok_eq!(state.peek_variable(0x10), 0x1122);
        assert_ok_eq!(state.peek_variable(0x80), 0x3344);
        assert_ok_eq!(state.peek_variable(0xFF), 0x5566);
    }

    #[test]
    fn test_peek_variable_local() {
        let mut map = test_map(3);
        // Variable 0x10 = G00
        map[0x100] = 0x11;
        map[0x101] = 0x22;
        // Variable 0x80 = G70
        map[0x1E0] = 0x33;
        map[0x1E1] = 0x44;
        // Variable 0xFF = GEF
        map[0x2DE] = 0x55;
        map[0x2DF] = 0x66;
        let m = Memory::new(map);
        let mut state = assert_ok!(State::new(m));
        state.frames.push(Frame::new(
            0x400,
            0x401,
            &[0x1234, 0x5678, 0x9abc, 0xdef0],
            2,
            &[],
            None,
            0x500,
        ));
        assert_ok_eq!(state.peek_variable(0x1), 0x1234);
        assert_ok_eq!(state.peek_variable(0x2), 0x5678);
        assert_ok_eq!(state.peek_variable(0x3), 0x9ABC);
        assert_ok_eq!(state.peek_variable(0x4), 0xDEF0);
        // Unset locals
        for i in 0x5..0x10 {
            assert!(state.peek_variable(i).is_err());
        }
    }

    #[test]
    fn test_peek_variable_stack() {
        let mut map = test_map(3);
        // Variable 0x10 = G00
        map[0x100] = 0x11;
        map[0x101] = 0x22;
        // Variable 0x80 = G70
        map[0x1E0] = 0x33;
        map[0x1E1] = 0x44;
        // Variable 0xFF = GEF
        map[0x2DE] = 0x55;
        map[0x2DF] = 0x66;
        let m = Memory::new(map);
        let mut state = assert_ok!(State::new(m));
        state.frames.push(Frame::new(
            0x400,
            0x401,
            &[0x1234, 0x5678, 0x9abc, 0xdef0],
            2,
            &[0x9876, 0x5432],
            None,
            0x500,
        ));
        assert!(state.current_frame().is_ok_and(|x| x.stack().len() == 2));
        assert_ok_eq!(state.peek_variable(0), 0x5432);
        assert_ok_eq!(state.peek_variable(0), 0x5432);
        assert!(state.current_frame().is_ok_and(|x| x.stack().len() == 2));
    }

    #[test]
    fn test_set_variable_global() {
        let mut map = test_map(3);
        // Variable 0x10 = G00
        map[0x100] = 0x11;
        map[0x101] = 0x22;
        // Variable 0x80 = G70
        map[0x1E0] = 0x33;
        map[0x1E1] = 0x44;
        // Variable 0xFF = GEF
        map[0x2DE] = 0x55;
        map[0x2DF] = 0x66;
        let m = Memory::new(map);
        let mut state = assert_ok!(State::new(m));
        assert!(state.set_variable(0x10, 0x2211).is_ok());
        assert!(state.set_variable(0x80, 0x4433).is_ok());
        assert!(state.set_variable(0xFF, 0x6655).is_ok());
        assert_ok_eq!(state.variable(0x10), 0x2211);
        assert_ok_eq!(state.variable(0x80), 0x4433);
        assert_ok_eq!(state.variable(0xFF), 0x6655);
    }

    #[test]
    fn test_set_variable_local() {
        let mut map = test_map(3);
        // Variable 0x10 = G00
        map[0x100] = 0x11;
        map[0x101] = 0x22;
        // Variable 0x80 = G70
        map[0x1E0] = 0x33;
        map[0x1E1] = 0x44;
        // Variable 0xFF = GEF
        map[0x2DE] = 0x55;
        map[0x2DF] = 0x66;
        let m = Memory::new(map);
        let mut state = assert_ok!(State::new(m));
        state.frames.push(Frame::new(
            0x400,
            0x401,
            &[0x1234, 0x5678, 0x9abc, 0xdef0],
            2,
            &[],
            None,
            0x500,
        ));
        assert!(state.set_variable(0x1, 0x4321).is_ok());
        assert!(state.set_variable(0x2, 0x8765).is_ok());
        assert!(state.set_variable(0x3, 0xCBA9).is_ok());
        assert!(state.set_variable(0x4, 0x0FED).is_ok());
        assert_ok_eq!(state.variable(0x1), 0x4321);
        assert_ok_eq!(state.variable(0x2), 0x8765);
        assert_ok_eq!(state.variable(0x3), 0xCBA9);
        assert_ok_eq!(state.variable(0x4), 0x0FED);
        // Unset locals
        for i in 0x5..0x10 {
            assert!(state.set_variable(i, 0x9999).is_err());
        }
    }

    #[test]
    fn test_set_variable_stack() {
        let mut map = test_map(3);
        // Variable 0x10 = G00
        map[0x100] = 0x11;
        map[0x101] = 0x22;
        // Variable 0x80 = G70
        map[0x1E0] = 0x33;
        map[0x1E1] = 0x44;
        // Variable 0xFF = GEF
        map[0x2DE] = 0x55;
        map[0x2DF] = 0x66;
        let m = Memory::new(map);
        let mut state = assert_ok!(State::new(m));
        state.frames.push(Frame::new(
            0x400,
            0x401,
            &[0x1234, 0x5678, 0x9abc, 0xdef0],
            2,
            &[0x9876, 0x5432],
            None,
            0x500,
        ));
        assert!(state.current_frame().is_ok_and(|x| x.stack().len() == 2));
        assert!(state.set_variable(0, 0x1234).is_ok());
        assert!(state.current_frame().is_ok_and(|x| x.stack().len() == 3));
        assert!(state.set_variable(0, 0x5678).is_ok());
        assert!(state.current_frame().is_ok_and(|x| x.stack().len() == 4));
        assert_ok_eq!(state.variable(0), 0x5678);
        assert_ok_eq!(state.variable(0), 0x1234);
        assert_ok_eq!(state.variable(0), 0x5432);
        assert_ok_eq!(state.variable(0), 0x9876);
        assert!(state.variable(0).is_err());
    }

    #[test]
    fn test_set_variable_indirect_global() {
        let mut map = test_map(3);
        // Variable 0x10 = G00
        map[0x100] = 0x11;
        map[0x101] = 0x22;
        // Variable 0x80 = G70
        map[0x1E0] = 0x33;
        map[0x1E1] = 0x44;
        // Variable 0xFF = GEF
        map[0x2DE] = 0x55;
        map[0x2DF] = 0x66;
        let m = Memory::new(map);
        let mut state = assert_ok!(State::new(m));
        assert!(state.set_variable_indirect(0x10, 0x2211).is_ok());
        assert!(state.set_variable_indirect(0x80, 0x4433).is_ok());
        assert!(state.set_variable_indirect(0xFF, 0x6655).is_ok());
        assert_ok_eq!(state.variable(0x10), 0x2211);
        assert_ok_eq!(state.variable(0x80), 0x4433);
        assert_ok_eq!(state.variable(0xFF), 0x6655);
    }

    #[test]
    fn test_set_variable_indirect_local() {
        let mut map = test_map(3);
        // Variable 0x10 = G00
        map[0x100] = 0x11;
        map[0x101] = 0x22;
        // Variable 0x80 = G70
        map[0x1E0] = 0x33;
        map[0x1E1] = 0x44;
        // Variable 0xFF = GEF
        map[0x2DE] = 0x55;
        map[0x2DF] = 0x66;
        let m = Memory::new(map);
        let mut state = assert_ok!(State::new(m));
        state.frames.push(Frame::new(
            0x400,
            0x401,
            &[0x1234, 0x5678, 0x9abc, 0xdef0],
            2,
            &[],
            None,
            0x500,
        ));
        assert!(state.set_variable_indirect(0x1, 0x4321).is_ok());
        assert!(state.set_variable_indirect(0x2, 0x8765).is_ok());
        assert!(state.set_variable_indirect(0x3, 0xCBA9).is_ok());
        assert!(state.set_variable_indirect(0x4, 0x0FED).is_ok());
        assert_ok_eq!(state.variable(0x1), 0x4321);
        assert_ok_eq!(state.variable(0x2), 0x8765);
        assert_ok_eq!(state.variable(0x3), 0xCBA9);
        assert_ok_eq!(state.variable(0x4), 0x0FED);
        // Unset locals
        for i in 0x5..0x10 {
            assert!(state.set_variable_indirect(i, 0x9999).is_err());
        }
    }

    #[test]
    fn test_set_variable_indirect_stack() {
        let mut map = test_map(3);
        // Variable 0x10 = G00
        map[0x100] = 0x11;
        map[0x101] = 0x22;
        // Variable 0x80 = G70
        map[0x1E0] = 0x33;
        map[0x1E1] = 0x44;
        // Variable 0xFF = GEF
        map[0x2DE] = 0x55;
        map[0x2DF] = 0x66;
        let m = Memory::new(map);
        let mut state = assert_ok!(State::new(m));
        state.frames.push(Frame::new(
            0x400,
            0x401,
            &[0x1234, 0x5678, 0x9abc, 0xdef0],
            2,
            &[0x9876, 0x5432],
            None,
            0x500,
        ));
        assert!(state.current_frame().is_ok_and(|x| x.stack().len() == 2));
        assert!(state.set_variable_indirect(0, 0x1234).is_ok());
        assert!(state.current_frame().is_ok_and(|x| x.stack().len() == 2));
        assert!(state.variable(0).is_ok_and(|x| x == 0x1234));
        assert!(state.current_frame().is_ok_and(|x| x.stack().len() == 1));
        assert!(state.set_variable_indirect(0, 0x5678).is_ok());
        assert!(state.current_frame().is_ok_and(|x| x.stack().len() == 1));
        assert_ok_eq!(state.variable(0), 0x5678);
        assert!(state.variable(0).is_err());
    }

    #[test]
    fn test_push() {
        let mut map = test_map(3);
        // Variable 0x10 = G00
        map[0x100] = 0x11;
        map[0x101] = 0x22;
        // Variable 0x80 = G70
        map[0x1E0] = 0x33;
        map[0x1E1] = 0x44;
        // Variable 0xFF = GEF
        map[0x2DE] = 0x55;
        map[0x2DF] = 0x66;
        let m = Memory::new(map);
        let mut state = assert_ok!(State::new(m));
        state.frames.push(Frame::new(
            0x400,
            0x401,
            &[0x1234, 0x5678, 0x9abc, 0xdef0],
            2,
            &[0x9876, 0x5432],
            None,
            0x500,
        ));
        assert!(state.current_frame().is_ok_and(|x| x.stack().len() == 2));
        assert!(state.push(0x1234).is_ok());
        assert!(state.current_frame().is_ok_and(|x| x.stack().len() == 3));
        assert!(state.push(0x5678).is_ok());
        assert!(state.current_frame().is_ok_and(|x| x.stack().len() == 4));
        assert_ok_eq!(state.variable(0), 0x5678);
        assert_ok_eq!(state.variable(0), 0x1234);
        assert_ok_eq!(state.variable(0), 0x5432);
        assert_ok_eq!(state.variable(0), 0x9876);
        assert!(state.variable(0).is_err());
    }

    #[test]
    fn test_instruction() {
        let mut map = test_map(3);
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        let m: Memory = Memory::new(map);
        let state = assert_ok!(State::new(m));
        assert_eq!(
            state.instruction(0x400),
            &[
                0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
                0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16
            ]
        )
    }

    #[test]
    fn test_routine_header_v3() {
        let mut map = vec![0; 0x11000];
        map[0] = 3;
        map[0x10000] = 0xF;
        for (i, b) in (1..0x10).enumerate() {
            map[0x10001 + (i * 2)] = b * 0x11;
            map[0x10002 + (i * 2)] = b * 0x11;
        }
        let m = Memory::new(map);
        let state = assert_ok!(State::new(m));
        let (address, vars) = assert_ok!(state.routine_header(0x10000));
        assert_eq!(address, 0x1001F);
        assert_eq!(
            vars,
            vec![
                0x1111, 0x2222, 0x3333, 0x4444, 0x5555, 0x6666, 0x7777, 0x8888, 0x9999, 0xAAAA,
                0xBBBB, 0xCCCC, 0xDDDD, 0xEEEE, 0xFFFF,
            ],
        );
    }

    #[test]
    fn test_routine_header_v4() {
        let mut map = vec![0; 0x11000];
        map[0] = 4;
        map[0x10000] = 0xF;
        for (i, b) in (1..0x10).enumerate() {
            map[0x10001 + (i * 2)] = b * 0x11;
            map[0x10002 + (i * 2)] = b * 0x11;
        }
        let m = Memory::new(map);
        let state = assert_ok!(State::new(m));
        let (address, vars) = assert_ok!(state.routine_header(0x10000));
        assert_eq!(address, 0x1001F);
        assert_eq!(
            vars,
            vec![
                0x1111, 0x2222, 0x3333, 0x4444, 0x5555, 0x6666, 0x7777, 0x8888, 0x9999, 0xAAAA,
                0xBBBB, 0xCCCC, 0xDDDD, 0xEEEE, 0xFFFF,
            ],
        );
    }

    #[test]
    fn test_routine_header_v5() {
        let mut map = vec![0; 0x11000];
        map[0] = 5;
        map[0x10000] = 0xF;
        let m = Memory::new(map);
        let state = assert_ok!(State::new(m));
        let (address, vars) = assert_ok!(state.routine_header(0x10000));
        assert_eq!(address, 0x10001);
        assert_eq!(vars, vec![0; 15],);
    }

    #[test]
    fn test_routine_header_v8() {
        let mut map = vec![0; 0x11000];
        map[0] = 8;
        map[0x10000] = 0xF;
        let m = Memory::new(map);
        let state = assert_ok!(State::new(m));
        let (address, vars) = assert_ok!(state.routine_header(0x10000));
        assert_eq!(address, 0x10001);
        assert_eq!(vars, vec![0; 15],);
    }

    #[test]
    fn test_string_literal() {
        let mut map = vec![0; 0x11000];
        for (i, b) in (0..0xF).enumerate() {
            map[0x10000 + (i * 2)] = (b + 1) * 0x11;
            map[0x10001 + (i * 2)] = (b + 1) * 0x11;
        }
        let m = Memory::new(map);
        let state = assert_ok!(State::new(m));
        assert_ok_eq!(
            state.string_literal(0x10000),
            vec![0x1111, 0x2222, 0x3333, 0x4444, 0x5555, 0x6666, 0x7777, 0x8888]
        );
    }

    #[test]
    fn test_packed_routine_address_v3() {
        let map = test_map(3);
        let m = Memory::new(map);
        let state = assert_ok!(State::new(m));
        assert_ok_eq!(state.packed_routine_address(0x400), 0x800);
    }

    #[test]
    fn test_packed_routine_address_v4() {
        let map = test_map(4);
        let m = Memory::new(map);
        let state = assert_ok!(State::new(m));
        assert_ok_eq!(state.packed_routine_address(0x400), 0x1000);
    }

    #[test]
    fn test_packed_routine_address_v5() {
        let map = test_map(4);
        let m = Memory::new(map);
        let state = assert_ok!(State::new(m));
        assert_ok_eq!(state.packed_routine_address(0x400), 0x1000);
    }

    #[test]
    fn test_packed_routine_address_v7() {
        let mut map = test_map(7);
        // Routine offset is 0x100;
        map[0x28] = 0x1;
        let m = Memory::new(map);
        let state = assert_ok!(State::new(m));
        assert_ok_eq!(state.packed_routine_address(0x400), 0x1800);
    }

    #[test]
    fn test_packed_routine_address_v8() {
        let map = test_map(8);
        let m = Memory::new(map);
        let state = assert_ok!(State::new(m));
        assert_ok_eq!(state.packed_routine_address(0x400), 0x2000);
    }

    #[test]
    fn test_packed_routine_address_invalid() {
        let map = test_map(6);
        let m = Memory::new(map);
        let state = assert_ok!(State::new(m));
        assert!(state.packed_routine_address(0x400).is_err());
    }

    #[test]
    fn test_packed_string_address_v3() {
        let map = test_map(3);
        let m = Memory::new(map);
        let state = assert_ok!(State::new(m));
        assert_ok_eq!(state.packed_string_address(0x400), 0x800);
    }

    #[test]
    fn test_packed_string_address_v4() {
        let map = test_map(4);
        let m = Memory::new(map);
        let state = assert_ok!(State::new(m));
        assert_ok_eq!(state.packed_string_address(0x400), 0x1000);
    }

    #[test]
    fn test_packed_string_address_v5() {
        let map = test_map(4);
        let m = Memory::new(map);
        let state = assert_ok!(State::new(m));
        assert_ok_eq!(state.packed_string_address(0x400), 0x1000);
    }

    #[test]
    fn test_packed_string_address_v7() {
        let mut map = test_map(7);
        // String offset is 0x100;
        map[0x2A] = 0x1;
        let m = Memory::new(map);
        let state = assert_ok!(State::new(m));
        assert_ok_eq!(state.packed_string_address(0x400), 0x1800);
    }

    #[test]
    fn test_packed_string_address_v8() {
        let map = test_map(8);
        let m = Memory::new(map);
        let state = assert_ok!(State::new(m));
        assert_ok_eq!(state.packed_string_address(0x400), 0x2000);
    }

    #[test]
    fn test_packed_string_address_invalid() {
        let map = test_map(6);
        let m = Memory::new(map);
        let state = assert_ok!(State::new(m));
        assert!(state.packed_string_address(0x400).is_err());
    }

    #[test]
    fn test_is_input_interrupt() {
        let map = test_map(4);
        let m = Memory::new(map);
        let mut state = assert_ok!(State::new(m));
        assert!(!state.is_input_interrupt());
        state.read_interrupt_result = Some(0x10);
        assert!(state.is_input_interrupt());
    }

    #[test]
    fn test_call_routine() {
        let mut map = vec![0; 0x11000];
        map[0] = 5;
        map[0x10000] = 0xF;
        let m = Memory::new(map);
        let mut state = assert_ok!(State::new(m));
        assert_eq!(state.frame_count(), 0);
        assert_ok_eq!(
            state.call_routine(
                0x10000,
                &vec![0x1111, 0x2222, 0x3333],
                Some(StoreResult::new(0x401, 0x80)),
                0x402
            ),
            0x10001
        );
        assert_eq!(state.frame_count(), 1);
        let frame = assert_ok!(state.current_frame());
        assert_eq!(frame.address(), 0x10000);
        assert_eq!(frame.pc(), 0x10001);
        assert_eq!(frame.argument_count(), 3);
        assert_eq!(
            frame.local_variables(),
            &[0x1111, 0x2222, 0x3333, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        );
        assert_some_eq!(frame.result(), &StoreResult::new(0x401, 0x80));
        assert_eq!(frame.return_address(), 0x402);
        assert!(!frame.input_interrupt());
        assert!(!frame.sound_interrupt());
    }

    #[test]
    fn test_call_routine_no_store() {
        let mut map = vec![0; 0x11000];
        map[0] = 5;
        map[0x10000] = 0xF;
        let m = Memory::new(map);
        let mut state = assert_ok!(State::new(m));
        assert_eq!(state.frame_count(), 0);
        assert_ok_eq!(
            state.call_routine(0x10000, &vec![0x1111, 0x2222, 0x3333], None, 0x402),
            0x10001
        );
        assert_eq!(state.frame_count(), 1);
        let frame = assert_ok!(state.current_frame());
        assert_eq!(frame.address(), 0x10000);
        assert_eq!(frame.pc(), 0x10001);
        assert_eq!(frame.argument_count(), 3);
        assert_eq!(
            frame.local_variables(),
            &[0x1111, 0x2222, 0x3333, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        );
        assert!(frame.result().is_none());
        assert_eq!(frame.return_address(), 0x402);
        assert!(!frame.input_interrupt());
        assert!(!frame.sound_interrupt());
    }

    #[test]
    fn test_call_routine_0() {
        let mut map = vec![0; 0x11000];
        map[0] = 5;
        map[0x0E] = 0x04;
        map[0x0C] = 0x01;
        map[0x10000] = 0xF;
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        let m = Memory::new(map);
        let mut state = assert_ok!(State::new(m));
        assert_eq!(state.frame_count(), 0);
        assert_ok_eq!(
            state.call_routine(
                0,
                &vec![0x1111, 0x2222, 0x3333],
                Some(StoreResult::new(0x401, 0x80)),
                0x402
            ),
            0x402
        );
        assert_eq!(state.frame_count(), 0);
        assert_ok_eq!(state.variable(0x80), 0);
    }

    #[test]
    fn test_call_routine_0_no_store() {
        let mut map = vec![0; 0x11000];
        map[0] = 5;
        map[0x0E] = 0x04;
        map[0x0C] = 0x01;
        map[0x10000] = 0xF;
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        let m = Memory::new(map);
        let mut state = assert_ok!(State::new(m));
        assert_eq!(state.frame_count(), 0);
        assert_ok_eq!(
            state.call_routine(0, &vec![0x1111, 0x2222, 0x3333], None, 0x402),
            0x402
        );
        assert_eq!(state.frame_count(), 0);
        assert_ok_eq!(state.variable(0x80), 0xE0E1);
    }

    #[test]
    fn test_call_read_interrupt() {
        let mut map = vec![0; 0x11000];
        map[0] = 5;
        map[0x10000] = 0xF;
        let m = Memory::new(map);
        let mut state = assert_ok!(State::new(m));
        assert_eq!(state.frame_count(), 0);
        state.set_read_interrupt();
        assert!(state.read_interrupt_pending());
        assert_ok_eq!(state.call_read_interrupt(0x10000, 0x402), 0x10001);
        assert_eq!(state.frame_count(), 1);
        assert_some_eq!(state.read_interrupt_result(), 0);
        let frame = assert_ok!(state.current_frame());
        assert_eq!(frame.address(), 0x10000);
        assert_eq!(frame.pc(), 0x10001);
        assert_eq!(frame.argument_count(), 0);
        assert_eq!(frame.local_variables(), &[0; 15]);
        assert!(frame.result().is_none());
        assert_eq!(frame.return_address(), 0x402);
        assert!(frame.input_interrupt());
        assert!(!frame.sound_interrupt());
    }

    #[test]
    fn test_call_read_interrupt_not_pending() {
        let mut map = vec![0; 0x11000];
        map[0] = 5;
        map[0x10000] = 0xF;
        let m = Memory::new(map);
        let mut state = assert_ok!(State::new(m));
        assert_eq!(state.frame_count(), 0);
        assert!(state.call_read_interrupt(0x10000, 0x402).is_err());
        assert_eq!(state.frame_count(), 0);
    }

    #[test]
    fn test_read_interrupt() {
        let map = test_map(5);
        let m = Memory::new(map);
        let mut state = assert_ok!(State::new(m));
        state.set_read_interrupt();
        state.read_interrupt_result = Some(0);
        assert!(state.read_interrupt_pending());
        assert_some_eq!(state.read_interrupt_result(), 0);
        state.clear_read_interrupt();
        assert!(!state.read_interrupt_pending());
        assert!(state.read_interrupt_result().is_none());
    }

    #[test]
    fn test_sound_interrupt() {
        let map = test_map(5);
        let m = Memory::new(map);
        let mut state = assert_ok!(State::new(m));
        assert!(state.sound_interrupt().is_none());
        state.set_sound_interrupt(0x1234);
        assert_some_eq!(state.sound_interrupt(), 0x1234);
        state.clear_sound_interrupt();
        assert!(state.sound_interrupt().is_none());
    }

    #[test]
    fn test_call_sound_interrupt() {
        let mut map = vec![0; 0x11000];
        map[0] = 5;
        map[0x10000] = 0xF;
        let m = Memory::new(map);
        let mut state = assert_ok!(State::new(m));
        assert_eq!(state.frame_count(), 0);
        state.set_sound_interrupt(0x10000);
        assert_ok_eq!(state.call_sound_interrupt(0x402), 0x10001);
        assert_eq!(state.frame_count(), 1);
        let frame = assert_ok!(state.current_frame());
        assert_eq!(frame.address(), 0x10000);
        assert_eq!(frame.pc(), 0x10001);
        assert_eq!(frame.argument_count(), 0);
        assert_eq!(frame.local_variables(), &[0; 15]);
        assert!(frame.result().is_none());
        assert_eq!(frame.return_address(), 0x402);
        assert!(!frame.input_interrupt());
        assert!(frame.sound_interrupt());
    }

    #[test]
    fn test_call_sound_interrupt_not_pending() {
        let mut map = vec![0; 0x11000];
        map[0] = 5;
        map[0x10000] = 0xF;
        let m = Memory::new(map);
        let mut state = assert_ok!(State::new(m));
        assert_eq!(state.frame_count(), 0);
        assert!(state.call_sound_interrupt(0x402).is_err());
        assert_eq!(state.frame_count(), 0);
    }

    #[test]
    fn test_return_routine() {
        let mut map = vec![0; 0x11000];
        map[0] = 5;
        map[0x0E] = 0x04;
        map[0x0C] = 0x01;
        map[0x10000] = 0xF;
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        let m = Memory::new(map);
        let mut state = assert_ok!(State::new(m));
        state.frames.push(Frame::new(
            0x400,
            0x40A,
            &[0x1111, 0x2222, 0x3333],
            1,
            &[0x1234, 0x5678],
            None,
            0x5A5,
        ));
        state.frames.push(Frame::new(
            0x600,
            0x40C,
            &[0x4444],
            1,
            &[0x1234],
            Some(StoreResult::new(0x40D, 2)),
            0x40E,
        ));
        assert_eq!(state.frame_count(), 2);
        assert_ok_eq!(state.return_routine(0x9876), 0x40E);
        assert_eq!(state.frame_count(), 1);
        assert_ok_eq!(state.variable(1), 0x1111);
        assert_ok_eq!(state.variable(2), 0x9876);
        assert_ok_eq!(state.variable(3), 0x3333);
    }

    #[test]
    fn test_return_routine_no_store() {
        let mut map = vec![0; 0x11000];
        map[0] = 5;
        map[0x0E] = 0x04;
        map[0x0C] = 0x01;
        map[0x10000] = 0xF;
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        let m = Memory::new(map);
        let mut state = assert_ok!(State::new(m));
        state.frames.push(Frame::new(
            0x400,
            0x40A,
            &[0x1111, 0x2222, 0x3333],
            1,
            &[0x1234, 0x5678],
            None,
            0x5A5,
        ));
        state.frames.push(Frame::new(
            0x600,
            0x40C,
            &[0x4444],
            1,
            &[0x1234],
            None,
            0x40E,
        ));
        assert_eq!(state.frame_count(), 2);
        assert_ok_eq!(state.return_routine(0x9876), 0x40E);
        assert_eq!(state.frame_count(), 1);
        assert_ok_eq!(state.variable(1), 0x1111);
        assert_ok_eq!(state.variable(2), 0x2222);
        assert_ok_eq!(state.variable(3), 0x3333);
    }

    #[test]
    fn test_return_routine_no_frame() {
        let mut map = vec![0; 0x11000];
        map[0] = 5;
        map[0x0E] = 0x04;
        map[0x0C] = 0x01;
        map[0x10000] = 0xF;
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        let m = Memory::new(map);
        let mut state = assert_ok!(State::new(m));
        assert_eq!(state.frame_count(), 0);
        assert!(state.return_routine(0x9876).is_err());
        assert_eq!(state.frame_count(), 0);
    }

    #[test]
    fn test_return_routine_input_interrupt() {
        let mut map = vec![0; 0x11000];
        map[0] = 5;
        map[0x0E] = 0x04;
        map[0x0C] = 0x01;
        map[0x10000] = 0xF;
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        let m = Memory::new(map);
        let mut state = assert_ok!(State::new(m));
        state.frames.push(Frame::new(
            0x400,
            0x40A,
            &[0x1111, 0x2222, 0x3333],
            1,
            &[0x1234, 0x5678],
            None,
            0x5A5,
        ));
        state.frames.push(Frame::new(
            0x600,
            0x40C,
            &[0x4444],
            1,
            &[0x1234],
            None,
            0x40E,
        ));
        let cf = assert_ok!(state.current_frame_mut());
        cf.set_input_interrupt(true);
        state.set_read_interrupt();
        assert_eq!(state.frame_count(), 2);
        assert_ok_eq!(state.return_routine(0x9876), 0x40E);
        assert_eq!(state.frame_count(), 1);
        assert_ok_eq!(state.variable(1), 0x1111);
        assert_ok_eq!(state.variable(2), 0x2222);
        assert_ok_eq!(state.variable(3), 0x3333);
        assert_some_eq!(state.read_interrupt_result(), 0x9876);
    }

    #[test]
    fn test_throw() {
        let mut map = vec![0; 0x11000];
        map[0] = 5;
        map[0x0E] = 0x04;
        map[0x0C] = 0x01;
        map[0x10000] = 0xF;
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        let m = Memory::new(map);
        let mut state = assert_ok!(State::new(m));
        state.frames.push(Frame::new(
            0x400,
            0x40A,
            &[0x1111, 0x2222, 0x3333],
            1,
            &[0x1234, 0x5678],
            None,
            0x5A5,
        ));
        state.frames.push(Frame::new(
            0x600,
            0x40C,
            &[0x4444],
            1,
            &[0x1234],
            Some(StoreResult::new(0x40D, 0)),
            0x40E,
        ));
        state
            .frames
            .push(Frame::new(0x4A0, 0x4AA, &[], 0, &[], None, 0x999));
        assert_eq!(state.frame_count(), 3);
        assert_ok_eq!(state.throw(2, 0x9876), 0x40E);
        assert_eq!(state.frame_count(), 1);
        assert_ok_eq!(state.variable(1), 0x1111);
        assert_ok_eq!(state.variable(2), 0x2222);
        assert_ok_eq!(state.variable(3), 0x3333);
        assert_ok_eq!(state.peek_variable(0), 0x9876);
        assert_eq!(state.current_frame().unwrap().stack().len(), 3);
    }

    #[test]
    fn test_pc() {
        let mut map = vec![0; 0x11000];
        map[0] = 5;
        map[0x0E] = 0x04;
        map[0x0C] = 0x01;
        map[0x10000] = 0xF;
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        let m = Memory::new(map);
        let mut state = assert_ok!(State::new(m));
        state.frames.push(Frame::new(
            0x400,
            0x40A,
            &[0x1111, 0x2222, 0x3333],
            1,
            &[0x1234, 0x5678],
            None,
            0x5A5,
        ));
        assert_ok_eq!(state.pc(), 0x40A);
        assert!(state.set_pc(0x500).is_ok());
        assert_ok_eq!(state.pc(), 0x500);
        assert_eq!(state.current_frame().unwrap().pc(), 0x500);
    }

    #[test]
    fn test_argument_count() {
        let mut map = vec![0; 0x11000];
        map[0] = 5;
        map[0x0E] = 0x04;
        map[0x0C] = 0x01;
        map[0x10000] = 0xF;
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        let m = Memory::new(map);
        let mut state = assert_ok!(State::new(m));
        state.frames.push(Frame::new(
            0x400,
            0x40A,
            &[0x1111, 0x2222, 0x3333],
            1,
            &[0x1234, 0x5678],
            None,
            0x5A5,
        ));
        assert_ok_eq!(state.argument_count(), 1);
        assert_eq!(state.current_frame().unwrap().argument_count(), 1);
    }

    #[test]
    fn test_save() {
        let mut map = test_map(3);
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        map[0x02] = 0x12;
        map[0x03] = 0x34;
        map[0x12] = b'2';
        map[0x13] = b'3';
        map[0x14] = b'0';
        map[0x15] = b'7';
        map[0x16] = b'1';
        map[0x17] = b'5';
        map[0x1C] = 0x56;
        map[0x1D] = 0x78;

        let m = Memory::new(map.clone());
        let mut state = assert_ok!(State::new(m));
        // See memory.rs tests ... change dynamic memory a little bit
        // so the compressed memory isn't just runs of 0s
        assert!(state.write_byte(0x200, 0xFC).is_ok());
        assert!(state.write_byte(0x280, 0x10).is_ok());
        assert!(state.write_byte(0x300, 0xFD).is_ok());

        state.frames.push(Frame::new(
            0x500,
            0x501,
            &[0x1122, 0x3344, 0x5566],
            2,
            &[0x1111, 0x2222],
            Some(StoreResult::new(0x402, 0x80)),
            0x48E,
        ));
        state.frames.push(Frame::new(
            0x480,
            0x48C,
            &[0x8899, 0xaabb],
            0,
            &[],
            None,
            0x623,
        ));

        let v = assert_ok!(state.save(0x9abc));
        assert_eq!(
            v,
            [
                b'F', b'O', b'R', b'M', 0x00, 0x00, 0x00, 0x54, b'I', b'F', b'Z', b'S', b'I', b'F',
                b'h', b'd', 0x00, 0x00, 0x00, 0x0D, 0x12, 0x34, 0x32, 0x33, 0x30, 0x37, 0x31, 0x35,
                0x56, 0x78, 0x00, 0x9a, 0xbc, 0x00, b'C', b'M', b'e', b'm', 0x00, 0x00, 0x00, 0x0D,
                0x00, 0xFF, 0x00, 0xFF, 0xFC, 0x00, 0x7E, 0x90, 0x00, 0x7E, 0xFD, 0x00, 0xFE, 0x00,
                b'S', b't', b'k', b's', 0x00, 0x00, 0x00, 0x1E, 0x00, 0x04, 0x8E, 0x03, 0x80, 0x03,
                0x00, 0x02, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x11, 0x11, 0x22, 0x22, 0x00, 0x06,
                0x23, 0x12, 0x00, 0x00, 0x00, 0x00, 0x88, 0x99, 0xaa, 0xbb
            ]
        );
    }

    #[test]
    fn test_restore_state_cmem() {
        let mut map = test_map(5);
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        map[0x02] = 0x12;
        map[0x03] = 0x34;
        map[0x12] = b'2';
        map[0x13] = b'3';
        map[0x14] = b'0';
        map[0x15] = b'7';
        map[0x16] = b'1';
        map[0x17] = b'5';
        map[0x1C] = 0x56;
        map[0x1D] = 0x78;

        let m = Memory::new(map.clone());
        let mut state = assert_ok!(State::new(m));
        assert!(state.initialize(40, 132, (3, 6), true).is_ok());
        // Turn on transcripting ... it should survive the restore
        assert!(header::set_flag2(&mut state, Flags2::Transcripting).is_ok());

        assert_eq!(state.frame_count(), 1);

        let quetzal = assert_ok!(Quetzal::try_from(vec![
            b'F', b'O', b'R', b'M', 0x00, 0x00, 0x00, 0x56, b'I', b'F', b'Z', b'S', b'I', b'F',
            b'h', b'd', 0x00, 0x00, 0x00, 0x0D, 0x12, 0x34, 0x32, 0x33, 0x30, 0x37, 0x31, 0x35,
            0x56, 0x78, 0x00, 0x9a, 0xbc, 0x00, b'C', b'M', b'e', b'm', 0x00, 0x00, 0x00, 0x0D,
            0x00, 0xFF, 0x00, 0xFF, 0xFC, 0x00, 0x7E, 0x90, 0x00, 0x7E, 0xFD, 0x00, 0xFE, 0x00,
            b'S', b't', b'k', b's', 0x00, 0x00, 0x00, 0x1E, 0x00, 0x04, 0x8E, 0x03, 0x80, 0x03,
            0x00, 0x02, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x11, 0x11, 0x22, 0x22, 0x00, 0x06,
            0x23, 0x12, 0x00, 0x00, 0x00, 0x00, 0x88, 0x99, 0xaa, 0xbb,
        ]));
        let pc = assert_ok!(state.restore_state(quetzal));
        assert_some_eq!(pc, 0x9abc);
        assert_ok_eq!(header::flag2(&state, Flags2::Transcripting), 1);
        assert_ok_eq!(
            header::field_byte(&state, HeaderField::DefaultForeground),
            3
        );
        assert_ok_eq!(
            header::field_byte(&state, HeaderField::DefaultBackground),
            6
        );
        assert_ok_eq!(header::field_byte(&state, HeaderField::ScreenLines), 40);
        assert_ok_eq!(header::field_byte(&state, HeaderField::ScreenColumns), 132);
        assert_ok_eq!(state.read_byte(0x200), 0xFC);
        assert_ok_eq!(state.read_byte(0x280), 0x10);
        assert_ok_eq!(state.read_byte(0x300), 0xFD);
        assert_eq!(state.frame_count(), 2);
    }

    #[test]
    fn test_restore_state_umem() {
        let mut map = test_map(5);
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        map[0x02] = 0x12;
        map[0x03] = 0x34;
        map[0x12] = b'2';
        map[0x13] = b'3';
        map[0x14] = b'0';
        map[0x15] = b'7';
        map[0x16] = b'1';
        map[0x17] = b'5';
        map[0x1C] = 0x56;
        map[0x1D] = 0x78;

        let m = Memory::new(map.clone());
        let mut state = assert_ok!(State::new(m));
        assert!(state.initialize(40, 132, (3, 6), true).is_ok());
        // Turn on transcripting ... it should survive the restore
        assert!(header::set_flag2(&mut state, Flags2::Transcripting).is_ok());

        assert_eq!(state.frame_count(), 1);

        let mut mem_data = map[..0x400].to_vec();
        mem_data[0x200] = 0xFC;
        mem_data[0x280] = 0x10;
        mem_data[0x300] = 0xFD;
        let mut qvec = [
            b'F', b'O', b'R', b'M', 0x00, 0x00, 0x04, 0x49, b'I', b'F', b'Z', b'S', b'I', b'F',
            b'h', b'd', 0x00, 0x00, 0x00, 0x0D, 0x12, 0x34, 0x32, 0x33, 0x30, 0x37, 0x31, 0x35,
            0x56, 0x78, 0x00, 0x9a, 0xbc, 0x00, b'U', b'M', b'e', b'm', 0x00, 0x00, 0x04, 0x00,
        ]
        .to_vec();
        qvec.append(&mut mem_data);
        qvec.append(
            &mut [
                b'S', b't', b'k', b's', 0x00, 0x00, 0x00, 0x1E, 0x00, 0x04, 0x8E, 0x03, 0x80, 0x03,
                0x00, 0x02, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x11, 0x11, 0x22, 0x22, 0x00, 0x06,
                0x23, 0x12, 0x00, 0x00, 0x00, 0x00, 0x88, 0x99, 0xaa, 0xbb,
            ]
            .to_vec(),
        );
        let quetzal = assert_ok!(Quetzal::try_from(qvec));
        let pc = assert_ok!(state.restore_state(quetzal));
        assert_some_eq!(pc, 0x9abc);
        assert_ok_eq!(header::flag2(&state, Flags2::Transcripting), 1);
        assert_ok_eq!(
            header::field_byte(&state, HeaderField::DefaultForeground),
            3
        );
        assert_ok_eq!(
            header::field_byte(&state, HeaderField::DefaultBackground),
            6
        );
        assert_ok_eq!(header::field_byte(&state, HeaderField::ScreenLines), 40);
        assert_ok_eq!(header::field_byte(&state, HeaderField::ScreenColumns), 132);
        assert_ok_eq!(state.read_byte(0x200), 0xFC);
        assert_ok_eq!(state.read_byte(0x280), 0x10);
        assert_ok_eq!(state.read_byte(0x300), 0xFD);
        assert_eq!(state.frame_count(), 2);
    }

    // #[test]
    // fn test_restore_state_no_mem() {
    //     let mut map = test_map(5);
    //     for (i, b) in (0x40..0x800).enumerate() {
    //         map[i + 0x40] = b as u8;
    //     }
    //     map[0x02] = 0x12;
    //     map[0x03] = 0x34;
    //     map[0x12] = b'2';
    //     map[0x13] = b'3';
    //     map[0x14] = b'0';
    //     map[0x15] = b'7';
    //     map[0x16] = b'1';
    //     map[0x17] = b'5';
    //     map[0x1C] = 0x56;
    //     map[0x1D] = 0x78;

    //     let m = Memory::new(map.clone());
    //     let mut state = assert_ok!(State::new(m));
    //     assert!(state.initialize(40, 132, (3, 6), true).is_ok());
    //     // Turn on transcripting ... it should survive the restore
    //     assert!(header::set_flag2(&mut state, Flags2::Transcripting).is_ok());

    //     assert_eq!(state.frame_count(), 1);
    //     let quetzal = Quetzal::new(
    //         IFhd::new(
    //             0x1234,
    //             &[b'2', b'3', b'0', b'7', b'1', b'6'],
    //             0x5678,
    //             0x9abcde,
    //         ),
    //         None,
    //         None,
    //         Stks::new(vec![]),
    //     );
    //     assert!(state.restore_state(quetzal).is_err());
    //     assert_eq!(state.frame_count(), 1);
    // }

    #[test]
    fn test_restore() {
        let mut map = test_map(5);
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        map[0x02] = 0x12;
        map[0x03] = 0x34;
        map[0x12] = b'2';
        map[0x13] = b'3';
        map[0x14] = b'0';
        map[0x15] = b'7';
        map[0x16] = b'1';
        map[0x17] = b'5';
        map[0x1C] = 0x56;
        map[0x1D] = 0x78;

        let m = Memory::new(map.clone());
        let mut state = assert_ok!(State::new(m));
        assert!(state.initialize(40, 132, (3, 6), true).is_ok());
        // Turn on transcripting ... it should survive the restore
        assert!(header::set_flag2(&mut state, Flags2::Transcripting).is_ok());

        assert_eq!(state.frame_count(), 1);

        let restore_data = vec![
            b'F', b'O', b'R', b'M', 0x00, 0x00, 0x00, 0x56, b'I', b'F', b'Z', b'S', b'I', b'F',
            b'h', b'd', 0x00, 0x00, 0x00, 0x0D, 0x12, 0x34, 0x32, 0x33, 0x30, 0x37, 0x31, 0x35,
            0x56, 0x78, 0x00, 0x9a, 0xbc, 0x00, b'C', b'M', b'e', b'm', 0x00, 0x00, 0x00, 0x0D,
            0x00, 0xFF, 0x00, 0xFF, 0xFC, 0x00, 0x7E, 0x90, 0x00, 0x7E, 0xFD, 0x00, 0xFE, 0x00,
            b'S', b't', b'k', b's', 0x00, 0x00, 0x00, 0x1E, 0x00, 0x04, 0x8E, 0x03, 0x80, 0x03,
            0x00, 0x02, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x11, 0x11, 0x22, 0x22, 0x00, 0x06,
            0x23, 0x12, 0x00, 0x00, 0x00, 0x00, 0x88, 0x99, 0xaa, 0xbb,
        ];
        let pc = assert_ok!(state.restore(restore_data));
        assert_some_eq!(pc, 0x9abc);
        assert_ok_eq!(header::flag2(&state, Flags2::Transcripting), 1);
        assert_ok_eq!(
            header::field_byte(&state, HeaderField::DefaultForeground),
            3
        );
        assert_ok_eq!(
            header::field_byte(&state, HeaderField::DefaultBackground),
            6
        );
        assert_ok_eq!(header::field_byte(&state, HeaderField::ScreenLines), 40);
        assert_ok_eq!(header::field_byte(&state, HeaderField::ScreenColumns), 132);
        assert_ok_eq!(state.read_byte(0x200), 0xFC);
        assert_ok_eq!(state.read_byte(0x280), 0x10);
        assert_ok_eq!(state.read_byte(0x300), 0xFD);
        assert_eq!(state.frame_count(), 2);
    }

    #[test]
    fn test_restore_wrong_release() {
        let mut map = test_map(5);
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        map[0x02] = 0x12;
        map[0x03] = 0x35;
        map[0x12] = b'2';
        map[0x13] = b'3';
        map[0x14] = b'0';
        map[0x15] = b'7';
        map[0x16] = b'1';
        map[0x17] = b'5';
        map[0x1C] = 0x56;
        map[0x1D] = 0x78;

        let m = Memory::new(map.clone());
        let mut state = assert_ok!(State::new(m));
        assert!(state.initialize(40, 132, (3, 6), true).is_ok());
        // Turn on transcripting ... it should survive the restore
        assert!(header::set_flag2(&mut state, Flags2::Transcripting).is_ok());

        assert_eq!(state.frame_count(), 1);

        let restore_data = vec![
            b'F', b'O', b'R', b'M', 0x00, 0x00, 0x00, 0x56, b'I', b'F', b'Z', b'S', b'I', b'F',
            b'h', b'd', 0x00, 0x00, 0x00, 0x0D, 0x12, 0x34, 0x32, 0x33, 0x30, 0x37, 0x31, 0x35,
            0x56, 0x78, 0x00, 0x9a, 0xbc, 0x00, b'C', b'M', b'e', b'm', 0x00, 0x00, 0x00, 0x0D,
            0x00, 0xFF, 0x00, 0xFF, 0xFC, 0x00, 0x7E, 0x90, 0x00, 0x7E, 0xFD, 0x00, 0xFE, 0x00,
            b'S', b't', b'k', b's', 0x00, 0x00, 0x00, 0x1E, 0x00, 0x04, 0x8E, 0x03, 0x80, 0x03,
            0x00, 0x02, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x11, 0x11, 0x22, 0x22, 0x00, 0x06,
            0x23, 0x12, 0x00, 0x00, 0x00, 0x00, 0x88, 0x99, 0xaa, 0xbb,
        ];
        assert!(state.restore(restore_data).is_err());
        assert_eq!(state.frame_count(), 1);
    }

    #[test]
    fn test_restore_wrong_serial() {
        let mut map = test_map(5);
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        map[0x02] = 0x12;
        map[0x03] = 0x34;
        map[0x12] = b'2';
        map[0x13] = b'1';
        map[0x14] = b'0';
        map[0x15] = b'7';
        map[0x16] = b'1';
        map[0x17] = b'5';
        map[0x1C] = 0x56;
        map[0x1D] = 0x78;

        let m = Memory::new(map.clone());
        let mut state = assert_ok!(State::new(m));
        assert!(state.initialize(40, 132, (3, 6), true).is_ok());
        // Turn on transcripting ... it should survive the restore
        assert!(header::set_flag2(&mut state, Flags2::Transcripting).is_ok());

        assert_eq!(state.frame_count(), 1);

        let restore_data = vec![
            b'F', b'O', b'R', b'M', 0x00, 0x00, 0x00, 0x56, b'I', b'F', b'Z', b'S', b'I', b'F',
            b'h', b'd', 0x00, 0x00, 0x00, 0x0D, 0x12, 0x34, 0x32, 0x33, 0x30, 0x37, 0x31, 0x35,
            0x56, 0x78, 0x00, 0x9a, 0xbc, 0x00, b'C', b'M', b'e', b'm', 0x00, 0x00, 0x00, 0x0D,
            0x00, 0xFF, 0x00, 0xFF, 0xFC, 0x00, 0x7E, 0x90, 0x00, 0x7E, 0xFD, 0x00, 0xFE, 0x00,
            b'S', b't', b'k', b's', 0x00, 0x00, 0x00, 0x1E, 0x00, 0x04, 0x8E, 0x03, 0x80, 0x03,
            0x00, 0x02, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x11, 0x11, 0x22, 0x22, 0x00, 0x06,
            0x23, 0x12, 0x00, 0x00, 0x00, 0x00, 0x88, 0x99, 0xaa, 0xbb,
        ];
        assert!(state.restore(restore_data).is_err());
        assert_eq!(state.frame_count(), 1);
    }

    #[test]
    fn test_restore_wrong_checksum() {
        let mut map = test_map(5);
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        map[0x02] = 0x12;
        map[0x03] = 0x34;
        map[0x12] = b'2';
        map[0x13] = b'3';
        map[0x14] = b'0';
        map[0x15] = b'7';
        map[0x16] = b'1';
        map[0x17] = b'5';
        map[0x1C] = 0x57;
        map[0x1D] = 0x78;

        let m = Memory::new(map.clone());
        let mut state = assert_ok!(State::new(m));
        assert!(state.initialize(40, 132, (3, 6), true).is_ok());
        // Turn on transcripting ... it should survive the restore
        assert!(header::set_flag2(&mut state, Flags2::Transcripting).is_ok());

        assert_eq!(state.frame_count(), 1);

        let restore_data = vec![
            b'F', b'O', b'R', b'M', 0x00, 0x00, 0x00, 0x56, b'I', b'F', b'Z', b'S', b'I', b'F',
            b'h', b'd', 0x00, 0x00, 0x00, 0x0D, 0x12, 0x34, 0x32, 0x33, 0x30, 0x37, 0x31, 0x35,
            0x56, 0x78, 0x00, 0x9a, 0xbc, 0x00, b'C', b'M', b'e', b'm', 0x00, 0x00, 0x00, 0x0D,
            0x00, 0xFF, 0x00, 0xFF, 0xFC, 0x00, 0x7E, 0x90, 0x00, 0x7E, 0xFD, 0x00, 0xFE, 0x00,
            b'S', b't', b'k', b's', 0x00, 0x00, 0x00, 0x1E, 0x00, 0x04, 0x8E, 0x03, 0x80, 0x03,
            0x00, 0x02, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x11, 0x11, 0x22, 0x22, 0x00, 0x06,
            0x23, 0x12, 0x00, 0x00, 0x00, 0x00, 0x88, 0x99, 0xaa, 0xbb,
        ];
        assert!(state.restore(restore_data).is_err());
        assert_eq!(state.frame_count(), 1);
    }

    #[test]
    fn test_save_undo() {
        let mut map = test_map(3);
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        map[0x02] = 0x12;
        map[0x03] = 0x34;
        map[0x12] = b'2';
        map[0x13] = b'3';
        map[0x14] = b'0';
        map[0x15] = b'7';
        map[0x16] = b'1';
        map[0x17] = b'5';
        map[0x1C] = 0x56;
        map[0x1D] = 0x78;

        let m = Memory::new(map.clone());
        let mut state = assert_ok!(State::new(m));
        // See memory.rs tests ... change dynamic memory a little bit
        // so the compressed memory isn't just runs of 0s
        assert!(state.write_byte(0x200, 0xFC).is_ok());
        assert!(state.write_byte(0x280, 0x10).is_ok());
        assert!(state.write_byte(0x300, 0xFD).is_ok());

        state.frames.push(Frame::new(
            0x500,
            0x501,
            &[0x1122, 0x3344, 0x5566],
            2,
            &[0x1111, 0x2222],
            Some(StoreResult::new(0x402, 0x80)),
            0x48E,
        ));
        state.frames.push(Frame::new(
            0x480,
            0x48C,
            &[0x8899, 0xaabb],
            0,
            &[],
            None,
            0x623,
        ));

        assert_eq!(state.undo_stack.len(), 0);
        assert!(state.save_undo(0x9abc).is_ok());
        assert!(state.undo_stack.back().is_some());
        let quetzal = assert_some!(state.undo_stack.back());
        let ifhd = quetzal.ifhd();
        assert_eq!(ifhd.release_number(), 0x1234);
        assert_eq!(ifhd.serial_number(), "230715".as_bytes());
        assert_eq!(ifhd.checksum(), 0x5678);
        assert_eq!(ifhd.pc(), 0x9abc);
        // let cmem = assert_some!(quetzal.mem());
        assert_eq!(
            quetzal.mem().memory(),
            &[0x00, 0xFF, 0x00, 0xFF, 0xFC, 0x00, 0x7E, 0x90, 0x00, 0x7E, 0xFD, 0x00, 0xFE]
        );
        let stks = quetzal.stks();
        assert_eq!(stks.stks().len(), 2);
        assert_eq!(stks.stks()[0].return_address(), 0x48E);
        assert_eq!(stks.stks()[0].flags(), 0x3);
        assert_eq!(stks.stks()[0].result_variable(), 0x80);
        assert_eq!(stks.stks()[0].arguments(), 3);
        assert_eq!(stks.stks()[0].variables(), &[0x1122, 0x3344, 0x5566]);
        assert_eq!(stks.stks()[0].stack(), &[0x1111, 0x2222]);
        assert_eq!(stks.stks()[1].return_address(), 0x623);
        assert_eq!(stks.stks()[1].flags(), 0x12);
        assert_eq!(stks.stks()[1].result_variable(), 0);
        assert_eq!(stks.stks()[1].arguments(), 0);
        assert_eq!(stks.stks()[1].variables(), &[0x8899, 0xaabb]);
        assert!(stks.stks()[1].stack().is_empty());
    }

    #[test]
    fn test_save_undo_multiple() {
        let mut map = test_map(3);
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        map[0x02] = 0x12;
        map[0x03] = 0x34;
        map[0x12] = b'2';
        map[0x13] = b'3';
        map[0x14] = b'0';
        map[0x15] = b'7';
        map[0x16] = b'1';
        map[0x17] = b'5';
        map[0x1C] = 0x56;
        map[0x1D] = 0x78;

        let m = Memory::new(map.clone());
        let mut state = assert_ok!(State::new(m));
        // See memory.rs tests ... change dynamic memory a little bit
        // so the compressed memory isn't just runs of 0s
        assert!(state.write_byte(0x200, 0xFC).is_ok());
        assert!(state.write_byte(0x280, 0x10).is_ok());
        assert!(state.write_byte(0x300, 0xFD).is_ok());

        state.frames.push(Frame::new(
            0x500,
            0x501,
            &[0x1122, 0x3344, 0x5566],
            2,
            &[0x1111, 0x2222],
            Some(StoreResult::new(0x402, 0x80)),
            0x48E,
        ));
        state.frames.push(Frame::new(
            0x480,
            0x48C,
            &[0x8899, 0xaabb],
            0,
            &[],
            None,
            0x623,
        ));

        for i in 0..10 {
            assert_eq!(state.undo_stack.len(), i);
            assert!(state.save_undo(0x1111 * (i + 1)).is_ok());
        }
        assert_eq!(state.undo_stack.len(), 10);
        assert!(state.save_undo(0xcccc).is_ok());
        assert_eq!(state.undo_stack.len(), 10);
        // The oldest entry (pc = 0x1111) should have been dropped
        assert!(state
            .undo_stack
            .front()
            .is_some_and(|x| x.ifhd().pc() == 0x2222))
    }

    #[test]
    fn test_restore_undo() {
        let mut map = test_map(3);
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        map[0x02] = 0x12;
        map[0x03] = 0x34;
        map[0x12] = b'2';
        map[0x13] = b'3';
        map[0x14] = b'0';
        map[0x15] = b'7';
        map[0x16] = b'1';
        map[0x17] = b'5';
        map[0x1C] = 0x56;
        map[0x1D] = 0x78;

        let m = Memory::new(map.clone());
        let mut state = assert_ok!(State::new(m));
        // See memory.rs tests ... change dynamic memory a little bit
        // so the compressed memory isn't just runs of 0s
        assert!(state.write_byte(0x200, 0xFC).is_ok());
        assert!(state.write_byte(0x280, 0x10).is_ok());
        assert!(state.write_byte(0x300, 0xFD).is_ok());

        state.frames.push(Frame::new(
            0x500,
            0x501,
            &[0x1122, 0x3344, 0x5566],
            2,
            &[0x1111, 0x2222],
            Some(StoreResult::new(0x402, 0x80)),
            0x48E,
        ));
        state.frames.push(Frame::new(
            0x480,
            0x48C,
            &[0x8899, 0xaabb],
            0,
            &[],
            None,
            0x623,
        ));

        assert!(state.save_undo(0x9876).is_ok());
        // Change dynamic memory
        assert!(state.write_byte(0x200, 0x0C).is_ok());
        assert!(state.write_byte(0x280, 0x90).is_ok());
        assert!(state.write_byte(0x300, 0x0D).is_ok());
        // Drop a frame
        assert!(state.frames.pop().is_some());

        assert_eq!(state.frame_count(), 1);
        assert_eq!(state.undo_stack.len(), 1);
        let pc = assert_ok!(state.restore_undo());
        assert_some_eq!(pc, 0x9876);
        assert_eq!(state.undo_stack.len(), 0);
        assert_eq!(state.frame_count(), 2);
        assert_ok_eq!(state.read_byte(0x200), 0xFC);
        assert_ok_eq!(state.read_byte(0x280), 0x10);
        assert_ok_eq!(state.read_byte(0x300), 0xFD);
    }

    #[test]
    fn test_restore_undo_no_state() {
        let mut map = test_map(3);
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        map[0x02] = 0x12;
        map[0x03] = 0x34;
        map[0x12] = b'2';
        map[0x13] = b'3';
        map[0x14] = b'0';
        map[0x15] = b'7';
        map[0x16] = b'1';
        map[0x17] = b'5';
        map[0x1C] = 0x56;
        map[0x1D] = 0x78;

        let m = Memory::new(map.clone());
        let mut state = assert_ok!(State::new(m));
        // See memory.rs tests ... change dynamic memory a little bit
        // so the compressed memory isn't just runs of 0s
        assert!(state.write_byte(0x200, 0xFC).is_ok());
        assert!(state.write_byte(0x280, 0x10).is_ok());
        assert!(state.write_byte(0x300, 0xFD).is_ok());

        state.frames.push(Frame::new(
            0x500,
            0x501,
            &[0x1122, 0x3344, 0x5566],
            2,
            &[0x1111, 0x2222],
            Some(StoreResult::new(0x402, 0x80)),
            0x48E,
        ));

        assert_eq!(state.undo_stack.len(), 0);
        assert!(state.restore_undo().is_err());
        assert_eq!(state.undo_stack.len(), 0);
        assert_eq!(state.frame_count(), 1);
        assert_ok_eq!(state.read_byte(0x200), 0xFC);
        assert_ok_eq!(state.read_byte(0x280), 0x10);
        assert_ok_eq!(state.read_byte(0x300), 0xFD);
    }

    #[test]
    fn test_restart() {
        let mut map = test_map(5);
        for (i, b) in (0x40..0x800).enumerate() {
            map[i + 0x40] = b as u8;
        }
        map[0x02] = 0x12;
        map[0x03] = 0x34;
        map[0x12] = b'2';
        map[0x13] = b'3';
        map[0x14] = b'0';
        map[0x15] = b'7';
        map[0x16] = b'1';
        map[0x17] = b'5';
        map[0x1C] = 0x56;
        map[0x1D] = 0x78;

        let m = Memory::new(map.clone());
        let mut state = assert_ok!(State::new(m));
        assert!(state.initialize(24, 80, (9, 2), true).is_ok());
        assert!(header::set_flag2(&mut state, Flags2::Transcripting).is_ok());
        assert!(state.write_byte(0x200, 0xFC).is_ok());
        assert!(state.write_byte(0x280, 0x10).is_ok());
        assert!(state.write_byte(0x300, 0xFD).is_ok());
        assert!(state.set_variable(0x80, 0x8899).is_ok());

        assert_ok_eq!(state.restart(), 0x400);
        assert_ok_eq!(header::flag2(&state, Flags2::Transcripting), 1);
        assert_ok_eq!(
            header::field_byte(&state, HeaderField::DefaultForeground),
            9
        );
        assert_ok_eq!(
            header::field_byte(&state, HeaderField::DefaultBackground),
            2
        );
        assert_ok_eq!(header::field_byte(&state, HeaderField::ScreenLines), 24);
        assert_ok_eq!(header::field_byte(&state, HeaderField::ScreenColumns), 80);
    }
}
