use tauri::AppHandle;

use crate::streamer::{self, Streamer};

#[derive(Debug, PartialEq)]
pub(crate) enum Command {
    Play(String),
    Pause,
    Stop,
    StopSync,
    Stopped,
}

#[derive(Debug)]
pub(crate) struct Player {
    streamer: Option<Streamer>,
}

impl Player {
    fn new() -> Self {
        Self { streamer: None }
    }

    pub(crate) fn instance() -> &'static mut Player {
        static mut PLAYER: Option<Player> = None;

        // Always in the main thread
        unsafe {
            if PLAYER.is_none() {
                PLAYER = Some(Self::new())
            }

            PLAYER.as_mut().unwrap()
        }
    }

    pub(crate) fn command(&mut self, app_handle: AppHandle, command: Command) {
        match command {
            Command::Play(uri) => self.play(app_handle, uri.to_owned()),
            Command::Pause => self.pause(),
            Command::Stop => self.stop(),
            Command::StopSync => self.stop_sync(),
            Command::Stopped => self.stopped(),
        }
    }

    fn play(&mut self, app_handle: AppHandle, uri: String) {
        if let Some(streamer) = self.get_streamer_if_active() {
            streamer.send(streamer::Message::StopAndSendNewUri(uri));
            return;
        }

        Streamer::start(app_handle, uri.to_owned());
    }

    fn pause(&mut self) {
        if let Some(streamer) = self.get_streamer_if_active() {
            streamer.send(streamer::Message::Pause);
        }
    }

    fn stop(&mut self) {
        if let Some(streamer) = self.get_streamer_if_active() {
            streamer.send(streamer::Message::Stop);
        }
    }

    fn stop_sync(&mut self) {
        if let Some(streamer) = self.get_streamer_if_active() {
            streamer.send(streamer::Message::StopSync);
            streamer.wait_until_end();
        }

        self.set_streamer_status_off();
    }

    fn stopped(&mut self) {
        self.set_streamer_status_off();
    }

    fn get_streamer_if_active(&mut self) -> &Option<Streamer> {
        // TODO CHANGE

        if let Some(streamer) = &self.streamer {
            if streamer.is_active() {}
            self.set_streamer_status_off();
        }

        &None
    }

    fn set_streamer_status_off(&mut self) {
        self.streamer = None;
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
