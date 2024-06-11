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
pub(crate) enum Status {
    Active,
    Async,
    Sync,
    PlayNext(String),
    Inactive,
}

#[derive(Debug)]
pub(crate) struct StreamerPipe {
    pub(crate) bus: Mutex<*mut GstBus>,
    pub(crate) streamer_lock: Mutex<()>,
    pub(crate) status: Mutex<Status>,
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

pub(crate) unsafe fn cstring_ptr_to_str<'a>(ptr: *const i8) -> &'a str {
    CStr::from_ptr(ptr).to_str().unwrap()
}

impl StreamerPipe {
    pub(crate) fn new() -> Self {
        Self {
            bus: Mutex::new(null_mut()),
            streamer_lock: Mutex::new(()),
            status: Mutex::new(Status::Inactive),
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

        if bus.is_null() {
            eprintln!("Unable to send the message to the streamer because the bus is null.");
            return;
        }

        unsafe {
            let gst_msg = gst_message_new_application(null_mut(), structure);
            gst_bus_post(*bus, gst_msg);
        }
    }
}
