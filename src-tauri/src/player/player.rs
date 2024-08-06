use std::{fmt::Debug, sync::Arc};

use super::{
    streamer::Streamer,
    streamer_pipe::{Message, StreamerPipe},
};

pub(crate) trait Player: Debug + Send + Sync {
    fn play(&self, uri: &str);
    fn pause(&self);
    fn stop(&self);
    fn end(&self);
}

pub(crate) fn new_arc(
    streamer: Arc<dyn Streamer>,
    streamer_pipe: Arc<dyn StreamerPipe>,
) -> Arc<dyn Player> {
    Arc::new(Player_::new(streamer, streamer_pipe))
}

#[derive(Debug)]
struct Player_ {
    streamer: Arc<dyn Streamer>,
    streamer_pipe: Arc<dyn StreamerPipe>,
}

unsafe impl Send for Player_ {}
unsafe impl Sync for Player_ {}

impl Player_ {
    fn new(streamer: Arc<dyn Streamer>, streamer_pipe: Arc<dyn StreamerPipe>) -> Self {
        Self {
            streamer,
            streamer_pipe,
        }
    }
}

impl Player for Player_ {
    fn play(&self, uri: &str) {
        if self.streamer.is_running() {
            self.streamer_pipe.send(Message::Next(uri.to_owned()));
            return;
        }

        self.streamer.play(uri);
    }

    fn pause(&self) {
        if self.streamer.is_running() {
            self.streamer_pipe.send(Message::Pause);
        }
    }

    fn stop(&self) {
        if self.streamer.is_running() {
            self.streamer_pipe.send(Message::Stop);
        }
    }

    fn end(&self) {
        self.streamer.end();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use crate::player::{
        player::{Player, Player_},
        streamer::Streamer,
        streamer_pipe::{Message, StreamerPipe},
    };

    #[derive(Debug)]
    struct MockStreamer {
        is_running: bool,
        uri: Mutex<String>,
    }

    impl MockStreamer {
        fn new(is_running: bool) -> Self {
            Self {
                is_running,
                uri: Mutex::default(),
            }
        }
    }

    impl Streamer for MockStreamer {
        fn is_running(&self) -> bool {
            self.is_running
        }

        fn start_thread(&self) {}

        fn play(&self, uri: &str) {
            *self.uri.lock().unwrap() = uri.to_string();
        }

        fn end(&self) {}
    }

    #[derive(Debug, Default)]
    struct MockStreamerPipe {
        message: Mutex<Message>,
    }

    impl StreamerPipe for MockStreamerPipe {
        fn send(&self, message: Message) {
            *self.message.lock().unwrap() = message;
        }
    }

    #[test]
    fn test_play_when_streamer_inactive() {
        let streamer = Arc::new(MockStreamer::new(false));
        let streamer_pipe = Arc::<MockStreamerPipe>::default();
        let player = Player_::new(streamer.clone(), streamer_pipe.clone());

        player.play("test_uri");

        assert_eq!(*streamer.uri.lock().unwrap(), "test_uri");
    }

    #[test]
    fn test_play_when_streamer_active() {
        let streamer = Arc::new(MockStreamer::new(true));
        let streamer_pipe = Arc::<MockStreamerPipe>::default();
        let player = Player_::new(streamer.clone(), streamer_pipe.clone());

        player.play("test_uri");

        let sent_message_lock = streamer_pipe.message.lock().unwrap();
        assert!(matches!(&*sent_message_lock, Message::Next(_)));
        assert!(if let Message::Next(uri) = &*sent_message_lock {
            uri.eq("test_uri")
        } else {
            false
        });
    }

    #[test]
    fn test_pause() {
        let streamer = Arc::new(MockStreamer::new(true));
        let streamer_pipe = Arc::<MockStreamerPipe>::default();
        let player = Player_::new(streamer.clone(), streamer_pipe.clone());

        player.pause();

        let sent_message_lock = streamer_pipe.message.lock().unwrap();
        assert!(matches!(&*sent_message_lock, Message::Pause));
    }

    #[test]
    fn test_stop() {
        let streamer = Arc::new(MockStreamer::new(true));
        let streamer_pipe = Arc::<MockStreamerPipe>::default();
        let player = Player_::new(streamer.clone(), streamer_pipe.clone());

        player.stop();

        let sent_message_lock = streamer_pipe.message.lock().unwrap();
        assert!(matches!(&*sent_message_lock, Message::Stop));
    }
}
