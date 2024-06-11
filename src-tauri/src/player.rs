use std::{sync::Arc, thread};

use tauri::AppHandle;

use crate::{
    streamer::Streamer,
    streamer_pipe::{Status, StreamerPipe},
};

#[derive(Clone, Debug)]
pub(crate) struct Player {
    streamer_pipe: Arc<StreamerPipe>,
}

const STREAMER_THREAD_NAME: &str = "streamer";

impl Player {
    pub(crate) fn new() -> Self {
        Self {
            streamer_pipe: Arc::new(StreamerPipe::new()),
        }
    }

    pub(crate) fn play(&self, app_handle: AppHandle, uri: &str) {
        if let Some(streamer_pipe) = self.get_pipe_if_active() {
            streamer_pipe.send_stop_and_send_new_uri(uri);
            return;
        }

        self.start_streamer(app_handle, uri);
    }

    pub(crate) fn pause(&self) {
        if let Some(streamer_pipe) = self.get_pipe_if_active() {
            streamer_pipe.send_pause();
        }
    }

    pub(crate) fn stop(&self) {
        if let Some(streamer_pipe) = self.get_pipe_if_active() {
            streamer_pipe.send_stop();
        }
    }

    pub(crate) fn stop_sync(&self) {
        if let Some(streamer_pipe) = self.get_pipe_if_active() {
            streamer_pipe.send_stop_sync();
        }
        self.wait_until_end();
    }

    pub(crate) fn stopped(&self) {
        // TODO
        if self.get_pipe_if_active().is_some() {
            self.wait_until_end();
        }
    }

    fn start_streamer(&self, app_handle: AppHandle, uri: &str) {
        if self.is_active() {
            panic!("Streamer thread already active.")
        }

        let uri_owned = uri.to_owned();
        let streamer_pipe_clone = Arc::clone(&self.streamer_pipe);

        thread::Builder::new()
            .name(STREAMER_THREAD_NAME.to_string())
            .spawn(move || {
                Streamer::new(streamer_pipe_clone, app_handle, uri_owned).start();
            })
            .unwrap();
    }

    fn get_pipe_if_active(&self) -> Option<&StreamerPipe> {
        if self.is_active() {
            return Some(&self.streamer_pipe);
        }

        None
    }

    fn is_active(&self) -> bool {
        matches!(&*self.streamer_pipe.status.lock().unwrap(), Status::Active)
    }

    fn wait_until_end(&self) {
        let _unused = self.streamer_pipe.streamer_lock.lock().unwrap();
    }
}
// #[cfg(test)]
// mod tests {

//     use std::thread;

//     use super::{Player, PlayerStatus};

//     struct Streamer {}

//     impl Streamer {
//         fn new(uri: String) -> Self {
//             Self {}
//         }

//         fn start(&mut self) {
//             thread::park();
//         }
//     }

//     struct StreamerEvent {}

//     // TODO
//     impl StreamerEvent {}

//     #[test]
//     fn test_play_new() {
//         let mut player = Player {
//             uri: None,
//             streamer_join_handle: None,
//             status: PlayerStatus::Empty,
//         };

//         player.play_new("uri");

//         assert_eq!(player.get_state().state.unwrap(), GST_STATE_PLAYING);
//         assert_eq!(player.uri, Some("uri".to_string()));
//         assert_eq!(player.status, PlayerStatus::Play);
//         assert!(player.pipeline.is_some());
//     }

//     #[test]
//     fn test_pause() {
//         let mut player = Player {
//             pipeline: None,
//             uri: None,
//             status: PlayerStatus::Empty,
//         };

//         player.play_new("uri");
//         player.pause();

//         assert_eq!(player.get_state().state.unwrap(), GST_STATE_PAUSED);
//         assert_eq!(player.status, PlayerStatus::Pause);
//     }

//     #[test]
//     fn test_pause_pause() {
//         let mut player = Player {
//             pipeline: None,
//             uri: None,
//             status: PlayerStatus::Empty,
//         };

//         player.play_new("uri");
//         player.pause();
//         player.pause();

//         assert_eq!(player.get_state().state.unwrap(), GST_STATE_PLAYING);
//         assert_eq!(player.status, PlayerStatus::Play);
//     }

//     #[test]
//     fn test_pause_play() {
//         let mut player = Player {
//             pipeline: None,
//             uri: None,
//             status: PlayerStatus::Empty,
//         };

//         player.play_new("uri");
//         player.pause();
//         player.play();

//         assert_eq!(player.get_state().state.unwrap(), GST_STATE_PLAYING);
//         assert_eq!(player.status, PlayerStatus::Play);
//     }

//     #[test]
//     fn test_stop() {
//         let mut player = Player {
//             pipeline: None,
//             uri: None,
//             status: PlayerStatus::Empty,
//         };

//         player.play_new("uri");
//         player.stop();

//         assert!(player.get_state().state.is_none());
//         assert_eq!(player.status, PlayerStatus::Stop);
//         assert!(player.pipeline.is_none());
//     }
// }
