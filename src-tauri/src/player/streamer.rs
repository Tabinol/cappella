use std::{
    fmt::Debug,
    sync::{
        mpsc::{Receiver, Sender},
        Arc, Mutex,
    },
    thread::{self},
};

use dyn_clone::DynClone;

use super::{
    streamer_loop::StreamerLoop,
    streamer_pipe::{Message, StreamerPipe},
};

const THREAD_NAME: &str = "streamer";

#[derive(Clone, Debug)]
pub(crate) enum Status {
    None,
    Wait,
    Play(String),
    PlayNext(String),
    End,
}

pub(crate) trait Streamer: Debug + DynClone + Send + Sync {
    fn is_running(&self) -> bool;
    fn start_thread(&self, receiver: Receiver<Status>);
    fn play(&self, uri: &str);
    fn end(&self);
}

dyn_clone::clone_trait_object!(Streamer);

#[derive(Clone, Debug)]
pub(crate) struct ImplStreamer {
    streamer_pipe: Box<dyn StreamerPipe>,
    streamer_loop: Box<dyn StreamerLoop>,
    status: Arc<Mutex<Status>>,
    sender: Sender<Status>,
    streamer_thread_lock: Arc<Mutex<()>>,
}

impl ImplStreamer {
    pub(crate) fn new(
        streamer_pipe: Box<dyn StreamerPipe>,
        streamer_loop: Box<dyn StreamerLoop>,
        status: Arc<Mutex<Status>>,
        sender: Sender<Status>,
        streamer_thread_lock: Arc<Mutex<()>>,
    ) -> Box<dyn Streamer> {
        Box::new(Self {
            streamer_pipe,
            streamer_loop,
            status,
            sender,
            streamer_thread_lock,
        })
    }
}

impl Streamer for ImplStreamer {
    fn is_running(&self) -> bool {
        matches!(&*self.status.lock().unwrap(), Status::Play(_))
    }

    fn start_thread(&self, receiver: Receiver<Status>) {
        let streamer_loop = self.streamer_loop.clone();
        thread::Builder::new()
            .name(THREAD_NAME.to_string())
            .spawn(move || {
                streamer_loop.run(receiver);
            })
            .unwrap();
    }

    fn play(&self, uri: &str) {
        if matches!(&*self.status.lock().unwrap(), Status::Play(_)) {
            self.streamer_pipe.send(Message::Next(uri.to_owned()));
        } else {
            self.sender.send(Status::Play(uri.to_owned())).unwrap();
        }
    }

