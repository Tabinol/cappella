// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use player::Player;
use tauri::Manager;

mod commands;
mod player;
mod streamer;

pub(crate) const MAIN_WINDOW_LABEL: &str = "main";

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::play,
            commands::pause,
            commands::stop,
        ])
        .on_window_event(move |event| {
            if event.window().label().eq(MAIN_WINDOW_LABEL) {
                match event.event() {
                    tauri::WindowEvent::Destroyed => {
                        Player::instance()
                            .command(event.window().app_handle(), player::Command::StopSync);
                    }
                    _ => {}
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
