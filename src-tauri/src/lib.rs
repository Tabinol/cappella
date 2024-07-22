use std::sync::{mpsc::channel, Arc, Mutex, OnceLock};

use ::tauri::Manager;
use frontend::frontend_pipe::ImplFrontendPipe;
use gstreamer::gstreamer::ImplGstreamer;
use player::{
    player::{ImplPlayer, Player},
    streamer::{self, ImplStreamer, Status, Streamer},
    streamer_loop::ImplStreamerLoop,
    streamer_pipe::ImplStreamerPipe,
};
use tauri::tauri_app_handle::{ImplTauriAppHandle, TauriAppHandle};

mod frontend;
mod gstreamer;
mod player;
mod tauri;
mod utils;

pub(crate) const MAIN_WINDOW_LABEL: &str = "main";

const PLAYER: OnceLock<Arc<dyn Player>> = OnceLock::new();

#[::tauri::command]
fn play(uri: &str) {
    PLAYER.get().unwrap().play(uri);
}

#[::tauri::command]
fn pause() {
    PLAYER.get().unwrap().pause();
}

#[::tauri::command]
fn stop() {
    PLAYER.get().unwrap().stop();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    ::tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let app_handle = app.app_handle().clone();
            let tauri_app_handle = Arc::new(ImplTauriAppHandle::new(app_handle));
            setup(tauri_app_handle);
            Ok(())
        })
        .invoke_handler(::tauri::generate_handler![play, pause, stop,])
        .on_window_event(move |window, event| {
            if window.label().eq(MAIN_WINDOW_LABEL) {
                match event {
                    ::tauri::WindowEvent::Destroyed => {
                        PLAYER.get().unwrap().end();
                    }
                    _ => {}
                }
            }
        })
        .run(::tauri::generate_context!())
        .expect("error while running tauri application");
}

pub fn setup(tauri_app_handle: Arc<dyn TauriAppHandle>) {
    let (sender, receiver) = channel::<Status>();
    let status = Arc::new(Mutex::new(streamer::Status::None));
    let streamer_thread_lock = Arc::new(Mutex::new(()));

    let gstreamer = Arc::new(ImplGstreamer::default());
    let streamer_pipe = Arc::new(ImplStreamerPipe::new(gstreamer.clone()));
    let frontend_pipe = Arc::new(ImplFrontendPipe::new(tauri_app_handle.clone()));
    let streamer_loop = Arc::new(ImplStreamerLoop::new(
        frontend_pipe.clone(),
        gstreamer.clone(),
        Arc::clone(&status),
    ));
    let streamer = Arc::new(ImplStreamer::new(
        streamer_pipe.clone(),
        streamer_loop.clone(),
        Arc::clone(&status),
        sender,
    ));
    let player = Arc::new(ImplPlayer::new(streamer.clone(), streamer_pipe.clone()));

    PLAYER.set(player.clone()).unwrap();
    streamer.start_thread(receiver);
}
