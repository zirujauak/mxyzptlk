use super::header;
use super::state::State;

fn object_address(state: &State, object: usize) -> usize {
    if object == 0 {
        0
    } else {
        match state.version {
            1 | 2 | 3 => header::object_table(state) + 62 + (9 * (object - 1)),
            4 | 5 | 6 | 7 | 8 => header::object_table(state) + 126 + (14 * (object - 1)),
            // TODO: Error
            _ => 0,
        }
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
                state.byte_value(address) & mask == mask
            } else {
                warn!("Invalid attribute #{:02x}", attribute);
                false
            }
        }
        4 | 5 | 6 | 7 | 8 => {
            if attribute < 48 {
                state.byte_value(address) & mask == mask
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
    let attribute_byte = state.byte_value(address);
    match state.version {
        1 | 2 | 3 => {
            if attribute < 32 {
                state.set_byte(address, attribute_byte | mask)
            } else {
                warn!("Invalid attribute #{:02x}", attribute)
            }
        }
        4 | 5 | 6 | 7 | 8 => {
            if attribute < 48 {
                state.set_byte(address, attribute_byte | mask)
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
    let byte = state.byte_value(address + offset);

    match state.version {
        1 | 2 | 3 => {
            if attribute < 32 {
                state.set_byte(address + offset, byte & !mask);
            }
        }
        4 | 5 | 6 | 7 | 8 => {
            if attribute < 48 {
                state.set_byte(address + offset, byte & !mask)
            }
        }
        _ => {}
    }
}

fn property_table_address(state: &State, object: usize) -> usize {
    let object_table = object_address(state, object);
    match state.version {
        1 | 2 | 3 => state.word_value(object_table + 7) as usize,
        4 | 5 | 6 | 7 | 8 => state.word_value(object_table + 12) as usize,
        _ => 0,
    }
}

fn property_size(state: &State, property_address: usize) -> usize {
    let size_byte = state.byte_value(property_address);
    match state.version {
        1 | 2 | 3 => (size_byte as usize / 32) + 1,
        4 | 5 | 6 | 7 | 8 => match size_byte & 0xC0 {
            0x40 => 2,
            0x20 => 1,
            _ => {
                let size = state.byte_value(property_address + 1) as usize & 0x3F;
                if size == 0 {
                    64
                } else {
                    size
                }
            }
        },
        _ => 0,
    }
}

fn property_data_address(state: &State, property_address: usize) -> usize {
    match state.version {
        1 | 2 | 3 => property_address + 1,
        4 | 5 | 6 | 7 | 8 => {
            if state.byte_value(property_address) & 0x80 == 0x80 {
                property_address + 2
            } else {
                property_address + 1
            }
        }
        _ => 0,
    }
}

fn property_address(state: &State, object: usize, property: u8) -> usize {
    let property_table = property_table_address(state, object);
    let header_size = state.byte_value(property_table) as usize;
    let mut property_address = property_table + 1 + (header_size * 2);

    let mut size_byte = state.byte_value(property_address);
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
                    size_byte = state.byte_value(property_address);
                }
            }
            4 | 5 | 6 | 7 | 8 => {
                let prop_num = size_byte & 0x3F;
                let mut prop_data = 1;
                let prop_size = if size_byte & 0x80 == 0x80 {
                    prop_data = 2;
                    let size = state.byte_value(property_address + 1) as usize & 0x3F;
                    if size == 0 {
                        64
                    } else {
                        size
                    }
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
                    size_byte = state.byte_value(property_address);
                }
            }
            _ => return 0,
        }
    }
    return 0;
}

fn default_property(state: &State, property: u8) -> u16 {
    let object_table = header::object_table(state);
    let property_address = object_table + ((property as usize - 1) * 2);
    state.word_value(property_address)
}

pub fn property(state: &State, object: usize, property: u8) -> u16 {
    let property_address = property_address(state, object, property);
    if property_address == 0 {
        default_property(state, property)
    } else {
        let size = property_size(state, property_address);
        let property_data_address = property_data_address(state, property_address);
        match size {
            1 => state.byte_value(property_data_address) as u16,
            2 => state.word_value(property_data_address),
            _ => panic!("GET_PROP for property with length > 2"),
        }
    }
}

pub fn property_data_addr(state: &State, object: usize, property: u8) -> usize {
    let property_address = property_address(state, object, property);
    if property_address == 0 {
        0
    } else {
        property_data_address(state, property_address)
    }
}

