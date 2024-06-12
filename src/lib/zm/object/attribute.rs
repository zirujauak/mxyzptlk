//! Object [attribute](https://inform-fiction.org/zmachine/standards/z1point1/sect12.html#one) utility functions
use crate::{error::*, recoverable_error, zmachine::ZMachine};

use super::object_address;

/// Gets the value of an attribute for an object
///
/// # Arguments
/// * `zmachine` - Reference to the zmachine
/// * `object` - Object number
/// * `attribute` - Attribute number
///
/// # Returns
/// [Result] with the attribute value - `true` when set, `false` when clear - or a [RuntimeError]
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

/// Set an attribute for an object
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `object` - Object number
/// * `attribute` - Attribute number
///
/// # Returns
/// Empty [Result] or a [RuntimeError]
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

/// Clear an attribute for an object
///
/// # Arguments
/// * `zmachine` - Mutable reference to the zmachine
/// * `object` - Object number
/// * `attribute` - Attribute number
///
/// # Returns
/// Empty [Result] or a [RuntimeError]
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
