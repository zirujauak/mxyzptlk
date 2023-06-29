use std::io::Write;

use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use tempfile::NamedTempFile;

use crate::error::{RuntimeError, ErrorCode};

use super::Player;

pub struct RodioPlayer {
    _output_stream: Option<OutputStream>,
    _output_stream_handle: Option<OutputStreamHandle>,
    current_effect: u32,
    sink: Option<Sink>,
}

impl Player for RodioPlayer {
    fn is_playing(&mut self) -> bool {
        if let Some(sink) = self.get_sink().as_mut() {
            !sink.empty()
        } else {
            false
        }
    }

    fn play_sound(&mut self, sound: &Vec<u8>, volume: u8, repeats: u8) -> Result<(), RuntimeError> {
        match NamedTempFile::new() {
            Ok(mut write) => match write.reopen() {
                Ok(read) => match write.write_all(&sound) {
                    Ok(_) => match Decoder::new(read) {
                        Ok(source) => match self.get_sink() {
                            Some(sink) => {
                                sink.set_volume(volume as f32 / 128.0);
                                // V5
                                if repeats == 0 {
                                    sink.append(source.repeat_infinite())
                                }
                                for _ in 0..repeats {
                                    sink.append(Decoder::new(write.reopen().unwrap()).unwrap());
                                }

                                sink.play();
                            }
                            None => error!(target: "app::trace", "No sink"),
                        },
                        Err(e) => error!(target: "app::trace", "Error decoding sound: {}", e),
                    },
                    Err(e) => error!(target: "app::trace", "Error writing tempfile: {}", e),
                },
                Err(e) => error!(target: "app::trace", "Error writing tempfile: {}", e),
            },
            Err(e) => error!(target: "app::trace", "Error opening tempfile: {}", e),
        }
        Ok(())
    }

    fn stop_sound(&mut self) {
        if let Some(sink) = self.get_sink() {
            sink.stop()
        }

        self.current_effect = 0;
    }

    fn change_volume(&mut self, volume: u8) {
        if let Some(sink) = self.get_sink() {
            let v = volume as f32 / 128.0;
            sink.set_volume(v);
        }
    }
}

impl RodioPlayer {
    pub fn new() -> Result<RodioPlayer, RuntimeError> {
        match OutputStream::try_default() {
            Ok((output_stream, output_stream_handle)) => {
                match Sink::try_new(&output_stream_handle) {
                    Ok(sink) => Ok(RodioPlayer {
                        _output_stream: Some(output_stream),
                        _output_stream_handle: Some(output_stream_handle),
                        current_effect: 0,
                        sink: Some(sink),
                    }),
                    Err(e) => {
                        error!(target: "app::trace", "Error initializing sink: {}", e);
                        Err(RuntimeError::new(ErrorCode::System, format!("Error initializing sink: {}", e)))
                    },
                }
            }
            Err(e) => {
                error!(target: "app::trace", "Error opening sound output stream: {}", e);
                Err(RuntimeError::new(ErrorCode::System, format!("Error initializing output stream: {}", e)))
            }
        }
    }

    fn get_sink(&self) -> Option<&Sink> {
        self.sink.as_ref()
    }
}
