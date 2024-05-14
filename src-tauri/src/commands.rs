use tauri::{command, AppHandle};

use crate::player::{self, Player};

#[command]
pub(crate) fn play(app_handle: AppHandle, uri: &str) {
    Player::instance().command(app_handle, player::Command::Play(uri.to_owned()));
}

#[command]
pub(crate) fn pause(app_handle: AppHandle) {
    Player::instance().command(app_handle, player::Command::Pause);
}

#[command]
pub(crate) fn stop(app_handle: AppHandle) {
    Player::instance().command(app_handle, player::Command::Stop);
}
