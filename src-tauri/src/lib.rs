use player_state::PlayerState;
use tauri::{AppHandle, Manager, State};

mod player;
mod player_state;
mod streamer;
mod streamer_pipe;

pub(crate) const MAIN_WINDOW_LABEL: &str = "main";

#[tauri::command]
fn play(player_state: State<PlayerState>, app_handle: AppHandle, uri: &str) {
    player_state.player_mut().play(app_handle, uri);
}

#[tauri::command]
fn pause(player_state: State<PlayerState>) {
    player_state.player_mut().pause();
}

#[tauri::command]
fn stop(player_state: State<PlayerState>) {
    player_state.player_mut().stop();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![play, pause, stop,])
        .manage(PlayerState::new())
        .on_window_event(move |window, event| {
            let player_state = window.state::<PlayerState>();
            let player = player_state.player_mut();
            if window.label().eq(MAIN_WINDOW_LABEL) {
                match event {
                    tauri::WindowEvent::Destroyed => {
                        player.stop_sync();
                    }
                    _ => {}
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
