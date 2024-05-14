use core::fmt;
use std::{
    ffi::{c_char, CStr, CString},
    ptr::null_mut,
    sync::{Arc, Mutex},
    thread,
};

use gstreamer::glib::gobject_ffi::G_TYPE_STRING;
use gstreamer_sys::{
    gst_bus_post, gst_bus_timed_pop_filtered, gst_element_get_bus, gst_element_query_duration,
    gst_element_query_position, gst_element_set_state, gst_init, gst_message_get_structure,
    gst_message_new_application, gst_message_parse_state_changed, gst_message_unref,
    gst_object_unref, gst_parse_launch, gst_structure_get_name, gst_structure_get_string,
    gst_structure_new, GstBus, GstElement, GstMessage, GstObject, GstState, GST_CLOCK_TIME_NONE,
    GST_FORMAT_TIME, GST_MESSAGE_APPLICATION, GST_MESSAGE_DURATION_CHANGED, GST_MESSAGE_EOS,
    GST_MESSAGE_ERROR, GST_MESSAGE_STATE_CHANGED, GST_MSECOND, GST_STATE_CHANGE_FAILURE,
    GST_STATE_NULL, GST_STATE_PAUSED, GST_STATE_PLAYING,
};
use tauri::{AppHandle, Manager};

use crate::player::{self, Player};

const THREAD_NAME: &str = "streamer_loop";

const MESSAGE_NAME: &str = "internal";
const MESSAGE_JSON_PARAM_NAME: &str = "json";

const UPDATE_POSITION_MILLISECONDS: i64 = 100;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
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
enum Terminate {
    False,
    Async,
    Sync,
    PlayNext(String),
}

struct Data {
    app_handle: AppHandle,
    pipeline: *mut GstElement,
    is_playing: bool,
    duration: i64,
}

#[derive(Debug)]
pub(crate) struct Streamer {
    bus: Mutex<*mut GstBus>,
    lock: Mutex<()>,
    terminate: Mutex<Terminate>,
}

unsafe impl Send for Streamer {}
unsafe impl Sync for Streamer {}

impl Streamer {
    fn new() -> Self {
        Self {
            bus: Mutex::new(null_mut()),
            lock: Mutex::new(()),
            terminate: Mutex::new(Terminate::False),
        }
    }

    /*
     * Main Thread
     */

    pub(crate) fn start(app_handle: AppHandle, uri: String) -> Arc<Self> {
        let streamer = Arc::new(Self::new());
        let streamer_copy = Arc::clone(&streamer);

        thread::Builder::new()
            .name(THREAD_NAME.to_string())
            .spawn(move || streamer.lock_and_gst(app_handle, uri))
            .unwrap();

        streamer_copy
    }

    pub(crate) fn send(&self, message: Message) {
        let bus = self.bus.lock().unwrap();

        if bus.is_null() {
            eprintln!("Unable to send the message to the streamer because the bus is null. Message: {message}");
            return;
        }

        unsafe {
            let structure = gst_structure_new(
                MESSAGE_NAME.as_ptr() as *const i8,
                MESSAGE_JSON_PARAM_NAME.as_ptr() as *const i8,
                G_TYPE_STRING,
                serde_json::to_string(&message).unwrap().as_ptr() as *const i8,
            );
            let gst_msg = gst_message_new_application(null_mut(), structure);
            gst_bus_post(bus.to_owned(), gst_msg);
        }
    }

    pub(crate) fn is_active(&self) -> bool {
        if matches!(&*self.terminate.lock().unwrap(), Terminate::False) {
            return true;
        }

        // Wait if the streamer is not Totally Terminated.
        self.wait_until_end();

        false
    }

    pub(crate) fn wait_until_end(&self) {
        let lock = self.lock.lock().unwrap();
        drop(lock);
    }

    /*
     * Streamer Thread
     */

    fn lock_and_gst(&self, app_handle: AppHandle, uri: String) {
        let lock = self.lock.lock().unwrap();
        unsafe {
            self.gst(app_handle, uri);
        }
        drop(lock);
    }

    unsafe fn gst(&self, app_handle: AppHandle, uri: String) {
        let mut data = Data {
            app_handle,
            pipeline: null_mut(),
            is_playing: true,
            duration: GST_CLOCK_TIME_NONE as i64,
        };
        let args = std::env::args()
            .map(|arg| CString::new(arg).unwrap())
            .collect::<Vec<CString>>();

        let mut c_args = args
            .iter()
            .map(|arg| arg.clone().into_raw())
            .collect::<Vec<*mut c_char>>();

        gst_init(&mut (c_args.len() as i32), &mut c_args.as_mut_ptr());

        let pipeline_description = format!("playbin uri=\"{uri}\"");
        data.pipeline = gst_parse_launch(pipeline_description.as_ptr() as *const i8, null_mut());

        let mut bus = self.bus.lock().unwrap();
        *bus = gst_element_get_bus(data.pipeline);

        if gst_element_set_state(data.pipeline, GST_STATE_PLAYING) == GST_STATE_CHANGE_FAILURE {
            gst_object_unref(data.pipeline as *mut GstObject);
            panic!("GStreamer returns a failure.");
        }

        // gst_bus_add_signal_watch(*bus); TODO Need this?
        drop(bus);

        self.loop_gst(&mut data);

        gst_object_unref(self.bus.lock().unwrap().to_owned() as *mut GstObject);
        gst_element_set_state(data.pipeline, GST_STATE_NULL);
        gst_object_unref(data.pipeline as *mut GstObject);

        match &*self.terminate.lock().unwrap() {
            Terminate::False => panic!("Streamer end with 'terminate=false' should not happen."),
            Terminate::Async => self.player_command(&data, player::Command::Stopped),
            Terminate::Sync => {} // *********** FIND SOMETHING TO KNOW IF IS STREAMER IS}
            Terminate::PlayNext(uri) => {
                self.player_command(&data, player::Command::Play(uri.to_owned()))
            }
        }
    }

