use std::sync::{mpsc::channel, Arc, Mutex, OnceLock};

use tauri::Manager;

use frontend_pipe::ImplFrontendPipe;
use local_app_handle::ImplLocalAppHandle;
use local_gstreamer::ImplLocalGstreamer;
use player::{ImplPlayer, Player};
use streamer::{ImplStreamer, Status};
use streamer_loop::ImplStreamerLoop;
use streamer_pipe::ImplStreamerPipe;

mod frontend_pipe;
mod local_app_handle;
mod local_gstreamer;
mod local_gstreamer_message;
mod local_gstreamer_pipeline;
mod player;
mod pointer;
mod streamer;
mod streamer_loop;
mod streamer_pipe;
mod utils;

pub(crate) const MAIN_WINDOW_LABEL: &str = "main";

const PLAYER: OnceLock<Box<dyn Player>> = OnceLock::new();

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
            let (sender, receiver) = channel::<Status>();
            let status = Arc::new(Mutex::new(streamer::Status::None));
            let streamer_thread_lock = Arc::new(Mutex::new(()));

            let local_app_handle = ImplLocalAppHandle::new(app_handle);
            let local_gstreamer = ImplLocalGstreamer::new();
            let streamer_pipe = ImplStreamerPipe::new(local_gstreamer.clone());
            let frontend_pipe = ImplFrontendPipe::new(local_app_handle.clone());
            let streamer_loop = ImplStreamerLoop::new(
                frontend_pipe.clone(),
                local_gstreamer.clone(),
                Arc::clone(&status),
                Arc::clone(&streamer_thread_lock),
            );
            let streamer = ImplStreamer::new(
                streamer_pipe.clone(),
                streamer_loop.clone(),
                Arc::clone(&status),
                sender,
                Arc::clone(&streamer_thread_lock),
            );
            let player = ImplPlayer::new(streamer.clone(), streamer_pipe.clone());

            PLAYER.set(player.clone()).unwrap();
            streamer.start_thread(receiver);

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
