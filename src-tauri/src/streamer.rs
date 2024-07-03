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
    sender: Sender<Status>,
    join_handle: Arc<Mutex<*mut JoinHandle<()>>>,
}

unsafe impl Send for ImplStreamer {}
unsafe impl Sync for ImplStreamer {}

impl ImplStreamer {
    pub(crate) fn new(
        streamer_pipe: Arc<dyn StreamerPipe>,
        streamer_loop: Arc<dyn StreamerLoop>,
        status: Arc<Mutex<Status>>,
        sender: Sender<Status>,
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

// #[cfg(test)]
// mod tests {
//     use std::{
//         ffi::{c_char, c_int},
//         ptr::addr_of_mut,
//     };

//     use gstreamer_sys::{
//         GstClockTime, GstObject, GstState, GstStateChangeReturn, GST_STATE_NULL, GST_STATE_PAUSED,
//         GST_STATE_PLAYING,
//     };

//     use super::{Player, PlayerStatus};

//     struct GstElement {}

//     static mut GST_STATE: GstState = GST_STATE_NULL;
//     static mut GST_ELEMENT: GstElement = GstElement {};

//     #[no_mangle]
//     #[allow(unused_variables)]
//     extern "C" fn gst_init(argc: *mut c_int, argv: *mut *mut *mut c_char) {}

//     #[no_mangle]
//     #[allow(unused_variables)]
//     extern "C" fn gst_parse_launch(
//         pipeline_description: *const c_char,
//         error: *mut *mut glib_sys::GError,
//     ) -> *mut GstElement {
//         unsafe { addr_of_mut!(GST_ELEMENT) }
//     }

//     #[no_mangle]
//     #[allow(unused_variables)]
//     extern "C" fn gst_element_set_state(
//         element: *mut GstElement,
//         state: GstState,
//     ) -> GstStateChangeReturn {
//         unsafe { GST_STATE = state };
//         0
//     }

//     #[no_mangle]
//     #[allow(unused_assignments, unused_variables)]
//     extern "C" fn gst_element_get_state(
//         element: *mut GstElement,
//         state: *mut GstState,
//         pending: *mut GstState,
//         timeout: GstClockTime,
//     ) -> GstStateChangeReturn {
//         unsafe {
//             state.replace(GST_STATE);
//         }
//         0
//     }

//     #[no_mangle]
//     #[allow(unused_variables)]
//     extern "C" fn gst_object_unref(object: *mut GstObject) {}
// }
