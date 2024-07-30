use std::sync::Arc;

use crate::player::player::Player;

pub(crate) struct TauriState {
    player: Arc<dyn Player>,
}

impl TauriState {
    pub(crate) fn new(player: Arc<dyn Player>) -> Self {
        Self { player }
    }

    pub(crate) fn player(&self) -> &dyn Player {
        &*self.player
    }
}
