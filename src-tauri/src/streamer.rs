use core::fmt;
use std::{
    collections::VecDeque,
    ffi::CString,
    ptr::null_mut,
    sync::{Arc, Mutex, OnceLock},
    thread,
};

use gstreamer_sys::{gst_bus_post, gst_message_new_application, gst_structure_new_empty, GstBus};
use tauri::AppHandle;

use crate::streamer_thread::StreamerThread;

#[derive(Clone, Debug)]
pub(crate) enum Message {
    Pause,
    Move,
    Stop,
    StopAndSendNewUri(String),
    StopSync,
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug)]
pub(crate) enum Status {
    Active,
    Async,
    Sync,
    PlayNext(String),
    Inactive,
}

#[derive(Debug)]
pub(crate) struct Share {
    pub(crate) bus: Mutex<*mut GstBus>,
    pub(crate) lock: Mutex<()>,
    pub(crate) status: Mutex<Status>,
    pub(crate) queue: Mutex<VecDeque<Message>>,
}

unsafe impl Send for Share {}
unsafe impl Sync for Share {}

#[derive(Debug)]
pub(crate) struct Streamer {
    share: Option<Arc<Share>>,
}

const THREAD_NAME: &str = "streamer_loop";

pub(crate) fn message_name() -> &'static CString {
    static MESSAGE_NAME: OnceLock<CString> = OnceLock::new();
    MESSAGE_NAME.get_or_init(|| CString::new("player_request").unwrap())
}

impl Streamer {
    pub(crate) fn new() -> Self {
        Self { share: None }
    }

    pub(crate) fn start(&mut self, app_handle: AppHandle, uri: String) {
        if self.is_active() {
            panic!("Streamer thread already active.")
        }

        let share = Arc::new(Share {
            bus: Mutex::new(null_mut()),
            lock: Mutex::new(()),
            status: Mutex::new(Status::Active),
            queue: Mutex::new(VecDeque::new()),
        });

        let share_clone = Arc::clone(&share);

        thread::Builder::new()
            .name(THREAD_NAME.to_string())
            .spawn(move || {
                StreamerThread::new(share, app_handle, uri).start();
            })
            .unwrap();

        self.share = Some(share_clone);
    }

    pub(crate) fn send(&self, message: Message) {
        if let Some(share) = &self.share {
            let bus = share.bus.lock().unwrap();

            if bus.is_null() {
                eprintln!("Unable to send the message to the streamer because the bus is null. Message: {message}");
                return;
            }

            unsafe {
                share.queue.lock().unwrap().push_back(message);

                let structure = gst_structure_new_empty(message_name().as_ptr());
                let gst_msg = gst_message_new_application(null_mut(), structure);
                gst_bus_post(*bus, gst_msg);
            }
        } else {
            eprintln!("Trying to send a message to a null streamer thread. Message: {message}");
        }
    }

    pub(crate) fn is_active(&mut self) -> bool {
        if let Some(share) = &self.share {
            if matches!(&*share.status.lock().unwrap(), Status::Active) {
                return true;
            }

            // Wait if the streamer is not Totally Terminated.
            self.wait_until_end();
        }

        false
    }

    pub(crate) fn wait_until_end(&mut self) {
        if let Some(share) = &self.share {
            let lock = share.lock.lock().unwrap();
            drop(lock);
        }

        self.share = None;
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
