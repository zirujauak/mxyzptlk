use core::fmt;
use std::{collections::HashMap, io::Write};

use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use tempfile::NamedTempFile;

use crate::{
    error::RuntimeError,
    iff::blorb::{oggv::OGGV, Blorb},
};

pub struct Sound {
    number: u32,
    repeats: Option<u32>,
    data: Vec<u8>,
}

impl fmt::Display for Sound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Sound {}, repeats: {:?},  {} bytes",
            self.number,
            self.repeats,
            self.data.len()
        )
    }
}

impl Sound {
    pub fn from_oggv(number: u32, oggv: &OGGV, repeats: Option<&u32>) -> Sound {
        Sound {
            number,
            repeats: repeats.copied(),
            data: oggv.data().clone(),
        }
    }
}

pub struct Sounds {
    sounds: HashMap<u32, Sound>,
    _output_stream: Option<OutputStream>,
    _output_stream_handle: Option<OutputStreamHandle>,
    current_effect: u32,
    sink: Option<Sink>,
}

impl From<Blorb> for Sounds {
    fn from(value: Blorb) -> Self {
        let mut sounds = HashMap::new();
        let mut loops = HashMap::new();
        if let Some(l) = value.sloop() {
            for entry in l.entries() {
                loops.insert(entry.number(), entry.repeats());
            }
        }

        if let Some(ridx) = value.ridx() {
            for index in ridx.entries() {
                if index.usage().eq("Snd ") {
                    if let Some(oggv) = value.snds().get(&(index.start() as usize)) {
                        let s = Sound::from_oggv(index.number(), oggv, loops.get(&index.number()));
                        info!(target: "app::sound", "Sound: {}", s);
                        sounds.insert(index.number(), s);
                    }
                }
            }
        }

        Sounds::new(sounds)
    }
}

impl Sounds {
    pub fn new(sounds: HashMap<u32, Sound>) -> Sounds {
        if sounds.is_empty() {
            Sounds {
                sounds,
                _output_stream: None,
                _output_stream_handle: None,
                current_effect: 0,
                sink: None,
            }
        } else {
            match OutputStream::try_default() {
                Ok((output_stream, output_stream_handle)) => {
                    match Sink::try_new(&output_stream_handle) {
                        Ok(sink) => Sounds {
                            sounds,
                            _output_stream: Some(output_stream),
                            _output_stream_handle: Some(output_stream_handle),
                            current_effect: 0,
                            sink: Some(sink),
                        },
                        Err(e) => {
                            error!(target: "app::trace", "Error initializing sink: {}", e);
                            Sounds {
                                sounds: HashMap::new(),
                                _output_stream: None,
                                _output_stream_handle: None,
                                current_effect: 0,
                                sink: None,
                            }
                        }
                    }
                }
                Err(e) => {
                    error!(target: "app::trace", "Error opening sound output stream: {}", e);
                    Sounds {
                        sounds: HashMap::new(),
                        _output_stream: None,
                        _output_stream_handle: None,
                        current_effect: 0,
                        sink: None,
                    }
                }
            }
        }
    }

    // pub fn from_blorb(blorb: Blorb, version: u8) -> Sounds {
    //     let mut sounds = HashMap::new();
    //     let mut loops = HashMap::new();
    //     if version != 5 {
    //         if let Some(l) = blorb.sloop() {
    //             for entry in l.entries() {
    //                 loops.insert(entry.number(), entry.repeats());
    //             }
    //         }
    //     }

    //     if let Some(ridx) = blorb.ridx() {
    //         for index in ridx.entries() {
    //             if index.usage().eq("Snd ") {
    //                 if let Some(oggv) = blorb.snds().get(&(index.start() as usize)) {
    //                     let s = Sound::from_oggv(index.number(), oggv, loops.get(&index.number()));
    //                     info!(target: "app::sound", "Sound: {}", s);
    //                     sounds.insert(index.number(), s);
    //                 }
    //             }
    //         }
    //     }

    //     let (_output_stream, _output_stream_handle, sink) = match OutputStream::try_default() {
    //         Ok((output_stream, output_stream_handle)) => {
    //             match Sink::try_new(&output_stream_handle) {
    //                 Ok(sink) => (Some(output_stream), Some(output_stream_handle), Some(sink)),
    //                 Err(e) => {
    //                     error!(target: "app::trace", "Error initializing sink: {}", e);
    //                     (None, None, None)
    //                 }
    //             }
    //         }
    //         Err(e) => {
    //             error!(target: "app::trace", "Error opening sound output stream: {}", e);
    //             (None, None, None)
    //         }
    //     };

    //     Sounds {
    //         sounds,
    //         _output_stream,
    //         _output_stream_handle,
    //         current_effect: 0,
    //         sink: sink,
    //     }
    // }

    pub fn sounds(&self) -> &HashMap<u32, Sound> {
        &self.sounds
    }

    pub fn current_effect(&self) -> u16 {
        self.current_effect as u16
    }

    fn get_sound(&self, effect: u32) -> Option<&Sound> {
        self.sounds.get(&effect)
    }

    fn get_sink(&self) -> Option<&Sink> {
        self.sink.as_ref()
    }

    pub fn is_playing(&mut self) -> bool {
        if let Some(sink) = self.get_sink().as_mut() {
            !sink.empty()
        } else {
            false
        }
    }

    pub fn play_sound(&mut self, effect: u16, volume: u8, repeats: Option<u8>) -> Result<(), RuntimeError> {
        info!(target: "app::sound", "play_sound({}, {}, {:?})", effect, volume, repeats);
        match NamedTempFile::new() {
            Ok(mut write) => match write.reopen() {
                Ok(read) => match self.get_sound(effect as u32) {
                    Some(s) => match write.write_all(&s.data) {
                        Ok(_) => match Decoder::new(read) {
                            Ok(source) => match self.get_sink() {
                                Some(sink) => {
                                    sink.set_volume(volume as f32 / 128.0);
                                    // V5 
                                    if let Some(r) = repeats {
                                        if r == 255 {
                                            sink.append(source.repeat_infinite());
                                        } else {
                                            for _ in 0..r {
                                                sink.append(
                                                    Decoder::new(write.reopen().unwrap()).unwrap(),
                                                );
                                            }
                                        }
                                    } else if let Some(r) = s.repeats {
                                        if r == 0 {
                                            sink.append(source.repeat_infinite());
                                        } else {
                                            for _ in 0..r {
                                                sink.append(
                                                    Decoder::new(write.reopen().unwrap()).unwrap(),
                                                );
                                            }
                                        }
                                    }

                                    sink.play();
                                    info!(target: "app::sound", "Sink len/empty: {}/{}", sink.len(), sink.empty());
                                    self.current_effect = effect as u32;
                                }
                                None => error!(target: "app::trace", "No sink"),
                            },
                            Err(e) => error!(target: "app::trace", "Error decoding sound: {}", e),
                        },
                        Err(e) => error!(target: "app::trace", "Error writing tempfile: {}", e),
                    },
                    None => error!(target: "app::trace", "Sound effect {} not found", effect),
                },
                Err(e) => error!(target: "app::trace", "Error opening tempfile: {}", e),
            },
            Err(e) => error!(target: "app::trace", "Error creating tempfile: {}", e),
        }
        Ok(())
    }

    pub fn stop_sound(&mut self) {
        if let Some(sink) = self.get_sink() {
            sink.stop()
        }

        self.current_effect = 0;
    }

    pub fn change_volume(&mut self, volume: u8) {
        if let Some(sink) = self.get_sink() {
            let v = volume as f32 / 128.0;
            sink.set_volume(v);
        }
    }
}
