use std::fmt::Debug;

use crate::streamer;

pub trait Front: Debug + Send + Sync {
    fn play(&self, app_handle_addr: usize, uri: &str);
    fn pause(&self);
    fn stop(&self);
    fn wait_until_end(&self);
}

pub fn new_box(
    streamer_front: Box<dyn streamer::front::Front>,
    streamer_pipe: Box<dyn streamer::pipe::Pipe>,
) -> Box<dyn Front> {
    Box::new(Front_ {
        streamer_front,
        streamer_pipe,
    })
}

#[derive(Debug)]
struct Front_ {
    streamer_front: Box<dyn streamer::front::Front>,
    streamer_pipe: Box<dyn streamer::pipe::Pipe>,
}

unsafe impl Send for Front_ {}
unsafe impl Sync for Front_ {}

impl Front for Front_ {
    fn play(&self, app_handle_addr: usize, uri: &str) {
        if self.streamer_front.is_running() {
            self.streamer_pipe
                .send(streamer::message::Message::Play(
                    app_handle_addr,
                    uri.to_owned(),
                ))
                .unwrap_or_else(|err| eprintln!("Error on Play: {err}"));
        } else {
            self.streamer_front.start(app_handle_addr, uri);
        }
    }

    fn pause(&self) {
        self.streamer_pipe
            .send(streamer::message::Message::Pause)
            .unwrap_or_else(|err| eprintln!("Error on Pause: {err}"));
    }

    fn stop(&self) {
        self.streamer_pipe
            .send(streamer::message::Message::Stop)
            .unwrap_or_else(|err| eprintln!("Error on Stop: {err}"));
    }

    fn wait_until_end(&self) {
        self.streamer_front.wait_until_end();
    }
}

// #[cfg(test)]
// mod tests {
//     use std::sync::{Arc, Mutex};

//     use crate::player::{
//         player_front::{Player, Player_},
//         streamer::Streamer,
//         streamer_pipe::{Message, StreamerPipe},
//     };

//     #[derive(Debug)]
//     struct MockStreamer {
//         is_running: bool,
//         uri: Mutex<String>,
//     }

//     impl MockStreamer {
//         fn new(is_running: bool) -> Self {
//             Self {
//                 is_running,
//                 uri: Mutex::default(),
//             }
//         }
//     }

//     impl Streamer for MockStreamer {
//         fn is_running(&self) -> bool {
//             self.is_running
//         }

//         fn start_thread(&self) {}

//         fn play(&self, uri: &str) {
//             *self.uri.lock().unwrap() = uri.to_string();
//         }

//         fn end(&self) {}
//     }

//     #[derive(Debug, Default)]
//     struct MockStreamerPipe {
//         message: Mutex<Message>,
//     }

//     impl StreamerPipe for MockStreamerPipe {
//         fn send(&self, message: Message) {
//             *self.message.lock().unwrap() = message;
//         }
//     }

//     #[test]
//     fn test_play_when_streamer_inactive() {
//         let streamer = Arc::new(MockStreamer::new(false));
//         let streamer_pipe = Arc::<MockStreamerPipe>::default();
//         let player = Player_::new(streamer.clone(), streamer_pipe.clone());

//         player.play("test_uri");

//         assert_eq!(*streamer.uri.lock().unwrap(), "test_uri");
//     }

//     #[test]
//     fn test_play_when_streamer_active() {
//         let streamer = Arc::new(MockStreamer::new(true));
//         let streamer_pipe = Arc::<MockStreamerPipe>::default();
//         let player = Player_::new(streamer.clone(), streamer_pipe.clone());

//         player.play("test_uri");

//         let sent_message_lock = streamer_pipe.message.lock().unwrap();
//         assert!(matches!(&*sent_message_lock, Message::Next(_)));
//         assert!(if let Message::Next(uri) = &*sent_message_lock {
//             uri.eq("test_uri")
//         } else {
//             false
//         });
//     }

//     #[test]
//     fn test_pause() {
//         let streamer = Arc::new(MockStreamer::new(true));
//         let streamer_pipe = Arc::<MockStreamerPipe>::default();
//         let player = Player_::new(streamer.clone(), streamer_pipe.clone());

//         player.pause();

//         let sent_message_lock = streamer_pipe.message.lock().unwrap();
//         assert!(matches!(&*sent_message_lock, Message::Pause));
//     }

//     #[test]
//     fn test_stop() {
//         let streamer = Arc::new(MockStreamer::new(true));
//         let streamer_pipe = Arc::<MockStreamerPipe>::default();
//         let player = Player_::new(streamer.clone(), streamer_pipe.clone());

//         player.stop();

