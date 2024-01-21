//! [ZSCII](https://inform-fiction.org/zmachine/standards/z1point1/sect03.html) text encoding
use std::cmp::Ordering;

use crate::{
    error::*,
    fatal_error,
    zmachine::{header::HeaderField, ZMachine},
};

/// ZCode version 3+ [alphabets](https://inform-fiction.org/zmachine/standards/z1point1/sect03.html#two)
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

/// Decode an [abbreviation](https://inform-fiction.org/zmachine/standards/z1point1/sect03.html#three) to a string
///
/// # Arguments
/// * `zmachine` - Reference to the Z-machine
/// * `abbrev_table` - Abbreviation table index
/// * `index` - Abbreviation index within the table
///
/// # Returns
/// [Result] containing the abbreviation text or a [RuntimeError]
fn abbreviation(
    zmachine: &ZMachine,
    abbrev_table: u8,
    index: u8,
) -> Result<Vec<u16>, RuntimeError> {
    let abbreviation_table = zmachine.header_word(HeaderField::AbbreviationsTable)? as usize;
    let entry = (64 * (abbrev_table - 1)) + (index * 2);
    let word_addr = zmachine.read_word(abbreviation_table + entry as usize)? as usize;
    as_text(zmachine, word_addr * 2, true)
}

/// Read [ZSCII](https://inform-fiction.org/zmachine/standards/z1point1/sect03.html#one) from an address and decode it to a string
///
/// Note that it is illegal for an abbreviation to contain an abbreviation and a [RuntimeError] will
/// be returned.
///
/// # Arguments
/// * `zmachine` - Reference to the Z-machine
/// * `address` - Address of the text
/// * `is_abbreviation` - `true` when decoding an abbreviation, `false` if not.
///
/// # Returns
/// [Result] containing the decoded text or a [RuntimeError]
pub fn as_text(
    zmachine: &ZMachine,
    address: usize,
    is_abbreviation: bool,
) -> Result<Vec<u16>, RuntimeError> {
    from_vec(
        zmachine,
        &zmachine.string_literal(address)?,
        is_abbreviation,
    )
}

/// Decode a vector of ZSCII words to a string
///
/// Note that it is illegal for an abbreviation to contain an abbreviation and a [RuntimeError] will
/// be returned.
///
/// # Arguments:
/// * `zmachine` - Reference to the Z-machine
/// * `ztext` - Vector of encoded ztext
/// * `is_abbreviation` - `true` when decoding an abbreviation, `false` if not.
///
/// # Returns
/// [Result] containing the decoded text or a [RuntimeError]
pub fn from_vec(
    zmachine: &ZMachine,
    ztext: &Vec<u16>,
    is_abbreviation: bool,
) -> Result<Vec<u16>, RuntimeError> {
    let mut alphabet_shift: usize = 0;
    let mut s = Vec::new();

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
                let mut abbreviation = abbreviation(zmachine, abbrev, b)?;
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
                    1..=3 => {
                        if !is_abbreviation {
                            abbrev = b
                        } else {
                            return fatal_error!(
                                ErrorCode::InvalidAbbreviation,
                                "Abbreviations can't nest",
                            );
                        }
                    }
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
    }
    Ok(s)
}

/// Get the set of word separators from a dictionary
///
/// # Arguments
/// * `zmachine` - Reference to the Z-Machine
/// * `dictionary_address` - Address of the dictionary
///
/// # Returns
/// [Result] containing a vector of word separator characters or a [RuntimeError]
fn separators(zmachine: &ZMachine, dictionary_address: usize) -> Result<Vec<char>, RuntimeError> {
    let separator_count = zmachine.read_byte(dictionary_address)?;
    let mut sep = Vec::new();
    for i in 1..=separator_count as usize {
        let c = zmachine.read_byte(dictionary_address + i)? as char;
        sep.push(c);
    }

    Ok(sep)
}

/// Find the ztext value of a character.
///
/// # Arguments
/// * `zchar` - Character to look up
///
/// # Returns
/// Vector cotaining the ztext value of the character with any required alphabet shift.
/// If the character isn't part of the standard alphabet, a two-character 10-bit ZSCII
/// sequence is returned.
fn find_char(zchar: u16) -> Vec<u16> {
    let c = (zchar as u8) as char;
    if c == ' ' {
        return vec![0];
    }

    for i in 0..26 {
        if ALPHABET_V3[0][i] == c {
            return vec![i as u16 + 6];
        }
    }

    for i in 0..26 {
        if ALPHABET_V3[1][i] == c {
            return vec![4, i as u16 + 6];
        }
    }

    for i in 0..26 {
        if ALPHABET_V3[2][i] == c {
            return vec![5, i as u16 + 6];
        }
    }

    let z1 = (c as u8 >> 5) & 0x1f;
    let z2 = c as u8 & 0x1f;
    vec![5, 6, z1 as u16, z2 as u16]
}

/// Encode 3 5-bit ztext characters into a word
///
/// # Arguments
/// * `z1` - first character,
/// * `z2` - second character,
/// * `z3` - third character
///
/// # Return
/// Word encoding of the sequence: 01111122 22233333
fn as_word(z1: u16, z2: u16, z3: u16) -> u16 {
    ((z1 & 0x1F) << 10) | ((z2 & 0x1F) << 5) | z3 & 0x1F
}

