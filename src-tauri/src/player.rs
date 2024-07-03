use std::{fmt::Debug, sync::Arc};

use crate::{
    streamer::Streamer,
    streamer_pipe::{Message, StreamerPipe},
};

#[cfg(test)]
use mockall::automock;

#[cfg_attr(test, automock)]
pub(crate) trait Player: Debug + Send + Sync {
    fn play(&self, uri: &str);
    fn pause(&self);
    fn stop(&self);
    fn end(&self);
}

#[derive(Debug)]
pub(crate) struct ImplPlayer {
    streamer: Arc<dyn Streamer>,
    streamer_pipe: Arc<dyn StreamerPipe>,
}

impl ImplPlayer {
    pub(crate) fn new(streamer: Arc<dyn Streamer>, streamer_pipe: Arc<dyn StreamerPipe>) -> Self {
        Self {
            streamer,
            streamer_pipe,
        }
    }

    fn get_pipe_opt(&self) -> Option<Arc<dyn StreamerPipe>> {
        if self.streamer.is_running() {
            return Some(Arc::clone(&self.streamer_pipe));
        }

        None
    }
}

impl Player for ImplPlayer {
    fn play(&self, uri: &str) {
        if let Some(streamer_pipe) = self.get_pipe_opt() {
            streamer_pipe.send(Message::Next(uri.to_owned()));
            return;
        }

        self.streamer.play(uri);
    }

    fn pause(&self) {
        if let Some(streamer_pipe) = self.get_pipe_opt() {
            streamer_pipe.send(Message::Pause);
        }
    }

    fn stop(&self) {
        if let Some(streamer_pipe) = self.get_pipe_opt() {
            streamer_pipe.send(Message::Stop);
        }
    }

    fn end(&self) {
        self.streamer.end();
    }
}

// #[cfg(test)]
// mod tests {
//     use crate::{player::ImplPlayer, streamer::Streamer};

//     #[derive(Clone, Debug)]
//     struct MockStreamer {}

//     impl Streamer for MockStreamer {
//         fn is_running(&self) -> bool {
//             todo!()
//         }

//         fn start(&mut self) {
//             todo!()
//         }

//         fn play(&mut self, uri: &str) {
//             todo!()
//         }

//         fn end(&mut self) {
//             todo!()
//         }
//     }

//     #[test]
//     fn test_is_active() {
//         let mut player = ImplPlayer::new(streamer, streamer_pipe);

//         assert!(!player.is_active());

//         let faked_streamer_join_handle = thread::Builder::new()
//             .spawn(move || {
//                 thread::sleep(Duration::from_secs(2));
//             })
//             .unwrap();

//         player.set_streamer_join_handle(faked_streamer_join_handle);

//         assert!(player.is_active());
//         assert!(!player.get_streamer_join_handle_opt().unwrap().is_finished());

//         player.wait_until_end();

//         assert!(!player.is_active());
//         assert!(player.streamer_join_handle.is_null());
//     }
// }
