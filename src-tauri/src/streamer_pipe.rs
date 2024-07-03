use std::{
    ffi::{CStr, CString},
    fmt::Debug,
    ptr::{null, null_mut},
    sync::{Arc, Mutex},
};

use gstreamer::glib::gobject_ffi::G_TYPE_STRING;
use gstreamer_sys::{
    gst_bus_post, gst_message_new_application, gst_structure_new, GstBus, GstStructure,
};

#[cfg(test)]
use mockall::automock;

pub(crate) const MESSAGE_NAME: &str = "APP_MSG";
pub(crate) const MESSAGE_FIELD_JSON: &str = "JSON";

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub(crate) enum Message {
    None,
    Pause,
    Next(String),
    Stop,
    End,
}

#[cfg_attr(test, automock)]
pub(crate) trait StreamerPipe: Debug + Send + Sync {
    fn set_bus(&self, bus: *mut GstBus);
    fn send(&self, message: Message);
}

#[derive(Debug)]
pub(crate) struct ImplStreamerPipe {
    pub(crate) bus: Arc<Mutex<*mut GstBus>>,
}

unsafe impl Send for ImplStreamerPipe {}
unsafe impl Sync for ImplStreamerPipe {}

pub(crate) fn str_to_cstring(str: &str) -> CString {
    CString::new(str).unwrap()
}

pub(crate) fn string_to_cstring(string: String) -> CString {
    CString::new(string).unwrap()
}

pub(crate) unsafe fn cstring_ptr_to_str<'a>(ptr: *const i8) -> &'a str {
    CStr::from_ptr(ptr).to_str().unwrap()
}

impl ImplStreamerPipe {
    pub(crate) fn new() -> Self {
        Self {
            bus: Arc::new(Mutex::new(null_mut())),
        }
    }

    fn send_to_gst(&self, structure: *mut GstStructure) {
        let bus = self.bus.lock().unwrap();

        #[cfg(not(test))]
        if bus.is_null() {
            eprintln!("Unable to send the message to streamer.");
            return;
        }

        unsafe {
            let gst_msg = gst_message_new_application(null_mut(), structure);
            gst_bus_post(*bus, gst_msg);
        }
    }
}

impl StreamerPipe for ImplStreamerPipe {
    fn set_bus(&self, bus: *mut GstBus) {
        *self.bus.lock().unwrap() = bus;
    }

