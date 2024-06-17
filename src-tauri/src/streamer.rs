use std::{
    ffi::{c_char, CString},
    ptr::null_mut,
    sync::Arc,
};

use gstreamer_sys::{
    gst_bus_timed_pop_filtered, gst_element_get_bus, gst_element_query_duration,
    gst_element_query_position, gst_element_set_state, gst_init, gst_message_get_structure,
    gst_message_parse_state_changed, gst_message_unref, gst_object_unref, gst_parse_launch,
    gst_structure_get_name, gst_structure_get_string, GstBus, GstElement, GstMessage, GstObject,
    GstState, GST_CLOCK_TIME_NONE, GST_FORMAT_TIME, GST_MESSAGE_APPLICATION,
    GST_MESSAGE_DURATION_CHANGED, GST_MESSAGE_EOS, GST_MESSAGE_ERROR, GST_MESSAGE_STATE_CHANGED,
    GST_MSECOND, GST_STATE_CHANGE_FAILURE, GST_STATE_NULL, GST_STATE_PAUSED, GST_STATE_PLAYING,
};
use tauri::{AppHandle, Manager};

use crate::{
    player_state::PlayerState,
    streamer_pipe::{
        cstring_ptr_to_str, str_to_cstring, string_to_cstring, StreamerPipe, MESSAGE_FIELD_URI,
        MESSAGE_NAME_PAUSE, MESSAGE_NAME_STOP, MESSAGE_NAME_STOP_AND_SEND_NEW_URI,
        MESSAGE_NAME_STOP_SYNC,
    },
};

#[derive(Debug)]
pub(crate) enum Status {
    Active,
    Async,
    Sync,
    PlayNext(String),
}

#[derive(Debug)]
pub(crate) struct Streamer {
    streamer_pipe: Arc<StreamerPipe>,
    app_handle: AppHandle,
    uri: String,
    status: Status,
    pipeline: *mut GstElement,
    is_playing: bool,
    duration: i64,
}

unsafe impl Send for Streamer {}
unsafe impl Sync for Streamer {}

const UPDATE_POSITION_MILLISECONDS: i64 = 100;

impl Streamer {
    pub(crate) fn new(
        streamer_pipe: Arc<StreamerPipe>,
        app_handle: AppHandle,
        uri: String,
    ) -> Self {
        Self {
            streamer_pipe,
            app_handle,
            uri,
            status: Status::Active,
            pipeline: null_mut(),
            is_playing: true,
            duration: GST_CLOCK_TIME_NONE as i64,
        }
    }

    pub(crate) fn start(&mut self) {
        unsafe {
            self.gst();
        }
    }

    /**
     * Streamer thread
     */

    unsafe fn gst(&mut self) {
        let args = std::env::args()
            .map(|arg| string_to_cstring(arg))
            .collect::<Vec<CString>>();

        let mut c_args = args
            .iter()
            .map(|arg| arg.clone().into_raw())
            .collect::<Vec<*mut c_char>>();

        gst_init(&mut (c_args.len() as i32), &mut c_args.as_mut_ptr());

        let pipeline_description = str_to_cstring(format!("playbin uri=\"{}\"", self.uri).as_str());
        self.pipeline = gst_parse_launch(pipeline_description.as_ptr(), null_mut());

        let bus = gst_element_get_bus(self.pipeline);
        *self.streamer_pipe.bus.lock().unwrap() = bus;

        if gst_element_set_state(self.pipeline, GST_STATE_PLAYING) == GST_STATE_CHANGE_FAILURE {
            gst_object_unref(self.pipeline as *mut GstObject);
            panic!("GStreamer returns a failure.");
        }

        self.loop_gst(bus);

        *self.streamer_pipe.bus.lock().unwrap() = null_mut();
        gst_object_unref(bus as *mut GstObject);
        gst_element_set_state(self.pipeline, GST_STATE_NULL);
        gst_object_unref(self.pipeline as *mut GstObject);

        match &self.status {
            Status::Active => panic!("Streamer end with 'status=Active' should not happen."),
            Status::Async => self.player_stopped(),
            Status::Sync => {} // *********** FIND SOMETHING TO KNOW IF IS STREAMER IS
            Status::PlayNext(uri) => self.player_next(uri),
        }
    }

