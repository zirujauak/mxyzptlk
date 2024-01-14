//! [Sound effects](https://inform-fiction.org/zmachine/standards/z1point1/sect09.html)
use core::fmt;
use std::collections::HashMap;

// #[cfg(not(test))]
mod rodio_player;

#[cfg(test)]
mod test_player;

#[cfg(feature = "sndfile")]
mod loader;

#[cfg(not(test))]
use crate::sound::rodio_player::*;

#[cfg(test)]
use crate::sound::test_player::*;

use crate::iff::Chunk;
use crate::{blorb::Blorb, error::RuntimeError};

#[derive(Debug)]
/// Sound sample
pub struct Sample {
    /// Effect number
    number: u32,
    /// Default repeat count [Option]
    repeats: Option<u32>,
    /// Sample data
    data: Vec<u8>,
}

impl fmt::Display for Sample {
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

#[cfg(not(feature = "sndfile"))]
impl From<(u32, &Chunk, Option<&u32>)> for Sample {
    fn from((number, chunk, repeats): (u32, &Chunk, Option<&u32>)) -> Self {
        if chunk.id() == "OGGV" {
            Sample::new(number, chunk.data(), repeats)
        } else {
            Sample::new(number, &[], repeats)
        }
    }
}

#[cfg(feature = "sndfile")]
impl From<(u32, &Chunk, Option<&u32>)> for Sample {
    fn from((number, chunk, repeats): (u32, &Chunk, Option<&u32>)) -> Self {
        if chunk.id() == "OGGV" {
            Sample::new(number, chunk.data(), repeats)
        } else if chunk.id() == "FORM" && chunk.sub_id() == "AIFF" {
            debug!(target: "app::sound", "Converting sound {} from AIFF", number);
            match loader::convert_aiff(&Vec::from(chunk)) {
                Ok(s) => Sample::new(number, &s, repeats),
                Err(e) => {
                    error!(target: "app::sound", "Error converting AIFF resource: {}", e);
                    Sample::new(number, &[], repeats)
                }
            }
        } else {
            Sample::new(number, &[], repeats)
        }
    }
}

impl Sample {
    /// Constructor
    ///
    /// # Arguments
    /// * `number` - effect number
    /// * `data` - Sample data
    /// * `repeats` - [Option] with repeat count or [None]
    pub fn new(number: u32, data: &[u8], repeats: Option<&u32>) -> Sample {
        Sample {
            number,
            repeats: repeats.copied(),
            data: data.to_vec(),
        }
    }

    // pub fn number(&self) -> u32 {
    //     self.number
    // }

    // pub fn data(&self) -> &Vec<u8> {
    //     &self.data
    // }

    // pub fn repeats(&self) -> Option<&u32> {
    //     self.repeats.as_ref()
    // }
}

/// Sound player
pub trait Player {
    /// Player type name
    ///
    /// # Returns
    /// Player type name
    fn type_name(&self) -> &str;

    /// Check if a sample is currently playing
    ///
    /// # Returns
    /// `true` if a sample is currently playing, `false` if not
    fn is_playing(&mut self) -> bool;

    /// Start playback of a sample.
    ///
    /// Any sample currently playing is stopped.
    ///
    /// # Arguments
    /// * `sound` - Sample data
    /// * `volume` - Playback volume
    /// * `repeates` - Number of times to play the sample
    ///
    /// # Returns
    /// Empty [Result] or a [RuntimeError]
    fn play_sound(&mut self, sound: &[u8], volume: u8, repeats: u8) -> Result<(), RuntimeError>;

    /// Stop sample playback.
    ///
    /// Does nothing if no sample is currently playing.
    fn stop_sound(&mut self);

    /// Change the volume of any currently playing sample.
    ///
    /// Should be called when the currently playing sample repeating and is played again with a different
    /// volume to change the volume without abruptly interrupting the loop.
    ///
    /// # Arguments
    /// * `volume` - New playback volume.
    fn change_volume(&mut self, volume: u8);
}

impl fmt::Debug for dyn Player {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.type_name())
    }
}

#[derive(Debug, Default)]
/// Sound effect manager
pub struct Manager {
    /// Sound player
    player: Option<Box<dyn Player>>,
    /// Effect map
    sounds: HashMap<u32, Sample>,
    /// Currently playing effect number
    current_effect: u32,
    /// Routine to execute when current sample playback finishes or 0
    routine: usize,
}

