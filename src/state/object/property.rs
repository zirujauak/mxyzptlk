use crate::error::*;
use crate::state::header;
use crate::state::header::*;
use crate::state::State;

use super::*;

fn property_table_address(state: &State, object: usize) -> Result<usize, RuntimeError> {
    let object_address = object_address(state, object)?;
    let offset = match header::field_byte(state.memory(), HeaderField::Version)? {
        3 => 7,
        _ => 12,
    };

    let result = state.read_word(object_address + offset)? as usize;
    Ok(result)
}

fn address(state: &State, object: usize, property: u8) -> Result<usize, RuntimeError> {
    let property_table_address = property_table_address(state, object)?;
    let header_size = state.read_byte(property_table_address)? as usize;
    let mut property_address = property_table_address + 1 + (header_size * 2);
    let mut size_byte = state.read_byte(property_address)?;
    let version = header::field_byte(state.memory(), HeaderField::Version)?;
    while size_byte != 0 {
        if version == 3 {
            let prop_num = size_byte & 0x1F;
            let prop_size = (size_byte as usize / 32) + 1;
            if prop_num == property {
                return Ok(property_address);
            } else if prop_num < property {
                return Ok(0);
            } else {
                property_address = property_address + 1 + prop_size;
                size_byte = state.read_byte(property_address)?;
            }
        } else {
            let prop_num = size_byte & 0x3F;
            let mut prop_data = 1;
            let prop_size = if size_byte & 0x80 == 0x80 {
                prop_data = 2;
                let size = state.read_byte(property_address + 1)?;
                if size & 0x3f == 0 {
                    64
                } else {
                    size as usize & 0x3f
                }
            } else {
                if size_byte & 0x40 == 0x40 {
                    2
                } else {
                    1
                }
            };
            if prop_num == property {
                return Ok(property_address);
            } else if prop_num < property {
                return Ok(0);
            } else {
                property_address = property_address + prop_data + prop_size;
                size_byte = state.read_byte(property_address)?;
            }
        }
    }

    Ok(0)
}

fn size(state: &State, property_address: usize) -> Result<usize, RuntimeError> {
    let size_byte = state.read_byte(property_address)?;
    match header::field_byte(state.memory(), HeaderField::Version)? {
        3 => Ok((size_byte as usize / 32) + 1),
        _ => match size_byte & 0xc0 {
            0x40 => Ok(2),
            0x00 => Ok(1),
            _ => {
                let size = state.read_byte(property_address + 1)? as usize & 0x3F;
                if size == 0 {
                    Ok(64)
                } else {
                    Ok(size)
                }
            }
        },
    }
}

fn data_address(state: &State, property_address: usize) -> Result<usize, RuntimeError> {
    match header::field_byte(state.memory(), HeaderField::Version)? {
        3 => Ok(property_address + 1),
        _ => {
            let b = state.read_byte(property_address)?;
            if b & 0x80 == 0x80 {
                Ok(property_address + 2)
            } else {
                Ok(property_address + 1)
            }
        }
    }
}

pub fn property_data_address(
    state: &State,
    object: usize,
    property: u8,
) -> Result<usize, RuntimeError> {
    let property_address = address(state, object, property)?;
    if property_address == 0 {
        Ok(0)
    } else {
        data_address(state, property_address)
    }
}

pub fn property_length(state: &State, property_data_address: usize) -> Result<usize, RuntimeError> {
    if property_data_address == 0 {
        Ok(0)
    } else {
        let size_byte = state.read_byte(property_data_address - 1)?;
        match header::field_byte(state.memory(), HeaderField::Version)? {
            1 | 2 | 3 => size(state, property_data_address - 1),
            4 | 5 | 6 | 7 | 8 => {
                if size_byte & 0x80 == 0x80 {
                    size(state, property_data_address - 2)
                } else {
                    size(state, property_data_address - 1)
                }
            }
            _ => Ok(0),
        }
    }
}

pub fn short_name(state: &State, object: usize) -> Result<Vec<u16>, RuntimeError> {
    let property_table_address = property_table_address(state, object)?;
    let header_count = state.read_byte(property_table_address)? as usize;
    let mut ztext = Vec::new();
    for i in 0..header_count {
        ztext.push(state.read_word(property_table_address + 1 + (i * 2))?);
    }

    Ok(ztext)
}

fn default_property(state: &State, property: u8) -> Result<u16, RuntimeError> {
    let object_table = header::field_word(state.memory(), HeaderField::ObjectTable)? as usize;
    let property_address = object_table + ((property as usize - 1) * 2);
    state.read_word(property_address)
}

pub fn property(state: &State, object: usize, property: u8) -> Result<u16, RuntimeError> {
    let property_address = address(state, object, property)?;
    if property_address == 0 {
        default_property(state, property)
    } else {
        let property_size = size(state, property_address)?;
        let property_data_address = data_address(state, property_address)?;
        match property_size {
            1 => Ok(state.read_byte(property_data_address)? as u16),
            2 => state.read_word(property_data_address),
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

pub fn next_property(state: &State, object: usize, property: u8) -> Result<u8, RuntimeError> {
    let version = header::field_byte(state.memory(), HeaderField::Version)?;
    if property == 0 {
        let prop_table = property_table_address(state, object)?;
        let header_size = state.read_byte(prop_table)? as usize;
        let p1 = state.read_byte(prop_table + 1 + (header_size * 2))?;
        if version < 4 {
            Ok(p1 & 0x1f)
        } else {
            Ok(p1 & 0x3f)
        }
    } else {
        let prop_addr = address(state, object, property)?;
        if prop_addr == 0 {
            Ok(0)
        } else {
            let prop_len = size(state, prop_addr)?;
            let next_prop =
                state.read_byte(property_data_address(state, object, property)? + prop_len)?;
            if version < 4 {
                Ok(next_prop & 0x1f)
            } else {
                Ok(next_prop & 0x3f)
            }
        }
    }
}

pub fn set_property(
    state: &mut State,
    object: usize,
    property: u8,
    value: u16,
) -> Result<(), RuntimeError> {
    let property_address = address(state, object, property)?;
    if property_address == 0 {
        Err(RuntimeError::new(
            ErrorCode::ObjectTreeState,
            "Can't get properyt address for property 0".to_string(),
        ))
    } else {
        let property_size = size(state, property_address)?;
        let property_data = match header::field_byte(state.memory(), HeaderField::Version)? {
            3 => property_address + 1,
            _ => {
                let b = state.read_byte(property_address)?;
                if b & 0x80 == 0x80 {
                    property_address + 2
                } else {
                    property_address + 1
                }
            }
        };

        if property_size == 1 {
            state.write_byte(property_data, value as u8 & 0xFF)
        } else {
            state.write_word(property_data, value)
        }
    }
}
