use std::{
    fmt::Debug,
    sync::{
        mpsc::{channel, Sender},
        Arc, Mutex, MutexGuard, OnceLock,
    },
    thread::{self, JoinHandle},
};

use crate::player::streamer_loop::Status;

use super::{
    streamer_loop::StreamerLoop,
    streamer_pipe::{self, StreamerPipe},
};

const THREAD_NAME: &str = "streamer";

#[derive(Clone, Debug, Default)]
pub(crate) enum Message {
    #[default]
    None,
    Play(String),
    End,
}

pub(crate) trait Streamer: Debug + Send + Sync {
    fn is_running(&self) -> bool;
    fn start_thread(&self);
    fn play(&self, uri: &str);
    fn end(&self);
}

#[derive(Debug)]
pub(crate) struct ImplStreamer {
    streamer_pipe: Arc<dyn StreamerPipe>,
    streamer_loop: Arc<dyn StreamerLoop>,
    sender: OnceLock<Sender<Message>>,
    join_handle: Mutex<Option<JoinHandle<()>>>,
}

impl ImplStreamer {
    pub(crate) fn new(
        streamer_pipe: Arc<dyn StreamerPipe>,
        streamer_loop: Arc<dyn StreamerLoop>,
    ) -> ImplStreamer {
        Self {
            streamer_pipe,
            streamer_loop,
            sender: OnceLock::default(),
            join_handle: Mutex::default(),
        }
    }

    fn sender(&self) -> &Sender<Message> {
        self.sender
            .get()
            .expect("Message sender to gstreamer is not initialized.")
    }

    fn join_handle_try_lock(&self) -> MutexGuard<Option<JoinHandle<()>>> {
        self.join_handle
            .try_lock()
            .expect("The streamer join handle is already locked.")
    }
}

unsafe impl Send for ImplStreamer {}
unsafe impl Sync for ImplStreamer {}

impl Streamer for ImplStreamer {
    fn is_running(&self) -> bool {
        matches!(self.streamer_loop.status(), Status::Play(_))
    }

    fn start_thread(&self) {
        let (sender, receiver) = channel::<Message>();

        self.sender
            .set(sender)
            .expect("Cannot start gstreamer thread because the message sender is already set.");

        let streamer_loop = self.streamer_loop.clone();

        let mut join_handle = self.join_handle_try_lock();

        if join_handle.is_some() {
            panic!("The streamer thread is already started.");
        }

        *join_handle = Some(
            thread::Builder::new()
                .name(THREAD_NAME.to_string())
                .spawn(move || {
                    streamer_loop.run(receiver);
                })
                .unwrap(),
        );
    }

    fn play(&self, uri: &str) {
        if matches!(self.streamer_loop.status(), Status::Play(_)) {
            self.streamer_pipe
                .send(streamer_pipe::Message::Next(uri.to_owned()));
        } else {
            self.sender().send(Message::Play(uri.to_owned())).unwrap();
        }
    }

