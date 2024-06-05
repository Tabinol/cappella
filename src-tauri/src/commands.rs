use tauri::{command, AppHandle, State};

use crate::{player, player_state::PlayerState};

#[command]
pub(crate) fn play(player_state: State<PlayerState>, app_handle: AppHandle, uri: &str) {
    player_state
        .player_mut()
        .command(app_handle, player::Command::Play(uri.to_owned()));
}

#[command]
pub(crate) fn pause(player_state: State<PlayerState>, app_handle: AppHandle) {
    player_state
        .player_mut()
        .command(app_handle, player::Command::Pause);
}

#[command]
pub(crate) fn stop(player_state: State<PlayerState>, app_handle: AppHandle) {
    player_state
        .player_mut()
        .command(app_handle, player::Command::Stop);
}