    fn end(&self) {
        if self.streamer_thread_lock.try_lock().is_err() {
            if matches!(&*self.status.lock().unwrap(), Status::Play(_)) {
                self.streamer_pipe.send(Message::End);
            } else {
                self.sender.send(Status::End).unwrap();
            }

            let _streamer_thread_lock = self.streamer_thread_lock.lock().unwrap();
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use super::{ImplStreamer, Status};

//     use crate::{
//         streamer::Streamer,
//         streamer_loop::MockStreamerLoop,
//         streamer_pipe::{Message, MockStreamerPipe},
//     };

//     use std::sync::{mpsc::channel, Arc, Mutex};

//     #[test]
//     fn test_is_running_false() {
//         let streamer_pipe = MockStreamerPipe::new();
//         let streamer_loop = MockStreamerLoop::new();
//         let (sender, _receiver) = channel::<Status>();

//         let streamer = ImplStreamer::new(
//             Arc::new(streamer_pipe),
//             Arc::new(streamer_loop),
//             Arc::new(Mutex::new(Status::Wait)),
//             Arc::new(sender),
//             Arc::new(Mutex::new(())),
//         );

//         assert!(!streamer.is_running());
//     }

//     #[test]
//     fn test_is_running_true() {
//         let streamer_pipe = MockStreamerPipe::new();
//         let streamer_loop = MockStreamerLoop::new();
//         let (sender, _receiver) = channel::<Status>();

//         let streamer = ImplStreamer::new(
//             Arc::new(streamer_pipe),
//             Arc::new(streamer_loop),
//             Arc::new(Mutex::new(Status::Play("uri".to_string()))),
//             Arc::new(sender),
//             Arc::new(Mutex::new(())),
//         );

//         assert!(streamer.is_running());
//     }

//     #[test]
//     fn test_start_thread() {
//         let streamer_pipe = MockStreamerPipe::new();
//         let mut streamer_loop = MockStreamerLoop::new();
//         let (sender, receiver) = channel::<Status>();
//         let receiver_arc = Arc::new(Mutex::new(receiver));
//         let status = Arc::new(Mutex::new(Status::None));
//         let status_clone = Arc::clone(&status);

//         streamer_loop.expect_run().return_once(move || {
//             *status_clone.lock().unwrap() = receiver_arc.lock().unwrap().recv().unwrap();
//         });

//         let streamer = ImplStreamer::new(
//             Arc::new(streamer_pipe),
//             Arc::new(streamer_loop),
//             Arc::new(Mutex::new(Status::None)),
//             Arc::new(sender),
//             Arc::new(Mutex::new(())),
//         );

//         streamer.start_thread();

//         streamer.sender.send(Status::Wait).unwrap();
//         let _streamer_thread_lock = streamer.streamer_thread_lock.lock().unwrap();
//         let status_lock = status.lock().unwrap();

//         assert!(matches!(*status_lock, Status::Wait));
//     }

//     #[test]
//     fn test_play_on_wait() {
//         let streamer_pipe = MockStreamerPipe::new();
//         let mut streamer_loop = MockStreamerLoop::new();
//         let (sender, receiver) = channel::<Status>();
//         let sender_arc = Arc::new(sender);
//         let receiver_arc = Arc::new(Mutex::new(receiver));
//         let status = Arc::new(Mutex::new(Status::None));
//         let status_clone = Arc::clone(&status);

//         streamer_loop.expect_run().return_once(move || {
//             *status_clone.lock().unwrap() = receiver_arc.lock().unwrap().recv().unwrap();
//         });

//         let streamer = ImplStreamer::new(
//             Arc::new(streamer_pipe),
//             Arc::new(streamer_loop),
//             Arc::new(Mutex::new(Status::Wait)),
//             sender_arc,
//             Arc::new(Mutex::new(())),
//         );

//         streamer.start_thread();

//         streamer.play("new_uri");

//         let _streamer_thread_lock = streamer.streamer_thread_lock.lock().unwrap();
//         let status_lock = status.lock().unwrap();

//         assert!(matches!(*status_lock, Status::Play(_)));
//         assert!(if let Status::Play(uri) = &*status_lock {
//             uri.eq("new_uri")
//         } else {
//             false
//         });
//     }

//     #[test]
//     fn test_play_on_play() {
//         let mut streamer_pipe = MockStreamerPipe::new();
//         let streamer_loop = MockStreamerLoop::new();
//         let (sender, _receiver) = channel::<Status>();
//         let sender_arc = Arc::new(sender);
//         let message = Arc::new(Mutex::new(Message::None));
//         let message_clone = Arc::clone(&message);

//         streamer_pipe.expect_send().return_once(move |message| {
//             *message_clone.lock().unwrap() = message;
//         });

//         let streamer = ImplStreamer::new(
//             Arc::new(streamer_pipe),
//             Arc::new(streamer_loop),
//             Arc::new(Mutex::new(Status::Play("old_uri".to_string()))),
//             sender_arc,
//             Arc::new(Mutex::new(())),
//         );

//         streamer.play("new_uri");

//         let message_lock = message.lock().unwrap();

//         assert!(matches!(*message_lock, Message::Next(_)));
//         assert!(if let Message::Next(uri) = &*message_lock {
//             uri.eq("new_uri")
//         } else {
//             false
//         });
//     }

//     #[test]
//     fn test_end_on_play() {
//         let mut streamer_pipe = MockStreamerPipe::new();
//         let mut streamer_loop = MockStreamerLoop::new();
//         let (sender, receiver) = channel::<Status>();
//         let sender_arc = Arc::new(sender);
//         let sender_clone = Arc::clone(&sender_arc);
//         let receiver_arc = Arc::new(Mutex::new(receiver));
//         let status = Arc::new(Mutex::new(Status::None));
//         let status_clone = Arc::clone(&status);
//         let message = Arc::new(Mutex::new(Message::None));
//         let message_clone = Arc::clone(&message);

//         streamer_pipe.expect_send().return_once(move |message| {
//             *message_clone.lock().unwrap() = message;
//             sender_clone.send(Status::End).unwrap();
//         });
//         streamer_loop.expect_run().return_once(move || {
//             *status_clone.lock().unwrap() = receiver_arc.lock().unwrap().recv().unwrap();
//         });

//         let streamer = ImplStreamer::new(
//             Arc::new(streamer_pipe),
//             Arc::new(streamer_loop),
//             Arc::new(Mutex::new(Status::Play("uri".to_owned()))),
//             sender_arc,
//             Arc::new(Mutex::new(())),
//         );

//         streamer.start_thread();

//         streamer.end();

//         let status_lock = status.lock().unwrap();
//         let message_lock = message.lock().unwrap();

//         assert!(matches!(*status_lock, Status::End));
//         assert!(matches!(*message_lock, Message::End));
//     }

//     #[test]
//     fn test_end_on_wait() {
//         let streamer_pipe = MockStreamerPipe::new();
//         let mut streamer_loop = MockStreamerLoop::new();
//         let (sender, receiver) = channel::<Status>();
//         let sender_arc = Arc::new(sender);
//         let receiver_arc = Arc::new(Mutex::new(receiver));
//         let status = Arc::new(Mutex::new(Status::None));
//         let status_clone = Arc::clone(&status);

//         streamer_loop.expect_run().return_once(move || {
//             *status_clone.lock().unwrap() = receiver_arc.lock().unwrap().recv().unwrap();
//         });

//         let streamer = ImplStreamer::new(
//             Arc::new(streamer_pipe),
//             Arc::new(streamer_loop),
//             Arc::new(Mutex::new(Status::Wait)),
//             sender_arc,
//             Arc::new(Mutex::new(())),
//         );

//         streamer.start_thread();

//         streamer.end();

//         let status_lock = status.lock().unwrap();

//         assert!(matches!(*status_lock, Status::End));
//     }
// }