    fn end(&self) {
        let mut join_handle_lock = self.join_handle_try_lock();

        if let Some(join_handle) = join_handle_lock.take() {
            if !join_handle.is_finished() {
                if matches!(
                    self.streamer_loop.status(),
                    Status::Play(_) | Status::PlayNext(_)
                ) {
                    self.streamer_pipe.send(streamer_pipe::Message::Stop);
                }

                self.sender().send(Message::End).unwrap();
                join_handle.join().unwrap();
            } else {
                eprintln!("The streamer thread is already stopped.")
            }
        } else {
            eprintln!("The streamer thread is never started.");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{mpsc::Receiver, Arc, Mutex, RwLock},
        time::Duration,
    };

    use crate::player::{
        streamer::{self, ImplStreamer, Streamer},
        streamer_loop::StreamerLoop,
        streamer_pipe::{Message, StreamerPipe},
    };

    use super::Status;

    #[derive(Debug, Default)]
    struct MockStreamerPipe {
        last_message: RwLock<Message>,
    }

    impl StreamerPipe for MockStreamerPipe {
        fn send(&self, message: Message) {
            *self.last_message.write().unwrap() = message;
        }
    }

    #[derive(Debug, Default)]
    struct MockStreamerLoop {
        status: RwLock<Status>,
        last_message: RwLock<streamer::Message>,
        receiver: Mutex<Option<Receiver<super::Message>>>,
    }

    impl StreamerLoop for MockStreamerLoop {
        fn run(&self, receiver: Receiver<super::Message>) {
            let mut receiver_lock = self.receiver.lock().unwrap();
            *receiver_lock = Some(receiver);
            let last_message = (*receiver_lock)
                .as_ref()
                .unwrap()
                .recv_timeout(Duration::from_secs(10))
                .expect("Message timeout.");
            *self.last_message.write().unwrap() = last_message;
        }

        fn status(&self) -> Status {
            (*self.status.read().unwrap()).clone()
        }
    }

    #[test]
    fn test_is_running_false() {
        let streamer_pipe = Arc::<MockStreamerPipe>::default();
        let streamer_loop = Arc::<MockStreamerLoop>::default();

        *streamer_loop.status.write().unwrap() = Status::Wait;
        let streamer = ImplStreamer::new(streamer_pipe.clone(), streamer_loop.clone());

        assert!(!streamer.is_running());
    }

    #[test]
    fn test_is_running_true() {
        let streamer_pipe = Arc::<MockStreamerPipe>::default();
        let streamer_loop = Arc::<MockStreamerLoop>::default();

        *streamer_loop.status.write().unwrap() = Status::Play("uri".to_string());
        let streamer = ImplStreamer::new(streamer_pipe.clone(), streamer_loop.clone());

        assert!(streamer.is_running());
    }

    #[test]
    fn test_start_thread() {
        let streamer_pipe = Arc::<MockStreamerPipe>::default();
        let streamer_loop = Arc::<MockStreamerLoop>::default();

        let streamer = ImplStreamer::new(streamer_pipe.clone(), streamer_loop.clone());

        streamer.start_thread();

        streamer
            .sender
            .get()
            .unwrap()
            .send(streamer::Message::End)
            .unwrap();

        let join_handle = streamer.join_handle.lock().unwrap().take().unwrap();
        join_handle.join().unwrap();
        let last_message = (*streamer_loop.last_message.read().unwrap()).clone();

        assert!(matches!(last_message, streamer::Message::End));
    }

    #[test]
    fn test_play_on_wait() {
        let streamer_pipe = Arc::<MockStreamerPipe>::default();
        let streamer_loop = Arc::<MockStreamerLoop>::default();

        *streamer_loop.status.write().unwrap() = Status::Wait;
        let streamer = ImplStreamer::new(streamer_pipe.clone(), streamer_loop.clone());

        streamer.start_thread();

        streamer.play("new_uri");

        let join_handle = streamer.join_handle.lock().unwrap().take().unwrap();
        join_handle.join().unwrap();
        let status = (*streamer_loop.status.read().unwrap()).clone();

        assert!(matches!(status, Status::Play(_)));
        assert!(if let Status::Play(uri) = status {
            uri.eq("new_uri")
        } else {
            false
        });
    }

    #[test]
    fn test_play_on_play() {
        let streamer_pipe = Arc::<MockStreamerPipe>::default();
        let streamer_loop = Arc::<MockStreamerLoop>::default();

        *streamer_loop.status.write().unwrap() = Status::Play("old_uri".to_string());
        let streamer = ImplStreamer::new(streamer_pipe.clone(), streamer_loop.clone());

        streamer.play("new_uri");

        let message = (*streamer_pipe.last_message.read().unwrap()).clone();

        assert!(matches!(message, Message::Next(_)));
        assert!(if let Message::Next(uri) = message {
            uri.eq("new_uri")
        } else {
            false
        });
    }

    #[test]
    fn test_end_on_play() {
        let streamer_pipe = Arc::<MockStreamerPipe>::default();
        let streamer_loop = Arc::<MockStreamerLoop>::default();

        *streamer_loop.status.write().unwrap() = Status::Play("uri".to_owned());
        let streamer = ImplStreamer::new(streamer_pipe.clone(), streamer_loop.clone());

        streamer.start_thread();

        streamer.end();

        let status = (*streamer_loop.status.read().unwrap()).clone();
        let message = (*streamer_pipe.last_message.read().unwrap()).clone();

        assert!(matches!(status, Status::End));
        assert!(matches!(message, Message::Stop));
    }

    #[test]
    fn test_end_on_wait() {
        let streamer_pipe = Arc::<MockStreamerPipe>::default();
        let streamer_loop = Arc::<MockStreamerLoop>::default();

        *streamer_loop.status.write().unwrap() = Status::Wait;
        let streamer = ImplStreamer::new(streamer_pipe.clone(), streamer_loop.clone());

        streamer.start_thread();

        streamer.end();

        let status = (*streamer_loop.status.read().unwrap()).clone();

        assert!(matches!(status, Status::End));
    }
}
