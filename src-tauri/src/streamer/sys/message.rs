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

    pub fn structure(&self) -> Result<Structure, AppError> {
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

#[cfg(test)]
mod tests {
    use std::ptr::null_mut;

    use gstreamer_sys::{GST_STATE_NULL, GST_STATE_PAUSED, GST_STATE_PLAYING};

    use crate::streamer::sys::{
        common_tests::{RcRefCellTestStructure, TestObjectType, TestStructure, UNASSIGNED},
        message::Message,
    };

    #[test]
    fn test_new_ok() {
        let test_structure = TestStructure::new_arc_mutex(UNASSIGNED);
        let message_res = Message::new(test_structure.faked_gst_message());

        assert!(message_res.is_ok());
    }

    #[test]
    fn test_new_err() {
        let message_res = Message::new(null_mut());

        assert!(message_res.is_err());
    }

    #[test]
    fn test_structure_ok() {
        let test_structure = TestStructure::new_arc_mutex_assigned();
        let message = Message::new(test_structure.faked_gst_message()).unwrap();

        let structure_res = message.structure();

        assert!(structure_res.is_ok());
    }

    #[test]
    fn test_structure_err() {
        let test_structure = TestStructure::new_arc_mutex(UNASSIGNED);
        let message = Message::new(test_structure.faked_gst_message()).unwrap();

        let structure_res = message.structure();

        assert!(structure_res.is_err());
    }

    #[test]
    fn test_state_changer() {
        let test_structure = TestStructure::new_arc_mutex(UNASSIGNED);
        let message = Message::new(test_structure.faked_gst_message()).unwrap();

        let state = message.state_changed();

        assert_eq!(state.old_state(), GST_STATE_PAUSED);
        assert_eq!(state.new_state(), GST_STATE_PLAYING);
        assert_eq!(state.pending_state(), GST_STATE_NULL);
    }

    #[test]
    fn test_drop() {
        let test_structure = TestStructure::new_arc_mutex_assigned();
        {
            let _message = Message::new(test_structure.faked_gst_message());
        }

        assert!(
            test_structure.is_unref(TestObjectType::GstMessage),
            "The message is not unref."
        )
    }
}
