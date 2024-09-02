use std::fmt::Debug;

use gstreamer_sys::{
    gst_message_get_structure, gst_message_parse_state_changed, gst_message_unref, GstMessage,
    GstMessageType, GstState, GstStructure, GST_STATE_NULL,
};

use super::{state::State, structure::Structure};

#[derive(Debug)]
pub struct Message(*mut GstMessage);

impl Message {
    pub fn new(message: *mut GstMessage) -> Self {
        Self(message)
    }

    pub fn get(&self) -> *mut GstMessage {
        self.0
    }

    pub fn type_(&self) -> GstMessageType {
        unsafe { (*self.get()).type_ }
    }

    pub fn structure(&self) -> Structure {
        let structure_ptr = unsafe { gst_message_get_structure(self.get()) } as *mut GstStructure;

        Structure::new_from_message(structure_ptr)
    }

    pub fn state_changed(&self) -> State {
        let mut old_state: GstState = GST_STATE_NULL;
        let mut new_state: GstState = GST_STATE_NULL;
        let mut pending_state: GstState = GST_STATE_NULL;

        unsafe {
            gst_message_parse_state_changed(
                self.get(),
                &mut old_state,
                &mut new_state,
                &mut pending_state,
            )
        };

        State::new(old_state, new_state, pending_state)
    }
}

impl Drop for Message {
    fn drop(&mut self) {
        unsafe { gst_message_unref(self.get()) };
    }
}
