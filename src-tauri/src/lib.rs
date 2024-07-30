use std::sync::Arc;

use ::tauri::{Manager, State};
use frontend::frontend_pipe::ImplFrontendPipe;
use gstreamer::gstreamer::ImplGstreamer;
use player::{
    player::{ImplPlayer, Player},
    streamer::{ImplStreamer, Streamer},
    streamer_loop::ImplStreamerLoop,
    streamer_pipe::ImplStreamerPipe,
};
use tauri::{
    tauri_app_handle::{ImplTauriAppHandle, TauriAppHandle},
    tauri_state::TauriState,
};

mod frontend;
mod gstreamer;
mod player;
mod tauri;
mod utils;

pub(crate) const MAIN_WINDOW_LABEL: &str = "main";

#[::tauri::command]
fn play(state: State<TauriState>, uri: &str) {
    state.player().play(uri);
}

#[::tauri::command]
fn pause(state: State<TauriState>) {
    state.player().pause();
}

#[::tauri::command]
fn stop(state: State<TauriState>) {
    state.player().stop();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let gstreamer = Arc::new(ImplGstreamer::default());
    let streamer_pipe = Arc::new(ImplStreamerPipe::new(gstreamer.clone()));
    let tauri_app_handle = Arc::<ImplTauriAppHandle>::default();
    let frontend_pipe = Arc::new(ImplFrontendPipe::new(tauri_app_handle.clone()));
    let streamer_loop = Arc::new(ImplStreamerLoop::new(
        frontend_pipe.clone(),
        gstreamer.clone(),
    ));
    let streamer = Arc::new(ImplStreamer::new(
        streamer_pipe.clone(),
        streamer_loop.clone(),
    ));
    let player = Arc::new(ImplPlayer::new(streamer.clone(), streamer_pipe.clone()));

    let tauri_app_handle_clone = tauri_app_handle.clone();
    let player_clone = player.clone();

    ::tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(TauriState::new(player.clone()))
        .setup(move |app| {
            tauri_app_handle_clone.set_app_handle(app.app_handle().clone());
            streamer.start_thread();
            Ok(())
        })
        .invoke_handler(::tauri::generate_handler![play, pause, stop,])
        .on_window_event(move |window, event| {
            if window.label().eq(MAIN_WINDOW_LABEL) {
                match event {
                    ::tauri::WindowEvent::Destroyed => {
                        player_clone.end();
                    }
                    _ => {}
                }
            }
        })
        .run(::tauri::generate_context!())
        .expect("error while running tauri application");
}
