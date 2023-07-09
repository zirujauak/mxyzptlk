use core::fmt;
use std::collections::HashMap;

#[cfg(not(test))]
mod rodio_player;

#[cfg(test)]
mod test_player;

#[cfg(any(feature = "sndfile", test))]
mod loader;

#[cfg(not(test))]
use crate::sound::rodio_player::*;

#[cfg(test)]
use crate::sound::test_player::*;

use crate::{
    error::RuntimeError,
    iff::blorb::{aiff::AIFF, oggv::OGGV, Blorb},
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
            "Sound {}, repeats: {:?}, {} bytes",
            self.number,
            self.repeats,
            self.data.len()
        )
    }
}

impl From<(u32, &OGGV, Option<&u32>)> for Sound {
    fn from((number, oggv, repeats): (u32, &OGGV, Option<&u32>)) -> Self {
        Sound::new(number, oggv.data(), repeats)
    }
}

#[cfg(feature = "sndfile")]
impl From<(u32, &AIFF, Option<&u32>)> for Sound {
    fn from((number, aiff, repeats): (u32, &AIFF, Option<&u32>)) -> Self {
        match loader::convert_aiff(&Vec::from(aiff)) {
            Ok(sound) => Sound::new(number, &sound, repeats),
            Err(e) => {
                error!(target: "app::sound", "Error converting AIFF resource: {}", e);
                Sound::new(number, &[], repeats)
            }
        }
    }
}

#[cfg(not(feature = "sndfile"))]
impl From<(u32, &AIFF, Option<&u32>)> for Sound {
    fn from((number, _aiff, repeats): (u32, &AIFF, Option<&u32>)) -> Self {
        Sound::new(number, &[], repeats)
    }
}

impl Sound {
    pub fn new(number: u32, data: &[u8], repeats: Option<&u32>) -> Sound {
        Sound {
            number,
            repeats: repeats.copied(),
            data: data.to_vec(),
        }
    }
}

pub trait Player {
    fn is_playing(&mut self) -> bool;
    fn play_sound(&mut self, sound: &[u8], volume: u8, repeats: u8) -> Result<(), RuntimeError>;
    fn stop_sound(&mut self);
    fn change_volume(&mut self, volume: u8);
}

pub struct Manager {
    player: Option<Box<dyn Player>>,
    sounds: HashMap<u32, Sound>,
    current_effect: u32,
}

impl From<Blorb> for HashMap<u32, Sound> {
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
                    if let Some(oggv) = value.oggv().get(&(index.start() as usize)) {
                        let s = Sound::from((index.number(), oggv, loops.get(&index.number())));
                        // let s = Sound::from_oggv(index.number(), oggv, loops.get(&index.number()));
                        info!(target: "app::sound", "Sound: {}", s);
                        sounds.insert(index.number(), s);
                    } else if let Some(aiff) = value.aiff().get(&(index.start() as usize)) {
                        let s = Sound::from((index.number(), aiff, loops.get(&(index.number()))));
                        info!(target: "app::sound", "Sound: {}", s);
                        if !s.data.is_empty() {
                            sounds.insert(index.number(), s);
                        }
                    }
                }
            }
        }

        sounds
    }
}

impl Manager {
    #[cfg(test)]
    pub fn mock() -> Result<Manager, RuntimeError> {
        let mut sounds = HashMap::new();
        sounds.insert(3, Sound::new(1, &[0; 128], None));
        sounds.insert(4, Sound::new(1, &[0; 256], Some(&5)));

        Ok(Manager {
            player: Some(new_player()?),
            sounds,
            current_effect: 0,
        })
    }

    pub fn new(blorb: Blorb) -> Result<Manager, RuntimeError> {
        Ok(Manager {
            player: Some(new_player()?),
            sounds: HashMap::from(blorb),
            current_effect: 0,
        })
    }

    pub fn current_effect(&self) -> u32 {
        self.current_effect
    }

    pub fn sound_count(&self) -> usize {
        self.sounds.len()
    }

    pub fn is_playing(&mut self) -> bool {
        if let Some(p) = self.player.as_mut() {
            p.is_playing()
        } else {
            false
        }
    }

    pub fn play_sound(
        &mut self,
        effect: u16,
        volume: u8,
        repeats: Option<u8>,
    ) -> Result<(), RuntimeError> {
        debug!(target: "app::sound", "Playing sound effect {}, at volume {}, with repeats {:?}", effect, volume, repeats);
        if let Some(p) = self.player.as_mut() {
            match self.sounds.get(&(effect as u32)) {
                Some(sound) => {
                    let r = if let Some(r) = repeats {
                        if r == 255 {
                            0
                        } else {
                            r
                        }
                    } else if let Some(r) = sound.repeats {
                        r as u8
                    } else {
                        1
                    };

                    self.current_effect = effect as u32;
                    p.play_sound(&sound.data, volume, r)
                }
                None => {
                    error!(target: "app::sound", "Sound effect {} not found", effect);
                    Ok(())
                }
            }
        } else {
            Ok(())
        }
    }

    pub fn stop_sound(&mut self) {
        debug!(target: "app::sound", "Stopping sound playback");
        if let Some(p) = self.player.as_mut() {
            p.stop_sound()
        }

        self.current_effect = 0;
    }

    pub fn change_volume(&mut self, volume: u8) {
        debug!(target: "app::sound", "Changing volume of playing sound to {}", volume);
        if let Some(p) = self.player.as_mut() {
            p.change_volume(volume)
        }
    }
}
