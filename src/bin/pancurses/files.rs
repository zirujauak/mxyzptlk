/// Helper functions for file I/O
use std::path::Path;

use log::error;
use zm::{
    error::{ErrorCode, RuntimeError},
    recoverable_error,
};

/// Convert a string to a vector of u16 values
///
/// # Arguments
/// * `s` - String
///
/// # Returns
/// Vector of u16 values
fn string_to_vec_u16(s: String) -> Vec<u16> {
    s.chars().map(|c| c as u16).collect()
}

/// Find the first available filename.
///
/// File naming is `base`-`##`.`suffix`, starting at 01.  The first such filename that
/// doesn't exist in the current working directory is returned.
///
/// # Arguments
/// * `base` - base filename
/// * `suffix` - file type extension
///
/// # Returns
/// [Result] containing a filename as a vector of u16 character values or a [RuntimeError]
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

/// Find the last existing filename.
///
/// File naming is `base`-`##`.`suffix`, starting at 01.  The last such filename that
/// exists in the current working directory is returned.
///
/// # Arguments
/// * `base` - base filename
/// * `suffix` - file type extension
///
/// # Returns
/// [Result] containing a filename as a vector of u16 character values or a [RuntimeError]
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

/// Checks the existence of a file
///
/// # Arguments
/// * `name` - Filename
///
/// # Returns
/// `true` if the file exists, `false` if not
fn check_filename(name: &str) -> bool {
    match Path::new(name).try_exists() {
        Ok(b) => b,
        Err(e) => {
            error!(target: "app::state", "Error checking existence of {}: {}", name, e);
            false
        }
    }
}

/// Looks for a configuration file
///
/// The current working directory is checked first, then ~/.mxyzptlk/
///
/// # Arguments
/// * `name` - Filename
///
/// # Returns
/// [Option] with the path to the file, if found, else [None]
pub fn config_file(name: &str) -> Option<String> {
    if check_filename(name) {
        // Check the CWD first
        Some(name.to_string())
    } else if let Some(home) = dirs::home_dir() {
        // And then check ~/.mxyzptlk/{name} if not found
        let filename = format!("{}/.mxyzptlk/{}", home.to_str().unwrap(), name);
        if check_filename(&filename) {
            Some(filename)
        } else {
            None
        }
    } else {
        None
    }
}

/// Checks if a file exists
///
/// # Arguments
/// * `filename` - File name
///
/// # Returns
/// [Option] containing the filename if found, else [None]
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
            error!(target: "app::state", "Error checking existence of {}: {}", filename, e);
            None
        }
    }
}

/// Searches for a file by name and one or more extensions
///
/// # Arguments
/// * `base` - File name
/// * `extensions` - Vector of one or more extensions
///
/// # Returns
/// [Option] containing the first filename found, else [None]
pub fn find_existing(base: &str, extensions: &[&str]) -> Option<String> {
    for ext in extensions {
        let filename = format!("{}.{}", base, ext);
        if let Some(filename) = check_existing(&filename) {
            return Some(filename);
        }
    }

    None
}

#[cfg(test)]
mod test {
    use std::fs::{self, File};

    use crate::files::{
        check_existing, check_filename, find_existing, first_available, last_existing,
    };

    use super::string_to_vec_u16;

    #[test]
    fn test_string_to_vec_u16() {
        let s = "Some string value".to_string();
        let v = string_to_vec_u16(s);

        assert_eq!(
            v,
            [
                b'S', b'o', b'm', b'e', b' ', b's', b't', b'r', b'i', b'n', b'g', b' ', b'v', b'a',
                b'l', b'u', b'e'
            ]
            .map(|x| x as u16)
        )
    }

    #[test]
    fn test_first_available() {
        assert!(File::create("first-available-01.tst").is_ok());
        assert!(!File::create("first-available-02.tst").is_err_and(|_| {
            let _ = fs::remove_file("first-available-01.tst");
            true
        }));
        assert!(!File::create("first-available-03.yyy").is_err_and(|_| {
            let _ = fs::remove_file("first-available-01.tst");
            let _ = fs::remove_file("first-available-02.tst");
            true
        }));
        assert!(!File::create("first-available-04.tst").is_err_and(|_| {
            let _ = fs::remove_file("first-available-01.tst");
            let _ = fs::remove_file("first-available-02.tst");
            let _ = fs::remove_file("first-available-03.yyy");
            true
        }));

        let first = first_available("first-available", "tst");
        let _ = fs::remove_file("first-available-01.tst");
        let _ = fs::remove_file("first-available-02.tst");
        let _ = fs::remove_file("first-available-03.yyy");
        let _ = fs::remove_file("first-available-04.tst");

        assert!(first.is_ok());
        assert_eq!(
            first.unwrap(),
            [
                b'f', b'i', b'r', b's', b't', b'-', b'a', b'v', b'a', b'i', b'l', b'a', b'b', b'l',
                b'e', b'-', b'0', b'3', b'.', b't', b's', b't'
            ]
            .map(|x| x as u16)
        )
    }

    #[test]
    fn test_last_existing() {
        assert!(File::create("last-existing-01.tst").is_ok());
        assert!(!File::create("last-existing-02.tst").is_err_and(|_| {
            let _ = fs::remove_file("last-existing-01.tst");
            true
        }));
        assert!(!File::create("last-existing-03.yyy").is_err_and(|_| {
            let _ = fs::remove_file("last-existing-01.tst");
            let _ = fs::remove_file("last-existing-02.tst");
            true
        }));
        assert!(!File::create("last-existing-04.tst").is_err_and(|_| {
            let _ = fs::remove_file("last-existing-01.tst");
            let _ = fs::remove_file("last-existing-02.tst");
            let _ = fs::remove_file("last-existing-03.yyy");
            true
        }));

        let last = last_existing("last-existing", "tst");
        let _ = fs::remove_file("last-existing-01.tst");
        let _ = fs::remove_file("last-existing-02.tst");
        let _ = fs::remove_file("last-existing-03.yyy");
        let _ = fs::remove_file("last-existing-04.tst");

        assert!(last.is_ok_and(|x| x
            == [
                b'l', b'a', b's', b't', b'-', b'e', b'x', b'i', b's', b't', b'i', b'n', b'g', b'-',
                b'0', b'2', b'.', b't', b's', b't'
            ]
            .map(|x| x as u16)));
    }

    #[test]
    fn test_check_filename() {
        assert!(!check_filename("check-filename.tst"));
        assert!(File::create("check-filename.tst").is_ok());
        let existing = check_filename("check-filename.tst");
        let _ = fs::remove_file("check-filename.tst");
        assert!(existing);
    }

    #[test]
    fn test_check_existing() {
        assert!(check_existing("check-existing.tst").is_none());
        assert!(File::create("check-existing.tst").is_ok());
        let existing = check_existing("check-existing.tst");
        let _ = fs::remove_file("check-existing.tst");
        assert!(existing.is_some_and(|x| x == "check-existing.tst"));
    }

    #[test]
    fn test_find_existing() {
        assert!(find_existing("find-existing", &["test", "tst"]).is_none());
        assert!(File::create("find-existing.tst").is_ok());
        assert!(find_existing("find-existing", &["xxx", "yyy"]).is_none());
        let existing = find_existing("find-existing", &["tst", "test"]);
        let _ = fs::remove_file("find-existing.tst");
        assert!(existing.is_some_and(|x| x == "find-existing.tst"));
    }
}
