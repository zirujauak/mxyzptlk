use super::header;

pub fn word(high_byte: u8, low_byte: u8) -> u16 {
    ((high_byte as u16) << 8) & 0xFF00 | (low_byte as u16) & 0xFF
}
pub fn word_value(memory_map: &Vec<u8>, address: usize) -> u16 {
    word(
        byte_value(memory_map, address),
        byte_value(memory_map, address + 1),
    )
}

pub fn byte_value(memory_map: &Vec<u8>, address: usize) -> u8 {
    memory_map[address]
}

pub fn set_byte(memory_map: &mut Vec<u8>, address: usize, v: u8) {
    memory_map[address] = v;
    debug!("memory: set ${:05x} to #{:02x}", address, v)
}

pub fn set_word(memory_map: &mut Vec<u8>, address: usize, v: u16) {
    let hb = ((v >> 8) & 0xFF) as u8;
    let lb = (v & 0xFF) as u8;

    memory_map[address] = hb;
    memory_map[address + 1] = lb;

    debug!("memory: set ${:05x} to #{:04x}", address, v)
}

pub fn packed_address(memory_map: &Vec<u8>, version: u8, address: u16) -> usize {
    match version {
        1 | 2 | 3 => address as usize * 2,
        4 | 5 => address as usize * 4,
        6 | 7 => (address as usize * 4) + (header::routine_offset(memory_map) as usize * 8),
        8 => address as usize * 8,
        // TODO: error
        _ => 0,
    }
}
