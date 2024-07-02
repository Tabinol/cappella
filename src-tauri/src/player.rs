use std::fmt::Debug;

use dyn_clone::DynClone;

use crate::{
    streamer::Streamer,
    streamer_pipe::{Message, StreamerPipe},
};

pub(crate) trait Player: DynClone + Debug + Send + Sync {
    fn play(&mut self, uri: &str);
    fn pause(&mut self);
    fn stop(&mut self);
    fn end(&mut self);
}

dyn_clone::clone_trait_object!(Player);

#[derive(Clone, Debug)]
pub(crate) struct ImplPlayer {
    streamer: Box<dyn Streamer>,
    streamer_pipe: Box<dyn StreamerPipe>,
}

unsafe impl Send for ImplPlayer {}
unsafe impl Sync for ImplPlayer {}

impl ImplPlayer {
    pub(crate) fn new(streamer: Box<dyn Streamer>, streamer_pipe: Box<dyn StreamerPipe>) -> Self {
        Self {
            streamer,
            streamer_pipe,
        }
    }

    fn get_pipe_opt(&mut self) -> Option<Box<(dyn StreamerPipe + 'static)>> {
        if self.streamer.is_running() {
            return Some(self.streamer_pipe.clone());
        }

        None
    }
}

impl Player for ImplPlayer {
    fn play(&mut self, uri: &str) {
        if let Some(streamer_pipe) = self.get_pipe_opt() {
            streamer_pipe.send(Message::Next(uri.to_owned()));
            return;
        }

        self.streamer.play(uri);
    }

    fn pause(&mut self) {
        if let Some(streamer_pipe) = self.get_pipe_opt() {
            streamer_pipe.send(Message::Pause);
        }
    }

    fn stop(&mut self) {
        if let Some(streamer_pipe) = self.get_pipe_opt() {
            streamer_pipe.send(Message::Stop);
        }
    }

    fn end(&mut self) {
        self.streamer.end();
    }
}

// // #[cfg(test)]
// // mod tests {
// //     use std::time::Duration;

// //     use crate::{my_app_handle::MyAppHandle, player::Player};

// //     trait MockAppHandle {}

// //     impl<T: MyAppHandle> MockAppHandle for T {}

// //     struct Streamer {}
// //     struct StreamerPipe {}

// //     #[test]
// //     fn test_is_active() {
// //         let mut player = Player::new(Streamer {}, StreamerPipe {});

// //         assert!(!player.is_active());

// //         let faked_streamer_join_handle = thread::Builder::new()
// //             .spawn(move || {
// //                 thread::sleep(Duration::from_secs(2));
// //             })
// //             .unwrap();

// //         player.set_streamer_join_handle(faked_streamer_join_handle);

// //         assert!(player.is_active());
// //         assert!(!player.get_streamer_join_handle_opt().unwrap().is_finished());

// //         player.wait_until_end();

// //         assert!(!player.is_active());
// //         assert!(player.streamer_join_handle.is_null());
// //     }
// // }