    unsafe fn loop_gst(&mut self, bus: *mut GstBus) {
        'end_gst: loop {
            if !matches!(self.status, Status::Active) {
                break 'end_gst;
            }

            let msg = gst_bus_timed_pop_filtered(
                bus,
                (UPDATE_POSITION_MILLISECONDS * GST_MSECOND) as u64,
                GST_MESSAGE_STATE_CHANGED
                    | GST_MESSAGE_ERROR
                    | GST_MESSAGE_EOS
                    | GST_MESSAGE_DURATION_CHANGED
                    | GST_MESSAGE_APPLICATION,
            );

            if !msg.is_null() {
                self.handle_message(msg);
                gst_message_unref(msg);
            } else {
                if self.is_playing {
                    self.update_position();
                }
            }
        }
    }

    unsafe fn handle_message(&mut self, msg: *mut GstMessage) {
        match msg.read().type_ {
            GST_MESSAGE_ERROR => {
                eprintln!("Error received from element.");
                self.status = Status::Async;
            }
            GST_MESSAGE_EOS => {
                // TODO remove?
                println!("End-Of-Stream reached.");
                self.status = Status::Async;
            }
            GST_MESSAGE_DURATION_CHANGED => {
                self.duration = GST_CLOCK_TIME_NONE as i64;
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
                self.handle_application_message(msg);
            }
            _ => {
                eprintln!("Unexpected message received");
            }
        }
    }

    unsafe fn handle_application_message(&mut self, msg: *mut GstMessage) {
        let structure = gst_message_get_structure(msg);
        let name_ptr = gst_structure_get_name(structure);
        let name = cstring_ptr_to_str(name_ptr);

        match name {
            MESSAGE_NAME_PAUSE => {
                if self.is_playing {
                    gst_element_set_state(self.pipeline, GST_STATE_PAUSED);
                    self.is_playing = false;
                } else {
                    gst_element_set_state(self.pipeline, GST_STATE_PLAYING);
                    self.is_playing = true;
                }
            }
            MESSAGE_NAME_STOP => {
                // TODO remove?
                println!("Stop request (Async).");
                self.status = Status::Async;
            }
            MESSAGE_NAME_STOP_AND_SEND_NEW_URI => {
                let field_uri = str_to_cstring(MESSAGE_FIELD_URI);
                let uri_ptr = gst_structure_get_string(structure, field_uri.as_ptr());
                let uri = cstring_ptr_to_str(uri_ptr);

                // TODO remove?
                println!("Stop request (Async) and new uri '{uri}'.");
                self.status = Status::PlayNext(uri.to_owned());
            }
            MESSAGE_NAME_STOP_SYNC => {
                // TODO remove?
                println!("Stop request (Sync).");
                self.status = Status::Sync;
            }
            _ => {
                eprintln!("The message name is wrong: '{name}'");
            }
        }
    }

    unsafe fn update_position(&mut self) {
        let mut current: i64 = -1;

        if !gst_element_query_position(self.pipeline, GST_FORMAT_TIME, &mut current).is_positive() {
            eprintln!("Could not query current position.");
        }

        if self.duration == GST_CLOCK_TIME_NONE as i64 {
            if gst_element_query_duration(self.pipeline, GST_FORMAT_TIME, &mut self.duration)
                .is_negative()
            {
                eprintln!("Could not query current duration.");
            }
        }

        // TODO Temp
        println!("Position {} / {}", current, self.duration);
    }

    fn player_stopped(&self) {
        let app_handle = self.app_handle.clone();

        self.app_handle
            .run_on_main_thread(move || {
                app_handle.state::<PlayerState>().player_mut().stopped();
            })
            .unwrap();
    }

    fn player_next(&self, uri: &str) {
        let app_handle_clone = self.app_handle.clone();
        let uri_owned = uri.to_owned();
        self.app_handle
            .run_on_main_thread(move || {
                app_handle_clone
                    .state::<PlayerState>()
                    .player_mut()
                    .play(app_handle_clone.clone(), uri_owned.as_str());
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
