use std::io::Write;

use crate::recoverable_error;

use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use tempfile::NamedTempFile;

use crate::error::{ErrorCode, RuntimeError};

use super::Player;

pub struct RodioPlayer {
    _output_stream: Option<OutputStream>,
    _output_stream_handle: Option<OutputStreamHandle>,
    current_effect: u32,
    sink: Option<Sink>,
    volume_factor: f32,
}

impl Player for RodioPlayer {
    fn type_name(&self) -> &str {
        "RodioPlayer"
    }

    fn is_playing(&mut self) -> bool {
        if let Some(sink) = self.get_sink().as_mut() {
            !sink.empty()
        } else {
            false
        }
    }

    fn play_sound(&mut self, sound: &[u8], volume: u8, repeats: u8) -> Result<(), RuntimeError> {
        match NamedTempFile::new() {
            Ok(mut write) => {
                match write.reopen() {
                    Ok(read) => match write.write_all(sound) {
                        Ok(_) => {
                            match Decoder::new(read) {
                                Ok(source) => {
                                    match self.get_sink() {
                                        Some(sink) => {
                                            sink.set_volume(self.normalize_volume(volume));
                                            // V5
                                            if repeats == 0 {
                                                sink.append(source.repeat_infinite())
                                            } else {
                                                for _ in 0..repeats {
                                                    let source = match write.reopen() {
                                                    Ok(f) => match Decoder::new(f) {
                                                        Ok(source) => source,
                                                        Err(e) => return recoverable_error!(ErrorCode::SoundPlayback, "Error creating source for sound: {}", e),
                                                    },
                                                    Err(e) => return recoverable_error!(ErrorCode::SoundPlayback, "Error reopening tempfile for sound: {}", e),
                                                };
                                                    sink.append(source);
                                                }
                                            }

                                            sink.play();
                                        }
                                        None => error!(target: "app::sound", "rodio: No sink"),
                                    }
                                }

                                Err(e) => {
                                    error!(target: "app::sound", "rodio: Error decoding sound: {}", e)
                                }
                            }
                        }
                        Err(e) => {
                            error!(target: "app::sound", "rodio: Error writing tempfile: {}", e)
                        }
                    },
                    Err(e) => error!(target: "app::sound", "rodio: Error writing tempfile: {}", e),
                }
            }
            Err(e) => error!(target: "app::sound", "rodio: Error opening tempfile: {}", e),
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
            sink.set_volume(self.normalize_volume(volume));
        }
    }
}

pub fn new_player(volume_factor: f32) -> Result<Box<dyn Player>, RuntimeError> {
    match RodioPlayer::new(volume_factor) {
        Ok(r) => Ok(Box::new(r)),
        Err(e) => Err(e),
    }
}

impl RodioPlayer {
    pub fn new(volume_factor: f32) -> Result<RodioPlayer, RuntimeError> {
        match OutputStream::try_default() {
            Ok((output_stream, output_stream_handle)) => {
                match Sink::try_new(&output_stream_handle) {
                    Ok(sink) => Ok(RodioPlayer {
                        _output_stream: Some(output_stream),
                        _output_stream_handle: Some(output_stream_handle),
                        current_effect: 0,
                        sink: Some(sink),
                        volume_factor,
                    }),
                    Err(e) => {
                        error!(target: "app::sound", "rodio: Error initializing sink: {}", e);
                        recoverable_error!(
                            ErrorCode::SoundPlayback,
                            "Error initializing sink: {}",
                            e
                        )
                    }
                }
            }
            Err(e) => {
                error!(target: "app::sound", "rodio: Error opening sound output stream: {}", e);
                recoverable_error!(
                    ErrorCode::SoundPlayback,
                    "Error initializing output stream: {}",
                    e
                )
            }
        }
    }

    fn get_sink(&self) -> Option<&Sink> {
        self.sink.as_ref()
    }

    fn normalize_volume(&self, volume: u8) -> f32 {
        // Volume should range 1 - 8, with -1 being "very load"
        match volume {
            // Louder than 8 by 25%
            0xFF => (8.0 / self.volume_factor) * 1.25,
            // range from 0.125 - 1.0 seems to work
            (1..=8) => volume as f32 / self.volume_factor,
            // assume middle of range
            _ => 4.5 / self.volume_factor,
        }
    }
}
