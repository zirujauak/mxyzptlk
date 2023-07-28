use std::path::Path;

use crate::{
    error::{ErrorCode, RuntimeError},
    runtime_error,
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
            Err(e) => return runtime_error!(ErrorCode::System, "{}", e),
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
            Err(e) => return runtime_error!(ErrorCode::System, "{}", e),
        }

        n += 1;
    }
}

pub fn config_file(name: &str) -> Option<String> {
    if let Some(home) = dirs::home_dir() {
        let filename = format!("{}/.mxyzptlk/{}", home.to_str().unwrap(), name);
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
