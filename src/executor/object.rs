use super::header;
use super::state::State;
use super::util;

fn object_address(state: &State, object: usize) -> usize {
    match state.version {
        1 | 2 | 3 => header::object_table(&state.memory_map()) + 62 + (9 * (object - 1)),
        4 | 5 | 6 | 7 | 8 => header::object_table(&state.memory_map()) + 126 + (14 * (object - 1)),
        // TODO: Error
        _ => 0,
    }
}

pub fn attribute(state: &State, object: usize, attribute: u8) -> bool {
    let object_address = object_address(state, object);
    let offset = attribute as usize / 8;
    let address = object_address + offset;
    let mask = 1 << (7 - (attribute % 8));
    match state.version {
        1 | 2 | 3 => {
            if attribute < 32 {
                util::byte_value(&state.memory_map(), address) & mask == mask
            } else {
                warn!("Invalid attribute #{:02x}", attribute);
                false
            }
        }
        4 | 5 | 6 | 7 | 8 => {
            if attribute < 48 {
                util::byte_value(&state.memory_map(), address) & mask == mask
            } else {
                warn!("Invalid attribute #{:02x}", attribute);
                false
            }
        }
        _ => false,
    }
}

pub fn set_attribute(state: &mut State, object: usize, attribute: u8) {
    let object_address = object_address(state, object);
    let offset = attribute as usize / 8;
    let address = object_address + offset;
    let mask = 1 << (7 - (attribute % 8));
    let attribute_byte = util::byte_value(&state.memory_map(), address);
    match state.version {
        1 | 2 | 3 => {
            if attribute < 32 {
                util::set_byte(state.memory_map_mut(), address, attribute_byte | mask)
            } else {
                warn!("Invalid attribute #{:02x}", attribute)
            }
        }
        4 | 5 | 6 | 7 | 8 => {
            if attribute < 48 {
                util::set_byte(state.memory_map_mut(), address, attribute_byte | mask)
            } else {
                warn!("Invalid attribute #{:02x}", attribute)
            }
        }
        _ => {}
    }
}

pub fn clear_attribute(state: &mut State, object: usize, attribute: u8) {
    let address = object_address(state, object);
    let mask: u8 = 1 << 7 - (attribute % 8);
    let offset = attribute as usize / 8;
    let byte = util::byte_value(&state.memory_map(), address + offset);

    match state.version {
        1 | 2 | 3 => {
            if attribute < 32 {
                util::set_byte(state.memory_map_mut(), address + offset, byte & !mask);
            }
        }
        4 | 5 | 6 | 7 | 8 => {
            if attribute < 48 {
                util::set_byte(state.memory_map_mut(), address + offset, byte & !mask)
            }
        }
        _ => {}
    }
}

fn property_table_address(state: &State, object: usize) -> usize {
    let object_table = object_address(state, object);
    match state.version {
        1 | 2 | 3 => util::word_value(&state.memory_map(), object_table + 7) as usize,
        4 | 5 | 6 | 7 | 8 => util::word_value(&state.memory_map(), object_table + 12) as usize,
        _ => 0,
    }
}

fn property_size(state: &State, property_address: usize) -> u8 {
    match state.version {
        1 | 2 | 3 => {
            let size_byte = util::byte_value(&state.memory_map(), property_address);
            (size_byte / 32) + 1
        }
        4 | 5 | 6 | 7 | 8 => {
            let size_byte = util::byte_value(&state.memory_map(), property_address);
            if size_byte & 0x80 == 0x80 {
                util::byte_value(&state.memory_map(), property_address + 1) & 0x3F
            } else {
                if size_byte & 0x40 == 0x40 {
                    2
                } else {
                    1
                }
            }
        }
        _ => 0,
    }
}

