use std::sync::{Arc, Mutex, MutexGuard};

use crate::player::Player;

#[derive(Debug)]
pub(crate) struct PlayerState {
    player: Arc<Mutex<Player>>,
}

impl PlayerState {
    pub(crate) fn new(player: Player) -> Self {
        Self {
            player: Arc::new(Mutex::new(player)),
        }
    }

    pub(crate) fn player_mut(&self) -> MutexGuard<Player> {
        self.player.lock().unwrap()
    }
}
