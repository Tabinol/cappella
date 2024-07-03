use std::{
    alloc::{alloc, dealloc, Layout},
    fmt::Debug,
    ptr::{self, null_mut},
    sync::{mpsc::Sender, Arc, Mutex},
    thread::{self, JoinHandle},
};

use crate::{
    streamer_loop::StreamerLoop,
    streamer_pipe::{Message, StreamerPipe},
};

#[cfg(test)]
use mockall::automock;

const THREAD_NAME: &str = "streamer";

#[derive(Clone, Debug)]
pub(crate) enum Status {
    None,
    Wait,
    Play(String),
    PlayNext(String),
    End,
}

#[cfg_attr(test, automock)]
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
    status: Arc<Mutex<Status>>,
    sender: Arc<Sender<Status>>,
    join_handle: Arc<Mutex<*mut JoinHandle<()>>>,
}

unsafe impl Send for ImplStreamer {}
unsafe impl Sync for ImplStreamer {}

impl ImplStreamer {
    pub(crate) fn new(
        streamer_pipe: Arc<dyn StreamerPipe>,
        streamer_loop: Arc<dyn StreamerLoop>,
        status: Arc<Mutex<Status>>,
        sender: Arc<Sender<Status>>,
    ) -> Self {
        Self {
            streamer_pipe,
            streamer_loop,
            status,
            sender,
            join_handle: Arc::new(Mutex::new(null_mut())),
        }
    }
}

impl Streamer for ImplStreamer {
    fn is_running(&self) -> bool {
        matches!(&*self.status.lock().unwrap(), Status::Play(_))
    }

