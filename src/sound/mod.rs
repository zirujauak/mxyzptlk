use core::fmt;
use std::collections::HashMap;

pub mod rodio_player;

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

pub trait Player {
    fn is_playing(&mut self) -> bool;
    fn play_sound(&mut self, sound: &Vec<u8>, volume: u8, repeats: u8) -> Result<(), RuntimeError>;
    fn stop_sound(&mut self);
    fn change_volume(&mut self, volume: u8);
}

pub struct Engine {
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
                    if let Some(oggv) = value.snds().get(&(index.start() as usize)) {
                        let s = Sound::from_oggv(index.number(), oggv, loops.get(&index.number()));
                        info!(target: "app::sound", "Sound: {}", s);
                        sounds.insert(index.number(), s);
                    }
                }
            }
        }

        sounds
    }
}

impl Engine {
    pub fn new(player: Box<dyn Player>, blorb: Blorb) -> Engine {
        Engine {
            player: Some(player),
            sounds: HashMap::from(blorb),
            current_effect: 0,
        }
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
        info!(target: "app::sound", "play_sound({}, {}, {:?})", effect, volume, repeats);
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

                    p.play_sound(&sound.data, volume, r)
                }
                None => {
                    error!(target: "app::trace", "Sound effect {} not found", effect);
                    Ok(())
                }
            }
        } else {
            Ok(())
        }
    }

    pub fn stop_sound(&mut self) {
        if let Some(p) = self.player.as_mut() {
            p.stop_sound()
        }

        self.current_effect = 0;
    }

    pub fn change_volume(&mut self, volume: u8) {
        if let Some(p) = self.player.as_mut() {
            p.change_volume(volume)
        }
    }
}
