//! Sample Player implemented using [rodio](https://docs.rs/rodio/latest/rodio/)
use std::io::Write;

use crate::recoverable_error;

use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use tempfile::NamedTempFile;

use crate::error::{ErrorCode, RuntimeError};

use super::Player;

/// Player
pub struct RodioPlayer {
    /// Rodio output stream
    _output_stream: Option<OutputStream>,
    /// Rodio output stream handle
    _output_stream_handle: Option<OutputStreamHandle>,
    /// Currently playing effect
    current_effect: u32,
    /// Player sink
    sink: Option<Sink>,
    /// Volume normalization factor
    volume_factor: f32,
}

impl Player for RodioPlayer {
    fn type_name(&self) -> &str {
        "RodioPlayer"
    }

    fn is_playing(&mut self) -> bool {
        if let Some(sink) = self.sink.as_mut() {
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
                                    let vol = self.normalize_volume(volume);
                                    match self.sink.as_mut() {
                                        Some(sink) => {
                                            sink.set_volume(vol);
                                            // V5
                                            if repeats == 0 {
                                                sink.append(source.repeat_infinite());
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
        if let Some(sink) = self.sink.as_mut() {
            sink.stop()
        }

        self.current_effect = 0;
    }

    fn change_volume(&mut self, volume: u8) {
        let vol = self.normalize_volume(volume);
        if let Some(sink) = self.sink.as_mut() {
            sink.set_volume(vol);
        }
    }
}

/// Utility function to construct a new Player
///
/// # Arguments
/// * `volume_factor` - Volume normalization factor
///
/// # Returns
/// [Result] with a [Box] containing the [Player] implementation or a [RuntimeError]
pub fn new_player(volume_factor: f32) -> Result<Box<dyn Player>, RuntimeError> {
    match RodioPlayer::new(volume_factor) {
        Ok(r) => Ok(Box::new(r)),
        Err(e) => Err(e),
    }
}

impl RodioPlayer {
    /// Constructor
    ///
    /// # Arguments
    /// * `volume_factor` - volume nomralization factor
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

    /// Normalize playback volume
    ///
    /// # Arguments
    /// * `volume` - Playback volume -1 or a value from 1 to 8, with 1 being soft and 8 being loud, and -1 being very loud.
    ///
    /// # Returns
    /// [Sink] volume value, normalized for platform.
    fn normalize_volume(&self, volume: u8) -> f32 {
        // Volume should range 1 - 8, with -1 being "very loud"
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
