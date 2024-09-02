use ::tauri::{AppHandle, Manager, State, Window, WindowEvent};
use tauri::local_state::LocalState;

mod frontend;
mod player;
mod streamer;
mod tauri;

pub const MAIN_WINDOW_LABEL: &str = "main";

#[::tauri::command]
fn play(app_handle: AppHandle, state: State<LocalState>, uri: &str) {
    let app_handle_box = Box::new(app_handle);
    let app_handle_addr = Box::into_raw(app_handle_box) as usize;
    state.player_front().play(app_handle_addr, uri);
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
    let state = init();

    ::tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(state)
        .invoke_handler(::tauri::generate_handler![play, pause, stop,])
        .on_window_event(|window, event| on_window_event(window, event))
        .run(::tauri::generate_context!())
        .expect("error while running tauri application");
}

fn init() -> LocalState {
    // Step 1 in alphabetical order
    let streamer_bus = streamer::bus::new_arc();

    // Step 2 in alphabetical order
    let streamer_front = streamer::front::new_box(streamer_bus.clone());
    let streamer_pipe = streamer::pipe::new_box(streamer_bus.clone());

    // Step 3 in alphabetical order
    let player_front = player::front::new_box(streamer_front, streamer_pipe);

    // Step 4 return
    LocalState::new(player_front)
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
    let state = app_handle.state::<LocalState>();
    state.player_front().stop();
    state.player_front().wait_until_end();
}
