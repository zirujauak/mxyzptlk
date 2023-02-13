/// Read a word from a memory map
///
/// # Arguments:
///
/// * `m` - Memory map
/// * `a` - address
fn word_value(m: &Vec<u8>, a: usize) -> u16 {
    let hb = m[a];
    let lb = m[a + 1];

    (((hb as u16) << 8) & 0xFF00) + ((lb as u16) & 0xFF)
}

/// Return the object table byte address stored at $0010 in the memory map
///
/// # Arguments:
///
/// * `m` - Memory map
fn object_table_address(m: &Vec<u8>) -> usize {
    word_value(m, 10) as usize
}

/// Return the default value for a property
///
/// # Arguments:
///
/// * `m` - Memory map
/// * `p` - Property number
fn default_property(m: &Vec<u8>, p: u8) -> Vec<u8> {
    let ota = object_table_address(m);
    let pa = ota + (p as usize * 2);
    let mut v = Vec::new();
    v.push(m[pa]);
    v.push(m[pa + 1]);
    v
}

/// Calculate the address of an object in the memory map
///
/// # Arguments:
///
/// * `m` - Memory map
/// * `v` - ZMachine version (1-8)
/// * `o` - Object number (1 - ...)
fn object_address(m: &Vec<u8>, v: u8, o: usize) -> usize {
    match v {
        1 | 2 | 3 => object_table_address(m) + 62 + (9 * (o - 1)),
        4 | 5 | 6 | 7 | 8 => object_table_address(m) + 126 + (14 * (o - 1)),
        // TODO: Error
        _ => 0,
    }
}

/// Get an attribute value, 1 = set, 0 = clear
///
/// # Arguments:
///
/// * `m` - Memory map
/// * `v` - ZMachine verison (1-8)
/// * `o` - Object number
/// * `a` - Attribute number
pub fn attribute(m: &Vec<u8>, v: u8, o: usize, a: usize) -> u8 {
    let oa = object_address(m, v, o);
    match v {
        1 | 2 | 3 => {
            if a < 32 {
                (m[oa + (a / 8)] >> (7 - (a % 8))) & 1
            } else {
                0
            }
        }
        4 | 5 | 6 | 7 | 8 => {
            if a < 48 {
                let byte = m[oa + (a / 8)];
                (byte >> (7 - (a % 8))) & 0x1
            } else {
                0
            }
        }
        _ => 0,
    }
}

/// Set an attribute
///
/// # Arguments:
///
/// * `m` - Memory map
/// * `v` - ZMachine verison (1-8)
/// * `o` - Object number
/// * `a` - Attribute number
pub fn set_attribute(m: &mut Vec<u8>, v: u8, o: usize, a: usize) {
    let oa = object_address(m, v, o);
    let mask: u8 = 1 << 7 - (a % 8);
    match v {
        1 | 2 | 3 => {
            if a < 32 {
                m[oa + (a / 8)] = m[oa + (a / 8)] | mask
            }
        }
        4 | 5 | 6 | 7 | 8 => {
            if a < 48 {
                m[oa + (a / 8)] = m[oa + (a / 8)] | mask
            }
        }
        _ => {}
    }
}

/// Clear an attribute
///
/// # Arguments:
///
/// * `m` - Memory map
/// * `v` - ZMachine verison (1-8)
/// * `o` - Object number
/// * `a` - Attribute number
pub fn clear_attribute(m: &mut Vec<u8>, v: u8, o: usize, a: usize) {
    let oa = object_address(m, v, o);
    let mask: u8 = 1 << 7 - (a % 8);
    match v {
        1 | 2 | 3 => {
            if a < 32 {
                m[oa + (a / 8)] = m[oa + (a / 8)] & !mask
            }
        }
        4 | 5 | 6 | 7 | 8 => {
            if a < 48 {
                m[oa + (a / 8)] = m[oa + (a / 8)] & !mask
            }
        }
        _ => {}
    }
}