/// Perform a binary search for a word in a sorted [dictionary](https://inform-fiction.org/zmachine/standards/z1point1/sect13.html#two)
///
/// # Arguments
/// * `zmachine` - Reference to the z-machine
/// * `address` - Address of the first entry in the dictionary
/// * `entry_count` - Number of entries in the dictionary
/// * `entry_size` - Dictionary entry size
/// * `word` - Encoded ztext for the word to find
///
/// # Returns
/// [Result] containing the address of the matching dictionary entry or 0 if not found or a [RuntimeError]
fn search_entry(
    zmachine: &ZMachine,
    address: usize,
    entry_count: usize,
    entry_size: usize,
    word: &[u16],
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
        for (i, wrd) in word.iter().enumerate() {
            let w = zmachine.read_word(addr + (i * 2))?;
            match w.cmp(wrd) {
                Ordering::Greater => {
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
                }
                Ordering::Less => {
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
                Ordering::Equal => {}
            }
        }

        return Ok(addr);
    }

    Ok(0)
}

/// Perform a scan for a word in an unsorted dictionary
///
/// # Arguments
/// * `zmachine` - Reference to the z-machine
/// * `address` - Address of the first entry in the dictionary
/// * `entry_count` - Number of entries in the dictionary
/// * `entry_size` - Dictionary entry size
/// * `word` - Encoded ztext for the word to find
///
/// [Result] containing the address of the matching dictionary entry or 0 if not found or a [RuntimeError]
fn scan_entry(
    zmachine: &ZMachine,
    address: usize,
    entry_count: usize,
    entry_size: usize,
    words: &[u16],
) -> Result<usize, RuntimeError> {
    // Scan the unsorted dictionary
    'outer: for i in 0..entry_count {
        let entry_address = address + (i * entry_size);
        for (j, w) in words.iter().enumerate() {
            let ew = zmachine.read_word(entry_address + (j * 2))?;
            if ew != *w {
                continue 'outer;
            }
        }

        return Ok(entry_address);
    }

    Ok(0)
}

/// [Encode](https://inform-fiction.org/zmachine/standards/z1point1/sect03.html#seven) a word
///
/// # Arguments
/// * `word` - Word to encode as a vector of characters
/// * `words` - the number of encoded words in the result-  2 for v3 (6 characters) and 3 for v4+ (9 characters)
///
/// # Returns
/// Vector of encoded ztext words
pub fn encode_text(word: &mut Vec<u16>, words: usize) -> Vec<u16> {
    let mut zchars = Vec::new();

    // Read at most words * 3 characters from word
    word.truncate(words * 3);
    for c in word {
        zchars.append(&mut find_char(*c));
    }

    // Truncate or pad characters
    zchars.resize(words * 3, 5);

    debug!(target: "app::state", "LEXICAL ANALYSIS: zchars: {:?}", zchars);

    // Encode zchar triplets into encoded ZSCII words
    let mut zwords = Vec::new();
    for i in 0..words {
        let index = i * 3;
        let mut w = as_word(zchars[index], zchars[index + 1], zchars[index + 2]);
        if i == words - 1 {
            w |= 0x8000;
        }
        zwords.push(w);
    }

    zwords
}

/// Find the address of the dictionary entry for a word, if any.
///
/// # Argument
/// * `zmachine` - Reference to the Z-Machine
/// * `dictionary_address` - Address of the dictionary
/// * `word` - Word to find as a vector of characters
///
/// # Returns
/// [Result] containing the address of the matching dictionary entry or 0 if not found or a [RuntimeError]
fn from_dictionary(
    zmachine: &ZMachine,
    dictionary_address: usize,
    word: &[char],
) -> Result<usize, RuntimeError> {
    let separator_count = zmachine.read_byte(dictionary_address)? as usize;
    let entry_size = zmachine.read_byte(dictionary_address + separator_count + 1)? as usize;
    let entry_count = zmachine.read_word(dictionary_address + separator_count + 2)? as i16;
    let word_count = if zmachine.version() < 4 { 2 } else { 3 };
    debug!(target: "app::state", "LEXICAL ANALYSIS: dictionary @ {:04x}, {} separators, {} entries of size {}", dictionary_address, separator_count, entry_count, entry_size);

    let mut zchars = word.iter().map(|c| *c as u16).collect::<Vec<u16>>();
    let words = encode_text(&mut zchars, word_count);
    debug!(target: "app::state", "LEXICAL ANALYSIS: encoded text: {:?}", words);

    if entry_count > 0 {
        search_entry(
            zmachine,
            dictionary_address + separator_count + 4,
            entry_count as usize,
            entry_size,
            &words,
        )
    } else {
        scan_entry(
            zmachine,
            dictionary_address + separator_count + 4,
            i16::abs(entry_count) as usize,
            entry_size,
            &words,
        )
    }
}

