use std::fmt::Debug;

use dyn_clone::DynClone;

use super::{
    streamer::Streamer,
    streamer_pipe::{Message, StreamerPipe},
};

pub(crate) trait Player: Debug + DynClone + Send + Sync {
    fn play(&self, uri: &str);
    fn pause(&self);
    fn stop(&self);
    fn end(&self);
}

dyn_clone::clone_trait_object!(Player);

pub(crate) fn new_boxed(
    streamer: Box<dyn Streamer>,
    streamer_pipe: Box<dyn StreamerPipe>,
) -> Box<dyn Player> {
    Box::new(Player_::new(streamer, streamer_pipe))
}

#[derive(Clone, Debug)]
struct Player_ {
    streamer: Box<dyn Streamer>,
    streamer_pipe: Box<dyn StreamerPipe>,
}

unsafe impl Send for Player_ {}
unsafe impl Sync for Player_ {}

impl Player_ {
    fn new(streamer: Box<dyn Streamer>, streamer_pipe: Box<dyn StreamerPipe>) -> Self {
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
        streamer::Streamer,
        streamer_pipe::{Message, StreamerPipe},
    };

    #[derive(Clone, Debug)]
    struct MockStreamer {
        is_running: bool,
        uri: Arc<Mutex<String>>,
    }

    impl MockStreamer {
        fn new_boxed(is_running: bool) -> Box<Self> {
            Box::new(Self {
                is_running,
                uri: Arc::default(),
            })
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

    #[derive(Clone, Debug, Default)]
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
        let streamer = MockStreamer::new_boxed(false);
        let streamer_pipe = Box::<MockStreamerPipe>::default();
        let player = super::new_boxed(streamer.clone(), streamer_pipe.clone());

        player.play("test_uri");

        assert_eq!(*streamer.uri.lock().unwrap(), "test_uri");
    }

    #[test]
    fn test_play_when_streamer_active() {
        let streamer = MockStreamer::new_boxed(true);
        let streamer_pipe = Box::<MockStreamerPipe>::default();
        let player = super::new_boxed(streamer.clone(), streamer_pipe.clone());

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
        let streamer = MockStreamer::new_boxed(true);
        let streamer_pipe = Box::<MockStreamerPipe>::default();
        let player = super::new_boxed(streamer.clone(), streamer_pipe.clone());

        player.pause();

        let sent_message_lock = streamer_pipe.message.lock().unwrap();
        assert!(matches!(&*sent_message_lock, Message::Pause));
    }

    #[test]
    fn test_stop() {
        let streamer = MockStreamer::new_boxed(true);
        let streamer_pipe = Box::<MockStreamerPipe>::default();
        let player = super::new_boxed(streamer.clone(), streamer_pipe.clone());

        player.stop();

        let sent_message_lock = streamer_pipe.message.lock().unwrap();
        assert!(matches!(&*sent_message_lock, Message::Stop));
    }
}
