use serde_yaml::{self, Value};
use std::fs::File;

use crate::{error::{ErrorCode, RuntimeError}, runtime_error};

#[derive(Debug)]
pub struct Config {
    foreground: u8,
    background: u8,
    logging: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            foreground: 9,
            background: 2,
            logging: false,
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

                Ok(Config::new(foreground, background, logging))
            }
            Err(e) => runtime_error!(ErrorCode::System, "{}", e),
        }
    }
}

impl Config {
    pub fn new(foreground: u8, background: u8, logging: bool) -> Self {
        Config {
            foreground,
            background,
            logging,
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
}
