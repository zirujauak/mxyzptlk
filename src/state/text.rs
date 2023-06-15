use crate::error::*;
use crate::state::header;
use crate::state::header::*;
use crate::state::State;

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
fn abbreviation(state: &State, abbrev_table: u8, index: u8) -> Result<Vec<u16>, RuntimeError> {
    let abbreviation_table =
        header::field_word(state.memory(), HeaderField::AbbreviationsTable)? as usize;
    let entry = (64 * (abbrev_table - 1)) + (index * 2);
    let word_addr = state.read_word(abbreviation_table + entry as usize)? as usize;
    as_text(state, word_addr * 2)
}

/// Read ZSCII from an address and decode it to a string
///
/// # Arguments
///
/// * `m` - Memory map
/// * `v` - Version (1-8)
/// * `a` - Address of the ZSCII-encoded string
pub fn as_text(state: &State, address: usize) -> Result<Vec<u16>, RuntimeError> {
    let mut d = Vec::new();
    // If the last word read has bit 15 set, then we're done reading
    while match d.last() {
        Some(x) => *x,
        _ => 0,
    } & 0x8000
        == 0
    {
        let w = state.memory().read_word(address + (d.len() * 2))?;
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
pub fn from_vec(state: &State, ztext: &Vec<u16>) -> Result<Vec<u16>, RuntimeError> {
    let mut alphabet_shift = 0;
    let mut s = Vec::new();
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
                let mut abbreviation = abbreviation(state, abbrev, b)?;
                s.append(&mut abbreviation);
                abbrev = 0;
            } else if zscii_read1 {
                zscii_b1 = b;
                zscii_read2 = true;
                zscii_read1 = false;
            } else if zscii_read2 {
                let z = ((zscii_b1 << 5) as u16 & 0x3E0) + b as u16;
                s.push(z);
                zscii_read2 = false;
            } else {
                match b {
                    0 => s.push(0x20),
                    1 | 2 | 3 => abbrev = b,
                    4 => alphabet_shift = 1,
                    5 => alphabet_shift = 2,
                    6 => {
                        if alphabet_shift == 2 {
                            zscii_read1 = true;
                        } else {
                            s.push(ALPHABET_V3[alphabet_shift][b as usize - 6] as u16);
                        }
                    }
                    _ => s.push(ALPHABET_V3[alphabet_shift][b as usize - 6] as u16),
                }
            }
            if b != 4 && b != 5 {
                alphabet_shift = 0;
            }
        }

        i = i + 1;
    }
    Ok(s)
}

pub fn separators(state: &State, dictionary_address: usize) -> Result<Vec<char>, RuntimeError> {
    let separator_count = state.read_byte(dictionary_address)?;
    let mut sep = Vec::new();
    for i in 1..=separator_count as usize {
        let c = state.read_byte(dictionary_address + i)? as char;
        sep.push(c);
    }

    Ok(sep)
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

fn as_word(z1: u8, z2: u8, z3: u8) -> u16 {
    (((z1 as u16) & 0x1F) << 10) | (((z2 as u16) & 0x1F) << 5) | (z3 as u16) & 0x1F
}

pub fn from_default_dictionary(state: &State, word: &Vec<char>) -> Result<usize, RuntimeError> {
    self::from_dictionary(
        state,
        header::field_word(state.memory(), HeaderField::Dictionary)? as usize,
        word,
    )
}

fn search_entry(
    state: &State,
    address: usize,
    entry_count: usize,
    entry_size: usize,
    word: &Vec<u16>,
) -> Result<usize, RuntimeError> {
    let mut min = 0;
    let mut max = entry_count - 1;
    let mut pivot = max / 2;

    // Binary search:
    // Set min to first entry, max to last entry
    // Set pivot to halfway point in dictionary
    // If entry is too high, set max to pivot, reset pivot to halfway between min and max, and repeat
    // If entry is too low, set min to pivot, reset pivot to halfway between min and max, and repeat
    // If min exceeds max, the entry was not found
    'outer: loop {
        let addr = address + (pivot * entry_size);
        for i in 0..word.len() {
            let w = state.read_word(addr + (i * 2))?;
            if w > word[i] {
                if pivot == min {
                    break 'outer;
                }
                max = pivot - 1;
                let new_pivot = min + ((max - min) / 2);
                if new_pivot == pivot {
                    pivot = new_pivot - 1;
                } else {
                    pivot = new_pivot;
                }
                continue 'outer;
            } else if w < word[i] {
                if pivot == max {
                    break 'outer;
                }
                min = pivot + 1;
                let new_pivot = min + ((max - min) / 2);
                if new_pivot == pivot {
                    pivot = new_pivot + 1;
                } else {
                    pivot = new_pivot
                }
                if pivot > max {
                    break 'outer;
                }
                continue 'outer;
            }
        }

        return Ok(addr);
    }

    Ok(0)
}

fn scan_entry(
    state: &State,
    address: usize,
    entry_count: usize,
    entry_size: usize,
    words: &Vec<u16>,
) -> Result<usize, RuntimeError> {
    // Scan the table from the start
    let word_count = words.len() / 3;
    'outer: for i in 0..entry_count {
        let entry_address = address + (i * entry_size as usize);
        for j in 0..word_count {
            let ew = state.read_word(entry_address)?;
            if ew < words[j] {
                return Ok(0);
            } else if ew != words[j] {
                break 'outer;
            }
        }

        return Ok(entry_address);
    }

    Ok(0)
}

pub fn from_dictionary(
    state: &State,
    dictionary_address: usize,
    word: &Vec<char>,
) -> Result<usize, RuntimeError> {
    let dictionary_address = header::field_word(state.memory(), HeaderField::Dictionary)? as usize;
    let separator_count = state.read_byte(dictionary_address)? as usize;
    let entry_size = state.read_byte(dictionary_address + 1)? as usize;
    let entry_count = state.read_word(dictionary_address + 2 + separator_count)? as usize;
    let word_count = if state.memory().read_byte(0)? < 4 {
        2
    } else {
        3
    };

    let mut zchars = Vec::new();
    for i in 0..word_count * 3 {
        let index = i * 3;
        if let Some(c) = word.get(index) {
            zchars.append(&mut find_char(c))
        } else {
            zchars.push(5);
        }
    }
    let mut words = Vec::new();
    for i in 0..word_count {
        let index = i * 3;
        words.push(as_word(zchars[i], zchars[i + 1], zchars[i + 2]));
    }

    if entry_count > 0 {
        search_entry(
            state,
            dictionary_address + separator_count + 4,
            entry_count,
            entry_size,
            &words,
        )
    } else {
        scan_entry(
            state,
            dictionary_address + separator_count + 4,
            i16::abs(entry_count as i16) as usize,
            entry_size,
            &words,
        );
        'outer: for i in 0..i16::abs(entry_count as i16) as usize {
            let entry_address =
                dictionary_address + separator_count as usize + 4 + (i * entry_size as usize);
            for j in 0..word_count {
                let ew = state.read_word(entry_address)?;
                if ew < words[j] {
                    return Ok(0);
                } else if ew != words[j] {
                    break 'outer;
                }
            }

            return Ok(entry_address);
        }

        return Ok(0);
    }
}