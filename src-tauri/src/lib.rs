use std::sync::{Arc, OnceLock};

use ::tauri::Manager;
use player::player::Player;

mod frontend;
mod gstreamer;
mod player;
mod tauri;
mod utils;

pub(crate) const MAIN_WINDOW_LABEL: &str = "main";

static PLAYER: OnceLock<Arc<dyn Player>> = OnceLock::new();

fn player<'a>() -> &'a dyn Player {
    &**PLAYER.get().expect("Static player is not initialized.")
}

#[::tauri::command]
fn play(uri: &str) {
    player().play(uri);
}

#[::tauri::command]
fn pause() {
    player().pause();
}

#[::tauri::command]
fn stop() {
    player().stop();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    ::tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let gstreamer = gstreamer::gstreamer::new_arc();
            let streamer_pipe = player::streamer_pipe::new_arc(gstreamer.clone());
            let app_handle_box = Box::new(app.app_handle().clone());
            let frontend_pipe = frontend::frontend_pipe::new_arc(app_handle_box);
            let streamer_loop =
                player::streamer_loop::new_arc(frontend_pipe.clone(), gstreamer.clone());
            let streamer = player::streamer::new_arc(streamer_pipe.clone(), streamer_loop.clone());
            let player = player::player::new_arc(streamer.clone(), streamer_pipe.clone());
            PLAYER
                .set(player.clone())
                .expect("Static player is already initialized.");
            streamer.start_thread();
            Ok(())
        })
        .invoke_handler(::tauri::generate_handler![play, pause, stop,])
        .on_window_event(move |window, event| {
            if window.label().eq(MAIN_WINDOW_LABEL) {
                match event {
                    ::tauri::WindowEvent::Destroyed => {
                        player().end();
                    }
                    _ => {}
                }
            }
        })
        .run(::tauri::generate_context!())
        .expect("error while running tauri application");
}
