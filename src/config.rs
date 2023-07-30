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
}

impl Default for Config {
    fn default() -> Self {
        Config {
            foreground: 9,
            background: 2,
            logging: false,
            error_handling: ErrorHandling::ContinueWarnOnce,
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
                Ok(Config::new(foreground, background, logging, error_handling))
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
    ) -> Self {
        Config {
            foreground,
            background,
            logging,
            error_handling,
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
}
