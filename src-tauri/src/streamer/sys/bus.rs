use std::{fmt::Debug, time::Duration};

use glib_sys::GTRUE;
use gstreamer_sys::{
    gst_bus_post, gst_bus_timed_pop_filtered, gst_object_unref, GstBus, GstMessageType, GstObject,
};

use crate::local::app_error::AppError;

use super::message::Message;

#[derive(Debug)]
pub struct Bus(*mut GstBus);

impl Bus {
    pub fn new(bus: *mut GstBus) -> Result<Self, AppError> {
        if bus.is_null() {
            return Err(AppError::new("The bus pointer is null.".to_owned()));
        }

        Ok(Self(bus))
    }

    pub fn get(&self) -> *mut GstBus {
        self.0
    }

    pub fn post(&self, message: &Message) -> Result<(), AppError> {
        let message_ptr = message.get();

        if unsafe { gst_bus_post(self.get(), message_ptr) } != GTRUE {
            return Err(AppError::new(format!(
                "GStreamer returns `false` for the message: {message}"
            )));
        }

        Ok(())
    }

    pub fn timed_pop_filtered(
        &self,
        timeout: Duration,
        type_: GstMessageType,
    ) -> Result<Option<Message>, AppError> {
        let message_ptr = unsafe {
            gst_bus_timed_pop_filtered(self.get(), timeout.as_nanos().try_into()?, type_)
        };

        if !message_ptr.is_null() {
            return Ok(Some(Message::new(message_ptr)?));
        }

        Ok(None)
    }
}

impl Drop for Bus {
    fn drop(&mut self) {
        unsafe { gst_object_unref(self.get() as *mut GstObject) }
    }
}

#[cfg(test)]
mod test {
    use std::{ptr::null_mut, time::Duration};

    use glib_sys::{GFALSE, GTRUE};
    use gstreamer_sys::GST_MESSAGE_APPLICATION;

    use crate::streamer::sys::common_tests::{
        RcRefCellTestStructure, TestObjectType, TestStructure, UNASSIGNED,
    };
    use crate::streamer::sys::message::Message;

    use super::Bus;

    #[test]
    fn test_new_ok() {
        let test_structure = TestStructure::new_arc_mutex(UNASSIGNED);

        let bus_res = Bus::new(test_structure.faked_gst_bus());

        assert!(bus_res.is_ok());
    }

    #[test]
    fn test_new_err() {
        let bus_res = Bus::new(null_mut());

        assert!(bus_res.is_err());
    }

    #[test]
    fn test_post_ok() {
        let test_structure = TestStructure::new_arc_mutex_assigned();
        let bus = Bus::new(test_structure.faked_gst_bus()).unwrap();
        let message = Message::new(test_structure.faked_gst_message()).unwrap();

        test_structure.set_gst_bus_post_return(GTRUE);
        let result = bus.post(&message);

        assert!(result.is_ok())
    }

    #[test]
    fn test_post_err() {
        let test_structure = TestStructure::new_arc_mutex_assigned();
        let bus = Bus::new(test_structure.faked_gst_bus()).unwrap();
        let message = Message::new(test_structure.faked_gst_message()).unwrap();

        test_structure.set_gst_bus_post_return(GFALSE);
        let result = bus.post(&message);

        assert!(result.is_err())
    }

    #[test]
    fn test_timed_pop_filtered_true() {
        let test_structure = TestStructure::new_arc_mutex_assigned();
        let bus = Bus::new(test_structure.faked_gst_bus()).unwrap();

        test_structure.set_pop_message(true);
        let message = bus.timed_pop_filtered(Duration::from_secs(1), GST_MESSAGE_APPLICATION);

        assert!(message.unwrap().is_some(), "No message is returned.")
    }

    #[test]
    fn test_timed_pop_filtered_false() {
        let test_structure = TestStructure::new_arc_mutex_assigned();
        let bus = Bus::new(test_structure.faked_gst_bus()).unwrap();

        test_structure.set_pop_message(false);
        let message = bus.timed_pop_filtered(Duration::from_secs(1), GST_MESSAGE_APPLICATION);

        assert!(message.unwrap().is_none(), "No message should be popped.")
    }

    #[test]
    fn test_drop() {
        let test_structure = TestStructure::new_arc_mutex_assigned();
        {
            let _bus = Bus::new(test_structure.faked_gst_bus()).unwrap();
        }

        assert!(
            test_structure.is_unref(TestObjectType::GstBus),
            "The bus is not unref."
        )
    }
}
