use crate::{
    error::*,
    zmachine::{state::header::HeaderField, ZMachine},
};

pub mod attribute;
pub mod property;

fn object_address(zmachine: &ZMachine, object: usize) -> Result<usize, RuntimeError> {
    if object == 0 {
        Ok(0)
    } else {
        let table = zmachine.header_word(HeaderField::ObjectTable)? as usize;
        let (offset, size) = match zmachine.version() {
            3 => (62, 9),
            _ => (126, 14),
        };

        Ok(table + offset + (size * (object - 1)))
    }
}

fn relative(zmachine: &ZMachine, object: usize, offset: usize) -> Result<usize, RuntimeError> {
    if object == 0 {
        Ok(0)
    } else {
        let object_address = object_address(zmachine, object)?;

        match zmachine.version() {
            3 => Ok(zmachine.read_byte(object_address + offset)? as usize),
            _ => Ok(zmachine.read_word(object_address + offset)? as usize),
        }
    }
}
pub fn parent(zmachine: &ZMachine, object: usize) -> Result<usize, RuntimeError> {
    let offset = match zmachine.version() {
        3 => 4,
        _ => 6,
    };

    relative(zmachine, object, offset)
}

pub fn child(zmachine: &ZMachine, object: usize) -> Result<usize, RuntimeError> {
    let offset = match zmachine.version() {
        3 => 6,
        _ => 10,
    };

    relative(zmachine, object, offset)
}

pub fn sibling(zmachine: &ZMachine, object: usize) -> Result<usize, RuntimeError> {
    let offset = match zmachine.version() {
        3 => 5,
        _ => 8,
    };

    relative(zmachine, object, offset)
}

fn set_relative(
    zmachine: &mut ZMachine,
    offset: usize,
    object: usize,
    relative: usize,
) -> Result<(), RuntimeError> {
    let object_address = object_address(zmachine, object)?;

    match zmachine.version() {
        3 => zmachine.write_byte(object_address + offset, relative as u8),
        _ => zmachine.write_word(object_address + offset, relative as u16),
    }
}

pub fn set_parent(
    zmachine: &mut ZMachine,
    object: usize,
    parent: usize,
) -> Result<(), RuntimeError> {
    let offset = match zmachine.version() {
        3 => 4,
        _ => 6,
    };

    set_relative(zmachine, offset, object, parent)
}

pub fn set_child(zmachine: &mut ZMachine, object: usize, child: usize) -> Result<(), RuntimeError> {
    let offset = match zmachine.version() {
        3 => 6,
        _ => 10,
    };

    set_relative(zmachine, offset, object, child)
}

pub fn set_sibling(
    zmachine: &mut ZMachine,
    object: usize,
    sibling: usize,
) -> Result<(), RuntimeError> {
    let offset = match zmachine.version() {
        3 => 5,
        _ => 8,
    };

    set_relative(zmachine, offset, object, sibling)
}

#[cfg(test)]
mod tests {
    use crate::test_util::*;

    use super::*;

    #[test]
    fn test_parent_v3() {
        let mut map = test_map(3);
        map[0x0a] = 0x3;
        mock_object(&mut map, 1, vec![], (0, 0, 2));
        mock_object(&mut map, 2, vec![], (1, 3, 4));
        mock_object(&mut map, 4, vec![], (2, 5, 0));

        let zmachine = mock_zmachine(map);
        assert!(parent(&zmachine, 1).is_ok_and(|x| x == 0));
        assert!(parent(&zmachine, 2).is_ok_and(|x| x == 1));
        assert!(parent(&zmachine, 4).is_ok_and(|x| x == 2));
    }

    #[test]
    fn test_parent_v4() {
        let mut map = test_map(4);
        map[0x0a] = 0x3;
        mock_object(&mut map, 1, vec![], (0, 0, 2));
        mock_object(&mut map, 2, vec![], (1, 3, 4));
        mock_object(&mut map, 4, vec![], (2, 5, 0));

        let zmachine = mock_zmachine(map);
        assert!(parent(&zmachine, 1).is_ok_and(|x| x == 0));
        assert!(parent(&zmachine, 2).is_ok_and(|x| x == 1));
        assert!(parent(&zmachine, 4).is_ok_and(|x| x == 2));
    }

    #[test]
    fn test_child_v3() {
        let mut map = test_map(3);
        map[0x0a] = 0x3;
        mock_object(&mut map, 1, vec![], (0, 0, 2));
        mock_object(&mut map, 2, vec![], (1, 3, 4));
        mock_object(&mut map, 4, vec![], (2, 5, 0));

        let zmachine = mock_zmachine(map);
        assert!(child(&zmachine, 1).is_ok_and(|x| x == 2));
        assert!(child(&zmachine, 2).is_ok_and(|x| x == 4));
        assert!(child(&zmachine, 4).is_ok_and(|x| x == 0));
    }

    #[test]
    fn test_child_v4() {
        let mut map = test_map(4);
        map[0x0a] = 0x3;
        mock_object(&mut map, 1, vec![], (0, 0, 2));
        mock_object(&mut map, 2, vec![], (1, 3, 4));
        mock_object(&mut map, 4, vec![], (2, 5, 0));

        let zmachine = mock_zmachine(map);
        assert!(child(&zmachine, 1).is_ok_and(|x| x == 2));
        assert!(child(&zmachine, 2).is_ok_and(|x| x == 4));
        assert!(child(&zmachine, 4).is_ok_and(|x| x == 0));
    }

