use crate::{error::*, zmachine::ZMachine};

use super::object_address;

pub fn value(zmachine: &ZMachine, object: usize, attribute: u8) -> Result<bool, RuntimeError> {
    let object_address = object_address(zmachine, object)?;
    let offset = attribute as usize / 8;
    let address = object_address + offset;
    let mask = 1 << (7 - (attribute % 8));
    let max = match zmachine.version() {
        3 => 32,
        _ => 48,
    };

    if attribute < max {
        let value = zmachine.read_byte(address)?;
        Ok(value & mask == mask)
    } else {
        warn!(target: "app::object", "Request to set invalid attribute {} on object {}", attribute, object);
        Ok(false)
    }
}

pub fn set(zmachine: &mut ZMachine, object: usize, attribute: u8) -> Result<(), RuntimeError> {
    let object_address = object_address(zmachine, object)?;
    let offset = attribute as usize / 8;
    let address = object_address + offset;
    let mask = 1 << (7 - (attribute % 8));
    let max = match zmachine.version() {
        3 => 32,
        _ => 48,
    };

    if attribute < max {
        let attribute_byte = zmachine.read_byte(address)?;
        zmachine.write_byte(address, attribute_byte | mask)
    } else {
        warn!(target: "app::object", "Request to set invalid attribute {} on object {}", attribute, object);
        Ok(())
    }
}

