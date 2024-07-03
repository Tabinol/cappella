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

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use crate::{
        player::ImplPlayer,
        streamer::MockStreamer,
        streamer_pipe::{Message, MockStreamerPipe},
    };

    use super::Player;

    #[test]
    fn test_play_when_streamer_inactive() {
        let sent_uri: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
        let sent_uri_clone: Arc<Mutex<String>> = Arc::clone(&sent_uri);
        let mut streamer = MockStreamer::new();
        let streamer_pipe = MockStreamerPipe::new();
        streamer.expect_is_running().return_const(false);
        streamer
            .expect_play()
            .return_once(move |uri| *sent_uri_clone.lock().unwrap() = uri.to_string());

        let player = ImplPlayer::new(Arc::new(streamer), Arc::new(streamer_pipe));

        player.play("testuri");

        let sent_uri_lock = sent_uri.lock().unwrap();
        assert_eq!(&*sent_uri_lock, "testuri");
    }

    #[test]
    fn test_play_when_streamer_active() {
        let sent_message: Arc<Mutex<Message>> = Arc::new(Mutex::new(Message::None));
        let sent_message_clone = Arc::clone(&sent_message);
        let mut streamer = MockStreamer::new();
        let mut streamer_pipe = MockStreamerPipe::new();
        streamer.expect_is_running().return_const(true);
        streamer_pipe
            .expect_send()
            .return_once(move |message| *sent_message_clone.lock().unwrap() = message);

        let player = ImplPlayer::new(Arc::new(streamer), Arc::new(streamer_pipe));

        player.play("testuri");

        let sent_message_lock = &*sent_message.lock().unwrap();
        assert!(matches!(sent_message_lock, Message::Next(_)));
        assert!(if let Message::Next(uri) = sent_message_lock {
            uri.eq("testuri")
        } else {
            false
        });
    }

    #[test]
    fn test_pause() {
        let sent_message: Arc<Mutex<Message>> = Arc::new(Mutex::new(Message::None));
        let sent_message_clone = Arc::clone(&sent_message);
        let mut streamer = MockStreamer::new();
        let mut streamer_pipe = MockStreamerPipe::new();
        streamer.expect_is_running().return_const(true);
        streamer_pipe
            .expect_send()
            .return_once(move |message| *sent_message_clone.lock().unwrap() = message);

        let player = ImplPlayer::new(Arc::new(streamer), Arc::new(streamer_pipe));

        player.pause();

        let sent_message_lock = &*sent_message.lock().unwrap();
        assert!(matches!(sent_message_lock, Message::Pause));
    }

    #[test]
    fn test_stop() {
        let sent_message: Arc<Mutex<Message>> = Arc::new(Mutex::new(Message::None));
        let sent_message_clone = Arc::clone(&sent_message);
        let mut streamer = MockStreamer::new();
        let mut streamer_pipe = MockStreamerPipe::new();
        streamer.expect_is_running().return_const(true);
        streamer_pipe
            .expect_send()
            .return_once(move |message| *sent_message_clone.lock().unwrap() = message);

        let player = ImplPlayer::new(Arc::new(streamer), Arc::new(streamer_pipe));

        player.stop();

        let sent_message_lock = &*sent_message.lock().unwrap();
        assert!(matches!(sent_message_lock, Message::Stop));
    }
}