/// Find a word in a dictionary and store the result into the parse buffer
///
/// # Arguments
/// * `zmachine` - Reference to the Z-Machine
/// * `dictionary` - byte address of the dictionary
/// * `parse_buffer` - parse buffer address
/// * `flag` - if `true`, the parse buffer is only updated if the word is in the dictionary
/// * `parse_index` - index to the parse buffer
/// * `(word count, word_start)` - the number of words parsed and the starting index of the word from the text buffer
/// * `word` - Word to find
///
/// # Returns
/// [Result] with a tuple (new parse_index, new parsed word_count) or a [RuntimeError]
fn find_word(
    zmachine: &mut ZMachine,
    dictionary: usize,
    parse_buffer: usize,
    flag: bool,
    parse_index: usize,
    (word_count, word_start): (usize, usize),
    word: &Vec<char>,
) -> Result<(usize, usize), RuntimeError> {
    let entry = from_dictionary(zmachine, dictionary, word)?;
    let offset = if zmachine.version() < 5 { 1 } else { 2 };

    debug!(target: "app::state", "LEXICAL ANALYSIS: {:?} => {:04x}", word, entry);
    let parse_address = parse_buffer + 2 + (4 * parse_index);
    if !flag {
        store_parsed_entry(
            zmachine,
            word,
            word_start + offset,
            parse_address,
            entry as u16,
        )?;
        debug!(target: "app::state", "LEXICAL ANALYSIS: store to parse buffer {:04x}", parse_address);
        Ok((parse_index + 1, word_count + 1))
    } else if entry > 0 {
        let e = zmachine.read_word(parse_address)?;
        if e == 0 {
            store_parsed_entry(
                zmachine,
                word,
                word_start + offset,
                parse_address,
                entry as u16,
            )?;
            debug!(target: "app::state", "LEXICAL ANALYSIS: store to parse buffer {:04x}", parse_address);
            Ok((parse_index + 1, word_count + 1))
        } else {
            Ok((parse_index + 1, word_count))
        }
    } else {
        Ok((parse_index + 1, word_count))
    }
}

/// Store a word entry to the parse buffer
///
/// # Arguments
/// * `zmachine` - Reference to the Z-Machine
/// * `word` - Word text
/// * `word_start` - Index of the word position in the text buffer
/// * `entry_address` - Parse buffer entry address
/// * `entry` -  Dictionary entry address
///
/// # Returns
/// Empty [Result] or a [RuntimeError]
fn store_parsed_entry(
    zmachine: &mut ZMachine,
    word: &Vec<char>,
    word_start: usize,
    entry_address: usize,
    entry: u16,
) -> Result<(), RuntimeError> {
    debug!(target: "app::state", "LEXICAL_ANALYSIS: dictionary for {:?} => stored to ${:04x}: {:#04x}/{}/{}", word, entry_address, entry, word.len(), word_start);
    zmachine.write_word(entry_address, entry)?;
    zmachine.write_byte(entry_address + 2, word.len() as u8)?;
    zmachine.write_byte(entry_address + 3, word_start as u8)?;
    Ok(())
}

