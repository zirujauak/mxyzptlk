use crate::zmachine::State;
use crate::error::*;

use super::object_address;

pub fn value(
    state: &State,
    object: usize,
    attribute: u8,
) -> Result<bool, RuntimeError> {
    let object_address = object_address(state, object)?;
    let offset = attribute as usize / 8;
    let address = object_address + offset;
    let mask = 1 << (7 - (attribute % 8));
    let max = match state.version {
        3 => 32,
        _ => 48
    };

    if attribute < max {
        let value = state.read_byte(address)?;
        Ok(value & mask == mask)
    } else {
        todo!("Invalid attribute #")
    }
}

pub fn set(
    state: &mut State,
    object: usize,
    attribute: u8,
) -> Result<(), RuntimeError> {
    let object_address = object_address(state, object)?;
    let offset = attribute as usize / 8;
    let address = object_address + offset;
    let mask = 1 << (7 - (attribute % 8));
    let max = match state.version {
        3 => 32,
        _ => 48
    };

    if attribute < max {
        let attribute_byte = state.read_byte(address)?;
        state.write_byte(address, attribute_byte | mask)
    } else {
        todo!("Invalid attribute #")
    }
}

pub fn clear(
    state: &mut State,
    object: usize,
    attribute: u8,
) -> Result<(), RuntimeError> {
    let object_address = object_address(state, object)?;
    let offset = attribute as usize / 8;
    let address = object_address + offset;
    let mask: u8 = 1 << 7 - (attribute % 8);
    let max = match state.version {
        3 => 32,
        _ => 48
    };

    if attribute < max {
        let attribute_byte = state.read_byte(address)?;
        state.write_byte(address, attribute_byte & !mask)
    } else {
        todo!("Invalid attribute #")
    }
}
