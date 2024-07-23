use std::{
    fmt::Debug,
    sync::{
        mpsc::{Receiver, Sender},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
};

use super::{
    streamer_loop::StreamerLoop,
    streamer_pipe::{Message, StreamerPipe},
};

const THREAD_NAME: &str = "streamer";

#[derive(Clone, Debug, Default)]
pub(crate) enum Status {
    #[default]
    None,
    Wait,
    Play(String),
    PlayNext(String),
    End,
}

pub(crate) trait Streamer: Debug + Send + Sync {
    fn is_running(&self) -> bool;
    fn start_thread(&self, receiver: Receiver<Status>);
    fn play(&self, uri: &str);
    fn end(&self);
}
#[derive(Debug)]
pub(crate) struct ImplStreamer {
    streamer_pipe: Arc<dyn StreamerPipe>,
    streamer_loop: Arc<dyn StreamerLoop>,
    status: Arc<Mutex<Status>>,
    sender: Sender<Status>,
    join_handle: Mutex<Option<JoinHandle<()>>>,
}

impl ImplStreamer {
    pub(crate) fn new(
        streamer_pipe: Arc<dyn StreamerPipe>,
        streamer_loop: Arc<dyn StreamerLoop>,
        status: Arc<Mutex<Status>>,
        sender: Sender<Status>,
    ) -> ImplStreamer {
        Self {
            streamer_pipe,
            streamer_loop,
            status,
            sender,
            join_handle: Mutex::default(),
        }
    }
}

impl Streamer for ImplStreamer {
    fn is_running(&self) -> bool {
        matches!(&*self.status.lock().unwrap(), Status::Play(_))
    }

    fn start_thread(&self, receiver: Receiver<Status>) {
        let streamer_loop = self.streamer_loop.clone();
        let mut join_handle_lock = self.join_handle.lock().unwrap();

        if join_handle_lock.is_some() {
            panic!("The streamer thread is already started !");
        }

        *join_handle_lock = Some(
            thread::Builder::new()
                .name(THREAD_NAME.to_string())
                .spawn(move || {
                    streamer_loop.run(receiver);
                })
                .unwrap(),
        );
    }

    fn play(&self, uri: &str) {
        if matches!(&*self.status.lock().unwrap(), Status::Play(_)) {
            self.streamer_pipe.send(Message::Next(uri.to_owned()));
        } else {
            self.sender.send(Status::Play(uri.to_owned())).unwrap();
        }
    }

    fn end(&self) {
        let mut join_handle_lock = self.join_handle.lock().unwrap();

        if join_handle_lock.is_none() {
            return;
        }

        let join_handle = join_handle_lock.take().unwrap();
        if !join_handle.is_finished() {
            if matches!(
                &*self.status.lock().unwrap(),
                Status::Play(_) | Status::PlayNext(_)
            ) {
                self.streamer_pipe.send(Message::End);
            } else {
                self.sender.send(Status::End).unwrap();
            }

            join_handle.join().unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        mpsc::{channel, Receiver},
        Arc, Mutex,
    };

    use crate::player::{
        streamer::{ImplStreamer, Streamer},
        streamer_loop::StreamerLoop,
        streamer_pipe::{Message, StreamerPipe},
    };

    use super::Status;

    #[derive(Debug, Default)]
    struct MockStreamerPipe {
        last_message: Mutex<Option<Message>>,
    }

    impl StreamerPipe for MockStreamerPipe {
        fn send(&self, message: Message) {
            *self.last_message.lock().unwrap() = Some(message);
        }
    }

    #[derive(Debug, Default)]
    struct MockStreamerLoop {
        status: Arc<Mutex<Status>>,
    }

    impl StreamerLoop for MockStreamerLoop {
        fn run(&self, receiver: Receiver<Status>) {
            *self.status.lock().unwrap() = receiver.recv().unwrap();
            println!("status={:?}", *self.status.lock().unwrap());
        }
    }

    #[test]
    fn test_is_running_false() {
        let streamer_pipe = Arc::new(MockStreamerPipe::default());
        let streamer_loop = Arc::new(MockStreamerLoop::default());
        let (sender, _receiver) = channel::<Status>();

        *streamer_loop.status.lock().unwrap() = Status::Wait;
        let streamer = ImplStreamer::new(
            streamer_pipe.clone(),
            streamer_loop.clone(),
            streamer_loop.status.clone(),
            sender,
        );

        assert!(!streamer.is_running());
    }

    #[test]
    fn test_is_running_true() {
        let streamer_pipe = Arc::new(MockStreamerPipe::default());
        let streamer_loop = Arc::new(MockStreamerLoop::default());
        let (sender, _receiver) = channel::<Status>();

        *streamer_loop.status.lock().unwrap() = Status::Play("uri".to_string());
        let streamer = ImplStreamer::new(
            streamer_pipe.clone(),
            streamer_loop.clone(),
            streamer_loop.status.clone(),
            sender,
        );

        assert!(streamer.is_running());
    }

    #[test]
    fn test_start_thread() {
        let streamer_pipe = Arc::new(MockStreamerPipe::default());
        let streamer_loop = Arc::new(MockStreamerLoop::default());
        let (sender, receiver) = channel::<Status>();

        let streamer = ImplStreamer::new(
            streamer_pipe.clone(),
            streamer_loop.clone(),
            streamer_loop.status.clone(),
            sender,
        );

        streamer.start_thread(receiver);

        streamer.sender.send(Status::Wait).unwrap();

        let join_handle = streamer.join_handle.lock().unwrap().take().unwrap();
        join_handle.join().unwrap();
        let status_lock = streamer.status.lock().unwrap();

        assert!(matches!(*status_lock, Status::Wait));
    }

    #[test]
    fn test_play_on_wait() {
        let streamer_pipe = Arc::new(MockStreamerPipe::default());
        let streamer_loop = Arc::new(MockStreamerLoop::default());
        let (sender, receiver) = channel::<Status>();

        *streamer_loop.status.lock().unwrap() = Status::Wait;
        let streamer = ImplStreamer::new(
            streamer_pipe.clone(),
            streamer_loop.clone(),
            streamer_loop.status.clone(),
            sender,
        );

        streamer.start_thread(receiver);

        streamer.play("new_uri");

        let join_handle = streamer.join_handle.lock().unwrap().take().unwrap();
        join_handle.join().unwrap();
        let status_lock = streamer.status.lock().unwrap();

        assert!(matches!(*status_lock, Status::Play(_)));
        assert!(if let Status::Play(uri) = &*status_lock {
            uri.eq("new_uri")
        } else {
            false
        });
    }

    #[test]
    fn test_play_on_play() {
        let streamer_pipe = Arc::new(MockStreamerPipe::default());
        let streamer_loop = Arc::new(MockStreamerLoop::default());
        let (sender, _receiver) = channel::<Status>();

        *streamer_loop.status.lock().unwrap() = Status::Play("old_uri".to_string());
        let streamer = ImplStreamer::new(
            streamer_pipe.clone(),
            streamer_loop.clone(),
            streamer_loop.status.clone(),
            sender,
        );

        streamer.play("new_uri");

        let message_lock = streamer_pipe.last_message.lock().unwrap();

        assert!(message_lock.is_some());
        let message = message_lock.as_ref().unwrap();
        assert!(matches!(message, Message::Next(_)));
        assert!(if let Message::Next(uri) = message {
            uri.eq("new_uri")
        } else {
            false
        });
    }

    #[test]
    fn test_end_on_play() {
        let streamer_pipe = Arc::new(MockStreamerPipe::default());
        let streamer_loop = Arc::new(MockStreamerLoop::default());
        let (_sender, receiver) = channel::<Status>();

        let (fake_sender, _fake_receiver) = channel::<Status>();

        *streamer_loop.status.lock().unwrap() = Status::Play("uri".to_owned());
        let streamer = ImplStreamer::new(
            streamer_pipe.clone(),
            streamer_loop.clone(),
            streamer_loop.status.clone(),
            fake_sender,
        );

        streamer.start_thread(receiver);

        streamer.end();

        let status_lock = streamer.status.lock().unwrap();
        let message_lock = streamer_pipe.last_message.lock().unwrap();

        assert!(matches!(*status_lock, Status::End));
        assert!(message_lock.is_some());
        assert!(matches!(message_lock.as_ref().unwrap(), Message::End));
    }

    #[test]
    fn test_end_on_wait() {
        let streamer_pipe = Arc::new(MockStreamerPipe::default());
        let streamer_loop = Arc::new(MockStreamerLoop::default());
        let (sender, receiver) = channel::<Status>();

        *streamer_loop.status.lock().unwrap() = Status::Wait;
        let streamer = ImplStreamer::new(
            streamer_pipe.clone(),
            streamer_loop.clone(),
            streamer_loop.status.clone(),
            sender,
        );

        streamer.start_thread(receiver);

        streamer.end();

        let status_lock = streamer.status.lock().unwrap();

        assert!(matches!(*status_lock, Status::End));
    }
}
