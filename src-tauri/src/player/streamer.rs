use std::{
    alloc::{alloc, dealloc, Layout},
    fmt::Debug,
    ptr::{self, null_mut},
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
    join_handle: Mutex<*mut JoinHandle<()>>,
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
            join_handle: Mutex::new(null_mut()),
        }
    }
}

unsafe impl Send for ImplStreamer {}
unsafe impl Sync for ImplStreamer {}

impl Streamer for ImplStreamer {
    fn is_running(&self) -> bool {
        matches!(&*self.status.lock().unwrap(), Status::Play(_))
    }

    fn start_thread(&self, receiver: Receiver<Status>) {
        let streamer_loop = self.streamer_loop.clone();
        let mut join_handle = self.join_handle.lock().unwrap();
        unsafe {
            *join_handle = alloc(Layout::new::<JoinHandle<()>>()) as *mut JoinHandle<()>;
            **join_handle = thread::Builder::new()
                .name(THREAD_NAME.to_string())
                .spawn(move || {
                    streamer_loop.run(receiver);
                })
                .unwrap();
        }
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

        unsafe {
            if !join_handle_lock.is_null() {
                let join_handle = ptr::read(*join_handle_lock);

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

                dealloc(
                    *join_handle_lock as *mut u8,
                    Layout::new::<JoinHandle<()>>(),
                );
                *join_handle_lock = null_mut();
            }
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

    #[derive(Debug)]
    struct MockStreamerLoop {
        streamer_thread_lock: Arc<Mutex<()>>,
        status: Arc<Mutex<Status>>,
    }

    impl MockStreamerLoop {
        fn new(streamer_thread_lock: Arc<Mutex<()>>, status: Arc<Mutex<Status>>) -> Self {
            Self {
                streamer_thread_lock,
                status,
            }
        }
    }

    impl StreamerLoop for MockStreamerLoop {
        fn run(&self, receiver: Receiver<Status>) {
            let _streamer_thread_lock = self.streamer_thread_lock.lock().unwrap();
            *self.status.lock().unwrap() = receiver.recv().unwrap();
            println!("status={:?}", *self.status.lock().unwrap());
        }
    }

    #[test]
    fn test_is_running_false() {
        let streamer_thread_lock = Arc::new(Mutex::new(()));
        let status = Arc::new(Mutex::new(Status::Wait));
        let streamer_pipe = Arc::new(MockStreamerPipe::default());
        let streamer_loop = Arc::new(MockStreamerLoop::new(
            streamer_thread_lock.clone(),
            status.clone(),
        ));
        let (sender, _receiver) = channel::<Status>();

        let streamer = ImplStreamer::new(streamer_pipe, streamer_loop, status.clone(), sender);

        assert!(!streamer.is_running());
    }

    #[test]
    fn test_is_running_true() {
        let streamer_thread_lock = Arc::new(Mutex::new(()));
        let status = Arc::new(Mutex::new(Status::Play("uri".to_string())));
        let streamer_pipe = Arc::new(MockStreamerPipe::default());
        let streamer_loop = Arc::new(MockStreamerLoop::new(
            streamer_thread_lock.clone(),
            status.clone(),
        ));
        let (sender, _receiver) = channel::<Status>();

        let streamer = ImplStreamer::new(streamer_pipe, streamer_loop, status.clone(), sender);

        assert!(streamer.is_running());
    }

    #[test]
    fn test_start_thread() {
        let streamer_thread_lock = Arc::new(Mutex::new(()));
        let status = Arc::new(Mutex::new(Status::default()));
        let streamer_pipe = Arc::new(MockStreamerPipe::default());
        let streamer_loop = Arc::new(MockStreamerLoop::new(
            streamer_thread_lock.clone(),
            status.clone(),
        ));
        let (sender, receiver) = channel::<Status>();

        let streamer = ImplStreamer::new(streamer_pipe, streamer_loop, status.clone(), sender);

        streamer.start_thread(receiver);

        streamer.sender.send(Status::Wait).unwrap();
        let _streamer_thread_lock = streamer_thread_lock.lock().unwrap();
        let status_lock = status.lock().unwrap();

        assert!(matches!(*status_lock, Status::Wait));
    }

    // #[test]
    // fn test_play_on_wait() {
    //     let streamer_thread_lock = Arc::new(Mutex::new(()));
    //     let streamer_pipe = Arc::new(MockStreamerPipe::default());
    //     let streamer_loop = Arc::new(MockStreamerLoop::new(streamer_thread_lock.clone()));
    //     let (sender, receiver) = channel::<Status>();

    //     let streamer = ImplStreamer::new(
    //         streamer_pipe,
    //         streamer_loop,
    //         Arc::new(Mutex::new(Status::Wait)),
    //         sender,
    //         streamer_thread_lock.clone(),
    //     );

    //     streamer.start_thread(receiver);

    //     streamer.play("new_uri");

    //     let _streamer_thread_lock = streamer_thread_lock.lock().unwrap();
    //     let status_lock = streamer.status.lock().unwrap();

    //     assert!(matches!(*status_lock, Status::Play(_)));
    //     assert!(if let Status::Play(uri) = &*status_lock {
    //         uri.eq("new_uri")
    //     } else {
    //         false
    //     });
    // }

    // #[test]
    // fn test_play_on_play() {
    //     let streamer_thread_lock = Arc::new(Mutex::new(()));
    //     let streamer_pipe = Arc::new(MockStreamerPipe::default());
    //     let streamer_loop = Arc::new(MockStreamerLoop::new(streamer_thread_lock.clone()));
    //     let (sender, _receiver) = channel::<Status>();

    //     let streamer = ImplStreamer::new(
    //         streamer_pipe.clone(),
    //         streamer_loop,
    //         Arc::new(Mutex::new(Status::Play("old_uri".to_string()))),
    //         sender,
    //         streamer_thread_lock.clone(),
    //     );

    //     streamer.play("new_uri");

    //     let message_lock = streamer_pipe.message.lock().unwrap();

    //     assert!(matches!(*message_lock, Message::Next(_)));
    //     assert!(if let Message::Next(uri) = &*message_lock {
    //         uri.eq("new_uri")
    //     } else {
    //         false
    //     });
    // }

    // #[test]
    // fn test_end_on_play() {
    //     let streamer_thread_lock = Arc::new(Mutex::new(()));
    //     let streamer_pipe = Arc::new(MockStreamerPipe::default());
    //     let streamer_loop = Arc::new(MockStreamerLoop::new(streamer_thread_lock.clone()));
    //     let (sender, receiver) = channel::<Status>();

    //     *streamer_pipe.sender.lock().unwrap() = Some(sender);
    //     *streamer_pipe.next_status.lock().unwrap() = Some(Status::End);

    //     let (fake_sender, _fake_receiver) = channel::<Status>();

    //     let streamer = ImplStreamer::new(
    //         streamer_pipe.clone(),
    //         streamer_loop,
    //         Arc::new(Mutex::new(Status::Play("uri".to_owned()))),
    //         fake_sender,
    //         streamer_thread_lock.clone(),
    //     );

    //     streamer.start_thread(receiver);

    //     streamer.end();

    //     let status_lock = streamer.status.lock().unwrap();
    //     let message_lock = streamer_pipe.message.lock().unwrap();

    //     assert!(matches!(*status_lock, Status::End));
    //     assert!(matches!(*message_lock, Message::End));
    // }

    // #[test]
    // fn test_end_on_wait() {
    //     let streamer_thread_lock = Arc::new(Mutex::new(()));
    //     let streamer_pipe = Arc::new(MockStreamerPipe::default());
    //     let streamer_loop = Arc::new(MockStreamerLoop::new(streamer_thread_lock.clone()));
    //     let (sender, receiver) = channel::<Status>();

    //     let streamer = ImplStreamer::new(
    //         streamer_pipe,
    //         streamer_loop,
    //         Arc::new(Mutex::new(Status::Wait)),
    //         sender,
    //         streamer_thread_lock.clone(),
    //     );

    //     streamer.start_thread(receiver);

    //     streamer.end();

    //     let status_lock = streamer.status.lock().unwrap();

    //     assert!(matches!(*status_lock, Status::End));
    // }
}
