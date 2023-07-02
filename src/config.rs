use std::fs::File;
use serde_yaml::{self, Value};

use crate::error::{ErrorCode, RuntimeError};

#[derive(Debug)]
pub struct Config {
    terminal: String,
    foreground: u8,
    background: u8,
    logging: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config { terminal: "pancurses".to_string(), foreground: 9, background: 2, logging: false}
    }
}
impl Config {
    pub fn new(terminal: String, foreground: u8, background: u8, logging: bool) -> Self {
        Config { terminal, foreground, background, logging}
    }

    pub fn from_file(file: File) -> Result<Config, RuntimeError> {
        match serde_yaml::from_reader::<File, Value>(file) {
            Ok(data) => {
                let terminal = match data["terminal"].as_str() {
                    Some(t) => t.to_string(),
                    None => "pancurses".to_string(),
                };
                let foreground = match data["foreground"].as_u64() {
                    Some(v) => v as u8,
                    None => 9
                };
                let background = match data["background"].as_u64() {
                    Some(v) => v as u8,
                    None => 2
                };
                let logging = match data["logging"].as_str() {
                    Some(t) => t == "enabled",
                    None => false
                };
                
                Ok(Config::new(
                    terminal,
                    foreground,
                    background,
                    logging
                ))
            },
            Err(e) => {
                Err(RuntimeError::new(ErrorCode::System, format!("{}", e)))
            },
        }
    }

    pub fn terminal(&self) -> &str {
        &self.terminal
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
