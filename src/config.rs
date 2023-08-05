use serde_yaml::{self, Value};
use std::fs::File;

use crate::{
    error::{ErrorCode, RuntimeError},
    recoverable_error,
    zmachine::ErrorHandling,
};

#[derive(Debug)]
pub struct Config {
    foreground: u8,
    background: u8,
    logging: bool,
    error_handling: ErrorHandling,
    volume_factor: f32,
}

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
            foreground: 9,
            background: 2,
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

    pub fn foreground(&self) -> u8 {
        self.foreground
    }

    pub fn background(&self) -> u8 {
        self.background
    }

    pub fn logging(&self) -> bool {
        self.logging
    }

    pub fn error_handling(&self) -> ErrorHandling {
        self.error_handling
    }
    pub fn volume_factor(&self) -> f32 {
        self.volume_factor
    }
}