pub fn clear(zmachine: &mut ZMachine, object: usize, attribute: u8) -> Result<(), RuntimeError> {
    let object_address = object_address(zmachine, object)?;
    let offset = attribute as usize / 8;
    let address = object_address + offset;
    let mask: u8 = 1 << (7 - (attribute % 8));
    let max = match zmachine.version() {
        3 => 32,
        _ => 48,
    };

    if attribute < max {
        let attribute_byte = zmachine.read_byte(address)?;
        zmachine.write_byte(address, attribute_byte & !mask)
    } else {
        warn!(target: "app::object", "Request to set invalid attribute {} on object {}", attribute, object);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        object::attribute::{set, value},
        test_util::*,
    };

    use super::clear;

    #[test]
    fn test_value_v3() {
        let mut map = test_map(3);
        map[0x0a] = 0x03;
        mock_attributes(&mut map, 1, &[0xA5, 0x96, 0xC3, 0x42, 0xFF]);
        let zmachine = mock_zmachine(map);
        assert!(value(&zmachine, 1, 0).is_ok_and(|x| x));
        assert!(value(&zmachine, 1, 1).is_ok_and(|x| !x));
        assert!(value(&zmachine, 1, 2).is_ok_and(|x| x));
        assert!(value(&zmachine, 1, 3).is_ok_and(|x| !x));
        assert!(value(&zmachine, 1, 4).is_ok_and(|x| !x));
        assert!(value(&zmachine, 1, 5).is_ok_and(|x| x));
        assert!(value(&zmachine, 1, 6).is_ok_and(|x| !x));
        assert!(value(&zmachine, 1, 7).is_ok_and(|x| x));
        assert!(value(&zmachine, 1, 8).is_ok_and(|x| x));
        assert!(value(&zmachine, 1, 9).is_ok_and(|x| !x));
        assert!(value(&zmachine, 1, 10).is_ok_and(|x| !x));
        assert!(value(&zmachine, 1, 11).is_ok_and(|x| x));
        assert!(value(&zmachine, 1, 12).is_ok_and(|x| !x));
        assert!(value(&zmachine, 1, 13).is_ok_and(|x| x));
        assert!(value(&zmachine, 1, 14).is_ok_and(|x| x));
        assert!(value(&zmachine, 1, 15).is_ok_and(|x| !x));
        assert!(value(&zmachine, 1, 16).is_ok_and(|x| x));
        assert!(value(&zmachine, 1, 17).is_ok_and(|x| x));
        assert!(value(&zmachine, 1, 18).is_ok_and(|x| !x));
        assert!(value(&zmachine, 1, 19).is_ok_and(|x| !x));
        assert!(value(&zmachine, 1, 20).is_ok_and(|x| !x));
        assert!(value(&zmachine, 1, 21).is_ok_and(|x| !x));
        assert!(value(&zmachine, 1, 22).is_ok_and(|x| x));
        assert!(value(&zmachine, 1, 23).is_ok_and(|x| x));
        assert!(value(&zmachine, 1, 24).is_ok_and(|x| !x));
        assert!(value(&zmachine, 1, 25).is_ok_and(|x| x));
        assert!(value(&zmachine, 1, 26).is_ok_and(|x| !x));
        assert!(value(&zmachine, 1, 27).is_ok_and(|x| !x));
        assert!(value(&zmachine, 1, 28).is_ok_and(|x| !x));
        assert!(value(&zmachine, 1, 29).is_ok_and(|x| !x));
        assert!(value(&zmachine, 1, 30).is_ok_and(|x| x));
        assert!(value(&zmachine, 1, 31).is_ok_and(|x| !x));
        assert!(value(&zmachine, 1, 32).is_ok_and(|x| !x));
    }

    #[test]
    fn test_value_v4() {
        let mut map = test_map(4);
        map[0x0a] = 0x03;
        mock_attributes(&mut map, 1, &[0xA5, 0x96, 0xC3, 0x42, 0x81, 0x7E, 0xFF]);
        let zmachine = mock_zmachine(map);
        assert!(value(&zmachine, 1, 0).is_ok_and(|x| x));
        assert!(value(&zmachine, 1, 1).is_ok_and(|x| !x));
        assert!(value(&zmachine, 1, 2).is_ok_and(|x| x));
        assert!(value(&zmachine, 1, 3).is_ok_and(|x| !x));
        assert!(value(&zmachine, 1, 4).is_ok_and(|x| !x));
        assert!(value(&zmachine, 1, 5).is_ok_and(|x| x));
        assert!(value(&zmachine, 1, 6).is_ok_and(|x| !x));
        assert!(value(&zmachine, 1, 7).is_ok_and(|x| x));
        assert!(value(&zmachine, 1, 8).is_ok_and(|x| x));
        assert!(value(&zmachine, 1, 9).is_ok_and(|x| !x));
        assert!(value(&zmachine, 1, 10).is_ok_and(|x| !x));
        assert!(value(&zmachine, 1, 11).is_ok_and(|x| x));
        assert!(value(&zmachine, 1, 12).is_ok_and(|x| !x));
        assert!(value(&zmachine, 1, 13).is_ok_and(|x| x));
        assert!(value(&zmachine, 1, 14).is_ok_and(|x| x));
        assert!(value(&zmachine, 1, 15).is_ok_and(|x| !x));
        assert!(value(&zmachine, 1, 16).is_ok_and(|x| x));
        assert!(value(&zmachine, 1, 17).is_ok_and(|x| x));
        assert!(value(&zmachine, 1, 18).is_ok_and(|x| !x));
        assert!(value(&zmachine, 1, 19).is_ok_and(|x| !x));
        assert!(value(&zmachine, 1, 20).is_ok_and(|x| !x));
        assert!(value(&zmachine, 1, 21).is_ok_and(|x| !x));
        assert!(value(&zmachine, 1, 22).is_ok_and(|x| x));
        assert!(value(&zmachine, 1, 23).is_ok_and(|x| x));
        assert!(value(&zmachine, 1, 24).is_ok_and(|x| !x));
        assert!(value(&zmachine, 1, 25).is_ok_and(|x| x));
        assert!(value(&zmachine, 1, 26).is_ok_and(|x| !x));
        assert!(value(&zmachine, 1, 27).is_ok_and(|x| !x));
        assert!(value(&zmachine, 1, 28).is_ok_and(|x| !x));
        assert!(value(&zmachine, 1, 29).is_ok_and(|x| !x));
        assert!(value(&zmachine, 1, 30).is_ok_and(|x| x));
        assert!(value(&zmachine, 1, 31).is_ok_and(|x| !x));
        assert!(value(&zmachine, 1, 32).is_ok_and(|x| x));
        assert!(value(&zmachine, 1, 33).is_ok_and(|x| !x));
        assert!(value(&zmachine, 1, 34).is_ok_and(|x| !x));
        assert!(value(&zmachine, 1, 35).is_ok_and(|x| !x));
        assert!(value(&zmachine, 1, 36).is_ok_and(|x| !x));
        assert!(value(&zmachine, 1, 37).is_ok_and(|x| !x));
        assert!(value(&zmachine, 1, 38).is_ok_and(|x| !x));
        assert!(value(&zmachine, 1, 39).is_ok_and(|x| x));
        assert!(value(&zmachine, 1, 40).is_ok_and(|x| !x));
        assert!(value(&zmachine, 1, 41).is_ok_and(|x| x));
        assert!(value(&zmachine, 1, 42).is_ok_and(|x| x));
        assert!(value(&zmachine, 1, 43).is_ok_and(|x| x));
        assert!(value(&zmachine, 1, 44).is_ok_and(|x| x));
        assert!(value(&zmachine, 1, 45).is_ok_and(|x| x));
        assert!(value(&zmachine, 1, 46).is_ok_and(|x| x));
        assert!(value(&zmachine, 1, 47).is_ok_and(|x| !x));
        assert!(value(&zmachine, 1, 48).is_ok_and(|x| !x));
    }

    #[test]
    fn test_set_v3() {
        let mut map = test_map(3);
        mock_attributes(&mut map, 1, &[0x00, 0x00, 0x00, 0x00]);
        let mut zmachine = mock_zmachine(map);
        for i in 0..33 {
            if i % 2 == 1 {
                assert!(set(&mut zmachine, 1, i as u8).is_ok());
            }
            for j in 0..33 {
                assert!(
                    value(&zmachine, 1, j as u8).is_ok_and(|x| x == (j <= i && (j % 2 == 1))),
                    "Pass {}: Attribute {} should be {}",
                    i,
                    j,
                    (j >= i && (j % 2 == 1))
                );
            }
        }
    }

    #[test]
    fn test_set_v4() {
        let mut map = test_map(4);
        mock_attributes(&mut map, 1, &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        let mut zmachine = mock_zmachine(map);
        for i in 0..49 {
            if i % 2 == 1 {
                assert!(set(&mut zmachine, 1, i as u8).is_ok());
            }
            for j in 0..49 {
                assert!(
                    value(&zmachine, 1, j as u8).is_ok_and(|x| x == (j <= i && (j % 2 == 1))),
                    "Pass {}: Attribute {} should be {}",
                    i,
                    j,
                    (j >= i && (j % 2 == 1))
                );
            }
        }
    }

    #[test]
    fn test_clear_v3() {
        let mut map = test_map(3);
        mock_attributes(&mut map, 1, &[0xFF, 0xFF, 0xFF, 0xFF]);
        let mut zmachine = mock_zmachine(map);
        for i in 0..33 {
            if i % 2 == 0 {
                assert!(clear(&mut zmachine, 1, i as u8).is_ok());
            }
            for j in 0..33 {
                assert!(
                    value(&zmachine, 1, j as u8)
                        .is_ok_and(|x| x == (j < 32 && (j > i || (j % 2 == 1)))),
                    "Pass {}: Attribute {} should be {}",
                    i,
                    j,
                    (j >= i && (j % 2 == 1))
                );
            }
        }
    }

    #[test]
    fn test_clear_v4() {
        let mut map = test_map(4);
        mock_attributes(&mut map, 1, &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
        let mut zmachine = mock_zmachine(map);
        for i in 0..49 {
            if i % 2 == 0 {
                assert!(clear(&mut zmachine, 1, i as u8).is_ok());
            }
            for j in 0..49 {
                assert!(
                    value(&zmachine, 1, j as u8)
                        .is_ok_and(|x| x == (j < 48 && (j > i || (j % 2 == 1)))),
                    "Pass {}: Attribute {} should be {}",
                    i,
                    j,
                    (j >= i && (j % 2 == 1))
                );
            }
        }
    }
}
