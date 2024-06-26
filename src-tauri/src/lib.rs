use std::sync::{Arc, Mutex, OnceLock};

use player::Player;
use streamer::Streamer;
use streamer_pipe::StreamerPipe;
use tauri::{Manager, State};

mod my_app_handle;
mod player;
mod player_task;
mod streamer;
mod streamer_pipe;

pub(crate) const MAIN_WINDOW_LABEL: &str = "main";

#[tauri::command]
fn play(player: State<'_, Arc<Mutex<Player>>>, uri: &str) {
    player.lock().unwrap().play(uri);
}

#[tauri::command]
fn pause(player: State<'_, Arc<Mutex<Player>>>) {
    player.lock().unwrap().pause();
}

#[tauri::command]
fn stop(player: State<'_, Arc<Mutex<Player>>>) {
    player.lock().unwrap().stop();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    static PLAYER_CELL: OnceLock<Arc<Mutex<Player>>> = OnceLock::<Arc<Mutex<Player>>>::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![play, pause, stop,])
        .setup(|app| {
            let app_handle = Box::new(app.app_handle().clone());
            let streamer_pipe = StreamerPipe::new();
            let streamer = Streamer::new(streamer_pipe.clone(), app_handle.clone());
            let player = Arc::new(Mutex::new(Player::new(streamer, streamer_pipe)));

            PLAYER_CELL.set(Arc::clone(&player)).unwrap();
            app.manage(player);

            Ok(())
        })
        .on_window_event(move |window, event| {
            if window.label().eq(MAIN_WINDOW_LABEL) {
                match event {
                    tauri::WindowEvent::Destroyed => {
                        PLAYER_CELL.get().unwrap().lock().unwrap().stop_sync();
                    }
                    _ => {}
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
