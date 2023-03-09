use super::{header, state::State};

const ALPHABET_V3: [[char; 26]; 3] = [
    [
        'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r',
        's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
    ],
    [
        'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R',
        'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
    ],
    [
        ' ', '\r', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '.', ',', '!', '?', '_', '#',
        '\'', '"', '/', '\\', '-', ':', '(', ')',
    ],
];

pub fn valid_input(c: char) -> bool {
    if c as u8 >= 32 && c as u8 <= 126 {
        true
    } else if c as u8 >= 145 && c as u8 <= 254 {
        true
    } else {
        false
    }
}

/// Decode an abbreviation to a string
///
/// # Arguments
///
/// * `m` - Memory map
/// * `v` - Version (1-8)
/// * `t` - Abbreviation table (0 - 2)
/// * `i` - Abbreviation table index (0 - 31)
fn abbreviation(state: &State, abbrev_table: u8, index: u8) -> String {
    let abbreviation_table = state.word_value(24) as usize;
    let entry = (64 * (abbrev_table - 1)) + (index * 2);
    let word_addr = state.word_value(abbreviation_table + entry as usize) as usize;
    as_text(state, word_addr * 2)
}

/// Read ZSCII from an address and decode it to a string
///
/// # Arguments
///
/// * `m` - Memory map
/// * `v` - Version (1-8)
/// * `a` - Address of the ZSCII-encoded string
pub fn as_text(state: &State, address: usize) -> String {
    let mut d = Vec::new();
    // If the last word read has bit 15 set, then we're done reading
    while match d.last() {
        Some(x) => *x,
        _ => 0,
    } & 0x8000
        == 0
    {
        let w = state.word_value(address + (d.len() * 2));
        d.push(w);
    }

    from_vec(state, &d)
}

/// Decode a vector of ZSCII words to a string
///
/// # Arguments:
///
/// * `m` - Memory map
/// * `v` - Version (1-8)
/// * `z` - Vector of ZSCII-encoded words
pub fn from_vec(state: &State, ztext: &Vec<u16>) -> String {
    let mut alphabet_shift = 0;
    let mut s = String::new();
    let mut i = 0;

    let mut abbrev = 0;
    let mut zscii_read1 = false;
    let mut zscii_read2 = false;
    let mut zscii_b1 = 0;

    for w in ztext {
        let b1 = (w >> 10 & 0x1F) as u8;
        let b2 = (w >> 5 & 0x1F) as u8;
        let b3 = (w & 0x1F) as u8;

        for b in [b1, b2, b3] {
            if abbrev > 0 {
                s.push_str(&abbreviation(state, abbrev, b));
                abbrev = 0;
            } else if zscii_read1 {
                zscii_b1 = b;
                zscii_read2 = true;
                zscii_read1 = false;
            } else if zscii_read2 {
                let z = ((zscii_b1 << 5) as u16 & 0x3E0) + b as u16;
                s.push(char::from_u32(z as u32).unwrap());
                zscii_read2 = false;
            } else {
                match b {
                    0 => s.push(' '),
                    1 | 2 | 3 => abbrev = b,
                    4 => alphabet_shift = 1,
                    5 => alphabet_shift = 2,
                    6 => {
                        if alphabet_shift == 2 {
                            zscii_read1 = true;
                        } else {
                            s.push(ALPHABET_V3[alphabet_shift][b as usize - 6]);
                        }
                    }
                    _ => s.push(ALPHABET_V3[alphabet_shift][b as usize - 6]),
                }
            }
            if b != 4 && b != 5 {
                alphabet_shift = 0;
            }
        }

        i = i + 1;
    }
    s
}

pub fn separators(state: &State, dictionary_address: usize) -> Vec<char> {
    let separator_count = state.byte_value(dictionary_address);
    let mut sep = Vec::new();
    for i in 1..=separator_count as usize {
        sep.push(state.byte_value(dictionary_address + i) as char);
    }

    sep
}

fn find_char(c: &char) -> Vec<u8> {
    for i in 0..26 {
        if ALPHABET_V3[0][i] == *c {
            return vec![i as u8 + 6];
        }
    }

    for i in 0..26 {
        if ALPHABET_V3[2][i] == *c {
            return vec![5, i as u8 + 6];
        }
    }

    panic!("Unknown input {}", c)
}

fn word_value(z1: u8, z2: u8, z3: u8) -> u16 {
    (((z1 as u16) & 0x1F) << 10) | (((z2 as u16) & 0x1F) << 5) | (z3 as u16) & 0x1F
}

pub fn from_default_dictionary(state: &State, word: &Vec<char>) -> usize {
    self::from_dictionary(state, header::dictionary_table(state) as usize, word)
}

pub fn from_dictionary(state: &State, dictionary_address: usize, word: &Vec<char>) -> usize {
    trace!("Searching dictioary @ {:#05x} for {:?}", dictionary_address, word);
    let dictionary_address = header::dictionary_table(state) as usize;
    let separator_count = state.byte_value(dictionary_address) as usize;
    let entry_size = state.byte_value(dictionary_address + 1 + separator_count) as usize;
    // TODO: negative counts indicate unsorted table ... remember this when implementing searching
    let entry_count = i16::abs((state.word_value(dictionary_address + 1 + separator_count + 1)) as i16) as usize;

    if state.version < 4 {
        // Encode the input
        let mut w: Vec<u8> = Vec::new();
        for i in 0..6 {
            match word.get(i) {
                Some(c) => w.append(&mut find_char(c)),
                None => w.push(5),
            }
        }

        let w1 = word_value(w[0], w[1], w[2]);
        let w2 = word_value(w[3], w[4], w[5]) | 0x8000;

        for i in 0..entry_count {
            let entry_address = dictionary_address + separator_count + 4 + (i * entry_size);
            let e1 = state.word_value(entry_address);
            if e1 == w1 {
                let e2 = state.word_value(entry_address + 2);
                if e2 == w2 {
                    trace!("Entry {}", i + 1);
                    return entry_address;
                } else {
                    if w2 < e2 {
                        return 0;
                    }
                }
            } else {
                if w1 < e1 {
                    return 0;
                }
            }
        }
    } else {
        // Encode the input
        let mut w: Vec<u8> = Vec::new();
        for i in 0..9 {
            match word.get(i) {
                Some(c) => w.append(&mut find_char(c)),
                None => w.push(5),
            }
        }

        let w1 = word_value(w[0], w[1], w[2]);
        let w2 = word_value(w[3], w[4], w[5]);
        let w3 = word_value(w[6], w[7], w[8]) | 0x8000;

        for i in 0..entry_count {
            let entry_address = dictionary_address + separator_count + 4 + (i * entry_size);
            let e1 = state.word_value(entry_address);
            if e1 == w1 {
                let e2 = state.word_value(entry_address + 2);
                if e2 == w2 {
                    let e3 = state.word_value(entry_address + 4);
                    if e3 == w3 {
                        trace!("Entry {}", i + 1);
                        return entry_address;
                    } else {
                        if w3 < e3 {
                            return 0;
                        }
                    }
                } else {
                    if w2 < e2 {
                        return 0;
                    }
                }
            } else {
                if w1 < e1 {
                    return 0;
                }
            }
        }
    }

    0
}
