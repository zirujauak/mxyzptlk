use crate::error::*;
use crate::state::header;
use crate::state::header::*;
use crate::state::State;

pub mod attribute;
pub mod property;

fn object_address(state: &State, object: usize) -> Result<usize, RuntimeError> {
    if object == 0 {
        Ok(0)
    } else {
        let table = header::field_word(state, HeaderField::ObjectTable)? as usize;
        let (offset, size) = match state.version {
            3 => (62, 9),
            _ => (126, 14),
        };

        Ok(table + offset + (size * (object - 1)))
    }
}

fn relative(state: &State, object: usize, offset: usize) -> Result<usize, RuntimeError> {
    if object == 0 {
        Ok(0)
    } else {
        let object_address = object_address(state, object)?;

        match state.version {
            3 => Ok(state.read_byte(object_address + offset)? as usize),
            _ => Ok(state.read_word(object_address + offset)? as usize),
        }
    }
}
pub fn parent(state: &State, object: usize) -> Result<usize, RuntimeError> {
    let offset = match state.version {
        3 => 4,
        _ => 6,
    };

    relative(state, object, offset)
}

pub fn child(state: &State, object: usize) -> Result<usize, RuntimeError> {
    let offset = match state.version {
        3 => 6,
        _ => 10,
    };

    relative(state, object, offset)
}

pub fn sibling(state: &State, object: usize) -> Result<usize, RuntimeError> {
    let offset = match state.version {
        3 => 5,
        _ => 8,
    };

    relative(state, object, offset)
}

pub fn set_relative(
    state: &mut State,
    offset: usize,
    object: usize,
    relative: usize,
) -> Result<(), RuntimeError> {
    let object_address = object_address(state, object)?;

    match state.version {
        3 => state.write_byte(object_address + offset, relative as u8),
        _ => state.write_word(object_address + offset, relative as u16),
    }
}

pub fn set_parent(state: &mut State, object: usize, parent: usize) -> Result<(), RuntimeError> {
    let offset = match state.version {
        3 => 4,
        _ => 6,
    };

    set_relative(state, offset, object, parent)
}

pub fn set_child(state: &mut State, object: usize, child: usize) -> Result<(), RuntimeError> {
    let offset = match state.version {
        3 => 6,
        _ => 10,
    };

    set_relative(state, offset, object, child)
}

pub fn set_sibling(state: &mut State, object: usize, sibling: usize) -> Result<(), RuntimeError> {
    let offset = match state.version {
        3 => 5,
        _ => 8,
    };

    set_relative(state, offset, object, sibling)
}
