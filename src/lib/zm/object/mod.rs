//! [Object](https://inform-fiction.org/zmachine/standards/z1point1/sect12.html) utility functions

use crate::{
    error::*,
    zmachine::{header::HeaderField, ZMachine},
};

pub mod attribute;
pub mod property;

/// Gets the byte address of an object's table entry
///
/// If `object` is 0, 0 is returned.
///
/// # Arguments
/// * `zmachine` - Reference to the zmachine
/// * `object` - Object number
///
/// # Returns
/// [Result] with the byte address of the object table entry, 0, or a [RuntimeError]
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

/// Gets the byte address of an object's relative (sibling, child, or parent)
///
/// If `object` is 0, 0 is returned.
///
/// # Arguments
/// * `zmachine` - Reference to the zmachine
/// * `object` - Object number
/// * `offset` - Byte offset of the relative data in the object's table entry
///
/// # Returns
/// [Result] with the relative object number, 0, or a [RuntimeError]
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

/// Gets the byte address of an object's parent
///
/// If `object` is 0, 0 is returned
///
/// # Arguments
/// * `zmachine` - Reference to the zmachine
/// * `object` - Object number
///
/// # Returns
/// [Result] with the byte address of the parent, 0 or a [RuntimeError]
pub fn parent(zmachine: &ZMachine, object: usize) -> Result<usize, RuntimeError> {
    let offset = match zmachine.version() {
        3 => 4,
        _ => 6,
    };

    relative(zmachine, object, offset)
}

/// Gets the byte address of an object's child
///
/// If `object` is 0, 0 is returned
///
/// # Arguments
/// * `zmachine` - Reference to the zmachine
/// * `object` - Object number
///
/// # Returns
/// [Result] with the byte address of the child, 0 or a [RuntimeError]
pub fn child(zmachine: &ZMachine, object: usize) -> Result<usize, RuntimeError> {
    let offset = match zmachine.version() {
        3 => 6,
        _ => 10,
    };

    relative(zmachine, object, offset)
}
/// Gets the byte address of an object's first sibling
///
/// If `object` is 0, 0 is returned
///
/// # Arguments
/// * `zmachine` - Reference to the zmachine
/// * `object` - Object number
///
/// # Returns
/// [Result] with the byte address of the first sibling, 0 or a [RuntimeError]
pub fn sibling(zmachine: &ZMachine, object: usize) -> Result<usize, RuntimeError> {
    let offset = match zmachine.version() {
        3 => 5,
        _ => 8,
    };

    relative(zmachine, object, offset)
}

/// Sets the relative (parent, child, sigling) of an object
///
/// # Arguments
/// * `zmachine` - Reference to the zmachine
/// * `offset` - Byte offset of the relative data in the object's table entry
/// * `object` - Object number
/// * `relative` - New relative object number
///
/// # Returns
/// Empty [Result] or a [RuntimeError]
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

/// Sets the parent of an object.
///
/// This function only updates the `object` table entry and does *not* remove the `object`
/// from its previous parent or otherise update the object tree.
///
/// # Arguments
/// * `zmachine` - Reference to the zmachine
/// * `object` - Object number
/// * `parent` - New parent object number
///
/// # Returns
/// Empty [Result] or a [RuntimeError]
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

/// Sets the child of an object.
///
/// This function only updates the `object` table entry and does *not* update the sibling
/// of the new `child` object or otherwise update the object tree.
///
/// # Arguments
/// * `zmachine` - Reference to the zmachine
/// * `object` - Object number
/// * `child` - New child object number
///
/// # Returns
/// Empty [Result] or a [RuntimeError]
pub fn set_child(zmachine: &mut ZMachine, object: usize, child: usize) -> Result<(), RuntimeError> {
    let offset = match zmachine.version() {
        3 => 6,
        _ => 10,
    };

    set_relative(zmachine, offset, object, child)
}

/// Sets the first sibling of an object.
///
/// This function only updates the `object` table entry and does *not* update the new `sibling`
/// or otherise update the object tree.
///
/// # Arguments
/// * `zmachine` - Reference to the zmachine
/// * `object` - Object number
/// * `sibling` - New sibling object number
///
/// # Returns
/// Empty [Result] or a [RuntimeError]
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
