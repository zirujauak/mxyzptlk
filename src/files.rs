use std::path::Path;

use crate::{
    error::{ErrorCode, RuntimeError},
    recoverable_error,
};

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
            Err(e) => return recoverable_error!(ErrorCode::FileError, "{}", e),
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
            Err(e) => return recoverable_error!(ErrorCode::FileError, "{}", e),
        }

        n += 1;
    }
}

fn check_config(name: &str) -> bool {
    match Path::new(name).try_exists() {
        Ok(b) => b,
        Err(e) => {
            info!(target: "app::trace", "Error checking existence of {}: {}", name, e);
            false
        }
    }
}

pub fn config_file(name: &str) -> Option<String> {
    // Check ~/.mxyzptlk/{name} first
    if let Some(home) = dirs::home_dir() {
        let filename = format!("{}/.mxyzptlk/{}", home.to_str().unwrap(), name);
        if check_config(&filename) {
            return Some(filename);
        }
    }

    // If not there, check CWD
    if check_config(name) {
        Some(name.to_string())
    } else {
        None
    }
}

pub fn check_existing(filename: &str) -> Option<String> {
    match Path::new(&filename).try_exists() {
        Ok(b) => {
            if b {
                Some(filename.to_string())
            } else {
                None
            }
        }
        Err(e) => {
            info!(target: "app::trace", "Error checking existence of {}: {}", filename, e);
            None
        }
    }
}

pub fn find_existing(base: &str, extensions: &[&str]) -> Option<String> {
    for ext in extensions {
        let filename = format!("{}.{}", base, ext);
        if let Some(filename) = check_existing(&filename) {
            return Some(filename);
        }
    }

    None
}
