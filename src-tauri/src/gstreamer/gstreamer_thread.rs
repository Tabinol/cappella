use std::{
    ffi::{c_char, CString},
    ptr::null_mut,
    sync::{mpsc::Receiver, Arc},
    thread::{self, JoinHandle},
    time::Duration,
};

use gstreamer::ffi::{
    gst_bus_timed_pop_filtered, gst_element_get_bus, gst_element_query_duration,
    gst_element_query_position, gst_element_set_state, gst_init, gst_message_get_structure,
    gst_message_parse_state_changed, gst_object_unref, gst_parse_launch, gst_structure_get_name,
    GstElement, GstMessage, GstObject, GstState, GstStructure, GST_CLOCK_TIME_NONE,
    GST_FORMAT_TIME, GST_MESSAGE_APPLICATION, GST_MESSAGE_DURATION_CHANGED, GST_MESSAGE_EOS,
    GST_MESSAGE_ERROR, GST_MESSAGE_STATE_CHANGED, GST_MSECOND, GST_STATE_CHANGE_FAILURE,
    GST_STATE_NULL, GST_STATE_PAUSED, GST_STATE_PLAYING,
};
use gstreamer_sys::gst_message_unref;

use crate::utils::cstring_converter::{cstring_ptr_to_str, string_to_cstring};

use super::{
    gstreamer_bus::GstreamerBus,
    gstreamer_data::{Data, GstreamerData},
    gstreamer_message::GstreamerMessage,
    gstreamer_pipe::MESSAGE_NAME,
};

const THREAD_NAME: &str = "streamer";
const UPDATE_POSITION_MILLISECONDS: i64 = 100;
const RECEIVE_TIMEOUT_DURATION: Duration = Duration::from_secs(5);

#[derive(Debug)]
pub(crate) struct GstreamerThread {
    gstreamer_bus: Arc<dyn GstreamerBus>,
    gstreamer_data: Arc<dyn GstreamerData>,
    receiver: Receiver<GstreamerMessage>,
}

unsafe impl Send for GstreamerThread {}
unsafe impl Sync for GstreamerThread {}

impl GstreamerThread {
    pub(crate) fn start(
        gstreamer_bus: Arc<dyn GstreamerBus>,
        gstreamer_data: Arc<dyn GstreamerData>,
        receiver: Receiver<GstreamerMessage>,
    ) -> JoinHandle<()> {
        let self_ = Self::new(gstreamer_bus, gstreamer_data, receiver);

        thread::Builder::new()
            .name(THREAD_NAME.to_string())
            .spawn(move || {
                self_.gst_thread();
            })
            .unwrap()
    }

    fn new(
        gstreamer_bus: Arc<dyn GstreamerBus>,
        gstreamer_data: Arc<dyn GstreamerData>,
        receiver: Receiver<GstreamerMessage>,
    ) -> Self {
        Self {
            gstreamer_bus,
            gstreamer_data,
            receiver,
        }
    }

    fn gst_thread(&self) {
        let mut gstreamer_message = GstreamerMessage::default();

        while !matches!(gstreamer_message, GstreamerMessage::End) {
            if matches!(gstreamer_message, GstreamerMessage::None) {
                gstreamer_message = self.receiver.recv().unwrap();
            }

            gstreamer_message = match gstreamer_message {
                GstreamerMessage::None => {
                    eprintln!("GStreamer loop received a message with `None`.");
                    GstreamerMessage::None
                }
                GstreamerMessage::Play => {
                    if let Some(transfer) = self.gstreamer_data.consume() {
                        self.gst(transfer.uri, transfer.data)
                    } else {
                        eprintln!("No data sent to the streamer.");
                        GstreamerMessage::None
                    }
                }
                GstreamerMessage::Pause | GstreamerMessage::Stop => GstreamerMessage::None,
                GstreamerMessage::End => GstreamerMessage::End,
            }
        }
    }

    fn init(&self) {
        let args = std::env::args()
            .map(|arg| string_to_cstring(arg))
            .collect::<Vec<CString>>();

        let mut c_args = args
            .iter()
            .map(|arg| arg.clone().into_raw())
            .collect::<Vec<*mut c_char>>();

        unsafe { gst_init(&mut (c_args.len() as i32), &mut c_args.as_mut_ptr()) };
    }

    fn launch(&self, uri: String) -> *mut GstElement {
        let pipeline_description = string_to_cstring(format!("playbin uri=\"{uri}\""));
        let pipeline;

        unsafe {
            pipeline = gst_parse_launch(pipeline_description.as_ptr(), null_mut());
            self.gstreamer_bus.set(gst_element_get_bus(pipeline));

            if gst_element_set_state(pipeline, GST_STATE_PLAYING) == GST_STATE_CHANGE_FAILURE {
                gst_object_unref(pipeline as *mut GstObject);
                panic!("GStreamer returns a failure.");
            }
        }

        pipeline
    }

