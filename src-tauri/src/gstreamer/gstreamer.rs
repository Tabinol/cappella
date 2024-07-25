use std::{
    ffi::{c_char, CString},
    fmt::Debug,
    ptr::{null, null_mut},
    sync::{Arc, Mutex},
};

use gstreamer::glib::gobject_ffi::G_TYPE_STRING;
use gstreamer_sys::{
    gst_bus_post, gst_bus_timed_pop_filtered, gst_element_get_bus, gst_element_set_state, gst_init,
    gst_message_get_structure, gst_message_new_application, gst_object_unref, gst_parse_launch,
    gst_structure_new, GstBus, GstObject, GstStructure, GST_MESSAGE_APPLICATION,
    GST_MESSAGE_DURATION_CHANGED, GST_MESSAGE_EOS, GST_MESSAGE_ERROR, GST_MESSAGE_STATE_CHANGED,
    GST_MSECOND, GST_STATE_CHANGE_FAILURE, GST_STATE_PLAYING,
};

use crate::utils::cstring_converter::{str_to_cstring, string_to_cstring};

use super::{
    gstreamer_message::{GstreamerMessage, ImplGstreamerMessage},
    gstreamer_pipeline::{GstreamerPipeline, ImplGstreamerPipeline},
};

pub(crate) const GST_CLOCK_TIME_NONE: i64 = gstreamer_sys::GST_CLOCK_TIME_NONE as i64;

const UPDATE_POSITION_MILLISECONDS: i64 = 100;

pub(crate) trait Gstreamer: Debug + Send + Sync {
    fn init(&self);
    fn launch(&self, uri: &str) -> Box<dyn GstreamerPipeline>;
    fn bus_timed_pop_filtered(&self) -> Option<Box<dyn GstreamerMessage>>;
    fn send_to_gst(&self, name: &str, key: &str, value: &str);
}

#[derive(Debug)]
pub(crate) struct ImplGstreamer {
    bus: Arc<Mutex<*mut GstBus>>,
}

impl Default for ImplGstreamer {
    fn default() -> Self {
        Self {
            bus: Arc::new(Mutex::new(null_mut())),
        }
    }
}

unsafe impl Send for ImplGstreamer {}
unsafe impl Sync for ImplGstreamer {}

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

        let pipeline = unsafe {
            let pipeline = gst_parse_launch(pipeline_description.as_ptr(), null_mut());
            let mut bus = self.bus.lock().unwrap();

            if !bus.is_null() {
                panic!("The gst bus is already assigned.")
            }

            *bus = gst_element_get_bus(pipeline);

            if gst_element_set_state(pipeline, GST_STATE_PLAYING) == GST_STATE_CHANGE_FAILURE {
                gst_object_unref(pipeline as *mut GstObject);
                panic!("GStreamer returns a failure.");
            }

            pipeline
        };

        Box::new(ImplGstreamerPipeline::new(pipeline, self.bus.clone()))
    }

    fn bus_timed_pop_filtered(&self) -> Option<Box<dyn GstreamerMessage>> {
        let msg = unsafe {
            let bus = self.bus.lock().unwrap();

            if bus.is_null() {
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
            let structure = unsafe { gst_message_get_structure(msg) as *mut GstStructure };
            return Some(Box::new(ImplGstreamerMessage::new(msg, structure)));
        }

        None
    }

    fn send_to_gst(&self, name: &str, key: &str, value: &str) {
        let bus = self.bus.lock().unwrap();

        if bus.is_null() {
            eprintln!("Unable to send the message to streamer because the gst bus is null.");
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
            gst_bus_post(*bus, gst_msg);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::{c_char, c_int};

    use gstreamer::glib::ffi::GError;
    use gstreamer_sys::{GstBus, GstElement, GST_STATE_CHANGE_FAILURE};

    use crate::gstreamer::{
        gstreamer::Gstreamer,
        tests_common::{
            self, get_gst_bus_ptr, get_gst_element_ptr, ELEMENT_SET_STATE_RESULT, LOCK,
            OBJECT_UNREF_CALL_NB,
        },
    };

    use super::ImplGstreamer;

    static mut INIT_CALL_NB: u32 = 0;
    static mut PARSE_LAUNCH_CALL_NB: u32 = 0;
    static mut ELEMENT_GET_BUS_CALL_NB: u32 = 0;

    #[no_mangle]
    extern "C" fn gst_init(_argc: *mut c_int, _argv: *mut *mut *mut c_char) {
        unsafe { INIT_CALL_NB += 1 };
    }

    #[no_mangle]
    extern "C" fn gst_parse_launch(
        _pipeline_description: *const c_char,
        _error: *mut *mut GError,
    ) -> *mut GstElement {
        unsafe {
            PARSE_LAUNCH_CALL_NB += 1;
        }

        get_gst_element_ptr()
    }

    #[no_mangle]
    extern "C" fn gst_element_get_bus(_element: *mut GstElement) -> *mut GstBus {
        unsafe {
            ELEMENT_GET_BUS_CALL_NB += 1;
        }

        get_gst_bus_ptr()
    }

    fn before_each() {
        tests_common::before_each();

        unsafe {
            INIT_CALL_NB = 0;
            PARSE_LAUNCH_CALL_NB = 0;
            ELEMENT_GET_BUS_CALL_NB = 0;
        }
    }

    #[test]
    fn test_init() {
        before_each();

        let _lock = LOCK.lock().unwrap();
        let gstreamer = ImplGstreamer::default();

        gstreamer.init();

        assert_eq!(unsafe { INIT_CALL_NB }, 1);
    }

    #[test]
    fn test_launch() {
        before_each();

        let _lock = LOCK.lock().unwrap();
        let gstreamer = ImplGstreamer::default();

        gstreamer.launch("uri");

        assert_eq!(unsafe { PARSE_LAUNCH_CALL_NB }, 1);
        assert_eq!(unsafe { ELEMENT_GET_BUS_CALL_NB }, 1);
        assert_eq!(unsafe { OBJECT_UNREF_CALL_NB }, 2);
    }

    #[test]
    #[should_panic]
    fn test_launch_failure() {
        before_each();

        let _lock = LOCK.lock().unwrap();
        let gstreamer = ImplGstreamer::default();

        unsafe { ELEMENT_SET_STATE_RESULT = GST_STATE_CHANGE_FAILURE }
        gstreamer.launch("uri");

        assert_eq!(unsafe { PARSE_LAUNCH_CALL_NB }, 1);
        assert_eq!(unsafe { ELEMENT_GET_BUS_CALL_NB }, 1);
        assert_eq!(unsafe { OBJECT_UNREF_CALL_NB }, 1);
    }
}