    fn send(&self, message: Message) {
        let structure;
        let name = str_to_cstring(MESSAGE_NAME);
        let field_json = str_to_cstring(MESSAGE_FIELD_JSON);
        let json = serde_json::to_string(&message).unwrap();
        let json_cstring = string_to_cstring(json);

        unsafe {
            structure = gst_structure_new(
                name.as_ptr(),
                field_json.as_ptr(),
                G_TYPE_STRING,
                json_cstring.as_ptr(),
                null() as *const i8,
            );
        }

        self.send_to_gst(structure);
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::CString;

    use crate::streamer_pipe::{cstring_ptr_to_str, str_to_cstring, string_to_cstring};

    #[test]
    fn test_str_to_cstring() {
        let str = "abcd";
        let cstring = str_to_cstring(str);

        assert_eq!(cstring, CString::new("abcd").unwrap());
    }

    #[test]
    fn test_string_to_cstring() {
        let str = "abcd".to_string();
        let cstring = string_to_cstring(str);

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
        use std::{ptr::null_mut, sync::Mutex};

        use gstreamer::glib::ffi::{gboolean, GTRUE};
        use gstreamer_sys::{
            gst_message_get_structure, gst_message_unref, gst_structure_get_name,
            gst_structure_get_string, GstBus, GstMessage,
        };

        use crate::streamer_pipe::{
            self, cstring_ptr_to_str, str_to_cstring, ImplStreamerPipe, StreamerPipe,
            MESSAGE_FIELD_JSON, MESSAGE_NAME,
        };

        struct Message(*mut GstMessage);

        unsafe impl Sync for Message {}

        static mut MESSAGE: Message = Message(null_mut());
        static LOCK: Mutex<()> = Mutex::new(());

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
            let _lock = LOCK.lock().unwrap();
            let streamer_pipe = ImplStreamerPipe::new();
            let name;
            let result_message: streamer_pipe::Message;
            let message;

            streamer_pipe.send(streamer_pipe::Message::Pause);

            unsafe {
                let structure = gst_message_get_structure(MESSAGE.0);
                let name_ptr = gst_structure_get_name(structure);
                let field_json = str_to_cstring(MESSAGE_FIELD_JSON);
                let json_ptr = gst_structure_get_string(structure, field_json.as_ptr());
                let json = cstring_ptr_to_str(json_ptr);
                name = cstring_ptr_to_str(name_ptr);
                result_message = serde_json::from_str(json).unwrap();
                message = MESSAGE.0;
            }

            assert_eq!(name, MESSAGE_NAME);
            assert!(matches!(result_message, streamer_pipe::Message::Pause));

            unsafe {
                gst_message_unref(message);
                MESSAGE.0 = null_mut();
            }
        }

        #[test]
        fn test_send_next() {
            let _lock = LOCK.lock().unwrap();
            let streamer_pipe = ImplStreamerPipe::new();
            let name;
            let result_message: streamer_pipe::Message;
            let message;

            streamer_pipe.send(streamer_pipe::Message::Next("new_uri".to_string()));

            unsafe {
                let structure = gst_message_get_structure(MESSAGE.0);
                let name_ptr = gst_structure_get_name(structure);
                let field_json = str_to_cstring(MESSAGE_FIELD_JSON);
                let json_ptr = gst_structure_get_string(structure, field_json.as_ptr());
                let json = cstring_ptr_to_str(json_ptr);
                name = cstring_ptr_to_str(name_ptr);
                result_message = serde_json::from_str(json).unwrap();
                message = MESSAGE.0;
            }

            assert_eq!(name, MESSAGE_NAME);
            assert!(matches!(result_message, streamer_pipe::Message::Next(_)));
            assert!(if let streamer_pipe::Message::Next(uri) = result_message {
                uri.eq("new_uri")
            } else {
                false
            });

            unsafe {
                gst_message_unref(message);
                MESSAGE.0 = null_mut();
            }
        }

        #[test]
        fn test_send_stop() {
            let _lock = LOCK.lock().unwrap();
            let streamer_pipe = ImplStreamerPipe::new();
            let name;
            let result_message: streamer_pipe::Message;
            let message;

            streamer_pipe.send(streamer_pipe::Message::Stop);

            unsafe {
                let structure = gst_message_get_structure(MESSAGE.0);
                let name_ptr = gst_structure_get_name(structure);
                let field_json = str_to_cstring(MESSAGE_FIELD_JSON);
                let json_ptr = gst_structure_get_string(structure, field_json.as_ptr());
                let json = cstring_ptr_to_str(json_ptr);
                name = cstring_ptr_to_str(name_ptr);
                result_message = serde_json::from_str(json).unwrap();
                message = MESSAGE.0;
            }

            assert_eq!(name, MESSAGE_NAME);
            assert!(matches!(result_message, streamer_pipe::Message::Stop));

            unsafe {
                gst_message_unref(message);
                MESSAGE.0 = null_mut();
            }
        }

        #[test]
        fn test_send_end() {
            let _lock = LOCK.lock().unwrap();
            let streamer_pipe = ImplStreamerPipe::new();
            let name;
            let result_message: streamer_pipe::Message;
            let message;

            streamer_pipe.send(streamer_pipe::Message::End);

            unsafe {
                let structure = gst_message_get_structure(MESSAGE.0);
                let name_ptr = gst_structure_get_name(structure);
                let field_json = str_to_cstring(MESSAGE_FIELD_JSON);
                let json_ptr = gst_structure_get_string(structure, field_json.as_ptr());
                let json = cstring_ptr_to_str(json_ptr);
                name = cstring_ptr_to_str(name_ptr);
                result_message = serde_json::from_str(json).unwrap();
                message = MESSAGE.0;
            }

            assert_eq!(name, MESSAGE_NAME);
            assert!(matches!(result_message, streamer_pipe::Message::End));

            unsafe {
                gst_message_unref(message);
                MESSAGE.0 = null_mut();
            }
        }
    }
}
