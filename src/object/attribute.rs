use crate::{error::*, recoverable_error, zmachine::ZMachine};

use super::object_address;

pub fn value(zmachine: &ZMachine, object: usize, attribute: u8) -> Result<bool, RuntimeError> {
    let object_address = object_address(zmachine, object)?;
    let offset = attribute as usize / 8;
    let address = object_address + offset;
    let mask = 1 << (7 - (attribute % 8));
    let max = match zmachine.version() {
        3 => 32,
        _ => 48,
    };

    if attribute < max {
        let value = zmachine.read_byte(address)?;
        Ok(value & mask == mask)
    } else {
        recoverable_error!(
            ErrorCode::InvalidObjectAttribute,
            "Test of invalid attribute {} on object {}",
            attribute,
            object
        )
    }
}

pub fn set(zmachine: &mut ZMachine, object: usize, attribute: u8) -> Result<(), RuntimeError> {
    let object_address = object_address(zmachine, object)?;
    let offset = attribute as usize / 8;
    let address = object_address + offset;
    let mask = 1 << (7 - (attribute % 8));
    let max = match zmachine.version() {
        3 => 32,
        _ => 48,
    };

    if attribute < max {
        let attribute_byte = zmachine.read_byte(address)?;
        zmachine.write_byte(address, attribute_byte | mask)
    } else {
        recoverable_error!(
            ErrorCode::InvalidObjectAttribute,
            "Set of invalid attribute {} on object {}",
            attribute,
            object
        )
    }
}

pub fn clear(zmachine: &mut ZMachine, object: usize, attribute: u8) -> Result<(), RuntimeError> {
    let object_address = object_address(zmachine, object)?;
    let offset = attribute as usize / 8;
    let address = object_address + offset;
    let mask: u8 = 1 << (7 - (attribute % 8));
    let max = match zmachine.version() {
        3 => 32,
        _ => 48,
    };

    if attribute < max {
        let attribute_byte = zmachine.read_byte(address)?;
        zmachine.write_byte(address, attribute_byte & !mask)
    } else {
        recoverable_error!(
            ErrorCode::InvalidObjectAttribute,
            "Clear of invalid attribute {} on object {}",
            attribute,
            object
        )
    }
}
