use crate::{error::RuntimeError, test_util::set_play_sound};

use super::Player;

pub struct TestPlayer {
    playing: bool,
}

pub fn new_player() -> Result<Box<dyn Player>, RuntimeError> {
    Ok(Box::new(TestPlayer { playing: false }))
}

impl Player for TestPlayer {
    fn type_name(&self) -> &str {
        "TestPlayer"
    }

    fn is_playing(&mut self) -> bool {
        self.playing
    }

    fn play_sound(
        &mut self,
        sound: &[u8],
        volume: u8,
        repeats: u8,
    ) -> Result<(), crate::error::RuntimeError> {
        set_play_sound(sound.len(), volume, repeats);
        self.playing = true;
        Ok(())
    }

    fn stop_sound(&mut self) {
        set_play_sound(0, 0, 0);
        self.playing = false;
    }

    fn change_volume(&mut self, volume: u8) {
        if self.playing {
            set_play_sound(0, volume, 0);
        }
    }
}
