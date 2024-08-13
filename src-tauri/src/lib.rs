use std::{sync::mpsc::channel, thread::JoinHandle};

use ::tauri::{AppHandle, Manager, State, Window, WindowEvent};
use frontend::frontend_pipe;
use gstreamer::{
    gstreamer_bus::{self},
    gstreamer_data,
    gstreamer_message::GstreamerMessage,
    gstreamer_pipe,
    gstreamer_thread::GstreamerThread,
};
use player::player_front;
use tauri::local_state::LocalState;

mod frontend;
mod gstreamer;
mod player;
mod tauri;
mod utils;

pub(crate) const MAIN_WINDOW_LABEL: &str = "main";

#[::tauri::command]
fn play(app_handle: AppHandle, state: State<LocalState>, uri: &str) {
    let frontend_pipe = frontend_pipe::new_box(app_handle);
    state.player_front().play(frontend_pipe, uri);
}

#[::tauri::command]
fn pause(state: State<LocalState>) {
    state.player_front().pause();
}

#[::tauri::command]
fn stop(state: State<LocalState>) {
    state.player_front().stop();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let (gstreamer_join_handle, state) = init();

    ::tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(state)
        .invoke_handler(::tauri::generate_handler![play, pause, stop,])
        .on_window_event(|window, event| on_window_event(window, event))
        .run(::tauri::generate_context!())
        .expect("error while running tauri application");

    end(gstreamer_join_handle);
}

fn init() -> (JoinHandle<()>, LocalState) {
    // Step 0 Rust/Tauri inits
    let (sender, receiver) = channel::<GstreamerMessage>();

    // Step 1 in alphabetical order
    let gstreamer_bus = gstreamer_bus::new_arc();
    let gstreamer_data = gstreamer_data::new_arc();

    // Step 2 in alphabetical order
    let gstreamer_pipe = gstreamer_pipe::new_box(gstreamer_bus.clone(), sender);

    // Step 3 in alphabetical order
    let player_front = player_front::new_box(gstreamer_data.clone(), gstreamer_pipe);

    // Step 4 in alphabetical order
    let state = LocalState::new(player_front);

    // Start threads in alphabetical order
    let gstreamer_thread_join_handle =
        GstreamerThread::start(gstreamer_bus, gstreamer_data, receiver);

    (gstreamer_thread_join_handle, state)
}

fn on_window_event(window: &Window, event: &WindowEvent) {
    if window.label().eq(MAIN_WINDOW_LABEL) {
        let app_handle = window.app_handle();
        match event {
            ::tauri::WindowEvent::Destroyed => {
                end_streamer(app_handle);
            }
            _ => {}
        }
    }
}

fn end_streamer(app_handle: &AppHandle) {
    app_handle.state::<LocalState>().player_front().end();
}

fn end(gstreamer_join_handle: JoinHandle<()>) {
    if let Some(error) = gstreamer_join_handle.join().err() {
        eprint!("Error at from the join handle of the GStreamer: {error:?}")
    }
}
