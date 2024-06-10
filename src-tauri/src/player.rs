use tauri::AppHandle;

use crate::streamer::{self, Streamer};

#[derive(Clone, Debug)]
pub(crate) struct Player {
    streamer: Streamer,
}

impl Player {
    pub(crate) fn new() -> Self {
        Self {
            streamer: Streamer::new(),
        }
    }

    pub(crate) fn play(&mut self, app_handle: AppHandle, uri: String) {
        if let Some(streamer) = self.get_streamer_if_active() {
            streamer.send(streamer::Message::StopAndSendNewUri(uri));
            return;
        }

        self.streamer.start(app_handle, uri);
    }

    pub(crate) fn pause(&mut self) {
        if let Some(streamer) = self.get_streamer_if_active() {
            streamer.send(streamer::Message::Pause);
        }
    }

    pub(crate) fn stop(&mut self) {
        if let Some(streamer) = self.get_streamer_if_active() {
            streamer.send(streamer::Message::Stop);
        }
    }

    pub(crate) fn stop_sync(&mut self) {
        if let Some(streamer) = self.get_streamer_if_active() {
            streamer.send(streamer::Message::StopSync);
            streamer.wait_until_end();
        }
    }

    pub(crate) fn stopped(&mut self) {
        // TODO
        if let Some(streamer) = self.get_streamer_if_active() {
            streamer.wait_until_end();
        }
    }

    pub(crate) fn get_streamer_if_active(&mut self) -> Option<&mut Streamer> {
        if self.streamer.is_active() {
            return Some(&mut self.streamer);
        }

        None
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
