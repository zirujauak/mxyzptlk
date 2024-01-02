use crate::{
    error::*,
    zmachine::{header::HeaderField, ZMachine},
};

pub mod attribute;
pub mod property;

fn object_address(zmachine: &ZMachine, object: usize) -> Result<usize, RuntimeError> {
    if object == 0 {
        Ok(0)
    } else {
        let table = zmachine.header_word(HeaderField::ObjectTable)? as usize;
        let (offset, size) = match zmachine.version() {
            3 => (62, 9),
            _ => (126, 14),
        };

        Ok(table + offset + (size * (object - 1)))
    }
}

fn relative(zmachine: &ZMachine, object: usize, offset: usize) -> Result<usize, RuntimeError> {
    if object == 0 {
        Ok(0)
    } else {
        let object_address = object_address(zmachine, object)?;

        match zmachine.version() {
            3 => Ok(zmachine.read_byte(object_address + offset)? as usize),
            _ => Ok(zmachine.read_word(object_address + offset)? as usize),
        }
    }
}
pub fn parent(zmachine: &ZMachine, object: usize) -> Result<usize, RuntimeError> {
    let offset = match zmachine.version() {
        3 => 4,
        _ => 6,
    };

    relative(zmachine, object, offset)
}

pub fn child(zmachine: &ZMachine, object: usize) -> Result<usize, RuntimeError> {
    let offset = match zmachine.version() {
        3 => 6,
        _ => 10,
    };

    relative(zmachine, object, offset)
}

pub fn sibling(zmachine: &ZMachine, object: usize) -> Result<usize, RuntimeError> {
    let offset = match zmachine.version() {
        3 => 5,
        _ => 8,
    };

    relative(zmachine, object, offset)
}

fn set_relative(
    zmachine: &mut ZMachine,
    offset: usize,
    object: usize,
    relative: usize,
) -> Result<(), RuntimeError> {
    let object_address = object_address(zmachine, object)?;

    match zmachine.version() {
        3 => zmachine.write_byte(object_address + offset, relative as u8),
        _ => zmachine.write_word(object_address + offset, relative as u16),
    }
}

pub fn set_parent(
    zmachine: &mut ZMachine,
    object: usize,
    parent: usize,
) -> Result<(), RuntimeError> {
    let offset = match zmachine.version() {
        3 => 4,
        _ => 6,
    };

    set_relative(zmachine, offset, object, parent)
}

pub fn set_child(zmachine: &mut ZMachine, object: usize, child: usize) -> Result<(), RuntimeError> {
    let offset = match zmachine.version() {
        3 => 6,
        _ => 10,
    };

    set_relative(zmachine, offset, object, child)
}

pub fn set_sibling(
    zmachine: &mut ZMachine,
    object: usize,
    sibling: usize,
) -> Result<(), RuntimeError> {
    let offset = match zmachine.version() {
        3 => 5,
        _ => 8,
    };

    set_relative(zmachine, offset, object, sibling)
}
