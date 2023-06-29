use std::{
    fs::File,
    io::{Read, Write},
};

use sndfile::{
    Endian, MajorFormat, OpenOptions, ReadOptions, SubtypeFormat, WriteOptions, SndFileIO,
};
use tempfile::NamedTempFile;

use crate::error::{ErrorCode, RuntimeError};

fn tempfile(data: Option<&Vec<u8>>) -> Result<(NamedTempFile, File), RuntimeError> {
    match NamedTempFile::new() {
        Ok(mut tempfile) => match tempfile.reopen() {
            Ok(file) => {
                if let Some(d) = data {
                    match tempfile.write_all(&d) {
                        Ok(_) => Ok((tempfile, file)),
                        Err(e) => Err(RuntimeError::new(
                            ErrorCode::System,
                            format!("Error writing to tempfile: {}", e),
                        )),
                    }
                } else {
                    Ok((tempfile, file))
                }
            }
            Err(e) => Err(RuntimeError::new(
                ErrorCode::System,
                format!("Error opening tempfile: {}", e),
            )),
        },
        Err(e) => Err(RuntimeError::new(
            ErrorCode::System,
            format!("Error creating tempfile: {}", e),
        )),
    }
}

pub fn aiff_to_flac(data: &Vec<u8>) -> Result<Vec<u8>, RuntimeError> {
    let (_, source) = tempfile(Some(data))?;
    match OpenOptions::ReadOnly(ReadOptions::Auto)
        .from_file(source)
        .as_mut()
    {
        Ok(snd) => {
            let (mut destfile, dest) = tempfile(None)?;
            match OpenOptions::WriteOnly(WriteOptions::new(
                MajorFormat::FLAC,
                SubtypeFormat::PCM_16,
                Endian::File,
                snd.get_samplerate(),
                snd.get_channels(),
            ))
            .from_file(dest)
            .as_mut()
            {
                Ok(ws) => {
                    let data:Vec<f32> = snd.read_all_to_vec().expect("Error reading sound data");
                    ws.write_from_slice(&data).expect("Error writing converted sound data");
                    let mut x: Vec<u8> = Vec::new();
                    destfile
                        .read_to_end(&mut x)
                        .expect("Error reading from tempfile");
                    return Ok(x);
                }
                Err(e) => {
                    return Err(RuntimeError::new(
                        ErrorCode::System,
                        format!("Error opening output tempfile: {:?}", e),
                    ));
                }
            }
        }
        Err(e) => {
            Err(RuntimeError::new(
                ErrorCode::System,
                format!("Error loading AIFF file: {:?}", e),
            ))
        }
    }
}
