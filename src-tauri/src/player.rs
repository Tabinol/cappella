use std::{
    alloc::{alloc, dealloc, Layout},
    ptr::{self, null_mut},
    sync::{Arc, OnceLock},
    thread::{self, JoinHandle},
};

use tauri::AppHandle;

use crate::{streamer::Streamer, streamer_pipe::StreamerPipe};

#[derive(Clone, Debug)]
pub(crate) struct Player {
    app_handle: Option<AppHandle>,
    streamer_pipe: Arc<StreamerPipe>,
    streamer_join_handle: *mut JoinHandle<()>,
}

unsafe impl Send for Player {}
unsafe impl Sync for Player {}

const STREAMER_THREAD_NAME: &str = "streamer";

static mut INSTANCE: OnceLock<Player> = OnceLock::<Player>::new();

impl Player {
    pub(crate) fn init(app_handle: AppHandle) {
        unsafe {
            INSTANCE
                .set(Self::new(Some(app_handle)))
                .unwrap_or_else(|_| eprintln!("Trying to init 'Player' a second time. Ignoring."));
        }
    }
    pub(crate) fn instance() -> &'static mut Self {
        unsafe {
            INSTANCE
                .get_mut()
                .expect("'Player' instance not initialized.")
        }
    }

    fn new(app_handle: Option<AppHandle>) -> Self {
        Self {
            app_handle,
            streamer_pipe: Arc::new(StreamerPipe::new()),
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
        let streamer_pipe_clone = Arc::clone(&self.streamer_pipe);
        let app_handle_clone = self.app_handle.as_ref().unwrap().clone();
        let streamer_join_handle = thread::Builder::new()
            .name(STREAMER_THREAD_NAME.to_string())
            .spawn(move || {
                Streamer::new(streamer_pipe_clone, app_handle_clone, uri_owned).start();
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
        if let Some(streamer_join_handle) = self.get_streamer_join_handle_opt() {
            if !streamer_join_handle.is_finished() {
                return true;
            }

            self.wait_until_end();
        }

        false
    }

    fn wait_until_end(&mut self) {
        if self.streamer_join_handle == null_mut() {
            return;
        }

        let streamer_join_handle = unsafe { ptr::read(self.streamer_join_handle) };
        streamer_join_handle.join().unwrap();
        self.unset_streamer_join_handle();
    }

    fn get_streamer_join_handle_opt(&self) -> Option<&JoinHandle<()>> {
        if self.streamer_join_handle.is_null() {
            return None;
        }

        Some(unsafe { &*self.streamer_join_handle })
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn test_is_active() {
        let mut player = Player::new(None);

        assert!(!player.is_active());

        let faked_streamer_join_handle = thread::Builder::new()
            .spawn(move || {
                thread::sleep(Duration::from_secs(2));
            })
            .unwrap();

        player.set_streamer_join_handle(faked_streamer_join_handle);

        assert!(player.is_active());
        assert!(!player.get_streamer_join_handle_opt().unwrap().is_finished());

        player.wait_until_end();

        assert!(!player.is_active());
        assert!(player.streamer_join_handle.is_null());
    }
}
