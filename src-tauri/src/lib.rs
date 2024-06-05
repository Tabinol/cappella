use player::Player;
use player_state::PlayerState;
use tauri::Manager;

mod commands;
mod player;
mod player_state;
mod streamer;
mod streamer_thread;

pub(crate) const MAIN_WINDOW_LABEL: &str = "main";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::play,
            commands::pause,
            commands::stop,
        ])
        .manage(PlayerState::new(Player::new()))
        .on_window_event(move |window, event| {
            let player_state = window.state::<PlayerState>();
            let mut player = player_state.player_mut();
            if window.label().eq(MAIN_WINDOW_LABEL) {
                match event {
                    tauri::WindowEvent::Destroyed => {
                        player.command(window.app_handle().clone(), player::Command::StopSync);
                    }
                    _ => {}
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
