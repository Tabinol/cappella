use crate::player::player_front::PlayerFront;

pub(crate) struct LocalState {
    player_front: Box<dyn PlayerFront>,
}

impl LocalState {
    pub(crate) fn new(player_front: Box<dyn PlayerFront>) -> Self {
        Self { player_front }
    }

    pub(crate) fn player_front(&self) -> &dyn PlayerFront {
        &*self.player_front
    }
}
