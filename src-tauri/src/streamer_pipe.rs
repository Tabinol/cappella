use std::{
    ffi::{CStr, CString},
    ptr::{null, null_mut},
    sync::Mutex,
};

use gstreamer::glib::gobject_ffi::G_TYPE_STRING;
use gstreamer_sys::{
    gst_bus_post, gst_message_new_application, gst_structure_new, gst_structure_new_empty, GstBus,
    GstStructure,
};

#[derive(Debug)]
pub(crate) struct StreamerPipe {
    pub(crate) bus: Mutex<*mut GstBus>,
}

unsafe impl Send for StreamerPipe {}
unsafe impl Sync for StreamerPipe {}

pub(crate) const MESSAGE_NAME_PAUSE: &str = "APPLICATION_PAUSE";
pub(crate) const MESSAGE_NAME_STOP: &str = "APPLICATION_STOP";
pub(crate) const MESSAGE_NAME_STOP_SYNC: &str = "APPLICATION_STOP_SYNC";
pub(crate) const MESSAGE_NAME_STOP_AND_SEND_NEW_URI: &str = "APPLICATION_STOP_AND_SEND_NEW_URI";
pub(crate) const MESSAGE_FIELD_URI: &str = "URI";

pub(crate) fn str_to_cstring(str: &str) -> CString {
    CString::new(str).unwrap()
}

pub(crate) fn string_to_cstring(string: String) -> CString {
    CString::new(string).unwrap()
}

pub(crate) unsafe fn cstring_ptr_to_str<'a>(ptr: *const i8) -> &'a str {
    CStr::from_ptr(ptr).to_str().unwrap()
}

impl StreamerPipe {
    pub(crate) fn new() -> Self {
        Self {
            bus: Mutex::new(null_mut()),
        }
    }

    pub(crate) fn send_pause(&self) {
        let structure;
        let name = str_to_cstring(MESSAGE_NAME_PAUSE);

        unsafe {
            structure = gst_structure_new_empty(name.as_ptr());
        }

        self.send(structure);
    }

    pub(crate) fn send_stop(&self) {
        let structure;
        let name = str_to_cstring(MESSAGE_NAME_STOP);

        unsafe {
            structure = gst_structure_new_empty(name.as_ptr());
        }

        self.send(structure);
    }

    pub(crate) fn send_stop_sync(&self) {
        let structure;
        let name = str_to_cstring(MESSAGE_NAME_STOP_SYNC);

        unsafe {
            structure = gst_structure_new_empty(name.as_ptr());
        }

        self.send(structure);
    }

    pub(crate) fn send_stop_and_send_new_uri(&self, uri: &str) {
        let structure;
        let name = str_to_cstring(MESSAGE_NAME_STOP_AND_SEND_NEW_URI);
        let field_uri = str_to_cstring(MESSAGE_FIELD_URI);
        let uri_cstring = str_to_cstring(uri);

        unsafe {
            structure = gst_structure_new(
                name.as_ptr(),
                field_uri.as_ptr(),
                G_TYPE_STRING,
                uri_cstring.as_ptr(),
                null() as *const i8,
            );
        }

        self.send(structure);
    }

    fn send(&self, structure: *mut GstStructure) {
        let bus = self.bus.lock().unwrap();

        unsafe {
            let gst_msg = gst_message_new_application(null_mut(), structure);
            gst_bus_post(*bus, gst_msg);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_str_to_cstring() {
        let str = "abcd";
        let cstring = str_to_cstring(str);

        assert_eq!(cstring, CString::new("abcd").unwrap());
    }

    #[test]
    fn test_cstring_ptr_to_str() {
        let cstring = CString::new("abcd").unwrap();
        let cstring_ptr = cstring.as_ptr();
        let str = unsafe { cstring_ptr_to_str(cstring_ptr) };

        assert_eq!(str, "abcd");
    }

    mod tests {
        use std::ptr::null_mut;

        use gstreamer::glib::ffi::{gboolean, GTRUE};
        use gstreamer_sys::{
            gst_message_get_structure, gst_message_unref, gst_structure_get_name,
            gst_structure_get_string, GstBus, GstMessage,
        };

        use crate::streamer_pipe::{
            cstring_ptr_to_str, str_to_cstring, StreamerPipe, MESSAGE_FIELD_URI,
            MESSAGE_NAME_PAUSE, MESSAGE_NAME_STOP, MESSAGE_NAME_STOP_AND_SEND_NEW_URI,
            MESSAGE_NAME_STOP_SYNC,
        };

        struct Message(*mut GstMessage);

        unsafe impl Sync for Message {}

        static mut MESSAGE: Message = Message(null_mut());

        #[no_mangle]
        #[allow(unused_variables)]
        extern "C" fn gst_bus_post(bus: *mut GstBus, message: *mut GstMessage) -> gboolean {
            unsafe {
                MESSAGE.0 = message;
            }

            GTRUE
        }

        #[test]
        fn test_send_pause() {
            let streamer_pipe = StreamerPipe::new();
            let name;
            let message;

            streamer_pipe.send_pause();

            unsafe {
                let structure = gst_message_get_structure(MESSAGE.0);
                let name_ptr = gst_structure_get_name(structure);
                name = cstring_ptr_to_str(name_ptr);
                message = MESSAGE.0;
            }

            assert_eq!(name, MESSAGE_NAME_PAUSE);

            unsafe {
                gst_message_unref(message);
                MESSAGE.0 = null_mut();
            }
        }

        #[test]
        fn test_send_stop() {
            let streamer_pipe = StreamerPipe::new();
            let name;
            let message;

            streamer_pipe.send_stop();

            unsafe {
                let structure = gst_message_get_structure(MESSAGE.0);
                let name_ptr = gst_structure_get_name(structure);
                name = cstring_ptr_to_str(name_ptr);
                message = MESSAGE.0;
            }

            assert_eq!(name, MESSAGE_NAME_STOP);

            unsafe {
                gst_message_unref(message);
                MESSAGE.0 = null_mut();
            }
        }

        #[test]
        fn test_send_stop_sync() {
            let streamer_pipe = StreamerPipe::new();
            let name;
            let message;

            streamer_pipe.send_stop_sync();

            unsafe {
                let structure = gst_message_get_structure(MESSAGE.0);
                let name_ptr = gst_structure_get_name(structure);
                name = cstring_ptr_to_str(name_ptr);
                message = MESSAGE.0;
            }

            assert_eq!(name, MESSAGE_NAME_STOP_SYNC);

            unsafe {
                gst_message_unref(message);
                MESSAGE.0 = null_mut();
            }
        }

        #[test]
        fn test_send_stop_and_send_new_uri() {
            let streamer_pipe = StreamerPipe::new();
            let name;
            let uri;
            let message;

            streamer_pipe.send_stop_and_send_new_uri("newuri");

            unsafe {
                let structure = gst_message_get_structure(MESSAGE.0);
                let name_ptr = gst_structure_get_name(structure);
                let field_uri = str_to_cstring(MESSAGE_FIELD_URI);
                let uri_ptr = gst_structure_get_string(structure, field_uri.as_ptr());
                uri = cstring_ptr_to_str(uri_ptr);
                name = cstring_ptr_to_str(name_ptr);
                message = MESSAGE.0;
            }

            assert_eq!(name, MESSAGE_NAME_STOP_AND_SEND_NEW_URI);
            assert_eq!(uri, "newuri");

            unsafe {
                gst_message_unref(message);
                MESSAGE.0 = null_mut();
            }
        }
    }
}
