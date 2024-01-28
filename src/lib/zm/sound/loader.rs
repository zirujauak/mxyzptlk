//! Utility function to convert sound sample format from AIFF to FLAC
use std::{
    fs::File,
    io::{Read, Write},
};

use sndfile::{
    Endian, MajorFormat, OpenOptions, ReadOptions, SndFileIO, SubtypeFormat, WriteOptions,
};
use tempfile::NamedTempFile;

use crate::error::{ErrorCode, RuntimeError};
use crate::recoverable_error;

/// Creates a temporary file used during sample conversion
///
/// # Arguments
/// * `data` - Raw sample data
///
/// # Returns
/// [Result] with a tuple of ([NamedTempFile], [File]) or a [RuntimeError]
fn tempfile(data: Option<&Vec<u8>>) -> Result<(NamedTempFile, File), RuntimeError> {
    match NamedTempFile::new() {
        Ok(mut tempfile) => match tempfile.reopen() {
            Ok(file) => {
                if let Some(d) = data {
                    match tempfile.write_all(d) {
                        Ok(_) => Ok((tempfile, file)),
                        Err(e) => {
                            recoverable_error!(
                                ErrorCode::SoundConversion,
                                "Error writing to tempfile: {}",
                                e
                            )
                        }
                    }
                } else {
                    Ok((tempfile, file))
                }
            }
            Err(e) => {
                recoverable_error!(ErrorCode::SoundConversion, "Error opening tempfile: {}", e)
            }
        },
        Err(e) => recoverable_error!(ErrorCode::SoundConversion, "Error creating tempfile: {}", e),
    }
}

/// Converts a sound sample from AIFF to FLAC
///
/// # Arguments
/// * `data` - Raw AIFF sample data
///
/// # Returns
/// [Result] with raw FLAC sample data or a [RuntimeError]
pub fn convert_aiff(data: &Vec<u8>) -> Result<Vec<u8>, RuntimeError> {
    let (_, source) = tempfile(Some(data))?;
    match OpenOptions::ReadOnly(ReadOptions::Auto)
        .from_file(source)
        .as_mut()
    {
        Ok(snd) => {
            let (mut destfile, dest) = tempfile(None)?;
            match OpenOptions::WriteOnly(WriteOptions::new(
                MajorFormat::FLAC,
                SubtypeFormat::PCM_S8,
                Endian::File,
                snd.get_samplerate(),
                snd.get_channels(),
            ))
            .from_file(dest)
            .as_mut()
            {
                Ok(ws) => {
                    let v: Result<Vec<f32>, ()> = snd.read_all_to_vec();
                    match v {
                        Ok(data) => match ws.write_from_slice(&data) {
                            Ok(_) => {
                                let mut x: Vec<u8> = Vec::new();
                                match destfile.read_to_end(&mut x) {
                                    Ok(_) => Ok(x),
                                    Err(e) => recoverable_error!(
                                        ErrorCode::SoundConversion,
                                        "Error reading converted sound data: {}",
                                        e
                                    ),
                                }
                            }
                            Err(_) => recoverable_error!(
                                ErrorCode::SoundConversion,
                                "sndfile: Error writing convered sound data:"
                            ),
                        },
                        Err(_) => recoverable_error!(
                            ErrorCode::SoundConversion,
                            "sndfile: Error reading source sound data:"
                        ),
                    }
                }
                Err(e) => {
                    recoverable_error!(
                        ErrorCode::SoundConversion,
                        "Error opening output tempfile: {:?}",
                        e
                    )
                }
            }
        }
        Err(e) => recoverable_error!(
            ErrorCode::SoundConversion,
            "Error loading AIFF file: {:?}",
            e
        ),
    }
}
