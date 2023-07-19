use std::cmp::Ordering;

use crate::{
    error::*,
    zmachine::{state::header::HeaderField, ZMachine},
};

use super::object_address;

fn property_table_address(zmachine: &ZMachine, object: usize) -> Result<usize, RuntimeError> {
    let object_address = object_address(zmachine, object)?;
    let offset = match zmachine.version() {
        3 => 7,
        _ => 12,
    };

    let result = zmachine.read_word(object_address + offset)? as usize;
    Ok(result)
}

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

pub fn short_name(zmachine: &ZMachine, object: usize) -> Result<Vec<u16>, RuntimeError> {
    let property_table_address = property_table_address(zmachine, object)?;
    let header_count = zmachine.read_byte(property_table_address)? as usize;
    let mut ztext = Vec::new();
    for i in 0..header_count {
        ztext.push(zmachine.read_word(property_table_address + 1 + (i * 2))?);
    }

    Ok(ztext)
}

fn default_property(zmachine: &ZMachine, property: u8) -> Result<u16, RuntimeError> {
    let object_table = zmachine.header_word(HeaderField::ObjectTable)? as usize;
    let property_address = object_table + ((property as usize - 1) * 2);
    zmachine.read_word(property_address)
}

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
            _ => Err(RuntimeError::new(
                ErrorCode::PropertySize,
                format!(
                    "Read of property {} on object {} has size {}",
                    property, object, property_size
                ),
            )),
        }
    }
}

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

pub fn set_property(
    zmachine: &mut ZMachine,
    object: usize,
    property: u8,
    value: u16,
) -> Result<(), RuntimeError> {
    let property_address = address(zmachine, object, property)?;
    if property_address == 0 {
        Err(RuntimeError::new(
            ErrorCode::ObjectTreeState,
            "Can't get properyt address for property 0".to_string(),
        ))
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
        } else {
            zmachine.write_word(property_data, value)
        }
    }
}
