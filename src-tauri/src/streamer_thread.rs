use std::{
    ffi::{c_char, CStr, CString},
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
    streamer::{self, Message, Share, Status},
};

#[derive(Debug)]
pub(crate) struct StreamerThread {
    share: Arc<Share>,
    app_handle: AppHandle,
    uri: String,
    pipeline: *mut GstElement,
    is_playing: bool,
    duration: i64,
}

const UPDATE_POSITION_MILLISECONDS: i64 = 100;

impl StreamerThread {
    pub(crate) fn new(share: Arc<Share>, app_handle: AppHandle, uri: String) -> Self {
        Self {
            share,
            app_handle,
            uri,
            pipeline: null_mut(),
            is_playing: true,
            duration: GST_CLOCK_TIME_NONE as i64,
        }
    }

    pub(crate) fn start(&mut self) {
        let share_clone = Arc::clone(&self.share);
        let _unused = share_clone.streamer_lock.lock().unwrap();
        unsafe {
            self.gst();
        }
    }

    /**
     * Streamer thread
     */

    unsafe fn gst(&mut self) {
        *self.share.status.lock().unwrap() = Status::Active;
        let args = std::env::args()
            .map(|arg| CString::new(arg).unwrap())
            .collect::<Vec<CString>>();

        let mut c_args = args
            .iter()
            .map(|arg| arg.clone().into_raw())
            .collect::<Vec<*mut c_char>>();

        gst_init(&mut (c_args.len() as i32), &mut c_args.as_mut_ptr());

        let pipeline_description = CString::new(format!("playbin uri=\"{}\"", self.uri)).unwrap();
        self.pipeline = gst_parse_launch(pipeline_description.as_ptr(), null_mut());

        let bus = gst_element_get_bus(self.pipeline);
        *self.share.bus.lock().unwrap() = bus;

        if gst_element_set_state(self.pipeline, GST_STATE_PLAYING) == GST_STATE_CHANGE_FAILURE {
            gst_object_unref(self.pipeline as *mut GstObject);
            panic!("GStreamer returns a failure.");
        }

        self.loop_gst(bus);

        *self.share.bus.lock().unwrap() = null_mut();
        gst_object_unref(bus as *mut GstObject);
        gst_element_set_state(self.pipeline, GST_STATE_NULL);
        gst_object_unref(self.pipeline as *mut GstObject);

        match &*self.share.status.lock().unwrap() {
            Status::Active => panic!("Streamer end with 'status=Active' should not happen."),
            Status::Async => self.player_stopped(),
            Status::Sync => {} // *********** FIND SOMETHING TO KNOW IF IS STREAMER IS
            Status::PlayNext(uri) => self.player_next(uri),
            Status::Inactive => {
                panic!("Streamer is in inactive state but the loop is active.")
            }
        }

        *self.share.status.lock().unwrap() = Status::Inactive;
    }

    unsafe fn loop_gst(&mut self, bus: *mut GstBus) {
        'end_gst: loop {
            if !matches!(&*self.share.status.lock().unwrap(), Status::Active) {
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
                *self.share.status.lock().unwrap() = Status::Async;
            }
            GST_MESSAGE_EOS => {
                // TODO remove?
                println!("End-Of-Stream reached.");
                *self.share.status.lock().unwrap() = Status::Async;
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
        let message_fields = streamer::message_fields();
        let structure = gst_message_get_structure(msg);
        let name_ptr = gst_structure_get_name(structure);
        let name = CStr::from_ptr(name_ptr);

        if name.ne(message_fields.title.as_c_str()) {
            eprintln!("The message name is wrong: {}", name.to_str().unwrap());
        }

        let json_ptr = gst_structure_get_string(structure, message_fields.json_field.as_ptr());
        let json = CStr::from_ptr(json_ptr).to_str().unwrap();
        let message = serde_json::from_str(json)
            .expect("Unable to read the message from the player to the streamer.");

        match message {
            Message::Pause => {
                if self.is_playing {
                    gst_element_set_state(self.pipeline, GST_STATE_PAUSED);
                    self.is_playing = false;
                } else {
                    gst_element_set_state(self.pipeline, GST_STATE_PLAYING);
                    self.is_playing = true;
                }
            }
            Message::Move => {
                // TODO
            }
            Message::Stop => {
                // TODO remove?
                println!("Stop request (Async).");
                *self.share.status.lock().unwrap() = Status::Async;
            }
            Message::StopAndSendNewUri(uri) => {
                // TODO remove?
                println!("Stop request (Async) and new uri '{uri}'.");
                *self.share.status.lock().unwrap() = Status::PlayNext(uri);
            }
            Message::StopSync => {
                // TODO remove?
                println!("Stop request (Sync).");
                *self.share.status.lock().unwrap() = Status::Sync;
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
        let app_handle = self.app_handle.clone();
        let uri_owned = uri.to_owned();

        self.app_handle
            .run_on_main_thread(move || {
                app_handle
                    .state::<PlayerState>()
                    .player_mut()
                    .play(app_handle.clone(), uri_owned);
            })
            .unwrap();
    }
}
