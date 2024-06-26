use std::fmt::Debug;
use std::sync::{Arc, Mutex};

use dyn_clone::DynClone;
use tauri::{AppHandle, Manager, State};

use crate::player::Player;
use crate::player_task::{self, PlayerTask};

pub(crate) trait MyAppHandle: DynClone + Debug + Send + Sync {
    fn player(&self) -> State<Arc<Mutex<Player>>>;
    fn run_player_task_on_main_thread(&self, player_task: PlayerTask);
}

dyn_clone::clone_trait_object!(MyAppHandle);

impl MyAppHandle for AppHandle {
    fn player(&self) -> State<Arc<Mutex<Player>>> {
        self.state()
    }

    fn run_player_task_on_main_thread(&self, player_task: PlayerTask) {
        let player = Arc::clone(&self.player());
        self.run_on_main_thread(|| player_task::run(player, player_task))
            .unwrap();
    }
}