/// Parse a text buffer into a parse buffer.
///
/// # Arguments
/// * `zmachine` - Reference to the Z-Machine
/// * `text_buffer` - Input text buffer address
/// * `parse_buffer` - Parse buffer address
/// * `dictionary` - Dictionary address
/// * `flag` - If `true`, the parse buffer is not updated for words that aren't found in the dictionary
///
/// # Returns
/// Empty [Result] or a [RuntimeError]
pub fn parse_text(
    zmachine: &mut ZMachine,
    text_buffer: usize,
    parse_buffer: usize,
    dictionary: usize,
    flag: bool,
) -> Result<(), RuntimeError> {
    debug!(target: "app::state", "LEXICAL ANALYSIS: text @ {:04x}, parse @ {:04x}, dictionary @ {:04x}, skip {}", text_buffer, parse_buffer, dictionary, flag);
    let separators = separators(zmachine, dictionary)?;
    let mut word = Vec::new();
    let mut word_start: usize = 0;
    let mut word_count: usize = 0;
    let mut words: usize = 0;
    let mut data = Vec::new();

    if zmachine.version() < 5 {
        // Buffer is 0 terminated
        let mut i = 1;
        loop {
            let b = zmachine.read_byte(text_buffer + i)?;
            if b == 0 {
                break;
            } else {
                data.push(b);
                i += 1;
            }
        }
    } else {
        // Buffer size is stored in the second byte
        let n = zmachine.read_byte(text_buffer + 1)? as usize;
        for i in 0..n {
            data.push(zmachine.read_byte(text_buffer + 2 + i)?);
        }
    }

    let max_words = zmachine.read_byte(parse_buffer)? as usize;

    for (i, b) in data.iter().enumerate() {
        let c = (*b as char).to_ascii_lowercase();
        if word_count > max_words {
            break;
        }

        if separators.contains(&c) {
            // Store the word
            if !word.is_empty() {
                (word_count, words) = find_word(
                    zmachine,
                    dictionary,
                    parse_buffer,
                    flag,
                    word_count,
                    (words, word_start),
                    &word,
                )?;
            }

            // Store the separator
            if word_count < max_words {
                let sep = vec![c];
                (word_count, words) = find_word(
                    zmachine,
                    dictionary,
                    parse_buffer,
                    flag,
                    word_count,
                    (words, word_start + word.len()),
                    &sep,
                )?;
            }
            word.clear();
            word_start = i + 1;
        } else if c == ' ' {
            // Store the word but not the space
            if !word.is_empty() {
                (word_count, words) = find_word(
                    zmachine,
                    dictionary,
                    parse_buffer,
                    flag,
                    word_count,
                    (words, word_start),
                    &word,
                )?;
            }
            word.clear();
            word_start = i + 1;
        } else {
            word.push(c)
        }
    }

    // End of input, parse anything collected
    if !word.is_empty() && word_count < max_words {
        (_, words) = find_word(
            zmachine,
            dictionary,
            parse_buffer,
            flag,
            word_count,
            (words, word_start),
            &word,
        )?;
    }

    // If flag is true, then a previous analysis pass has already set the
    // correct parse buffer size
    if !flag {
        zmachine.write_byte(parse_buffer + 1, words as u8)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        assert_ok, assert_ok_eq,
        test_util::{mock_sorted_dictionary, mock_unsorted_dictionary, mock_zmachine, test_map},
    };

    use super::*;

    #[test]
    fn test_abbreviation() {
        let mut map = test_map(3);
        // Abbreviations table at 0x200
        map[0x18] = 0x2;
        // Abbreviation 1.0 = 'The ' @ 0x400
        //   4     19    D        A     space filler
        // 0 00100 11001 01101  1 01010 00000 00101
        // 132D A805
        map[0x200] = 0x02;
        map[0x201] = 0x00;
        map[0x400] = 0x13;
        map[0x401] = 0x2D;
        map[0x402] = 0xA8;
        map[0x403] = 0x05;
        // Abbreviation 1.31 = 'This ' @ 0x404
        //   4     19    D        E     18    space
        // 0 00100 11001 01101  1 01110 11000 00000
        // 132D BB00
        map[0x23E] = 0x02;
        map[0x23F] = 0x02;
        map[0x404] = 0x13;
        map[0x405] = 0x2D;
        map[0x406] = 0xBB;
        map[0x407] = 0x00;
        // Abbreviation 2.0 = 'else.' @ 0x408
        //   A     11    18       A     5     12
        // 0 01010 10001 11000  1 01010 00101 10010
        // 2A38 A8B2
        map[0x240] = 0x02;
        map[0x241] = 0x04;
        map[0x408] = 0x2A;
        map[0x409] = 0x38;
        map[0x40A] = 0xA8;
        map[0x40B] = 0xB2;
        // Abbreviation 2.31 = ' and ' @ 0x40C
        //   space 6     13       9     space filler
        // 0 00000 00110 10011  1 01001 00000 00101
        // 00D3 A405
        map[0x27E] = 0x02;
        map[0x27F] = 0x06;
        map[0x40C] = 0x00;
        map[0x40D] = 0xD3;
        map[0x40E] = 0xA4;
        map[0x40F] = 0x05;
        // Abbreviation 3.1 = 'mxyzpltk' @ 0x410
        //   12    1D    1E       1F    15    11       19    10    filler
        // 0 10010 11101 11110  0 11111 10101 10001  1 11001 10000 00101
        // 4BBE 7EB1 E605
        map[0x280] = 0x02;
        map[0x281] = 0x08;
        map[0x410] = 0x4B;
        map[0x411] = 0xBE;
        map[0x412] = 0x7E;
        map[0x413] = 0xB1;
        map[0x414] = 0xE6;
        map[0x415] = 0x05;
        // Abbreviation 3.32 = 'abbreviated' @ 0x416
        // 0 00110 00111 00111  0 10111 01010 11011  0 01110 00110 11001  1 01010 01001 00101
        // 18E7 5D5B 38D9 A925
        map[0x2BE] = 0x02;
        map[0x2BF] = 0x0B;
        map[0x416] = 0x18;
        map[0x417] = 0xE7;
        map[0x418] = 0x5D;
        map[0x419] = 0x5B;
        map[0x41A] = 0x38;
        map[0x41B] = 0xD9;
        map[0x41C] = 0xA9;
        map[0x41D] = 0x25;

        let zmachine = mock_zmachine(map);
        let abbrev = assert_ok!(abbreviation(&zmachine, 1, 0));
        assert_eq!(abbrev, [b'T' as u16, b'h' as u16, b'e' as u16, b' ' as u16]);
        let abbrev = assert_ok!(abbreviation(&zmachine, 1, 31));
        assert_eq!(
            abbrev,
            [
                b'T' as u16,
                b'h' as u16,
                b'i' as u16,
                b's' as u16,
                b' ' as u16
            ]
        );
        let abbrev = assert_ok!(abbreviation(&zmachine, 2, 0));
        assert_eq!(
            abbrev,
            [
                b'e' as u16,
                b'l' as u16,
                b's' as u16,
                b'e' as u16,
                b'.' as u16
            ]
        );
        let abbrev = assert_ok!(abbreviation(&zmachine, 2, 31));
        assert_eq!(
            abbrev,
            [
                b' ' as u16,
                b'a' as u16,
                b'n' as u16,
                b'd' as u16,
                b' ' as u16
            ]
        );
        let abbrev = assert_ok!(abbreviation(&zmachine, 3, 0));
        assert_eq!(
            abbrev,
            [
                b'm' as u16,
                b'x' as u16,
                b'y' as u16,
                b'z' as u16,
                b'p' as u16,
                b'l' as u16,
                b't' as u16,
                b'k' as u16
            ]
        );
        let abbrev = assert_ok!(abbreviation(&zmachine, 3, 31));
        assert_eq!(
            abbrev,
            [
                b'a' as u16,
                b'b' as u16,
                b'b' as u16,
                b'r' as u16,
                b'e' as u16,
                b'v' as u16,
                b'i' as u16,
                b'a' as u16,
                b't' as u16,
                b'e' as u16,
                b'd' as u16
            ]
        );
    }

    #[test]
    fn test_abbreviation_nested() {
        let mut map = test_map(3);
        // Abbreviations table at 0x200
        map[0x18] = 0x2;
        // Abbreviation 1.0 = 'A1,0' @ 0x400
        //   1     0     5
        // 1 00001 0000 00101
        // 8405
        map[0x200] = 0x02;
        map[0x201] = 0x00;
        map[0x400] = 0x84;
        map[0x401] = 0x05;
        let zmachine = mock_zmachine(map);
        assert!(abbreviation(&zmachine, 1, 0).is_err());
    }

    #[test]
    fn test_as_text() {
        let mut map = test_map(3);
        map[0x410] = 0x4B;
        map[0x411] = 0xBE;
        map[0x412] = 0x7E;
        map[0x413] = 0xB1;
        map[0x414] = 0xE6;
        map[0x415] = 0x05;
        let zmachine = mock_zmachine(map);
        assert_ok_eq!(
            as_text(&zmachine, 0x410, false),
            [
                b'm' as u16,
                b'x' as u16,
                b'y' as u16,
                b'z' as u16,
                b'p' as u16,
                b'l' as u16,
                b't' as u16,
                b'k' as u16
            ]
        );
    }

    #[test]
    fn test_from_vec() {
        let mut map = test_map(3);
        // Includes shift-up and shift-down
        // Et tu, Brutus?
        //   4     A     19       0     19    1A
        // 0 00100 01010 11001  0 00000 11001 11010
        //   5     13    0        4     7     17
        // 0 00101 10011 00000  0 00100 00111 10111
        //   1A    19    1A       18    5     15
        // 0 11010 11001 11010  1 11000 00101 10101
        // 1159 033A 1660 10F7 6B3A E0B5
        map[0x410] = 0x11;
        map[0x411] = 0x59;
        map[0x412] = 0x03;
        map[0x413] = 0x3A;
        map[0x414] = 0x16;
        map[0x415] = 0x60;
        map[0x416] = 0x10;
        map[0x417] = 0xF7;
        map[0x418] = 0x6B;
        map[0x419] = 0x3A;
        map[0x41A] = 0xE0;
        map[0x41B] = 0xB5;
        let zmachine = mock_zmachine(map);
        assert_ok_eq!(
            as_text(&zmachine, 0x410, false),
            [
                b'E' as u16,
                b't' as u16,
                b' ' as u16,
                b't' as u16,
                b'u' as u16,
                b',' as u16,
                b' ' as u16,
                b'B' as u16,
                b'r' as u16,
                b'u' as u16,
                b't' as u16,
                b'u' as u16,
                b's' as u16,
                b'?' as u16
            ]
        );
    }

    #[test]
    fn test_from_vec_zscii() {
        let mut map = test_map(3);
        // 10-bit ZSCII sequence
        // $100 @ 6% APY
        //   5     6     1        4     5     9
        // 0 00101 00110 00001  0 00100 00101 01001
        // 14C1 10A9
        //   5     8     5        8     0     5
        // 0 00101 01000 00101  0 01000 00000 00101
        // 1505 2005
        //   6     2     0        0     5     E
        // 0 00110 00010 00000  0 00000 00101 01110
        // 1840 00AE
        //   5     6     1        5     0     4
        // 0 00101 00110 00001  0 00101 00000 00100
        // 14C1 1404
        //   6     4     15       4     1E    5
        // 0 00110 00100 10101  1 00100 11110 00101
        // 1895 93C5
        map[0x410] = 0x14;
        map[0x411] = 0xC1;
        map[0x412] = 0x10;
        map[0x413] = 0xA9;
        map[0x414] = 0x15;
        map[0x415] = 0x05;
        map[0x416] = 0x20;
        map[0x417] = 0x05;
        map[0x418] = 0x18;
        map[0x419] = 0x40;
        map[0x41A] = 0x00;
        map[0x41B] = 0xAE;
        map[0x41C] = 0x14;
        map[0x41D] = 0xC1;
        map[0x41E] = 0x14;
        map[0x41F] = 0x04;
        map[0x420] = 0x18;
        map[0x421] = 0x95;
        map[0x422] = 0x93;
        map[0x423] = 0xC5;
        let zmachine = mock_zmachine(map);
        assert_ok_eq!(
            as_text(&zmachine, 0x410, false),
            [
                b'$' as u16,
                b'1' as u16,
                b'0' as u16,
                b'0' as u16,
                b' ' as u16,
                b'@' as u16,
                b' ' as u16,
                b'6' as u16,
                b'%' as u16,
                b' ' as u16,
                b'A' as u16,
                b'P' as u16,
                b'Y' as u16,
            ]
        );
    }

    #[test]
    fn test_from_vec_abbreviation() {
        let mut map = test_map(3);
        // Abbreviations table at 0x200
        map[0x18] = 0x2;
        // Abbreviation 3.1 = 'mxyzpltk' @ 0x410
        map[0x2BE] = 0x02;
        map[0x2BF] = 0x08;
        map[0x410] = 0x4B;
        map[0x411] = 0xBE;
        map[0x412] = 0x7E;
        map[0x413] = 0xB1;
        map[0x414] = 0xE6;
        map[0x415] = 0x05;

        // Hi, mxyzpltk!
        //   4     D     E        5     13    0
        // 0 00100 01101 01110  0 00101 10011 00000
        // 11AE 1660
        //   3     1F    5        14    5     5
        // 0 00011 11111 00101  1 10100 00101 00101
        // 0FE5 D0A5
        map[0x300] = 0x11;
        map[0x301] = 0xAE;
        map[0x302] = 0x16;
        map[0x303] = 0x60;
        map[0x304] = 0x0F;
        map[0x305] = 0xE5;
        map[0x306] = 0xD0;
        map[0x307] = 0xA5;

        let zmachine = mock_zmachine(map);
        assert_ok_eq!(
            as_text(&zmachine, 0x300, false),
            [
                b'H' as u16,
                b'i' as u16,
                b',' as u16,
                b' ' as u16,
                b'm' as u16,
                b'x' as u16,
                b'y' as u16,
                b'z' as u16,
                b'p' as u16,
                b'l' as u16,
                b't' as u16,
                b'k' as u16,
                b'!' as u16,
            ]
        );
    }

    #[test]
    fn test_separators() {
        let mut map = test_map(3);

        // 4 separators: ',', '.', '!', '?'
        map[0x300] = 4;
        map[0x301] = b',';
        map[0x302] = b'.';
        map[0x303] = b'!';
        map[0x304] = b'?';
        let zmachine = mock_zmachine(map);
        assert_ok_eq!(separators(&zmachine, 0x300), [',', '.', '!', '?']);
    }

    #[test]
    fn test_find_char() {
        // Space
        assert_eq!(find_char(b' ' as u16), [0x00]);
        // A0
        assert_eq!(find_char(b'a' as u16), [0x06]);
        assert_eq!(find_char(b'z' as u16), [0x1F]);
        // A1
        assert_eq!(find_char(b'A' as u16), [0x04, 0x06]);
        assert_eq!(find_char(b'Z' as u16), [0x04, 0x1F]);
        // A2
        assert_eq!(find_char(b'\r' as u16), [0x05, 0x07]);
        assert_eq!(find_char(b')' as u16), [0x05, 0x1F]);
        // Anything else becomes a 4 character ZSCII sequence
        assert_eq!(find_char(b'$' as u16), [0x05, 0x06, 0x01, 0x04])
    }

    #[test]
    fn test_search_entry() {
        let mut map = test_map(4);
        mock_sorted_dictionary(&mut map);
        let zmachine = mock_zmachine(map);
        // Look up each entry
        assert_ok_eq!(
            search_entry(&zmachine, 0x307, 8, 9, &[0x1A69, 0x14A5, 0x94A5]),
            0x307
        );
        assert_ok_eq!(
            search_entry(&zmachine, 0x307, 8, 9, &[0x1EFA, 0x6758, 0x94A5]),
            0x310
        );
        assert_ok_eq!(
            search_entry(&zmachine, 0x307, 8, 9, &[0x3551, 0x4685, 0x94A5]),
            0x319
        );
        assert_ok_eq!(
            search_entry(&zmachine, 0x307, 8, 9, &[0x3A7B, 0x2A79, 0xD2FE]),
            0x322
        );
        assert_ok_eq!(
            search_entry(&zmachine, 0x307, 8, 9, &[0x4694, 0x40A5, 0x94A5]),
            0x32B
        );
        assert_ok_eq!(
            search_entry(&zmachine, 0x307, 8, 9, &[0x4BBE, 0x7EB9, 0xC605]),
            0x334
        );
        assert_ok_eq!(
            search_entry(&zmachine, 0x307, 8, 9, &[0x60CE, 0x4697, 0x94A5]),
            0x33D
        );
        assert_ok_eq!(
            search_entry(&zmachine, 0x307, 8, 9, &[0x77DF, 0x7FC5, 0x94A5]),
            0x346
        );
        // Now look for something that isn't there
        assert_ok_eq!(
            search_entry(&zmachine, 0x307, 8, 9, &[0x3A7B, 0x2A79, 0xD2FF]),
            0
        );
    }

    #[test]
    fn test_scan_entry() {
        let mut map = test_map(4);
        mock_unsorted_dictionary(&mut map);
        let zmachine = mock_zmachine(map);
        // Look up each entry
        assert_ok_eq!(
            scan_entry(&zmachine, 0x307, 8, 9, &[0x1A69, 0x14A5, 0x94A5]),
            0x310
        );
        assert_ok_eq!(
            scan_entry(&zmachine, 0x307, 8, 9, &[0x1EFA, 0x6758, 0x94A5]),
            0x322
        );
        assert_ok_eq!(
            scan_entry(&zmachine, 0x307, 8, 9, &[0x3551, 0x4685, 0x94A5]),
            0x334
        );
        assert_ok_eq!(
            scan_entry(&zmachine, 0x307, 8, 9, &[0x3A7B, 0x2A79, 0xD2FE]),
            0x319
        );
        assert_ok_eq!(
            scan_entry(&zmachine, 0x307, 8, 9, &[0x4694, 0x40A5, 0x94A5]),
            0x307
        );
        assert_ok_eq!(
            scan_entry(&zmachine, 0x307, 8, 9, &[0x4BBE, 0x7EB9, 0xC605]),
            0x346
        );
        assert_ok_eq!(
            scan_entry(&zmachine, 0x307, 8, 9, &[0x60CE, 0x4697, 0x94A5]),
            0x33D
        );
        assert_ok_eq!(
            scan_entry(&zmachine, 0x307, 8, 9, &[0x77DF, 0x7FC5, 0x94A5]),
            0x32B
        );
        // Now look for something that isn't there
        assert_ok_eq!(
            scan_entry(&zmachine, 0x307, 8, 9, &[0x3A7B, 0x2A79, 0xD2FF]),
            0
        );
    }

    #[test]
    fn test_encode_word_v3() {
        // abbreviated; will be truncated in both V3 and V4+
        // 18E7 DD5B
        let mut word = vec![
            b'a' as u16,
            b'b' as u16,
            b'b' as u16,
            b'r' as u16,
            b'e' as u16,
            b'v' as u16,
            b'i' as u16,
            b'a' as u16,
            b't' as u16,
            b'e' as u16,
            b'd' as u16,
        ];
        assert_eq!(encode_text(&mut word, 2), vec![0x18E7, 0xDD5B]);
    }

    #[test]
    fn test_encode_word_v5() {
        // abbreviated; will be truncated in both V3 and V4+
        let mut word = vec![
            b'a' as u16,
            b'b' as u16,
            b'b' as u16,
            b'r' as u16,
            b'e' as u16,
            b'v' as u16,
            b'i' as u16,
            b'a' as u16,
            b't' as u16,
            b'e' as u16,
            b'd' as u16,
        ];
        assert_eq!(encode_text(&mut word, 3), vec![0x18E7, 0x5D5B, 0xB8D9]);
    }

    #[test]
    fn test_from_dictionary_search() {
        let mut map = test_map(4);
        mock_sorted_dictionary(&mut map);

        let zmachine = mock_zmachine(map);
        // Look up each entry
        assert_ok_eq!(from_dictionary(&zmachine, 0x300, &['a', 'n', 'd']), 0x307);
        assert_ok_eq!(
            from_dictionary(&zmachine, 0x300, &['b', 'r', 'u', 't', 'u', 's']),
            0x310
        );
        assert_ok_eq!(
            from_dictionary(&zmachine, 0x300, &['h', 'e', 'l', 'l', 'o']),
            0x319
        );
        assert_ok_eq!(
            from_dictionary(
                &zmachine,
                0x300,
                &['i', 'n', 'v', 'e', 'n', 't', 'o', 'r', 'y']
            ),
            0x322
        );
        assert_ok_eq!(
            from_dictionary(&zmachine, 0x300, &['l', 'o', 'o', 'k']),
            0x32B
        );
        assert_ok_eq!(
            from_dictionary(&zmachine, 0x300, &['m', 'x', 'y', 'z', 'p', 't', 'l', 'k']),
            0x334
        );
        assert_ok_eq!(
            from_dictionary(&zmachine, 0x300, &['s', 'a', 'i', 'l', 'o', 'r']),
            0x33D
        );
        assert_ok_eq!(
            from_dictionary(&zmachine, 0x300, &['x', 'y', 'z', 'z', 'y']),
            0x346
        );
        // Now look for something that isn't there
        assert_ok_eq!(from_dictionary(&zmachine, 0x300, &['n', 'o', 'p', 'e']), 0);
    }

    #[test]
    fn test_from_dictionary_scan() {
        let mut map = test_map(4);
        mock_unsorted_dictionary(&mut map);
        let zmachine = mock_zmachine(map);
        // Look up each entry
        assert_ok_eq!(from_dictionary(&zmachine, 0x300, &['a', 'n', 'd']), 0x310);
        assert_ok_eq!(
            from_dictionary(&zmachine, 0x300, &['b', 'r', 'u', 't', 'u', 's']),
            0x322
        );
        assert_ok_eq!(
            from_dictionary(&zmachine, 0x300, &['h', 'e', 'l', 'l', 'o']),
            0x334
        );
        assert_ok_eq!(
            from_dictionary(
                &zmachine,
                0x300,
                &['i', 'n', 'v', 'e', 'n', 't', 'o', 'r', 'y']
            ),
            0x319
        );
        assert_ok_eq!(
            from_dictionary(&zmachine, 0x300, &['l', 'o', 'o', 'k']),
            0x307
        );
        assert_ok_eq!(
            from_dictionary(&zmachine, 0x300, &['m', 'x', 'y', 'z', 'p', 't', 'l', 'k']),
            0x346
        );
        assert_ok_eq!(
            from_dictionary(&zmachine, 0x300, &['s', 'a', 'i', 'l', 'o', 'r']),
            0x33D
        );
        assert_ok_eq!(
            from_dictionary(&zmachine, 0x300, &['x', 'y', 'z', 'z', 'y']),
            0x32B
        );
        // Now look for something that isn't there
        assert_ok_eq!(from_dictionary(&zmachine, 0x300, &['n', 'o', 'p', 'e']), 0);
    }

    #[test]
    fn test_parse_text_search_v4() {
        let mut map = test_map(4);
        mock_sorted_dictionary(&mut map);

        // Text buffer is at 0x200
        // hello, sailor
        map[0x200] = 32;
        map[0x201] = b'h';
        map[0x202] = b'e';
        map[0x203] = b'l';
        map[0x204] = b'l';
        map[0x205] = b'o';
        map[0x206] = b',';
        map[0x207] = b' ';
        map[0x208] = b's';
        map[0x209] = b'a';
        map[0x20a] = b'i';
        map[0x20b] = b'l';
        map[0x20c] = b'o';
        map[0x20d] = b'r';
        map[0x20e] = 0;

        // Parse buffer is at 0x280
        // Allow up to 4 entries
        map[0x280] = 4;

        let mut zmachine = mock_zmachine(map);
        assert!(parse_text(&mut zmachine, 0x200, 0x280, 0x300, false).is_ok());
        // 3 entries
        assert_ok_eq!(zmachine.read_byte(0x281), 3);
        // hello
        assert_ok_eq!(zmachine.read_word(0x282), 0x319);
        assert_ok_eq!(zmachine.read_byte(0x284), 5);
        assert_ok_eq!(zmachine.read_byte(0x285), 1);
        // ,
        assert_ok_eq!(zmachine.read_word(0x286), 0);
        assert_ok_eq!(zmachine.read_byte(0x288), 1);
        assert_ok_eq!(zmachine.read_byte(0x289), 6);
        // sailor
        assert_ok_eq!(zmachine.read_word(0x28A), 0x33D);
        assert_ok_eq!(zmachine.read_byte(0x28C), 6);
        assert_ok_eq!(zmachine.read_byte(0x28D), 8);
    }

    #[test]
    fn test_parse_text_scan_v5() {
        let mut map = test_map(5);
        mock_unsorted_dictionary(&mut map);

        // Text buffer is at 0x200
        // hello, sailor
        map[0x200] = 32;
        map[0x201] = 13;
        map[0x202] = b'h';
        map[0x203] = b'e';
        map[0x204] = b'l';
        map[0x205] = b'l';
        map[0x206] = b'o';
        map[0x207] = b',';
        map[0x208] = b' ';
        map[0x209] = b's';
        map[0x20A] = b'a';
        map[0x20B] = b'i';
        map[0x20C] = b'l';
        map[0x20D] = b'o';
        map[0x20E] = b'r';

        // Parse buffer is at 0x280
        // Allow up to 4 entries
        map[0x280] = 4;

        let mut zmachine = mock_zmachine(map);
        assert!(parse_text(&mut zmachine, 0x200, 0x280, 0x300, false).is_ok());
        // 3 entries
        assert_ok_eq!(zmachine.read_byte(0x281), 3);
        // hello
        assert_ok_eq!(zmachine.read_word(0x282), 0x334);
        assert_ok_eq!(zmachine.read_byte(0x284), 5);
        assert_ok_eq!(zmachine.read_byte(0x285), 2);
        // ,
        assert_ok_eq!(zmachine.read_word(0x286), 0);
        assert_ok_eq!(zmachine.read_byte(0x288), 1);
        assert_ok_eq!(zmachine.read_byte(0x289), 7);
        // sailor
        assert_ok_eq!(zmachine.read_word(0x28A), 0x33D);
        assert_ok_eq!(zmachine.read_byte(0x28C), 6);
        assert_ok_eq!(zmachine.read_byte(0x28D), 9);
    }

    #[test]
    fn test_parse_text_v5_overlay() {
        let mut map = test_map(5);
        mock_unsorted_dictionary(&mut map);

        // Text buffer is at 0x200
        // hello, sailor
        map[0x200] = 32;
        map[0x201] = 13;
        map[0x202] = b'a';
        map[0x203] = b'd';
        map[0x204] = b'i';
        map[0x205] = b'o';
        map[0x206] = b's';
        map[0x207] = b',';
        map[0x208] = b' ';
        map[0x209] = b's';
        map[0x20A] = b'a';
        map[0x20B] = b'i';
        map[0x20C] = b'l';
        map[0x20D] = b'o';
        map[0x20E] = b'r';

        // Parse buffer is at 0x280
        // Allow up to 4 entries
        map[0x280] = 4;
        // Previously parsed words 1 and 2
        map[0x281] = 3;
        map[0x282] = 0x11;
        map[0x283] = 0x22;
        map[0x284] = 5;
        map[0x285] = 2;
        map[0x286] = 0x11;
        map[0x287] = 0x33;
        map[0x288] = 1;
        map[0x289] = 7;
        map[0x28A] = 0;
        map[0x28B] = 0;
        map[0x28C] = 6;
        map[0x28D] = 8;
        let mut zmachine = mock_zmachine(map);
        assert!(parse_text(&mut zmachine, 0x200, 0x280, 0x300, true).is_ok());
        // 3 entries
        assert_ok_eq!(zmachine.read_byte(0x281), 3);
        // previously parsed adios
        assert_ok_eq!(zmachine.read_word(0x282), 0x1122);
        assert_ok_eq!(zmachine.read_byte(0x284), 5);
        assert_ok_eq!(zmachine.read_byte(0x285), 2);
        // previously parsed ,
        assert_ok_eq!(zmachine.read_word(0x286), 0x1133);
        assert_ok_eq!(zmachine.read_byte(0x288), 1);
        assert_ok_eq!(zmachine.read_byte(0x289), 7);
        // sailor
        assert_ok_eq!(zmachine.read_word(0x28A), 0x33D);
        assert_ok_eq!(zmachine.read_byte(0x28C), 6);
        assert_ok_eq!(zmachine.read_byte(0x28D), 9);
    }
}