/// Return the parent object
///
/// # Arguments:
///
/// * `m` - Memory map
/// * `v` - ZMachine verison (1-8)
/// * `o` - Object number
pub fn parent(m: &Vec<u8>, v: u8, o: usize) -> usize {
    let oa = object_address(m, v, o);

    match v {
        1 | 2 | 3 => m[oa + 4] as usize,
        4 | 5 | 6 | 7 | 8 => word_value(m, oa + 6) as usize,
        _ => 0,
    }
}

/// Return the first sibling object
///
/// # Arguments:
///
/// * `m` - Memory map
/// * `v` - ZMachine verison (1-8)
/// * `o` - Object number
pub fn sibling(m: &Vec<u8>, v: u8, o: usize) -> usize {
    let oa = object_address(m, v, o);

    match v {
        1 | 2 | 3 => m[oa + 5] as usize,
        4 | 5 | 6 | 7 | 8 => word_value(m, oa + 8) as usize,
        _ => 0,
    }
}

/// Return the child object
///
/// # Arguments:
///
/// * `m` - Memory map
/// * `v` - ZMachine verison (1-8)
/// * `o` - Object number
pub fn child(m: &Vec<u8>, v: u8, o: usize) -> usize {
    let oa = object_address(m, v, o);

    match v {
        1 | 2 | 3 => m[oa + 6] as usize,
        4 | 5 | 6 | 7 | 8 => word_value(m, oa + 10) as usize,
        _ => 0,
    }
}

/// Get the address of the property table for an object
///
/// # Arguments:
///
/// * `m` - Memory map
/// * `v` - Version (1-8)
/// * `o` - Object number
fn property_table_address(m: &Vec<u8>, v: u8, o: usize) -> usize {
    let oa = object_address(m, v, o);
    match v {
        1 | 2 | 3 => word_value(m, oa + 7) as usize,
        4 | 5 | 6 | 7 | 8 => word_value(m, oa + 12) as usize,
        _ => 0,
    }
}

/// Get an object's short name as a vector of ZSCII words
///
/// # Arguments:
///
/// * `m` - Memory map
/// * `v` - Version (1-8)
/// * `o` - Object number
pub fn short_name(m: &Vec<u8>, v: u8, o: usize) -> Vec<u16> {
    let pt = property_table_address(m, v, o);
    let hs = m[pt] as usize;
    let mut r = Vec::new();
    for i in 0..hs {
        r.push(word_value(m, pt + 1 + (i * 2)))
    }
    r
}

/// Get a property value for an object as a vector of bytes.  If the property is not
/// set for the object itself, the default property value is returned.
///
/// # Arguments:
///
/// * `m` - Memory map
/// * `v` - Version (1-8)
/// * `o` - Object number
/// * `p` - Property number
pub fn property(m: &Vec<u8>, v: u8, o: usize, p: u8) -> Vec<u8> {
    let pt = property_table_address(m, v, o);
    let hs = m[pt] as usize;
    let mut pa = pt + 1 + (hs * 2);
    let mut r = Vec::new();
    while r.is_empty() {
        match v {
            // V1-3:
            //  * Size/number encoded in a single byte
            1 | 2 | 3 => {
                let sb = m[pa];
                let n = sb & 0x1F;
                let s = (sb as usize / 32) + 1;
                if n == p {
                    for i in 0..s {
                        r.push(m[pa + 1 + i])
                    }
                } else if n < p {
                    r = default_property(m, p);
                } else {
                    pa = pa + 1 + s;
                }
            },
            // V4+:
            //  * Size/number encoded in a single byte -or-
            //  * Number in first byte, size in second
            4 | 5 | 6 | 7 | 8 => {
                let mut pd = pa + 1;
                let sb = m[pa];
                let n = sb & 0x3F;
                let s = {
                    if sb & 0x80 == 0x80 {
                        pd = pd + 1;
                        m[pa + 1] as usize & 0x3F
                    } else {
                        if sb & 0x40 == 0x40 {
                            2
                        } else {
                            1
                        }
                    }
                };

                if n == p {
                    for i in 0..s {
                        r.push(m[pd + i]);
                    }
                } else if n < p {
                    r = default_property(m, p);
                } else {
                    pa = pd + s;
                }
            },
            _ => {}
        }
    }
    r
}
