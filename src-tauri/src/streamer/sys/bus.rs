use std::{fmt::Debug, ptr::NonNull, time::Duration};

use glib_sys::GTRUE;
use gstreamer_sys::{
    gst_bus_post, gst_bus_timed_pop_filtered, gst_object_unref, GstBus, GstMessage, GstMessageType,
    GstObject,
};

use super::message::Message;

#[derive(Debug)]
pub(crate) struct Bus(NonNull<GstBus>);

impl Bus {
    pub(crate) fn new(bus: NonNull<GstBus>) -> Self {
        Bus(bus)
    }
    pub(crate) fn get(&self) -> *mut GstBus {
        self.0.as_ptr()
    }

    pub(crate) fn post(&self, message: &Message) -> bool {
        let message = message.get();
        unsafe { gst_bus_post(self.get(), message) == GTRUE }
    }

    pub(crate) fn timed_pop_filtered(
        &self,
        timeout: Duration,
        type_: GstMessageType,
    ) -> Option<Message> {
        let message_ptr = unsafe {
            gst_bus_timed_pop_filtered(self.get(), timeout.as_nanos().try_into().unwrap(), type_)
        };

        if let Some(message) = NonNull::new(message_ptr as *mut GstMessage) {
            return Some(Message::new(message));
        }

        None
    }
}

impl Drop for Bus {
    fn drop(&mut self) {
        unsafe { gst_object_unref(self.get() as *mut GstObject) }
    }
}