//         let sent_message_lock = streamer_pipe.message.lock().unwrap();
//         assert!(matches!(&*sent_message_lock, Message::Stop));
//     }
// }

// #[cfg(test)]
// mod tests {
//     use std::{
//         sync::{mpsc::Receiver, Arc, Mutex, RwLock},
//         time::Duration,
//     };

//     use crate::player::{
//         streamer::{self, Streamer, Streamer_},
//         streamer_loop::StreamerLoop,
//         streamer_pipe::{Message, StreamerPipe},
//     };

//     use super::Status;

//     #[derive(Debug, Default)]
//     struct MockStreamerPipe {
//         last_message: RwLock<Message>,
//     }

//     impl StreamerPipe for MockStreamerPipe {
//         fn send(&self, message: Message) {
//             *self.last_message.write().unwrap() = message;
//         }
//     }

//     #[derive(Debug, Default)]
//     struct MockStreamerLoop {
//         status: RwLock<Status>,
//         last_message: RwLock<streamer::Message>,
//         receiver: Mutex<Option<Receiver<super::Message>>>,
//     }

//     impl StreamerLoop for MockStreamerLoop {
//         fn run(&self, receiver: Receiver<super::Message>) {
//             let mut receiver_lock = self.receiver.lock().unwrap();
//             *receiver_lock = Some(receiver);
//             let last_message = (*receiver_lock)
//                 .as_ref()
//                 .unwrap()
//                 .recv_timeout(Duration::from_secs(10))
//                 .expect("Message timeout.");
//             *self.last_message.write().unwrap() = last_message;
//         }

//         fn status(&self) -> Status {
//             (*self.status.read().unwrap()).clone()
//         }
//     }

//     #[test]
//     fn test_is_running_false() {
//         let streamer_pipe = Arc::<MockStreamerPipe>::default();
//         let streamer_loop = Arc::<MockStreamerLoop>::default();

//         *streamer_loop.status.write().unwrap() = Status::Wait;
//         let streamer = Streamer_::new(streamer_pipe.clone(), streamer_loop.clone());

//         assert!(!streamer.is_running());
//     }

//     #[test]
//     fn test_is_running_true() {
//         let streamer_pipe = Arc::<MockStreamerPipe>::default();
//         let streamer_loop = Arc::<MockStreamerLoop>::default();

//         *streamer_loop.status.write().unwrap() = Status::Play("uri".to_string());
//         let streamer = Streamer_::new(streamer_pipe.clone(), streamer_loop.clone());

//         assert!(streamer.is_running());
//     }

//     #[test]
//     fn test_start_thread() {
//         let streamer_pipe = Arc::<MockStreamerPipe>::default();
//         let streamer_loop = Arc::<MockStreamerLoop>::default();

//         let streamer = Streamer_::new(streamer_pipe.clone(), streamer_loop.clone());

//         streamer.start_thread();

//         streamer
//             .sender
//             .get()
//             .unwrap()
//             .send(streamer::Message::End)
//             .unwrap();

//         let join_handle = streamer.join_handle.try_write().unwrap().take().unwrap();
//         join_handle.join().unwrap();
//         let last_message = (*streamer_loop.last_message.read().unwrap()).clone();

//         assert!(matches!(last_message, streamer::Message::End));
//     }

//     #[test]
//     fn test_play_on_wait() {
//         let streamer_pipe = Arc::<MockStreamerPipe>::default();
//         let streamer_loop = Arc::<MockStreamerLoop>::default();

//         *streamer_loop.status.write().unwrap() = Status::Wait;
//         let streamer = Streamer_::new(streamer_pipe.clone(), streamer_loop.clone());

//         streamer.start_thread();

//         streamer.play("new_uri");

//         let join_handle = streamer.join_handle.try_write().unwrap().take().unwrap();
//         join_handle.join().unwrap();
//         let status = (*streamer_loop.status.read().unwrap()).clone();

//         assert!(matches!(status, Status::Play(_)));
//         assert!(if let Status::Play(uri) = status {
//             uri.eq("new_uri")
//         } else {
//             false
//         });
//     }

//     #[test]
//     fn test_play_on_play() {
//         let streamer_pipe = Arc::<MockStreamerPipe>::default();
//         let streamer_loop = Arc::<MockStreamerLoop>::default();

//         *streamer_loop.status.write().unwrap() = Status::Play("old_uri".to_string());
//         let streamer = Streamer_::new(streamer_pipe.clone(), streamer_loop.clone());

//         streamer.play("new_uri");

//         let message = (*streamer_pipe.last_message.read().unwrap()).clone();

//         assert!(matches!(message, Message::Next(_)));
//         assert!(if let Message::Next(uri) = message {
//             uri.eq("new_uri")
//         } else {
//             false
//         });
//     }

