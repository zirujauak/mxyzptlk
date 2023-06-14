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
        let table = header::field_word(state.memory(), HeaderField::ObjectTable)? as usize;
        let (offset, size) = match header::field_byte(state.memory(), HeaderField::Version)? {
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

        match header::field_byte(state.memory(), HeaderField::Version)? {
            3 => Ok(state.read_byte(object_address + offset)? as usize),
            _ => Ok(state.read_word(object_address + offset)? as usize)
        }
    }
}
pub fn parent(state: &State, object: usize) -> Result<usize, RuntimeError> {
    let offset = match header::field_byte(state.memory(), HeaderField::Version)? {
        3 => 4,
        _ => 6
    };

    relative(state, object, offset)
}

pub fn child(state: &State, object: usize) -> Result<usize, RuntimeError> {
    let offset = match header::field_byte(state.memory(), HeaderField::Version)? {
        3 => 6,
        _ => 10
    };    

    relative(state, object, offset)
}

pub fn sibling(state: &State, object: usize) -> Result<usize, RuntimeError> {
    let offset = match header::field_byte(state.memory(), HeaderField::Version)? {
        3 => 5,
        _ => 8
    };    

    relative(state, object, offset)
}