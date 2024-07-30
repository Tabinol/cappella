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

#[derive(Debug)]
pub(crate) struct ImplPlayer {
    streamer: Arc<dyn Streamer>,
    streamer_pipe: Arc<dyn StreamerPipe>,
}

impl ImplPlayer {
    pub(crate) fn new(
        streamer: Arc<dyn Streamer>,
        streamer_pipe: Arc<dyn StreamerPipe>,
    ) -> ImplPlayer {
        Self {
            streamer,
            streamer_pipe,
        }
    }
}

impl Player for ImplPlayer {
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
        player::{ImplPlayer, Player},
        streamer::Streamer,
        streamer_pipe::{Message, StreamerPipe},
    };

    #[derive(Debug)]
    struct MockStreamer {
        is_running: bool,
        uri: Arc<Mutex<String>>,
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

    #[derive(Debug)]
    struct MockStreamerPipe {
        message: Arc<Mutex<Message>>,
    }

    impl StreamerPipe for MockStreamerPipe {
        fn send(&self, message: Message) {
            *self.message.lock().unwrap() = message;
        }
    }

    #[test]
    fn test_play_when_streamer_inactive() {
        let streamer = Arc::new(MockStreamer {
            is_running: false,
            uri: Arc::new(Mutex::new(String::new())),
        });
        let streamer_pipe = Arc::new(MockStreamerPipe {
            message: Arc::new(Mutex::new(Message::None)),
        });

        let player = ImplPlayer::new(streamer.clone(), streamer_pipe.clone());

        player.play("test_uri");

        assert_eq!(*streamer.uri.lock().unwrap(), "test_uri");
    }

    #[test]
    fn test_play_when_streamer_active() {
        let streamer = Arc::new(MockStreamer {
            is_running: true,
            uri: Arc::new(Mutex::new(String::new())),
        });
        let streamer_pipe = Arc::new(MockStreamerPipe {
            message: Arc::new(Mutex::new(Message::None)),
        });

        let player = ImplPlayer::new(streamer.clone(), streamer_pipe.clone());

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
        let streamer = Arc::new(MockStreamer {
            is_running: true,
            uri: Arc::new(Mutex::new(String::new())),
        });
        let streamer_pipe = Arc::new(MockStreamerPipe {
            message: Arc::new(Mutex::new(Message::None)),
        });

        let player = ImplPlayer::new(streamer.clone(), streamer_pipe.clone());

        player.pause();

        let sent_message_lock = streamer_pipe.message.lock().unwrap();
        assert!(matches!(&*sent_message_lock, Message::Pause));
    }

    #[test]
    fn test_stop() {
        let streamer = Arc::new(MockStreamer {
            is_running: true,
            uri: Arc::new(Mutex::new(String::new())),
        });
        let streamer_pipe = Arc::new(MockStreamerPipe {
            message: Arc::new(Mutex::new(Message::None)),
        });

        let player = ImplPlayer::new(streamer.clone(), streamer_pipe.clone());

        player.stop();

        let sent_message_lock = streamer_pipe.message.lock().unwrap();
        assert!(matches!(&*sent_message_lock, Message::Stop));
    }
}
