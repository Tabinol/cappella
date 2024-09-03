use std::fmt::{Debug, Display};

use gstreamer_sys::{
    gst_message_get_structure, gst_message_parse_state_changed, gst_message_unref, GstMessage,
    GstMessageType, GstState, GstStructure, GST_STATE_NULL,
};

use crate::local::app_error::AppError;

use super::{state::State, structure::Structure};

#[derive(Debug)]
pub struct Message(*mut GstMessage);

impl Message {
    pub fn new(message: *mut GstMessage) -> Result<Self, AppError> {
        if message.is_null() {
            return Err(AppError::new("The message pointer is null.".to_owned()));
        }
        Ok(Self(message))
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

impl Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "message_ptr: {:?}", self.0)
    }
}

impl Drop for Message {
    fn drop(&mut self) {
        unsafe { gst_message_unref(self.get()) };
    }
}