    fn start_thread(&self) {
        let streamer_loop = Arc::clone(&self.streamer_loop);
        let join_handle = thread::Builder::new()
            .name(THREAD_NAME.to_string())
            .spawn(move || {
                streamer_loop.run();
            })
            .unwrap();

        unsafe {
            let mut join_handle_lock = self.join_handle.lock().unwrap();
            *join_handle_lock = alloc(Layout::new::<JoinHandle<()>>()) as *mut JoinHandle<()>;
            ptr::write(*join_handle_lock, join_handle);
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
        if !join_handle_lock.is_null() {
            if matches!(&*self.status.lock().unwrap(), Status::Play(_)) {
                self.streamer_pipe.send(Message::End);
            } else {
                self.sender.send(Status::End).unwrap();
            }

            unsafe {
                ptr::read(*join_handle_lock).join().unwrap();
                dealloc(
                    *join_handle_lock as *mut u8,
                    Layout::new::<JoinHandle<()>>(),
                );
                *join_handle_lock = null_mut();
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        alloc::{dealloc, Layout},
        ptr::{self, null_mut},
        sync::{mpsc::channel, Arc, Mutex},
        thread::JoinHandle,
    };

    use crate::{
        streamer::Streamer,
        streamer_loop::MockStreamerLoop,
        streamer_pipe::{Message, MockStreamerPipe},
    };

    use super::{ImplStreamer, Status};

    #[test]
    fn test_is_running_false() {
        let streamer_pipe = MockStreamerPipe::new();
        let streamer_loop = MockStreamerLoop::new();
        let (sender, _receiver) = channel::<Status>();

        let streamer = ImplStreamer::new(
            Arc::new(streamer_pipe),
            Arc::new(streamer_loop),
            Arc::new(Mutex::new(Status::Wait)),
            Arc::new(sender),
        );

        assert!(!streamer.is_running());
    }

    #[test]
    fn test_is_running_true() {
        let streamer_pipe = MockStreamerPipe::new();
        let streamer_loop = MockStreamerLoop::new();
        let (sender, _receiver) = channel::<Status>();

        let streamer = ImplStreamer::new(
            Arc::new(streamer_pipe),
            Arc::new(streamer_loop),
            Arc::new(Mutex::new(Status::Play("uri".to_string()))),
            Arc::new(sender),
        );

        assert!(streamer.is_running());
    }

    #[test]
    fn test_start_thread() {
        let streamer_pipe = MockStreamerPipe::new();
        let mut streamer_loop = MockStreamerLoop::new();
        let (sender, receiver) = channel::<Status>();
        let receiver_arc = Arc::new(Mutex::new(receiver));
        let status = Arc::new(Mutex::new(Status::None));
        let status_clone = Arc::clone(&status);

        streamer_loop.expect_run().return_once(move || {
            *status_clone.lock().unwrap() = receiver_arc.lock().unwrap().recv().unwrap();
        });

        let streamer = ImplStreamer::new(
            Arc::new(streamer_pipe),
            Arc::new(streamer_loop),
            Arc::new(Mutex::new(Status::None)),
            Arc::new(sender),
        );

        streamer.start_thread();

        streamer.sender.send(Status::Wait).unwrap();
        let mut join_handle_lock = streamer.join_handle.lock().unwrap();
        unsafe {
            ptr::read(*join_handle_lock).join().unwrap();
            dealloc(
                *join_handle_lock as *mut u8,
                Layout::new::<JoinHandle<()>>(),
            );
            *join_handle_lock = null_mut();
        };

        let status_lock = status.lock().unwrap();

        assert!(matches!(*status_lock, Status::Wait));
    }

    #[test]
    fn test_play_on_wait() {
        let streamer_pipe = MockStreamerPipe::new();
        let mut streamer_loop = MockStreamerLoop::new();
        let (sender, receiver) = channel::<Status>();
        let sender_arc = Arc::new(sender);
        let receiver_arc = Arc::new(Mutex::new(receiver));
        let status = Arc::new(Mutex::new(Status::None));
        let status_clone = Arc::clone(&status);

        streamer_loop.expect_run().return_once(move || {
            *status_clone.lock().unwrap() = receiver_arc.lock().unwrap().recv().unwrap();
        });

        let streamer = ImplStreamer::new(
            Arc::new(streamer_pipe),
            Arc::new(streamer_loop),
            Arc::new(Mutex::new(Status::Wait)),
            sender_arc,
        );

        streamer.start_thread();

        streamer.play("new_uri");

        let mut join_handle_lock = streamer.join_handle.lock().unwrap();
        unsafe {
            ptr::read(*join_handle_lock).join().unwrap();
            dealloc(
                *join_handle_lock as *mut u8,
                Layout::new::<JoinHandle<()>>(),
            );
            *join_handle_lock = null_mut();
        };

        let status_lock = status.lock().unwrap();

        assert!(matches!(*status_lock, Status::Play(_)));
        assert!(if let Status::Play(uri) = &*status_lock {
            uri.eq("new_uri")
        } else {
            false
        });
    }

    #[test]
    fn test_play_on_play() {
        let mut streamer_pipe = MockStreamerPipe::new();
        let streamer_loop = MockStreamerLoop::new();
        let (sender, _receiver) = channel::<Status>();
        let sender_arc = Arc::new(sender);
        let message = Arc::new(Mutex::new(Message::None));
        let message_clone = Arc::clone(&message);

        streamer_pipe.expect_send().return_once(move |message| {
            *message_clone.lock().unwrap() = message;
        });

        let streamer = ImplStreamer::new(
            Arc::new(streamer_pipe),
            Arc::new(streamer_loop),
            Arc::new(Mutex::new(Status::Play("old_uri".to_string()))),
            sender_arc,
        );

        streamer.play("new_uri");

        let message_lock = message.lock().unwrap();

        assert!(matches!(*message_lock, Message::Next(_)));
        assert!(if let Message::Next(uri) = &*message_lock {
            uri.eq("new_uri")
        } else {
            false
        });
    }

    #[test]
    fn test_end_on_play() {
        let mut streamer_pipe = MockStreamerPipe::new();
        let mut streamer_loop = MockStreamerLoop::new();
        let (sender, receiver) = channel::<Status>();
        let sender_arc = Arc::new(sender);
        let sender_clone = Arc::clone(&sender_arc);
        let receiver_arc = Arc::new(Mutex::new(receiver));
        let status = Arc::new(Mutex::new(Status::None));
        let status_clone = Arc::clone(&status);
        let message = Arc::new(Mutex::new(Message::None));
        let message_clone = Arc::clone(&message);

        streamer_pipe.expect_send().return_once(move |message| {
            *message_clone.lock().unwrap() = message;
            sender_clone.send(Status::End).unwrap();
        });
        streamer_loop.expect_run().return_once(move || {
            *status_clone.lock().unwrap() = receiver_arc.lock().unwrap().recv().unwrap();
        });

        let streamer = ImplStreamer::new(
            Arc::new(streamer_pipe),
            Arc::new(streamer_loop),
            Arc::new(Mutex::new(Status::Play("uri".to_owned()))),
            sender_arc,
        );

        streamer.start_thread();

        streamer.end();

        let status_lock = status.lock().unwrap();
        let message_lock = message.lock().unwrap();

        assert!(matches!(*status_lock, Status::End));
        assert!(matches!(*message_lock, Message::End));
    }

    #[test]
    fn test_end_on_wait() {
        let streamer_pipe = MockStreamerPipe::new();
        let mut streamer_loop = MockStreamerLoop::new();
        let (sender, receiver) = channel::<Status>();
        let sender_arc = Arc::new(sender);
        let receiver_arc = Arc::new(Mutex::new(receiver));
        let status = Arc::new(Mutex::new(Status::None));
        let status_clone = Arc::clone(&status);

        streamer_loop.expect_run().return_once(move || {
            *status_clone.lock().unwrap() = receiver_arc.lock().unwrap().recv().unwrap();
        });

        let streamer = ImplStreamer::new(
            Arc::new(streamer_pipe),
            Arc::new(streamer_loop),
            Arc::new(Mutex::new(Status::Wait)),
            sender_arc,
        );

        streamer.start_thread();

        streamer.end();

        let status_lock = status.lock().unwrap();

        assert!(matches!(*status_lock, Status::End));
    }
}