fn property_address(state: &State, object: usize, property: u8) -> usize {
    let property_table = property_table_address(state, object);
    let header_size = util::byte_value(&state.memory_map(), property_table) as usize;
    let mut property_address = property_table + 1 + (header_size * 2);

    let mut size_byte = util::byte_value(&state.memory_map(), property_address);
    while size_byte != 0 {
        match state.version {
            1 | 2 | 3 => {
                let prop_num = size_byte & 0x1F;
                let prop_size = (size_byte as usize / 32) + 1;
                if prop_num == property {
                    return property_address;
                } else if prop_num < property {
                    return 0;
                } else {
                    property_address = property_address + 1 + prop_size;
                    size_byte = util::byte_value(&state.memory_map(), property_address);
                }
            }
            4 | 5 | 6 | 7 | 8 => {
                let prop_num = size_byte & 0x3F;
                let mut prop_data = 1;
                let prop_size = if size_byte & 0x80 == 0x80 {
                    prop_data = 2;
                    util::byte_value(&state.memory_map(), property_address + 1) as usize & 0x3F
                } else {
                    if size_byte & 0x40 == 0x40 {
                        2
                    } else {
                        1
                    }
                };
                if prop_num == property {
                    return property_address;
                } else if prop_num < property {
                    return 0;
                } else {
                    property_address = property_address + prop_data + prop_size;
                    size_byte = util::byte_value(&state.memory_map(), property_address);
                }
            }
            _ => return 0,
        }
    }
    return 0;
}

fn default_property(state: &State, property: u8) -> u16 {
    let object_table = header::object_table(&state.memory_map());
    let property_address = object_table + (property as usize * 2);
    util::word_value(&state.memory_map(), property_address)
}

pub fn property(state: &State, object: usize, property: u8) -> u16 {
    let property_address = property_address(state, object, property);
    if property_address == 0 {
        default_property(state, property)
    } else {
        let size = property_size(state, property_address);
        match state.version {
            1 | 2 | 3 => match size {
                1 => util::byte_value(&state.memory_map(), property_address + 1) as u16,
                2 => util::word_value(&state.memory_map(), property_address + 1),
                _ => {
                    trace!("Can't get property with length {}", size);
                    panic!("Can't get property with length {}", size);
                }
            },
            4 | 5 | 6 | 7 | 8 => 0,
            _ => 0,
        }
    }
}
pub fn set_property(state: &mut State, object: usize, property: u8, value: u16) {
    trace!(
        "Set property #{:02} on object #{:04x} to #{:04x}",
        property,
        object,
        value
    );

    let property_address = property_address(state, object, property);
    if property_address == 0 {
        error!(
            "Object #{:04x} does not have property #{:02x}",
            object, property
        );
        panic!(
            "Set property #{:02x} on object #{:04x} - property does not exist",
            object, property
        );
    }
    let property_size = property_size(state, property_address);
    let property_data = match state.version {
        1 | 2 | 3 => property_address + 1,
        4 | 5 | 6 | 7 | 8 => {
            if property_address & 0x80 == 0x80 {
                property_address + 2
            } else {
                property_address + 1
            }
        }
        _ => 0,
    };

    match property_size {
        1 => util::set_byte(state.memory_map_mut(), property_data, (value & 0xFF) as u8),
        2 => util::set_word(state.memory_map_mut(), property_data, value),
        _ => {
            error!(
                "Object #{:04x} property #{:02x} has length {}",
                object, property, property_size
            );
            panic!(
                "Set property #{:02x} on object #{:04x} has length {}",
                object, property, property_size
            );
        }
    }
}

pub fn short_name(state: &State, object: usize) -> Vec<u16> {
    let property_table = property_table_address(state, object);
    let header_count = util::byte_value(&state.memory_map(), property_table);
    let mut ztext = Vec::new();
    for i in 0..header_count as usize {
        ztext.push(util::word_value(
            &state.memory_map(),
            property_table + 1 + (i * 2),
        ));
    }

    ztext
}

pub fn parent(state: &State, object: usize) -> usize {
    let object_address = object_address(state, object);

    match state.version {
        1 | 2 | 3 => util::byte_value(&state.memory_map(), object_address + 4) as usize,
        4 | 5 | 6 | 7 | 8 => util::word_value(&state.memory_map(), object_address + 6) as usize,
        _ => 0,
    }
}

pub fn child(state: &State, object: usize) -> usize {
    let object_address = object_address(state, object);

    match state.version {
        1 | 2 | 3 => util::byte_value(&state.memory_map(), object_address + 6) as usize,
        4 | 5 | 6 | 7 | 8 => util::word_value(&state.memory_map(), object_address + 10) as usize,
        _ => 0,
    }
}

pub fn sibling(state: &State, object: usize) -> usize {
    let object_address = object_address(state, object);

    match state.version {
        1 | 2 | 3 => util::byte_value(&state.memory_map(), object_address + 5) as usize,
        4 | 5 | 6 | 7 | 8 => util::word_value(&state.memory_map(), object_address + 8) as usize,
        _ => 0,
    }
}
