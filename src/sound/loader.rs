use std::{
    fs::File,
    io::{Read, Write},
};

use sndfile::{
    Endian, MajorFormat, OpenOptions, ReadOptions, SndFileIO, SubtypeFormat, WriteOptions,
};
use tempfile::NamedTempFile;

use crate::error::{ErrorCode, RuntimeError};

fn tempfile(data: Option<&Vec<u8>>) -> Result<(NamedTempFile, File), RuntimeError> {
    match NamedTempFile::new() {
        Ok(mut tempfile) => match tempfile.reopen() {
            Ok(file) => {
                if let Some(d) = data {
                    match tempfile.write_all(d) {
                        Ok(_) => Ok((tempfile, file)),
                        Err(e) => {
                            runtime_error!(ErrorCode::System, "Error writing to tempfile: {}", e)
                        }
                    }
                } else {
                    Ok((tempfile, file))
                }
            }
            Err(e) => runtime_error!(ErrorCode::System, "Error opening tempfile: {}", e),
        },
        Err(e) => runtime_error!(ErrorCode::System, "Error creating tempfile: {}", e),
    }
}

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
                                    Err(e) => runtime_error!(
                                        ErrorCode::System,
                                        "Error reading converted sound data: {}",
                                        e
                                    ),
                                }
                            }
                            Err(_) => runtime_error!(
                                ErrorCode::System,
                                "sndfile: Error writing convered sound data:"
                            ),
                        },
                        Err(_) => runtime_error!(
                            ErrorCode::System,
                            "sndfile: Error reading source sound data:"
                        ),
                    }
                }
                Err(e) => {
                    runtime_error!(ErrorCode::System, "Error opening output tempfile: {:?}", e)
                }
            }
        }
        Err(e) => runtime_error!(ErrorCode::System, "Error loading AIFF file: {:?}", e),
    }
}
