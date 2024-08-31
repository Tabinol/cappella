use crate::player::front::Front;

pub(crate) struct LocalState {
    player_front: Box<dyn Front>,
}

impl LocalState {
    pub(crate) fn new(player_front: Box<dyn Front>) -> Self {
        Self { player_front }
    }

    pub(crate) fn player_front(&self) -> &dyn Front {
        &*self.player_front
    }
}
