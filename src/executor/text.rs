use super::util;

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
        ' ', '\n', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '.', ',', '!', '?', '_', '#',
        '\'', '"', '/', '\\', '-', ':', '(', ')',
    ],
];

/// Decode an abbreviation to a string
///
/// # Arguments
///
/// * `m` - Memory map
/// * `v` - Version (1-8)
/// * `t` - Abbreviation table (0 - 2)
/// * `i` - Abbreviation table index (0 - 31)
fn abbreviation(memory_map: &Vec<u8>, version: u8, abbrev_table: u8, index: u8) -> String {
    let abbreviation_table = util::word_value(memory_map, 24) as usize;
    let entry = (64 * (abbrev_table - 1)) + (index * 2);
    let word_addr = util::word_value(memory_map, abbreviation_table + entry as usize) as usize;
    as_text(memory_map, version, word_addr * 2)
}

/// Read ZSCII from an address and decode it to a string
///
/// # Arguments
///
/// * `m` - Memory map
/// * `v` - Version (1-8)
/// * `a` - Address of the ZSCII-encoded string
pub fn as_text(memory_map: &Vec<u8>, version: u8, address: usize) -> String {
    let mut d = Vec::new();
    // If the last word read has bit 15 set, then we're done reading
    while match d.last() {
        Some(x) => *x,
        _ => 0,
    } & 0x8000
        == 0
    {
        let w = util::word_value(memory_map, address + (d.len() * 2));
        d.push(w);
    }

    from_vec(memory_map, version, &d)
}

/// Decode a vector of ZSCII words to a string
///
/// # Arguments:
///
/// * `m` - Memory map
/// * `v` - Version (1-8)
/// * `z` - Vector of ZSCII-encoded words
pub fn from_vec(memory_map: &Vec<u8>, version: u8, ztext: &Vec<u16>) -> String {
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
                s.push_str(&abbreviation(memory_map, version, abbrev, b));
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
