use std::{
    ffi::{c_char, CString},
    fmt::Debug,
    ptr::{null, null_mut},
    sync::{Arc, Mutex},
};

use dyn_clone::DynClone;
use gstreamer::glib::gobject_ffi::G_TYPE_STRING;
use gstreamer_sys::{
    gst_bus_post, gst_bus_timed_pop_filtered, gst_element_get_bus, gst_element_set_state, gst_init,
    gst_message_get_structure, gst_message_new_application, gst_object_unref, gst_parse_launch,
    gst_structure_new, GstBus, GstObject, GST_MESSAGE_APPLICATION, GST_MESSAGE_DURATION_CHANGED,
    GST_MESSAGE_EOS, GST_MESSAGE_ERROR, GST_MESSAGE_STATE_CHANGED, GST_MSECOND,
    GST_STATE_CHANGE_FAILURE, GST_STATE_PLAYING,
};

use crate::utils::{
    cstring_converter::{str_to_cstring, string_to_cstring},
    pointer::{PointerConst, PointerMut},
};

use super::{
    gstreamer_message::{GstreamerMessage, ImplGstreamerMessage},
    gstreamer_pipeline::{GstreamerPipeline, ImplGstreamerPipeline},
};

pub(crate) const GST_CLOCK_TIME_NONE: i64 = gstreamer_sys::GST_CLOCK_TIME_NONE as i64;

const UPDATE_POSITION_MILLISECONDS: i64 = 100;

pub(crate) trait Gstreamer: Debug + DynClone + Send + Sync {
    fn init(&self);
    fn launch(&self, uri: &str) -> Box<dyn GstreamerPipeline>;
    fn bus_timed_pop_filtered(&self) -> Option<Box<dyn GstreamerMessage>>;
    fn send_to_gst(&self, name: &str, key: &str, value: &str);
}

dyn_clone::clone_trait_object!(Gstreamer);

#[derive(Clone, Debug)]
pub(crate) struct ImplGstreamer {
    bus: Arc<Mutex<PointerMut<GstBus>>>,
}

impl ImplGstreamer {
    pub(crate) fn new() -> Box<dyn Gstreamer> {
        Box::new(Self {
            bus: Arc::new(Mutex::new(PointerMut::new(null_mut()))),
        })
    }
}

impl Gstreamer for ImplGstreamer {
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

    fn launch(&self, uri: &str) -> Box<dyn GstreamerPipeline> {
        let pipeline_description = string_to_cstring(format!("playbin uri=\"{uri}\""));

        let pipeline = PointerMut::new(unsafe {
            let pipeline = gst_parse_launch(pipeline_description.as_ptr(), null_mut());
            self.bus
                .lock()
                .unwrap()
                .replace(gst_element_get_bus(pipeline));

            if gst_element_set_state(pipeline, GST_STATE_PLAYING) == GST_STATE_CHANGE_FAILURE {
                gst_object_unref(pipeline as *mut GstObject);
                panic!("GStreamer returns a failure.");
            }

            pipeline
        });

        ImplGstreamerPipeline::new(pipeline, Arc::clone(&self.bus))
    }

    fn bus_timed_pop_filtered(&self) -> Option<Box<dyn GstreamerMessage>> {
        let msg = PointerMut::new(unsafe {
            gst_bus_timed_pop_filtered(
                self.bus.lock().unwrap().get(),
                (UPDATE_POSITION_MILLISECONDS * GST_MSECOND) as u64,
                GST_MESSAGE_STATE_CHANGED
                    | GST_MESSAGE_ERROR
                    | GST_MESSAGE_EOS
                    | GST_MESSAGE_DURATION_CHANGED
                    | GST_MESSAGE_APPLICATION,
            )
        });

        if !msg.get().is_null() {
            let structure = PointerConst::new(unsafe { gst_message_get_structure(msg.get()) });
            return Some(ImplGstreamerMessage::new(msg, structure));
        }

        None
    }

    fn send_to_gst(&self, name: &str, key: &str, value: &str) {
        let bus = self.bus.lock().unwrap();

        #[cfg(not(test))]
        if bus.get().is_null() {
            eprintln!("Unable to send the message to streamer.");
            return;
        }

        let structure;
        let name_cstring = str_to_cstring(name);
        let key_cstring = str_to_cstring(key);
        let value_cstring = str_to_cstring(value);

        unsafe {
            structure = gst_structure_new(
                name_cstring.as_ptr(),
                key_cstring.as_ptr(),
                G_TYPE_STRING,
                value_cstring.as_ptr(),
                null() as *const i8,
            );
        }

        unsafe {
            let gst_msg = gst_message_new_application(null_mut(), structure);
            gst_bus_post(bus.get(), gst_msg);
        }
    }
}
