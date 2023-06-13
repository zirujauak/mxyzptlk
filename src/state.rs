mod frame_stack;
pub mod header;
mod input;
mod instruction;
pub mod memory;
mod screen;

use crate::error::*;
use frame_stack::*;
use header::*;
use instruction::decoder::*;
use instruction::processor::*;
use instruction::*;
use memory::*;
use screen::*;
use std::fmt;

pub struct State {
    version: u8,
    memory: Memory,
    static_mark: usize,
    screen: Screen,
    frame_stack: FrameStack,
}

impl State {
    pub fn new(memory: Memory, rows: u32, columns: u32) -> Result<State, RuntimeError> {
        let version = header::field_byte(&memory, HeaderField::Version)?;
        let static_mark = header::field_word(&memory, HeaderField::StaticMark)? as usize;
        if version < 3 || version == 6 || version > 8 {
            Err(RuntimeError::new(
                ErrorCode::UnsupportedVersion,
                format!("Version {} is not currently supported", version),
            ))
        } else {
            let screen = match version {
                3 => Screen::new_v3(rows, columns, Color::White, Color::Black),
                4 => Screen::new_v4(rows, columns, Color::White, Color::Black),
                _ => Screen::new_v5(rows, columns, Color::White, Color::Black),
            };
            let frame_stack =
                FrameStack::new(header::field_word(&memory, HeaderField::InitialPC)? as usize);

            Ok(State {
                version,
                memory,
                static_mark: static_mark,
                screen,
                frame_stack,
            })
        }
    }

    pub fn initialize(&mut self) -> Result<(), RuntimeError> {
        // Set V3 Flags 1
        if self.version < 4 {
            header::clear_flag1(&mut self.memory, Flags1v3::StatusLineNotAvailable as u8)?;
            header::set_flag1(&mut self.memory, Flags1v3::ScreenSplitAvailable as u8)?;
            header::set_flag1(&mut self.memory, Flags1v3::VariablePitchDefault as u8)?;
        }

        // Set V4+ Flags 1
        if self.version > 3 {
            header::set_byte(
                &mut self.memory,
                HeaderField::DefaultBackground,
                Color::Black as u8,
            )?;
            header::set_byte(
                &mut self.memory,
                HeaderField::DefaultForeground,
                Color::White as u8,
            )?;
            header::set_byte(
                &mut self.memory,
                HeaderField::ScreenLines,
                self.screen.rows() as u8,
            )?;
            header::set_byte(
                &mut self.memory,
                HeaderField::ScreenColumns,
                self.screen.columns() as u8,
            )?;
        }

        // Set V5+ Flags 1
        if self.version > 4 {
            header::set_word(
                &mut self.memory,
                HeaderField::ScreenHeight,
                self.screen.rows() as u16,
            )?;
            header::set_word(
                &mut self.memory,
                HeaderField::ScreenWidth,
                self.screen.columns() as u16,
            )?;
            header::set_byte(&mut self.memory, HeaderField::FontWidth, 1)?;
            header::set_byte(&mut self.memory, HeaderField::FontHeight, 1)?;
            header::set_flag1(&mut self.memory, Flags1v4::ColoursAvailable as u8)?;
            header::set_flag1(&mut self.memory, Flags1v4::BoldfaceAvailable as u8)?;
            header::set_flag1(&mut self.memory, Flags1v4::ItalicAvailable as u8)?;
            header::set_flag1(&mut self.memory, Flags1v4::FixedSpaceAvailable as u8)?;
            header::set_flag1(&mut self.memory, Flags1v4::TimedInputAvailable as u8)?;
        }

        Ok(())
    }

    pub fn memory(&self) -> &Memory {
        &self.memory
    }

    pub fn screen(&self) -> &Screen {
        &self.screen
    }

    // MMU
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
            self.memory.write_word(address, value)
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

    // Variables
    pub fn variable(&mut self, variable: u8) -> Result<u16, RuntimeError> {
        self.frame_stack
            .variable(&mut self.memory, variable as usize)
    }

    pub fn peek_variable(&mut self, variable: u8) -> Result<u16, RuntimeError> {
        if variable == 0 {
            self.frame_stack.current_frame()?.peek()
        } else {
            self.frame_stack.variable(&mut self.memory, variable as usize)
        }
    }

    pub fn set_variable(&mut self, variable: u8, value: u16) -> Result<(), RuntimeError> {
        self.frame_stack
            .set_variable(&mut self.memory, variable as usize, value)
    }

    pub fn set_variable_indirect(&mut self, variable: u8, value: u16) -> Result<(), RuntimeError> {
        if variable == 0 {
            self.frame_stack.current_frame_mut()?.pop();
        } 
        self.frame_stack.set_variable(&mut self.memory, variable as usize, value)
    }

    // Routines
    pub fn call_routine(
        &mut self,
        address: usize,
        arguments: &Vec<u16>,
        result: Option<StoreResult>,
        return_address: usize,
    ) -> Result<usize, RuntimeError> {
        self.frame_stack.call_routine(
            &mut self.memory,
            address,
            arguments,
            result,
            return_address,
        )?;
        self.frame_stack.pc()
    }

    pub fn return_routine(&mut self, value: u16) -> Result<usize, RuntimeError> {
        let result = self.frame_stack.return_routine(&mut self.memory, value)?;
        match result {
            Some(r) => self.set_variable(r.variable(), value)?,
            None => ()
        }
        self.frame_stack.pc()
    }

    pub fn set_pc(&mut self, pc: usize) -> Result<(), RuntimeError> {
        self.frame_stack.current_frame_mut()?.set_pc(pc);
        Ok(())
    }

    // Run
    pub fn run(&mut self) -> Result<(), RuntimeError> {
        loop {
            let pc = self.frame_stack.pc()?;
            let instruction = decoder::decode_instruction(&self.memory, pc)?;
            let pc = processor::dispatch(self, &instruction)?;
            self.set_pc(pc)?
        }
    }
}
