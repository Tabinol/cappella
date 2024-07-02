use std::sync::{Mutex, OnceLock};

use player::{ImplPlayer, Player};
use streamer::{ImplStreamer, Streamer};
use streamer_pipe::{ImplStreamerPipe, StreamerPipe};
use tauri::{AppHandle, Manager};

mod player;
mod streamer;
mod streamer_pipe;

pub(crate) const MAIN_WINDOW_LABEL: &str = "main";

static PLAYER: OnceLock<Mutex<Box<dyn Player>>> = OnceLock::new();
static APP_HANDLE: OnceLock<Box<AppHandle>> = OnceLock::new();

#[tauri::command]
fn play(uri: &str) {
    PLAYER.get().unwrap().lock().unwrap().play(uri);
}

#[tauri::command]
fn pause() {
    PLAYER.get().unwrap().lock().unwrap().pause();
}

#[tauri::command]
fn stop() {
    PLAYER.get().unwrap().lock().unwrap().stop();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let app_handle = Box::new(app.app_handle().clone());
            APP_HANDLE.set(app_handle).unwrap();

            let streamer_pipe: Box<dyn StreamerPipe> = Box::new(ImplStreamerPipe::new());
            let mut streamer: Box<dyn Streamer> =
                Box::new(ImplStreamer::new(streamer_pipe.clone()));
            let player: Mutex<Box<dyn Player>> = Mutex::new(Box::new(ImplPlayer::new(
                streamer.clone(),
                streamer_pipe.clone(),
            )));
            PLAYER.set(player).unwrap();

            streamer.start();

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![play, pause, stop,])
        .on_window_event(move |window, event| {
            if window.label().eq(MAIN_WINDOW_LABEL) {
                match event {
                    tauri::WindowEvent::Destroyed => {
                        PLAYER.get().unwrap().lock().unwrap().end();
                    }
                    _ => {}
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