    #[test]
    fn test_sibling_v3() {
        let mut map = test_map(3);
        map[0x0a] = 0x3;
        mock_object(&mut map, 1, vec![], (0, 0, 2));
        mock_object(&mut map, 2, vec![], (1, 3, 4));
        mock_object(&mut map, 4, vec![], (2, 5, 0));

        let zmachine = mock_zmachine(map);
        assert!(sibling(&zmachine, 1).is_ok_and(|x| x == 0));
        assert!(sibling(&zmachine, 2).is_ok_and(|x| x == 3));
        assert!(sibling(&zmachine, 4).is_ok_and(|x| x == 5));
    }

    #[test]
    fn test_sibling_v4() {
        let mut map = test_map(4);
        map[0x0a] = 0x3;
        mock_object(&mut map, 1, vec![], (0, 0, 2));
        mock_object(&mut map, 2, vec![], (1, 3, 4));
        mock_object(&mut map, 4, vec![], (2, 5, 0));

        let zmachine = mock_zmachine(map);
        assert!(sibling(&zmachine, 1).is_ok_and(|x| x == 0));
        assert!(sibling(&zmachine, 2).is_ok_and(|x| x == 3));
        assert!(sibling(&zmachine, 4).is_ok_and(|x| x == 5));
    }

    #[test]
    fn test_set_parent_v3() {
        let mut map = test_map(3);
        map[0x0a] = 0x3;
        mock_object(&mut map, 1, vec![], (0, 0, 2));
        mock_object(&mut map, 2, vec![], (1, 3, 4));
        mock_object(&mut map, 4, vec![], (2, 5, 0));

        let mut zmachine = mock_zmachine(map);
        assert!(set_parent(&mut zmachine, 2, 4).is_ok());
        assert!(set_parent(&mut zmachine, 4, 1).is_ok());
        assert!(parent(&zmachine, 1).is_ok_and(|x| x == 0));
        assert!(parent(&zmachine, 2).is_ok_and(|x| x == 4));
        assert!(parent(&zmachine, 4).is_ok_and(|x| x == 1));
    }

    #[test]
    fn test_set_parent_v4() {
        let mut map = test_map(4);
        map[0x0a] = 0x3;
        mock_object(&mut map, 1, vec![], (0, 0, 2));
        mock_object(&mut map, 2, vec![], (1, 3, 4));
        mock_object(&mut map, 4, vec![], (2, 5, 0));

        let mut zmachine = mock_zmachine(map);
        assert!(set_parent(&mut zmachine, 2, 4).is_ok());
        assert!(set_parent(&mut zmachine, 4, 1).is_ok());
        assert!(parent(&zmachine, 1).is_ok_and(|x| x == 0));
        assert!(parent(&zmachine, 2).is_ok_and(|x| x == 4));
        assert!(parent(&zmachine, 4).is_ok_and(|x| x == 1));
    }

    #[test]
    fn test_set_child_v3() {
        let mut map = test_map(3);
        map[0x0a] = 0x3;
        mock_object(&mut map, 1, vec![], (0, 0, 2));
        mock_object(&mut map, 2, vec![], (1, 3, 4));
        mock_object(&mut map, 4, vec![], (2, 5, 0));

        let mut zmachine = mock_zmachine(map);
        assert!(set_child(&mut zmachine, 1, 4).is_ok());
        assert!(set_child(&mut zmachine, 2, 0).is_ok());
        assert!(child(&zmachine, 1).is_ok_and(|x| x == 4));
        assert!(child(&zmachine, 2).is_ok_and(|x| x == 0));
        assert!(child(&zmachine, 4).is_ok_and(|x| x == 0));
    }

    #[test]
    fn test_set_child_v4() {
        let mut map = test_map(4);
        map[0x0a] = 0x3;
        mock_object(&mut map, 1, vec![], (0, 0, 2));
        mock_object(&mut map, 2, vec![], (1, 3, 4));
        mock_object(&mut map, 4, vec![], (2, 5, 0));

        let mut zmachine = mock_zmachine(map);
        assert!(set_child(&mut zmachine, 1, 4).is_ok());
        assert!(set_child(&mut zmachine, 2, 0).is_ok());
        assert!(child(&zmachine, 1).is_ok_and(|x| x == 4));
        assert!(child(&zmachine, 2).is_ok_and(|x| x == 0));
        assert!(child(&zmachine, 4).is_ok_and(|x| x == 0));
    }

    #[test]
    fn test_set_sibling_v3() {
        let mut map = test_map(3);
        map[0x0a] = 0x3;
        mock_object(&mut map, 1, vec![], (0, 0, 2));
        mock_object(&mut map, 2, vec![], (1, 3, 4));
        mock_object(&mut map, 4, vec![], (2, 5, 0));

        let mut zmachine = mock_zmachine(map);
        assert!(set_sibling(&mut zmachine, 2, 5).is_ok());
        assert!(set_sibling(&mut zmachine, 4, 3).is_ok());
        assert!(sibling(&zmachine, 1).is_ok_and(|x| x == 0));
        assert!(sibling(&zmachine, 2).is_ok_and(|x| x == 5));
        assert!(sibling(&zmachine, 4).is_ok_and(|x| x == 3));
    }

    #[test]
    fn test_set_sibling_v4() {
        let mut map = test_map(4);
        map[0x0a] = 0x3;
        mock_object(&mut map, 1, vec![], (0, 0, 2));
        mock_object(&mut map, 2, vec![], (1, 3, 4));
        mock_object(&mut map, 4, vec![], (2, 5, 0));

        let mut zmachine = mock_zmachine(map);
        assert!(set_sibling(&mut zmachine, 2, 5).is_ok());
        assert!(set_sibling(&mut zmachine, 4, 3).is_ok());
        assert!(sibling(&zmachine, 1).is_ok_and(|x| x == 0));
        assert!(sibling(&zmachine, 2).is_ok_and(|x| x == 5));
        assert!(sibling(&zmachine, 4).is_ok_and(|x| x == 3));
    }
}
