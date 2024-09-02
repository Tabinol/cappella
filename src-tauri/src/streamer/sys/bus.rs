use std::{fmt::Debug, time::Duration};

use glib_sys::GTRUE;
use gstreamer_sys::{
    gst_bus_post, gst_bus_timed_pop_filtered, gst_object_unref, GstBus, GstMessageType, GstObject,
};

use super::message::Message;

#[derive(Debug)]
pub struct Bus(*mut GstBus);

impl Bus {
    pub fn new(bus: *mut GstBus) -> Self {
        Bus(bus)
    }
    pub fn get(&self) -> *mut GstBus {
        self.0
    }

    pub fn post(&self, message: &Message) -> bool {
        let message = message.get();
        unsafe { gst_bus_post(self.get(), message) == GTRUE }
    }

    pub fn timed_pop_filtered(&self, timeout: Duration, type_: GstMessageType) -> Option<Message> {
        let message_ptr = unsafe {
            gst_bus_timed_pop_filtered(self.get(), timeout.as_nanos().try_into().unwrap(), type_)
        };

        if !message_ptr.is_null() {
            return Some(Message::new(message_ptr));
        }

        None
    }
}

impl Drop for Bus {
    fn drop(&mut self) {
        unsafe { gst_object_unref(self.get() as *mut GstObject) }
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use glib_sys::{GFALSE, GTRUE};
    use gstreamer_sys::GST_MESSAGE_APPLICATION;

    use crate::streamer::sys::{
        common_tests::{ObjectType, RcRefCellTestStructure, TestStructure},
        message::Message,
    };

    use super::Bus;

    #[test]
    fn test_post_true() {
        let test_structure = TestStructure::new_arc_mutex();
        let bus = Bus::new(test_structure.faked_gst_bus());
        let message = Message::new(test_structure.faked_gst_message());

        test_structure.set_gst_bus_post_return(GTRUE);
        let result = bus.post(&message);

        assert!(result)
    }

    #[test]
    fn test_post_false() {
        let test_structure = TestStructure::new_arc_mutex();
        let bus = Bus::new(test_structure.faked_gst_bus());
        let message = Message::new(test_structure.faked_gst_message());

        test_structure.set_gst_bus_post_return(GFALSE);
        let result = bus.post(&message);

        assert!(!result)
    }

    #[test]
    fn test_timed_pop_filtered_true() {
        let test_structure = TestStructure::new_arc_mutex();
        let bus = Bus::new(test_structure.faked_gst_bus());

        test_structure.set_pop_message(true);
        let message = bus.timed_pop_filtered(Duration::from_secs(1), GST_MESSAGE_APPLICATION);

        assert!(message.is_some(), "No message is returned.")
    }

    #[test]
    fn test_timed_pop_filtered_false() {
        let test_structure = TestStructure::new_arc_mutex();
        let bus = Bus::new(test_structure.faked_gst_bus());

        test_structure.set_pop_message(false);
        let message = bus.timed_pop_filtered(Duration::from_secs(1), GST_MESSAGE_APPLICATION);

        assert!(message.is_none(), "No message should be popped.")
    }

    #[test]
    fn test_drop() {
        let test_structure = TestStructure::new_arc_mutex();
        {
            let _bus = Bus::new(test_structure.faked_gst_bus());
        }

        assert!(
            test_structure.is_unref(ObjectType::GstBus),
            "The bus is not unref."
        )
    }
}
