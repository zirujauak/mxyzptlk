//! Runtime configuration
use serde_yaml::{self, Value};
use std::fs::File;

use crate::{
    error::{ErrorCode, RuntimeError},
    recoverable_error,
    zmachine::ErrorHandling,
};

#[derive(Debug)]
/// Runtime configuration data
pub struct Config {
    /// Default foreground color
    foreground: u8,
    /// Default background color
    background: u8,
    /// Is logging enabled?
    logging: bool,
    /// Recoverable error handling
    error_handling: ErrorHandling,
    /// Platform-specific volume normalization factor
    volume_factor: f32,
}

/// Get the default volume normalization factor.
///
/// This value may be overriden by the `volume_factor` configuration key.
///
/// # Returns
/// Default volume normalization factor for the current operating system
fn default_volume_factor() -> f32 {
    if cfg!(target_os = "linux") {
        8.0
    } else if cfg!(target_os = "windows") {
        12.0
    } else {
        128.0
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            foreground: 9, // white text
            background: 2, // on a black background
            logging: false,
            error_handling: ErrorHandling::ContinueWarnOnce,
            volume_factor: default_volume_factor(),
        }
    }
}

impl TryFrom<File> for Config {
    type Error = RuntimeError;

    fn try_from(value: File) -> Result<Self, Self::Error> {
        match serde_yaml::from_reader::<File, Value>(value) {
            Ok(data) => {
                let foreground = match data["foreground"].as_u64() {
                    Some(v) => v as u8,
                    None => 9,
                };
                let background = match data["background"].as_u64() {
                    Some(v) => v as u8,
                    None => 2,
                };
                let logging = match data["logging"].as_str() {
                    Some(t) => t == "enabled",
                    None => false,
                };
                let error_handling = match data["error_handling"].as_str() {
                    Some(t) => match t {
                        "continue_warn_always" => ErrorHandling::ContinueWarnAlways,
                        "continue_warn_once" => ErrorHandling::ContinueWarnOnce,
                        "ignore" => ErrorHandling::Ignore,
                        "abort" => ErrorHandling::Abort,
                        _ => ErrorHandling::ContinueWarnOnce,
                    },
                    None => ErrorHandling::ContinueWarnOnce,
                };
                let volume_factor = match data["volume_factor"].as_f64() {
                    Some(t) => t as f32,
                    None => default_volume_factor(),
                };
                Ok(Config::new(
                    foreground,
                    background,
                    logging,
                    error_handling,
                    volume_factor,
                ))
            }
            Err(e) => recoverable_error!(ErrorCode::ConfigError, "{}", e),
        }
    }
}

impl Config {
    /// Constructo
    ///
    /// # Arugments
    /// * `foreground` - Default foreground (text) color
    /// * `background` - Default background color
    /// * `logging` - Logging enabled flag
    /// * `error_handling` - Recoverable error handling mode
    /// * `volume_factor` - Volume normalization factor
    pub fn new(
        foreground: u8,
        background: u8,
        logging: bool,
        error_handling: ErrorHandling,
        volume_factor: f32,
    ) -> Self {
        Config {
            foreground,
            background,
            logging,
            error_handling,
            volume_factor,
        }
    }

    /// Get the default foreground (text) color
    ///
    /// # Returns
    /// Default foreground color
    pub fn foreground(&self) -> u8 {
        self.foreground
    }

    /// Get the default background color
    ///
    /// # Returns
    /// Default background color
    pub fn background(&self) -> u8 {
        self.background
    }

    /// Get the logging flag
    ///
    /// # Returns
    /// Logging flag
    pub fn logging(&self) -> bool {
        self.logging
    }

    /// Get the recoverable error handling mode
    ///
    /// # Returns
    /// Error handling mode
    pub fn error_handling(&self) -> ErrorHandling {
        self.error_handling
    }

    /// Get the volume normalization factor
    ///
    /// # Returns
    /// Volume normalization factor
    pub fn volume_factor(&self) -> f32 {
        self.volume_factor
    }
}