    unsafe fn loop_gst(&self, data: &mut Data) {
        while matches!(&*self.terminate.lock().unwrap(), Terminate::False) {
            let msg = gst_bus_timed_pop_filtered(
                self.bus.lock().unwrap().to_owned(),
                (UPDATE_POSITION_MILLISECONDS * GST_MSECOND) as u64,
                GST_MESSAGE_STATE_CHANGED
                    | GST_MESSAGE_ERROR
                    | GST_MESSAGE_EOS
                    | GST_MESSAGE_DURATION_CHANGED
                    | GST_MESSAGE_APPLICATION,
            );

            if !msg.is_null() {
                self.handle_message(data, msg);
                gst_message_unref(msg);
            } else {
                if data.is_playing {
                    self.update_position(data);
                }
            }
        }
    }

    unsafe fn handle_message(&self, data: &mut Data, msg: *mut GstMessage) {
        match msg.read().type_ {
            GST_MESSAGE_ERROR => {
                eprintln!("Error received from element.");
                *self.terminate.lock().unwrap() = Terminate::Async;
            }
            GST_MESSAGE_EOS => {
                // TODO remove?
                println!("End-Of-Stream reached.");
                *self.terminate.lock().unwrap() = Terminate::Async;
            }
            GST_MESSAGE_DURATION_CHANGED => {
                data.duration = GST_CLOCK_TIME_NONE as i64;
            }
            GST_MESSAGE_STATE_CHANGED => {
                let mut old_state: GstState = GST_STATE_NULL;
                let mut new_state: GstState = GST_STATE_NULL;
                let mut pending_state: GstState = GST_STATE_NULL;
                gst_message_parse_state_changed(
                    msg,
                    &mut old_state,
                    &mut new_state,
                    &mut pending_state,
                );
            }
            GST_MESSAGE_APPLICATION => {
                self.handle_application_message(data, msg);
            }
            _ => {
                eprintln!("Unexpected message received");
            }
        }
    }

    unsafe fn handle_application_message(&self, data: &mut Data, msg: *mut GstMessage) {
        let structure = gst_message_get_structure(msg);
        let name_ptr = gst_structure_get_name(structure);
        let name = CStr::from_ptr(name_ptr).to_str().unwrap();

        if name.ne(MESSAGE_NAME) {
            eprintln!("Error in the gst application message name: {name}");
            return;
        }

        let value_ptr =
            gst_structure_get_string(structure, MESSAGE_JSON_PARAM_NAME.as_ptr() as *const i8);
        let value = CStr::from_ptr(value_ptr).to_str().unwrap();
        let message: Message = serde_json::from_str(value).unwrap();

        match message {
            Message::Pause => {
                if data.is_playing {
                    gst_element_set_state(data.pipeline, GST_STATE_PAUSED);
                } else {
                    gst_element_set_state(data.pipeline, GST_STATE_PLAYING);
                }
            }
            Message::Move => {
                // TODO
            }
            Message::Stop => {
                // TODO remove?
                println!("Stop request (Async).");
                *self.terminate.lock().unwrap() = Terminate::Async;
            }
            Message::StopAndSendNewUri(uri) => {
                // TODO remove?
                println!("Stop request (Async) and new uri '{uri}'.");
                *self.terminate.lock().unwrap() = Terminate::PlayNext(uri);
            }
            Message::StopSync => {
                // TODO remove?
                println!("Stop request (Sync).");
                *self.terminate.lock().unwrap() = Terminate::Sync;
            }
        }
    }

    unsafe fn update_position(&self, data: &mut Data) {
        let mut current: i64 = -1;

        if !gst_element_query_position(data.pipeline, GST_FORMAT_TIME, &mut current).is_positive() {
            eprintln!("Could not query current position.");
        }

        if data.duration == GST_CLOCK_TIME_NONE as i64 {
            if gst_element_query_duration(data.pipeline, GST_FORMAT_TIME, &mut data.duration)
                .is_negative()
            {
                eprintln!("Could not query current duration.");
            }
        }

        // TODO Temp
        println!("Position {} / {}", current, data.duration);
    }

    fn player_command(&self, data: &Data, command: player::Command) {
        let app_handle = data.app_handle.app_handle();

        data.app_handle
            .run_on_main_thread(move || {
                Player::instance().command(app_handle.app_handle(), command);
            })
            .unwrap();
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
