//! Object [property](https://inform-fiction.org/zmachine/standards/z1point1/sect12.html#four) utility functions
use std::cmp::Ordering;

use crate::{
    error::*,
    fatal_error,
    zmachine::{header::HeaderField, ZMachine},
};

use super::object_address;

/// Gets the property table byte address for an object
///
/// # Arguments
/// * `zmachine` - Reference to the zmachine
/// * `object` - Object number
///
/// # Returns
/// [Result] with the byte address of the property table or a [RuntimeError]
fn property_table_address(zmachine: &ZMachine, object: usize) -> Result<usize, RuntimeError> {
    let object_address = object_address(zmachine, object)?;
    let offset = match zmachine.version() {
        3 => 7,
        _ => 12,
    };

    let result = zmachine.read_word(object_address + offset)? as usize;
    Ok(result)
}

/// Gets the byte address for a specific property for an object.
///
/// If the property does not exist for the object, 0 is returned.
///
/// # Arguments
/// * `zmachine` - Reference to the zmachine
/// * `object` - Object number
/// * `property` - Property number
///
/// # Returns
/// [Result] with the byte address of the object's property, 0,  or a [RuntimeError]
fn address(zmachine: &ZMachine, object: usize, property: u8) -> Result<usize, RuntimeError> {
    let property_table_address = property_table_address(zmachine, object)?;
    let header_size = zmachine.read_byte(property_table_address)? as usize;
    let mut property_address = property_table_address + 1 + (header_size * 2);
    let mut size_byte = zmachine.read_byte(property_address)?;
    while size_byte != 0 {
        if zmachine.version() == 3 {
            let prop_num = size_byte & 0x1F;
            let prop_size = (size_byte as usize / 32) + 1;
            match prop_num.cmp(&property) {
                Ordering::Equal => return Ok(property_address),
                Ordering::Less => return Ok(0),
                _ => {
                    property_address = property_address + 1 + prop_size;
                    size_byte = zmachine.read_byte(property_address)?;
                }
            }
        } else {
            let prop_num = size_byte & 0x3F;
            let mut prop_data = 1;
            let prop_size = if size_byte & 0x80 == 0x80 {
                prop_data = 2;
                let size = zmachine.read_byte(property_address + 1)?;
                if size & 0x3f == 0 {
                    64
                } else {
                    size as usize & 0x3f
                }
            } else if size_byte & 0x40 == 0x40 {
                2
            } else {
                1
            };

            match prop_num.cmp(&property) {
                Ordering::Equal => return Ok(property_address),
                Ordering::Less => return Ok(0),
                _ => {
                    property_address = property_address + prop_data + prop_size;
                    size_byte = zmachine.read_byte(property_address)?;
                }
            }
        }
    }

    Ok(0)
}

/// Gets the size of a property
///
/// # Arguments
/// * `zmachine` - Reference to the zmachine
/// * `property_address` - Byte address of the the property
///
/// # Returns
/// [Result] property size in bytes or a [RuntimeError]
fn size(zmachine: &ZMachine, property_address: usize) -> Result<usize, RuntimeError> {
    let size_byte = zmachine.read_byte(property_address)?;
    match zmachine.version() {
        3 => Ok((size_byte as usize / 32) + 1),
        _ => match size_byte & 0xc0 {
            0x40 => Ok(2),
            0x00 => Ok(1),
            _ => {
                let size = zmachine.read_byte(property_address + 1)? as usize & 0x3F;
                if size == 0 {
                    Ok(64)
                } else {
                    Ok(size)
                }
            }
        },
    }
}

/// Gets the bytes address of a property's data
///
/// # Arguments
/// * `zmachine` - Reference to the zmachine
/// * `property_address` - Byte address of the property
///
/// # Returns
/// [Result] with the byte address of the property data or a [RuntimeError]
fn data_address(zmachine: &ZMachine, property_address: usize) -> Result<usize, RuntimeError> {
    match zmachine.version() {
        3 => Ok(property_address + 1),
        _ => {
            let b = zmachine.read_byte(property_address)?;
            if b & 0x80 == 0x80 {
                Ok(property_address + 2)
            } else {
                Ok(property_address + 1)
            }
        }
    }
}

/// Gets the byte address of an object's property
///
/// If the property does not exist for the object, 0 is returned.
/// # Arguments
/// * `zmachine` - Reference to the zmachine
/// * `object` - Object number
/// * `property` - Property number
///
/// # Returns
/// [Result] with the byte address of the property data, 0, or a [RuntimeError]
pub fn property_data_address(
    zmachine: &ZMachine,
    object: usize,
    property: u8,
) -> Result<usize, RuntimeError> {
    let property_address = address(zmachine, object, property)?;
    if property_address == 0 {
        Ok(0)
    } else {
        data_address(zmachine, property_address)
    }
}

/// Gets the length of a property's data
///
/// If the `property_data_address` is 0, 0 is returned.
/// # Arguments
/// * `zmachine` - Reference to the zmachine
/// * `property_data_address` - Byte address of the property's data
///
/// # Returns
/// [Result] the length of a property's data, 0, or a [RuntimeError]
pub fn property_length(
    zmachine: &ZMachine,
    property_data_address: usize,
) -> Result<usize, RuntimeError> {
    if property_data_address == 0 {
        Ok(0)
    } else {
        let size_byte = zmachine.read_byte(property_data_address - 1)?;
        match zmachine.version() {
            3 => size(zmachine, property_data_address - 1),
            _ => {
                if size_byte & 0x80 == 0x80 {
                    size(zmachine, property_data_address - 2)
                } else {
                    size(zmachine, property_data_address - 1)
                }
            }
        }
    }
}

