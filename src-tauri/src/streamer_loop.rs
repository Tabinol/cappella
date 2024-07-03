use std::{
    ffi::{c_char, CString},
    fmt::Debug,
    ptr::null_mut,
    sync::{mpsc::Receiver, Arc, Mutex},
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

use crate::{
    streamer::Status,
    streamer_pipe::{
        cstring_ptr_to_str, str_to_cstring, string_to_cstring, Message, StreamerPipe,
        MESSAGE_FIELD_JSON, MESSAGE_NAME,
    },
};

#[cfg(test)]
use mockall::automock;

const UPDATE_POSITION_MILLISECONDS: i64 = 100;

#[cfg_attr(test, automock)]
pub(crate) trait StreamerLoop: Debug + Send + Sync {
    fn run(&self);
}

#[derive(Debug)]
struct Data {
    uri: String,
    pipeline: *mut GstElement,
    is_playing: bool,
    duration: i64,
}

#[derive(Debug)]
pub(crate) struct ImplStreamerLoop {
    streamer_pipe: Arc<dyn StreamerPipe>,
    receiver: Arc<Receiver<Status>>,
    status: Arc<Mutex<Status>>,
}

unsafe impl Send for ImplStreamerLoop {}
unsafe impl Sync for ImplStreamerLoop {}

impl ImplStreamerLoop {
    pub(crate) fn new(
        streamer_pipe: Arc<dyn StreamerPipe>,
        receiver: Arc<Receiver<Status>>,
        status: Arc<Mutex<Status>>,
    ) -> Self {
        Self {
            streamer_pipe,
            receiver,
            status,
        }
    }

    fn gst_thread(&self) {
        *self.status.lock().unwrap() = Status::Wait;

        'end_gst_thread: loop {
            let status_clone = Arc::clone(&self.status);
            let mut current_status = status_clone.lock().unwrap().clone();

            if matches!(current_status, Status::Wait) {
                current_status = self.receiver.recv().unwrap();
                *status_clone.lock().unwrap() = current_status.clone();
            }

            if let Status::Play(uri) = current_status {
                let mut data = Data {
                    uri: uri.to_owned(),
                    pipeline: null_mut(),
                    is_playing: true,
                    duration: GST_CLOCK_TIME_NONE as i64,
                };
                unsafe { self.gst(&mut data) };
            }

            let mut status_lock = status_clone.lock().unwrap();

            if let Status::PlayNext(uri) = &*status_lock {
                *status_lock = Status::Play(uri.to_owned());
            }

            if matches!(&*status_lock, Status::End) {
                break 'end_gst_thread;
            }
        }
    }

    unsafe fn gst(&self, data: &mut Data) {
        let args = std::env::args()
            .map(|arg| string_to_cstring(arg))
            .collect::<Vec<CString>>();

        let mut c_args = args
            .iter()
            .map(|arg| arg.clone().into_raw())
            .collect::<Vec<*mut c_char>>();

        gst_init(&mut (c_args.len() as i32), &mut c_args.as_mut_ptr());

        let pipeline_description = str_to_cstring(format!("playbin uri=\"{}\"", data.uri).as_str());
        data.pipeline = gst_parse_launch(pipeline_description.as_ptr(), null_mut());

        let bus = gst_element_get_bus(data.pipeline);
        self.streamer_pipe.set_bus(bus);

        if gst_element_set_state(data.pipeline, GST_STATE_PLAYING) == GST_STATE_CHANGE_FAILURE {
            gst_object_unref(data.pipeline as *mut GstObject);
            panic!("GStreamer returns a failure.");
        }

        self.loop_gst(data, bus);

        self.streamer_pipe.set_bus(null_mut());
        gst_object_unref(bus as *mut GstObject);
        gst_element_set_state(data.pipeline, GST_STATE_NULL);
        gst_object_unref(data.pipeline as *mut GstObject);
    }

    unsafe fn loop_gst(&self, data: &mut Data, bus: *mut GstBus) {
        'end_gst: loop {
            let msg = gst_bus_timed_pop_filtered(
                bus,
                (UPDATE_POSITION_MILLISECONDS * GST_MSECOND) as u64,
                GST_MESSAGE_STATE_CHANGED
                    | GST_MESSAGE_ERROR
                    | GST_MESSAGE_EOS
                    | GST_MESSAGE_DURATION_CHANGED
                    | GST_MESSAGE_APPLICATION,
            );

            let status_clone = Arc::clone(&self.status);
            let mut status_lock = status_clone.lock().unwrap();

            if !msg.is_null() {
                let new_status_opt = self.handle_message(data, msg);

                if let Some(new_status) = new_status_opt {
                    *status_lock = new_status;
                }

                gst_message_unref(msg);
            } else {
                if data.is_playing {
                    self.update_position(data);
                }
            }

            if !matches!(&*status_lock, Status::Play(_)) {
                break 'end_gst;
            }
        }
    }

    unsafe fn handle_message(&self, data: &mut Data, msg: *mut GstMessage) -> Option<Status> {
        match msg.read().type_ {
            GST_MESSAGE_ERROR => {
                eprintln!("Error received from element.");
                Some(Status::Wait)
            }
            GST_MESSAGE_EOS => {
                // TODO remove?
                println!("End-Of-Stream reached.");
                Some(Status::Wait)
            }
            GST_MESSAGE_DURATION_CHANGED => {
                data.duration = GST_CLOCK_TIME_NONE as i64;
                None
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
                None
            }
            GST_MESSAGE_APPLICATION => self.handle_application_message(data, msg),
            _ => {
                eprintln!("Unexpected message received");
                None
            }
        }
    }

    unsafe fn handle_application_message(
        &self,
        data: &mut Data,
        msg: *mut GstMessage,
    ) -> Option<Status> {
        let structure = gst_message_get_structure(msg);
        let name_ptr = gst_structure_get_name(structure);
        let name = cstring_ptr_to_str(name_ptr);

        if name.ne(MESSAGE_NAME) {
            eprintln!("Streamer pipe message name error: {name}");
            return None;
        }

        let field_json = str_to_cstring(MESSAGE_FIELD_JSON);
        let json_ptr = gst_structure_get_string(structure, field_json.as_ptr());
        let json = cstring_ptr_to_str(json_ptr);
        let message = serde_json::from_str(json)
            .expect(format!("Unreadable streamer message: {json}").as_str());

        match message {
            Message::None => {
                eprintln!("Message with 'None' is an error.");
                None
            }
            Message::Pause => {
                if data.is_playing {
                    gst_element_set_state(data.pipeline, GST_STATE_PAUSED);
                    data.is_playing = false;
                } else {
                    gst_element_set_state(data.pipeline, GST_STATE_PLAYING);
                    data.is_playing = true;
                }
                None
            }
            Message::Stop => {
                // TODO remove?
                println!("Stop request.");
                Some(Status::Wait)
            }
            Message::Next(uri) => {
                // TODO remove?
                println!("Stop request (Async) and new uri '{uri}'.");
                Some(Status::PlayNext(uri))
            }
            Message::End => {
                // TODO remove?
                println!("End request.");
                Some(Status::End)
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
}

impl StreamerLoop for ImplStreamerLoop {
    fn run(&self) {
        self.gst_thread();
    }
}
