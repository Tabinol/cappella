use core::fmt;
use std::{
    ffi::CString,
    ptr::{null, null_mut},
    sync::{Arc, Mutex, OnceLock},
    thread,
};

use gstreamer::glib::gobject_ffi::G_TYPE_STRING;
use gstreamer_sys::{gst_bus_post, gst_message_new_application, gst_structure_new, GstBus};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::streamer_thread::StreamerThread;

#[derive(Debug, Serialize, Deserialize)]
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
    pub(crate) streamer_lock: Mutex<()>,
    pub(crate) status: Mutex<Status>,
}

unsafe impl Send for Share {}
unsafe impl Sync for Share {}

pub(crate) struct MessageFields {
    pub(crate) title: CString,
    pub(crate) json_field: CString,
}

#[derive(Clone, Debug)]
pub(crate) struct Streamer {
    share: Arc<Share>,
}

const THREAD_NAME: &str = "streamer_loop";

pub(crate) fn message_fields() -> &'static MessageFields {
    static MESSAGE_NAME: OnceLock<MessageFields> = OnceLock::new();
    MESSAGE_NAME.get_or_init(|| MessageFields {
        title: CString::new("player_request").unwrap(),
        json_field: CString::new("json").unwrap(),
    })
}

impl Streamer {
    pub(crate) fn new() -> Self {
        Self {
            share: Arc::new(Share {
                bus: Mutex::new(null_mut()),
                streamer_lock: Mutex::new(()),
                status: Mutex::new(Status::Inactive),
            }),
        }
    }

    pub(crate) fn start(&mut self, app_handle: AppHandle, uri: String) {
        if self.is_active() {
            panic!("Streamer thread already active.")
        }

        let share_clone = Arc::clone(&self.share);

        thread::Builder::new()
            .name(THREAD_NAME.to_string())
            .spawn(move || {
                StreamerThread::new(share_clone, app_handle, uri).start();
            })
            .unwrap();
    }

    pub(crate) fn send(&mut self, message: Message) {
        if let Some(share) = self.get_share_if_active() {
            let bus = share.bus.lock().unwrap();

            if bus.is_null() {
                eprintln!("Unable to send the message to the streamer because the bus is null. Message: {message}");
                return;
            }

            unsafe {
                let message_fields = message_fields();
                let json = serde_json::to_string(&message).unwrap();
                let json_cstring = CString::new(json).unwrap();
                let structure = gst_structure_new(
                    message_fields.title.as_ptr(),
                    message_fields.json_field.as_ptr(),
                    G_TYPE_STRING,
                    json_cstring.as_ptr(),
                    null() as *const i8,
                );
                let gst_msg = gst_message_new_application(null_mut(), structure);
                gst_bus_post(*bus, gst_msg);
            }
        } else {
            eprintln!("Trying to send a message to a null streamer thread. Message: {message}");
        }
    }

    pub(crate) fn get_share_if_active(&mut self) -> Option<&mut Arc<Share>> {
        if self.is_active() {
            return Some(&mut self.share);
        }

        None
    }

    pub(crate) fn is_active(&mut self) -> bool {
        matches!(&*self.share.status.lock().unwrap(), Status::Active)
    }

    pub(crate) fn wait_until_end(&mut self) {
        let _unused = self.share.streamer_lock.lock().unwrap();
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