    fn gst(&self, uri: String, mut data: Data) -> GstreamerMessage {
        self.init();
        data.pipeline = self.launch(uri);
        data.is_playing = true;
        data.duration = GST_CLOCK_TIME_NONE as i64;
        let mut gstreamer_message = GstreamerMessage::None;

        while !matches!(
            gstreamer_message,
            GstreamerMessage::Play | GstreamerMessage::Stop | GstreamerMessage::End
        ) {
            let msg = unsafe {
                let bus = self.gstreamer_bus.get_lock();

                if (*bus).is_null() {
                    panic!("The gst bus is null.");
                }

                gst_bus_timed_pop_filtered(
                    *bus,
                    (UPDATE_POSITION_MILLISECONDS * GST_MSECOND) as u64,
                    GST_MESSAGE_STATE_CHANGED
                        | GST_MESSAGE_ERROR
                        | GST_MESSAGE_EOS
                        | GST_MESSAGE_DURATION_CHANGED
                        | GST_MESSAGE_APPLICATION,
                )
            };

            if !msg.is_null() {
                gstreamer_message = self.handle_message(&mut data, msg);

                unsafe { gst_message_unref(msg as *mut GstMessage) };
            } else {
                if data.is_playing {
                    self.update_position(&mut data);
                }
            }
        }

        unsafe {
            let bus = self.gstreamer_bus.take();
            gst_object_unref(bus as *mut GstObject);
            gst_element_set_state(data.pipeline, GST_STATE_NULL);
            gst_object_unref(data.pipeline as *mut GstObject);
        };

        gstreamer_message
    }

    fn handle_message(&self, data: &mut Data, msg: *mut GstMessage) -> GstreamerMessage {
        match (unsafe { *msg }).type_ {
            GST_MESSAGE_ERROR => {
                eprintln!("Error received from element.");
                GstreamerMessage::Stop
            }
            GST_MESSAGE_EOS => {
                // TODO remove?
                println!("End-Of-Stream reached.");
                GstreamerMessage::Stop
            }
            GST_MESSAGE_DURATION_CHANGED => {
                data.duration = GST_CLOCK_TIME_NONE as i64;
                GstreamerMessage::None
            }
            GST_MESSAGE_STATE_CHANGED => {
                let mut old_state: GstState = GST_STATE_NULL;
                let mut new_state: GstState = GST_STATE_NULL;
                let mut pending_state: GstState = GST_STATE_NULL;
                unsafe {
                    gst_message_parse_state_changed(
                        msg,
                        &mut old_state,
                        &mut new_state,
                        &mut pending_state,
                    )
                };
                GstreamerMessage::None
            }
            GST_MESSAGE_APPLICATION => self.handle_application_message(data, msg),
            gst_message_type => {
                eprintln!("Unexpected message number received: {gst_message_type}");
                GstreamerMessage::None
            }
        }
    }

    fn handle_application_message(
        &self,
        data: &mut Data,
        msg: *mut GstMessage,
    ) -> GstreamerMessage {
        let structure = unsafe { gst_message_get_structure(msg) as *mut GstStructure };
        let name_ptr = unsafe { gst_structure_get_name(structure) };
        let name = unsafe { cstring_ptr_to_str(name_ptr) };

        if name.ne(MESSAGE_NAME) {
            eprintln!("Streamer pipe message name error: {name}");
            return GstreamerMessage::None;
        }

        let gstreamer_message = self
            .receiver
            .recv_timeout(RECEIVE_TIMEOUT_DURATION)
            .unwrap_or_default();

        match gstreamer_message {
            GstreamerMessage::Play => {
                // TODO remove?
                println!("Stop request (Async) and new uri.");
                GstreamerMessage::Play
            }
            GstreamerMessage::None => {
                eprintln!("Message with 'None' is an error due to a possible receive timeout.");
                GstreamerMessage::None
            }
            GstreamerMessage::Pause => {
                if data.is_playing {
                    unsafe { gst_element_set_state(data.pipeline, GST_STATE_PAUSED) };
                    data.is_playing = false;
                } else {
                    unsafe { gst_element_set_state(data.pipeline, GST_STATE_PLAYING) };
                    data.is_playing = true;
                }
                GstreamerMessage::None
            }
            GstreamerMessage::Stop => {
                // TODO remove?
                println!("Stop request.");
                GstreamerMessage::Stop
            }
            GstreamerMessage::End => {
                // TODO remove?
                println!("End request.");
                GstreamerMessage::End
            }
        }
    }

    fn update_position(&self, data: &mut Data) {
        let current: i64 = if let Some(position) = self.query_position(data.pipeline) {
            position
        } else {
            eprintln!("Could not query current position.");
            -1
        };

        if data.duration == GST_CLOCK_TIME_NONE as i64 {
            if let Some(new_duration) = self.query_duration(data.pipeline) {
                data.duration = new_duration;
            } else {
                eprintln!("Could not query current duration.");
            }
        }

        // TODO Temp
        println!("Position {} / {}", current, data.duration);
    }

    fn query_position(&self, pipeline: *mut GstElement) -> Option<i64> {
        let mut position: i64 = -1;
        let result =
            unsafe { gst_element_query_position(pipeline, GST_FORMAT_TIME, &mut position) };

        if !result.is_positive() {
            return None;
        }

        Some(position)
    }

    fn query_duration(&self, pipeline: *mut GstElement) -> Option<i64> {
        let mut duration: i64 = -1;
        let result =
            unsafe { gst_element_query_duration(pipeline, GST_FORMAT_TIME, &mut duration) };

        if !result.is_positive() {
            return None;
        }

        Some(duration)
    }
}