//     #[test]
//     fn test_end_on_play() {
//         let streamer_pipe = Arc::<MockStreamerPipe>::default();
//         let streamer_loop = Arc::<MockStreamerLoop>::default();

//         *streamer_loop.status.write().unwrap() = Status::Play("uri".to_owned());
//         let streamer = Streamer_::new(streamer_pipe.clone(), streamer_loop.clone());

//         streamer.start_thread();

//         streamer.end();

//         let status = (*streamer_loop.status.read().unwrap()).clone();
//         let message = (*streamer_pipe.last_message.read().unwrap()).clone();

//         assert!(matches!(status, Status::End));
//         assert!(matches!(message, Message::Stop));
//     }

//     #[test]
//     fn test_end_on_wait() {
//         let streamer_pipe = Arc::<MockStreamerPipe>::default();
//         let streamer_loop = Arc::<MockStreamerLoop>::default();

//         *streamer_loop.status.write().unwrap() = Status::Wait;
//         let streamer = Streamer_::new(streamer_pipe.clone(), streamer_loop.clone());

//         streamer.start_thread();

//         streamer.end();

//         let status = (*streamer_loop.status.read().unwrap()).clone();

//         assert!(matches!(status, Status::End));
//     }
// }

// #[cfg(test)]
// mod tests {
//     use std::ffi::{c_char, c_int};

//     use gstreamer::glib::ffi::GError;
//     use gstreamer::ffi::{GstBus, GstElement, GST_STATE_CHANGE_FAILURE};

//     use crate::gstreamer::{
//         gstreamer::{Gstreamer, Gstreamer_},
//         gstreamer_tests_common::{
//             self, get_gst_bus_ptr, get_gst_element_ptr, ELEMENT_SET_STATE_RESULT,
//             OBJECT_UNREF_CALL_NB,
//         },
//     };

//     static mut INIT_CALL_NB: u32 = 0;
//     static mut PARSE_LAUNCH_CALL_NB: u32 = 0;
//     static mut ELEMENT_GET_BUS_CALL_NB: u32 = 0;

//     #[no_mangle]
//     extern "C" fn gst_init(_argc: *mut c_int, _argv: *mut *mut *mut c_char) {
//         unsafe { INIT_CALL_NB += 1 };
//     }

//     #[no_mangle]
//     extern "C" fn gst_parse_launch(
//         _pipeline_description: *const c_char,
//         _error: *mut *mut GError,
//     ) -> *mut GstElement {
//         unsafe {
//             PARSE_LAUNCH_CALL_NB += 1;
//         }

//         get_gst_element_ptr()
//     }

//     #[no_mangle]
//     extern "C" fn gst_element_get_bus(_element: *mut GstElement) -> *mut GstBus {
//         unsafe {
//             ELEMENT_GET_BUS_CALL_NB += 1;
//         }

//         get_gst_bus_ptr()
//     }

//     fn before_each() {
//         gstreamer_tests_common::before_each();

//         unsafe {
//             INIT_CALL_NB = 0;
//             PARSE_LAUNCH_CALL_NB = 0;
//             ELEMENT_GET_BUS_CALL_NB = 0;
//         }
//     }

//     #[test]
//     fn test_init() {
//         let _lock = gstreamer_tests_common::lock();
//         before_each();

//         let gstreamer = Gstreamer_::default();

//         gstreamer.init();

//         assert_eq!(unsafe { INIT_CALL_NB }, 1);
//     }

//     #[test]
//     fn test_launch() {
//         let _lock = gstreamer_tests_common::lock();
//         before_each();

//         let gstreamer = Gstreamer_::default();

//         gstreamer.launch("uri");

//         assert_eq!(unsafe { PARSE_LAUNCH_CALL_NB }, 1);
//         assert_eq!(unsafe { ELEMENT_GET_BUS_CALL_NB }, 1);
//         assert_eq!(unsafe { OBJECT_UNREF_CALL_NB }, 2);
//     }

//     #[test]
//     #[should_panic]
//     fn test_launch_failure() {
//         let _lock = gstreamer_tests_common::lock();
//         before_each();

//         let gstreamer = Gstreamer_::default();

//         unsafe { ELEMENT_SET_STATE_RESULT = GST_STATE_CHANGE_FAILURE }
//         gstreamer.launch("uri");

//         assert_eq!(unsafe { PARSE_LAUNCH_CALL_NB }, 1);
//         assert_eq!(unsafe { ELEMENT_GET_BUS_CALL_NB }, 1);
//         assert_eq!(unsafe { OBJECT_UNREF_CALL_NB }, 1);
//     }
// }