impl From<Blorb> for HashMap<u32, Sample> {
    fn from(value: Blorb) -> Self {
        let mut sounds = HashMap::new();
        let mut loops = HashMap::new();
        if let Some(l) = value.loops() {
            for entry in l.entries() {
                loops.insert(entry.number(), entry.repeats());
            }
        }

        for index in value.ridx().indices() {
            if index.usage().eq("Snd ") {
                if let Some(chunk) = value.sounds().get(&(index.start())) {
                    let s = Sample::from((index.number(), chunk, loops.get(&index.number())));
                    if !s.data.is_empty() {
                        debug!(target: "app::sound", "Sound: {}", s);
                        sounds.insert(index.number(), s);
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
        sounds.insert(3, Sample::new(1, &[0; 128], None));
        sounds.insert(4, Sample::new(1, &[0; 256], Some(&5)));

        Ok(Manager {
            player: Some(new_player(128.0)?),
            sounds,
            current_effect: 0,
            ..Default::default()
        })
    }

    /// Constructor
    ///
    /// # Arguments
    /// * `volume_factor` - platform-specific volume factor used to normalize volume levels
    /// * `blorb` - [Blorb] containing sound sample resources
    pub fn new(volume_factor: f32, blorb: Blorb) -> Result<Manager, RuntimeError> {
        debug!(target: "app::sound", "Initializing sound manager with volume_factor {}", volume_factor);
        Ok(Manager {
            player: Some(new_player(volume_factor)?),
            sounds: HashMap::from(blorb),
            ..Default::default()
        })
    }

    /// Constructor that disabled sound playback
    pub fn none() -> Result<Manager, RuntimeError> {
        Ok(Manager {
            player: None,
            sounds: HashMap::new(),
            ..Default::default()
        })
    }

    /// Check if there is a sound playing
    ///
    /// # Returns
    /// `true` when a sound is currently playing, `false`` if not.
    pub fn is_playing(&mut self) -> bool {
        if let Some(p) = self.player.as_mut() {
            p.is_playing()
        } else {
            false
        }
    }

    /// Gets the end-of-sound routine address
    pub fn routine(&self) -> usize {
        self.routine
    }

    /// Clears the end-of-sound routine address
    pub fn clear_routine(&mut self) {
        self.routine = 0
    }

    /// Begin playback of a sample.
    ///
    /// Any currently playing sample is halted.
    ///
    /// # Arguments
    /// * `effect` - Sample number
    /// * `volume` - Playback volume
    /// * `repeats` - [Option] with number of repeats or [None]
    /// * `routine` - Byte address of a routine to execute when playback finished, or 0 for none.
    pub fn play_sound(
        &mut self,
        effect: u16,
        volume: u8,
        repeats: Option<u8>,
        routine: usize,
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
                    self.routine = routine;
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

    /// Halt sample playback.
    ///
    /// Any end-of-sound routine is *not* executed.
    pub fn stop_sound(&mut self) {
        debug!(target: "app::sound", "Stopping sound playback");
        if let Some(p) = self.player.as_mut() {
            p.stop_sound()
        }

        self.current_effect = 0;
        self.routine = 0;
    }

    /// Change the volume of any currently playing sample.
    ///
    /// # Arguments
    /// * `volume` - New playback volume
    pub fn change_volume(&mut self, volume: u8) {
        debug!(target: "app::sound", "Changing volume of playing sound to {}", volume);
        if let Some(p) = self.player.as_mut() {
            p.change_volume(volume)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        assert_ok, assert_some, assert_some_eq,
        test_util::{mock_blorb, play_sound},
    };

    use super::*;

    #[test]
    fn test_sound_from_chunk_oggv() {
        let oggv = Chunk::new_chunk(0, "OGGV", vec![1, 2, 3, 4]);
        let sound = Sample::from((1, &oggv, None));
        assert_eq!(sound.number, 1);
        assert_eq!(sound.data, &[1, 2, 3, 4]);
        assert!(sound.repeats.is_none());
    }

    #[test]
    fn test_sound_from_oggv_repeats() {
        let oggv: Chunk = Chunk::new_chunk(0, "OGGV", vec![1, 2, 3, 4]);
        let sound = Sample::from((1, &oggv, Some(&5)));
        assert_eq!(sound.number, 1);
        assert_eq!(sound.data, &[1, 2, 3, 4]);
        assert_some_eq!(sound.repeats, 5);
    }

    #[test]
    fn test_sound_from_aiff_no_sndfile() {
        let aiff = Chunk::new_form(0, "AIFF", vec![]);
        let sound = Sample::from((1, &aiff, None));
        assert_eq!(sound.number, 1);
        assert!(sound.data.is_empty());
        assert!(sound.repeats.is_none());
    }

    #[test]
    fn test_sound_from_aiff_no_sndfile_repeats() {
        let aiff = Chunk::new_form(0, "AIFF", vec![]);
        let sound = Sample::from((1, &aiff, Some(&5)));
        assert_eq!(sound.number, 1);
        assert!(sound.data.is_empty());
        assert_some_eq!(sound.repeats, &5);
    }

    #[test]
    fn test_hashmap_u32_sound_from_blorb() {
        let blorb = mock_blorb();
        let map = HashMap::from(blorb);
        let snd = assert_some!(map.get(&1));
        assert_eq!(snd.number, 1);
        assert_eq!(snd.data, &[1, 1, 1, 1]);
        assert_some_eq!(snd.repeats, &10);
        assert!(map.get(&2).is_none());
        let snd = assert_some!(map.get(&4));
        assert_eq!(snd.number, 4);
        assert_eq!(snd.data, &[4, 4, 4, 4]);
        assert!(snd.repeats.is_none());
    }

    #[test]
    fn test_manager_new() {
        let blorb = mock_blorb();
        let manager = assert_ok!(Manager::new(128.0, blorb));
        assert!(manager.player.is_some());
        assert_eq!(manager.sounds.len(), 2);
        assert_eq!(manager.current_effect, 0);
    }

    // #[test]
    // fn test_play_sound() {
    //     let blorb = mock_blorb();
    //     let mut manager = assert_ok!(Manager::new(128.0, blorb));
    //     assert!(manager.play_sound(1, 8, None).is_ok());
    //     assert!(manager.is_playing());
    //     assert!(manager.current_effect() == 1);
    //     assert_eq!(play_sound(), (4, 8, 10));
    // }

    // #[test]
    // fn test_play_sound_override_repeats() {
    //     let blorb = mock_blorb();
    //     let mut manager = assert_ok!(Manager::new(128.0, blorb));
    //     assert!(manager.play_sound(1, 8, Some(1)).is_ok());
    //     assert!(manager.is_playing());
    //     assert!(manager.current_effect() == 1);
    //     assert_eq!(play_sound(), (4, 8, 1));
    // }

    // #[test]
    // fn test_play_sound_invalid_effect() {
    //     let blorb = mock_blorb();
    //     let mut manager = assert_ok!(Manager::new(128.0, blorb));
    //     assert!(manager.play_sound(3, 8, Some(1)).is_ok());
    //     assert!(!manager.is_playing());
    //     assert!(manager.current_effect() == 0);
    //     assert_eq!(play_sound(), (0, 0, 0));
    // }

    // #[test]
    // fn test_stop_sound() {
    //     let blorb = mock_blorb();
    //     let mut manager = assert_ok!(Manager::new(128.0, blorb));
    //     assert!(manager.play_sound(4, 4, Some(1)).is_ok());
    //     assert!(manager.is_playing());
    //     assert_eq!(manager.current_effect(), 4);
    //     assert_eq!(play_sound(), (4, 4, 1));
    //     manager.stop_sound();
    //     assert!(!manager.is_playing());
    //     assert_eq!(manager.current_effect(), 0);
    //     assert_eq!(play_sound(), (0, 0, 0));
    // }

    #[test]
    fn test_stop_sound_not_playing() {
        let blorb = mock_blorb();
        let mut manager = assert_ok!(Manager::new(128.0, blorb));
        manager.stop_sound();
        assert!(!manager.is_playing());
        assert_eq!(manager.current_effect, 0);
        assert_eq!(play_sound(), (0, 0, 0));
    }

    // #[test]
    // fn test_change_volume() {
    //     let blorb = mock_blorb();
    //     let mut manager = assert_ok!(Manager::new(128.0, blorb));
    //     assert!(manager.play_sound(4, 4, Some(1)).is_ok());
    //     assert!(manager.is_playing());
    //     assert_eq!(manager.current_effect(), 4);
    //     assert_eq!(play_sound(), (4, 4, 1));
    //     manager.change_volume(8);
    //     assert!(manager.is_playing());
    //     assert_eq!(manager.current_effect(), 4);
    //     assert_eq!(play_sound(), (0, 8, 0));
    // }

    #[test]
    fn test_change_volume_not_playing() {
        let blorb = mock_blorb();
        let mut manager = assert_ok!(Manager::new(128.0, blorb));
        manager.change_volume(8);
        assert!(!manager.is_playing());
        assert_eq!(manager.current_effect, 0);
        assert_eq!(play_sound(), (0, 0, 0));
    }
}
