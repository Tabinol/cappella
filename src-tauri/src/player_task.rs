use std::sync::{Arc, Mutex};

use crate::player::Player;

#[derive(Clone, Debug)]
pub(crate) enum PlayerTask {
    Stopped,
    Next(String),
}

pub(crate) fn run(player: Arc<Mutex<Player>>, player_task: PlayerTask) {
    let mut player_lock = player.lock().unwrap();
    match player_task {
        PlayerTask::Stopped => player_lock.stopped(),
        PlayerTask::Next(uri) => player_lock.play(&uri),
    }
}