pub fn property_length(state: &State, property_data_address: usize) -> usize {
    if property_data_address == 0 {
        0
    } else {
        let size_byte = state.byte_value(property_data_address - 1);
        match state.version {
            1 | 2 | 3 => property_size(state, property_data_address - 1),
            4 | 5 | 6 | 7 | 8 => {
                if size_byte & 0x80 == 0x80 {
                    property_size(state, property_data_address - 2)
                } else {
                    property_size(state, property_data_address - 1)
                }
            }
            _ => 0,
        }
    }
}

pub fn next_property(state: &State, object: usize, property: u8) -> u8 {
    if property == 0 {
        let prop_table = property_table_address(state, object);
        let header_size = state.byte_value(prop_table) as usize;
        let p1 = state.byte_value(prop_table + 1 + (header_size * 2));
        if state.version < 4 {
            p1 & 0x1F
        } else {
            p1 & 0x3F
        }
    } else {
        let prop_data_addr = property_data_addr(state, object, property);
        if prop_data_addr == 0 {
            0
        } else {
            let prop_len = property_length(state, prop_data_addr);
            let next_prop = state.byte_value(prop_data_addr + prop_len);
            if state.version < 4 {
                next_prop & 0x1F
            } else {
                next_prop & 0x3F
            }
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
            property, object
        );
    }

    let property_size = property_size(state, property_address);
    trace!(
        "Object {} property {} size {}",
        object,
        property,
        property_size
    );
    let property_data = match state.version {
        1 | 2 | 3 => property_address + 1,
        4 | 5 | 6 | 7 | 8 => {
            if state.byte_value(property_address) & 0x80 == 0x80 {
                property_address + 2
            } else {
                property_address + 1
            }
        }
        _ => 0,
    };

    match property_size {
        1 => state.set_byte(property_data, (value & 0xFF) as u8),
        2 => state.set_word(property_data, value),
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
    let header_count = state.byte_value(property_table);
    let mut ztext = Vec::new();
    for i in 0..header_count as usize {
        ztext.push(state.word_value(property_table + 1 + (i * 2)));
    }

    ztext
}

pub fn parent(state: &State, object: usize) -> usize {
    if object == 0 {
        warn!("parent called on object 0");
        0
    } else {
        let object_address = object_address(state, object);

        match state.version {
            1 | 2 | 3 => state.byte_value(object_address + 4) as usize,
            4 | 5 | 6 | 7 | 8 => state.word_value(object_address + 6) as usize,
            _ => 0,
        }
    }
}

pub fn set_parent(state: &mut State, object: usize, parent: usize) {
    let object_address = object_address(state, object);
    match state.version {
        1 | 2 | 3 => state.set_byte(object_address as usize + 4, parent as u8),
        4 | 5 | 6 | 7 | 8 => state.set_word(object_address as usize + 6, parent as u16),
        _ => {}
    }
}

pub fn child(state: &State, object: usize) -> usize {
    if object == 0 {
        warn!("child called on object 0");
        0
    } else {
        let object_address = object_address(state, object);

        match state.version {
            1 | 2 | 3 => state.byte_value(object_address + 6) as usize,
            4 | 5 | 6 | 7 | 8 => state.word_value(object_address + 10) as usize,
            _ => 0,
        }
    }
}

pub fn set_child(state: &mut State, object: usize, child: usize) {
    let object_address = object_address(state, object);
    match state.version {
        1 | 2 | 3 => state.set_byte(object_address as usize + 6, child as u8),
        4 | 5 | 6 | 7 | 8 => state.set_word(object_address as usize + 10, child as u16),
        _ => {}
    }
}

pub fn sibling(state: &State, object: usize) -> usize {
    if object == 0 {
        warn!("sibling called on object 0");
        0
    } else {
        let object_address = object_address(state, object);

        match state.version {
            1 | 2 | 3 => state.byte_value(object_address + 5) as usize,
            4 | 5 | 6 | 7 | 8 => state.word_value(object_address + 8) as usize,
            _ => 0,
        }
    }
}

pub fn set_sibling(state: &mut State, object: usize, sibling: usize) {
    let object_address = object_address(state, object);
    match state.version {
        1 | 2 | 3 => state.set_byte(object_address as usize + 5, sibling as u8),
        4 | 5 | 6 | 7 | 8 => state.set_word(object_address as usize + 8, sibling as u16),
        _ => {}
    }
}
