use ::tauri::{Manager, State};
use player::player::Player;

mod frontend;
mod gstreamer;
mod player;
mod utils;

pub(crate) const MAIN_WINDOW_LABEL: &str = "main";

#[::tauri::command]
fn play(player: State<Box<dyn Player>>, uri: &str) {
    player.play(uri);
}

#[::tauri::command]
fn pause(player: State<Box<dyn Player>>) {
    player.pause();
}

#[::tauri::command]
fn stop(player: State<Box<dyn Player>>) {
    player.stop();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let gstreamer = crate::gstreamer::gstreamer::new_boxed();
    let streamer_pipe = crate::player::streamer_pipe::new_boxed(gstreamer.clone());
    let frontend_pipe = crate::frontend::frontend_pipe::new_boxed();
    let streamer_loop =
        crate::player::streamer_loop::new_boxed(frontend_pipe.clone(), gstreamer.clone());
    let streamer = player::streamer::new_boxed(streamer_pipe.clone(), streamer_loop.clone());
    let player = player::player::new_boxed(streamer.clone(), streamer_pipe.clone());

    let frontend_pipe_setup_clone = frontend_pipe.clone();
    let player_manage_clone = player.clone();
    let player_end_clone = player.clone();

    ::tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(player_manage_clone)
        .setup(move |app| {
            frontend_pipe_setup_clone.set_app_handle(app.app_handle());
            streamer.start_thread();
            Ok(())
        })
        .invoke_handler(::tauri::generate_handler![play, pause, stop,])
        .on_window_event(move |window, event| {
            if window.label().eq(MAIN_WINDOW_LABEL) {
                match event {
                    ::tauri::WindowEvent::Destroyed => {
                        player_end_clone.end();
                    }
                    _ => {}
                }
            }
        })
        .run(::tauri::generate_context!())
        .expect("error while running tauri application");
}
