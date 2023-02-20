pub fn word(high_byte: u8, low_byte: u8) -> u16 {
    ((high_byte as u16) << 8) & 0xFF00 | (low_byte as u16) & 0xFF
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

