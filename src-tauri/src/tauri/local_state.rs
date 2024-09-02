use crate::player::front::Front;

pub struct LocalState {
    player_front: Box<dyn Front>,
}

impl LocalState {
    pub fn new(player_front: Box<dyn Front>) -> Self {
        Self { player_front }
    }

    pub fn player_front(&self) -> &dyn Front {
        &*self.player_front
    }
}
