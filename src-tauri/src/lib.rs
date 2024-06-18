use player::Player;
use tauri::Manager;

mod player;
mod streamer;
mod streamer_pipe;

pub(crate) const MAIN_WINDOW_LABEL: &str = "main";

#[tauri::command]
fn play(uri: &str) {
    Player::instance().play(uri);
}

#[tauri::command]
fn pause() {
    Player::instance().pause();
}

#[tauri::command]
fn stop() {
    Player::instance().stop();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![play, pause, stop,])
        .setup(|app| {
            Player::init(app.app_handle().clone());
            Ok(())
        })
        .on_window_event(move |window, event| {
            let player = Player::instance();
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
