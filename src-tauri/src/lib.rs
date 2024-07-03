use std::sync::{mpsc::channel, Arc, Mutex, OnceLock};

use player::{ImplPlayer, Player};
use streamer::{ImplStreamer, Status, Streamer};
use streamer_loop::{ImplStreamerLoop, StreamerLoop};
use streamer_pipe::{ImplStreamerPipe, StreamerPipe};
use tauri::{AppHandle, Manager};

mod player;
mod streamer;
mod streamer_loop;
mod streamer_pipe;

pub(crate) const MAIN_WINDOW_LABEL: &str = "main";

static PLAYER: OnceLock<Arc<dyn Player>> = OnceLock::new();
static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

#[tauri::command]
fn play(uri: &str) {
    PLAYER.get().unwrap().play(uri);
}

#[tauri::command]
fn pause() {
    PLAYER.get().unwrap().pause();
}

#[tauri::command]
fn stop() {
    PLAYER.get().unwrap().stop();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let app_handle = app.app_handle().clone();
            APP_HANDLE.set(app_handle).unwrap();

            let (sender, receiver) = channel::<Status>();
            let status = Arc::new(Mutex::new(streamer::Status::None));
            let streamer_pipe: Arc<dyn StreamerPipe> = Arc::new(ImplStreamerPipe::new());
            let streamer_loop: Arc<dyn StreamerLoop> = Arc::new(ImplStreamerLoop::new(
                Arc::clone(&streamer_pipe),
                Arc::new(receiver),
                Arc::clone(&status),
            ));
            let streamer: Arc<dyn Streamer> = Arc::new(ImplStreamer::new(
                Arc::clone(&streamer_pipe),
                Arc::clone(&streamer_loop),
                Arc::clone(&status),
                Arc::new(sender),
            ));
            let player: Arc<dyn Player> = Arc::new(ImplPlayer::new(
                Arc::clone(&streamer),
                Arc::clone(&streamer_pipe),
            ));

            PLAYER.set(player).unwrap();
            streamer.start_thread();

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![play, pause, stop,])
        .on_window_event(move |window, event| {
            if window.label().eq(MAIN_WINDOW_LABEL) {
                match event {
                    tauri::WindowEvent::Destroyed => {
                        PLAYER.get().unwrap().end();
                    }
                    _ => {}
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
