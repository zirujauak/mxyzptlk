use std::path::Path;

use crate::error::{ErrorCode, RuntimeError};

fn string_to_vec_u16(s: String) -> Vec<u16> {
    s.chars().map(|c| c as u16).collect()
}

pub fn first_available(base: &str, suffix: &str) -> Result<Vec<u16>, RuntimeError> {
    let mut n = 1;
    loop {
        let filename = format!("{}-{:02}.{}", base, n, suffix);
        match Path::new(&filename).try_exists() {
            Ok(b) => {
                if !b {
                    return Ok(string_to_vec_u16(filename));
                }
            }
            Err(e) => return Err(RuntimeError::new(ErrorCode::System, format!("{}", e))),
        }

        n += 1;
    }
}

pub fn last_existing(base: &str, suffix: &str) -> Result<Vec<u16>, RuntimeError> {
    let mut n = 1;
    loop {
        let filename = format!("{}-{:02}.{}", base, n, suffix);
        match Path::new(&filename).try_exists() {
            Ok(b) => {
                if !b {
                    if n > 1 {
                        return Ok(string_to_vec_u16(format!(
                            "{}-{:02}.{}",
                            base,
                            n - 1,
                            suffix
                        )));
                    } else {
                        return Ok(string_to_vec_u16(format!("{}.{}", base, suffix)));
                    }
                }
            }
            Err(e) => return Err(RuntimeError::new(ErrorCode::System, format!("{}", e))),
        }

        n += 1;
    }
}

pub fn check_existing(base: &str, extensions: &[&str]) -> Option<String> {
    for ext in extensions {
        let filename = format!("{}.{}", base, ext);
        let p = Path::new(&filename);
        match p.try_exists() {
            Ok(b) => if b { return Some(filename) },
            Err(e) => {
                info!(target: "app::trace", "Error checking existence of {}: {}", filename, e)
            }
        }
    }

    None
}