/// Gets the ztext of the short name of an object
///
/// # Arguments
/// * `zmachine` - Reference to the zmachine
/// * `object` - Object number
///
/// # Returns
/// [Result] with a vector of ztext words or a [RuntimeError]
pub fn short_name(zmachine: &ZMachine, object: usize) -> Result<Vec<u16>, RuntimeError> {
    let property_table_address = property_table_address(zmachine, object)?;
    let header_count = zmachine.read_byte(property_table_address)? as usize;
    let mut ztext = Vec::new();
    for i in 0..header_count {
        ztext.push(zmachine.read_word(property_table_address + 1 + (i * 2))?);
    }

    Ok(ztext)
}

/// Gets the default value of a property
///
/// # Arguments
/// * `zmachine` - Reference to the zmachine
/// * `property` - Property number
///
/// # Returns
/// [Result] with the default word value of a property or a [RuntimeError]
fn default_property(zmachine: &ZMachine, property: u8) -> Result<u16, RuntimeError> {
    let object_table = zmachine.header_word(HeaderField::ObjectTable)? as usize;
    let property_address = object_table + ((property as usize - 1) * 2);
    zmachine.read_word(property_address)
}

/// Gets the value of a property for an object
///
/// The property value must be either a byte or a word value. If the property does not exist
/// for the object, the default property word value is returned.
///
/// # Arguments
/// * `zmachine` - Reference to the zmachine
/// * `object` - Object number
/// * `property` - Property number
///
/// # Returns
/// [Result] with the property value or a [RuntimeError]
pub fn property(zmachine: &ZMachine, object: usize, property: u8) -> Result<u16, RuntimeError> {
    let property_address = address(zmachine, object, property)?;
    if property_address == 0 {
        default_property(zmachine, property)
    } else {
        let property_size = size(zmachine, property_address)?;
        let property_data_address = data_address(zmachine, property_address)?;
        match property_size {
            1 => Ok(zmachine.read_byte(property_data_address)? as u16),
            2 => zmachine.read_word(property_data_address),
            _ => fatal_error!(
                ErrorCode::InvalidObjectPropertySize,
                "Read of property {} on object {} should have size 1 or 2, was {}",
                property,
                object,
                property_size
            ),
        }
    }
}

/// Gets the next property set on an object.
///
/// Properties are ordered in descending order by number.  If `property` is 0, the first property number on the object is returned.
/// If there is no next property, 0 is returned.
///
/// # Arguments
/// * `zmachine` - Reference to the zmachine
/// * `object` - Object number
/// * `property` - Property number
///
/// # Returns
/// [Result] with the next property number set for the object, 0, or a [RuntimeError]
pub fn next_property(zmachine: &ZMachine, object: usize, property: u8) -> Result<u8, RuntimeError> {
    if property == 0 {
        let prop_table = property_table_address(zmachine, object)?;
        let header_size = zmachine.read_byte(prop_table)? as usize;
        let p1 = zmachine.read_byte(prop_table + 1 + (header_size * 2))?;
        if zmachine.version() < 4 {
            Ok(p1 & 0x1f)
        } else {
            Ok(p1 & 0x3f)
        }
    } else {
        let prop_addr = address(zmachine, object, property)?;
        if prop_addr == 0 {
            Ok(0)
        } else {
            let prop_len = size(zmachine, prop_addr)?;
            let next_prop = zmachine
                .read_byte(property_data_address(zmachine, object, property)? + prop_len)?;
            if zmachine.version() < 4 {
                Ok(next_prop & 0x1f)
            } else {
                Ok(next_prop & 0x3f)
            }
        }
    }
}

/// Sets the value of a proeprty for an object.
///
/// The property must exist on the object and must be either a byte or word value.
///
/// # Arguments
/// * `zmachine` - Reference to the zmachine
/// * `object` - Object number
/// * `property` - Property number
/// * `value` - Byte or word value to set
///
/// # Returns
/// Empty [Result] or a [RuntimeError]
pub fn set_property(
    zmachine: &mut ZMachine,
    object: usize,
    property: u8,
    value: u16,
) -> Result<(), RuntimeError> {
    let property_address = address(zmachine, object, property)?;
    if property_address == 0 {
        fatal_error!(
            ErrorCode::InvalidObjectProperty,
            "Object {} does not have property {}",
            object,
            property
        )
    } else {
        let property_size = size(zmachine, property_address)?;
        let property_data = match zmachine.version() {
            3 => property_address + 1,
            _ => {
                let b = zmachine.read_byte(property_address)?;
                if b & 0x80 == 0x80 {
                    property_address + 2
                } else {
                    property_address + 1
                }
            }
        };

        if property_size == 1 {
            zmachine.write_byte(property_data, value as u8)
        } else if property_size == 2 {
            zmachine.write_word(property_data, value)
        } else {
            fatal_error!(
                ErrorCode::InvalidObjectProperty,
                "Object {} property {} size ({}) is not a byte or a word",
                object,
                property,
                property_size
            )
        }
    }
}
