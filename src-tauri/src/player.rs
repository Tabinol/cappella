use std::{
    alloc::{alloc, dealloc, Layout},
    ptr::{self, null_mut},
    thread::{self, JoinHandle},
};

use crate::{streamer::Streamer, streamer_pipe::StreamerPipe};

#[derive(Clone, Debug)]
pub(crate) struct Player {
    streamer: Streamer,
    streamer_pipe: StreamerPipe,
    streamer_join_handle: *mut JoinHandle<()>,
}

unsafe impl Send for Player {}
unsafe impl Sync for Player {}

const STREAMER_THREAD_NAME: &str = "streamer";

impl Player {
    pub(crate) fn new(streamer: Streamer, streamer_pipe: StreamerPipe) -> Self {
        Self {
            streamer,
            streamer_pipe,
            streamer_join_handle: null_mut(),
        }
    }

    pub(crate) fn play(&mut self, uri: &str) {
        if let Some(streamer_pipe) = self.get_pipe_opt() {
            streamer_pipe.send_stop_and_send_new_uri(uri);
            return;
        }

        self.start_streamer(uri);
    }

    pub(crate) fn pause(&mut self) {
        if let Some(streamer_pipe) = self.get_pipe_opt() {
            streamer_pipe.send_pause();
        }
    }

    pub(crate) fn stop(&mut self) {
        if let Some(streamer_pipe) = self.get_pipe_opt() {
            streamer_pipe.send_stop();
        }
    }

    pub(crate) fn stop_sync(&mut self) {
        if let Some(streamer_pipe) = self.get_pipe_opt() {
            streamer_pipe.send_stop_sync();
        }
        self.wait_until_end();
    }

    pub(crate) fn stopped(&mut self) {
        // TODO
        if self.get_pipe_opt().is_some() {
            self.wait_until_end();
        }
    }

    fn start_streamer(&mut self, uri: &str) {
        if self.is_active() {
            panic!("Streamer thread already active.")
        }

        let uri_owned = uri.to_owned();
        let mut streamer_clone = self.streamer.clone();
        let streamer_join_handle = thread::Builder::new()
            .name(STREAMER_THREAD_NAME.to_string())
            .spawn(move || {
                streamer_clone.run(uri_owned);
            })
            .unwrap();

        self.set_streamer_join_handle(streamer_join_handle);
    }

    fn get_pipe_opt(&mut self) -> Option<&StreamerPipe> {
        if self.is_active() {
            return Some(&self.streamer_pipe);
        }

        None
    }

    fn is_active(&mut self) -> bool {
        if self.streamer.is_running() {
            return true;
        }

        self.wait_until_end();

        false
    }

    fn wait_until_end(&mut self) {
        if self.streamer_join_handle.is_null() {
            return;
        }

        let streamer_join_handle = unsafe { ptr::read(self.streamer_join_handle) };
        streamer_join_handle.join().unwrap();
        self.unset_streamer_join_handle();
    }

    fn set_streamer_join_handle(&mut self, streamer_join_handle: JoinHandle<()>) {
        unsafe {
            self.streamer_join_handle =
                alloc(Layout::new::<JoinHandle<()>>()) as *mut JoinHandle<()>;
            ptr::write(self.streamer_join_handle, streamer_join_handle);
        }
    }

    fn unset_streamer_join_handle(&mut self) {
        unsafe {
            dealloc(
                self.streamer_join_handle as *mut u8,
                Layout::new::<JoinHandle<()>>(),
            );
        }

        self.streamer_join_handle = null_mut();
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
