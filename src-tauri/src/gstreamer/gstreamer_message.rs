use std::fmt::Debug;

use gstreamer_sys::{
    gst_message_parse_state_changed, gst_message_unref, gst_structure_get_name,
    gst_structure_get_string, GstMessage, GstStructure, GST_MESSAGE_APPLICATION,
    GST_MESSAGE_DURATION_CHANGED, GST_MESSAGE_EOS, GST_MESSAGE_ERROR, GST_MESSAGE_STATE_CHANGED,
};

use crate::utils::cstring_converter::{cstring_ptr_to_str, str_to_cstring};

use super::gstreamer_pipeline::{GstState, GST_STATE_NULL};

#[derive(Clone, Debug, Default)]
pub(crate) enum MsgType {
    #[default]
    None,
    Error,
    Eos,
    DurationChanged,
    StateChanged,
    Application,
    Unsupported(u32),
}

pub(crate) trait GstreamerMessage: Debug {
    fn msg_type(&self) -> MsgType;
    fn parse_state_changed(&self);
    fn name(&self) -> &str;
    fn read(&self, name: &str) -> &str;
}

#[derive(Debug)]
pub(crate) struct ImplGstreamerMessage {
    gst_message: *mut GstMessage,
    gst_structure: *const GstStructure,
}

impl ImplGstreamerMessage {
    pub(crate) fn new(
        gst_message: *mut GstMessage,
        gst_structure: *const GstStructure,
    ) -> ImplGstreamerMessage {
        Self {
            gst_message,
            gst_structure,
        }
    }
}

impl GstreamerMessage for ImplGstreamerMessage {
    fn msg_type(&self) -> MsgType {
        let type_ = unsafe { (*self.gst_message).type_ };

        match type_ {
            GST_MESSAGE_ERROR => MsgType::Error,
            GST_MESSAGE_EOS => MsgType::Eos,
            GST_MESSAGE_DURATION_CHANGED => MsgType::DurationChanged,
            GST_MESSAGE_STATE_CHANGED => MsgType::StateChanged,
            GST_MESSAGE_APPLICATION => MsgType::Application,
            gst_message_type => MsgType::Unsupported(gst_message_type),
        }
    }

    fn parse_state_changed(&self) {
        let mut old_state: GstState = GST_STATE_NULL;
        let mut new_state: GstState = GST_STATE_NULL;
        let mut pending_state: GstState = GST_STATE_NULL;
        unsafe {
            gst_message_parse_state_changed(
                self.gst_message,
                &mut old_state,
                &mut new_state,
                &mut pending_state,
            )
        };
    }

    fn name(&self) -> &str {
        let name_ptr = unsafe { gst_structure_get_name(self.gst_structure) };

        unsafe { cstring_ptr_to_str(name_ptr) }
    }

    fn read(&self, name: &str) -> &str {
        unsafe {
            let name_cstring = str_to_cstring(name);
            let value_ptr = gst_structure_get_string(self.gst_structure, name_cstring.as_ptr());
            cstring_ptr_to_str(value_ptr)
        }
    }
}

impl Drop for ImplGstreamerMessage {
    fn drop(&mut self) {
        unsafe { gst_message_unref(self.gst_message as *mut GstMessage) };
    }
}

#[cfg(test)]
mod tests {
    use std::ptr::{null, null_mut};

    use gstreamer::glib::gobject_ffi::G_TYPE_STRING;
    use gstreamer_sys::{
        gst_message_new_application, gst_structure_new, gst_structure_new_empty, GstMessage,
    };

    use crate::{
        gstreamer::{
            gstreamer_message::{GstreamerMessage, ImplGstreamerMessage, MsgType},
            gstreamer_pipeline::{GstState, GST_STATE_NULL},
            tests_common::{self, LOCK},
        },
        utils::cstring_converter::str_to_cstring,
    };

    static mut MESSAGE: *mut GstMessage = null_mut();
    static mut OLD_STATE: GstState = 0;
    static mut NEW_STATE: GstState = 0;
    static mut PENDING: GstState = 0;
    static mut MESSAGE_UNREF_CALL_NB: u32 = 0;

    #[no_mangle]
    extern "C" fn gst_message_parse_state_changed(
        message: *mut GstMessage,
        old_state: *mut GstState,
        new_state: *mut GstState,
        pending: *mut GstState,
    ) {
        unsafe {
            MESSAGE = message;
            OLD_STATE = *old_state;
            NEW_STATE = *new_state;
            PENDING = *pending;
        }
    }

    fn before_each() {
        tests_common::before_each();

        unsafe {
            MESSAGE = null_mut();
            OLD_STATE = GST_STATE_NULL;
            NEW_STATE = GST_STATE_NULL;
            PENDING = GST_STATE_NULL;
            MESSAGE_UNREF_CALL_NB = 0;
        }
    }

    #[no_mangle]
    extern "C" fn gst_message_unref(_msg: *mut GstMessage) {
        unsafe { MESSAGE_UNREF_CALL_NB += 1 };
    }

    #[test]
    fn test_msg_type_application() {
        before_each();

        let _lock = LOCK.lock().unwrap();
        let structure_name = str_to_cstring("structure_name");
        let structure = unsafe { gst_structure_new_empty(structure_name.as_ptr()) };
        let msg = unsafe { gst_message_new_application(null_mut(), structure) };
        let gstreamer_message = ImplGstreamerMessage::new(msg, structure);

        let msg_type = gstreamer_message.msg_type();

        assert!(matches!(msg_type, MsgType::Application));
    }

    #[test]
    fn test_parse_state_changed() {
        before_each();

        let _lock = LOCK.lock().unwrap();
        let structure_name = str_to_cstring("structure_name");
        let structure = unsafe { gst_structure_new_empty(structure_name.as_ptr()) };
        let msg = unsafe { gst_message_new_application(null_mut(), structure) };
        let gstreamer_message = ImplGstreamerMessage::new(msg, structure);

        gstreamer_message.parse_state_changed();

        unsafe {
            assert_eq!(MESSAGE, msg);
            assert_eq!(OLD_STATE, GST_STATE_NULL);
            assert_eq!(NEW_STATE, GST_STATE_NULL);
            assert_eq!(PENDING, GST_STATE_NULL);
        }
    }

    #[test]
    fn test_name() {
        before_each();

        let _lock = LOCK.lock().unwrap();
        let structure_name = str_to_cstring("structure_name");
        let structure = unsafe { gst_structure_new_empty(structure_name.as_ptr()) };
        let msg = unsafe { gst_message_new_application(null_mut(), structure) };
        let gstreamer_message = ImplGstreamerMessage::new(msg, structure);

        let name = gstreamer_message.name();

        assert_eq!(name, "structure_name");
    }

    #[test]
    fn test_read() {
        before_each();

        let _lock = LOCK.lock().unwrap();
        let structure_name = str_to_cstring("structure_name");
        let key = str_to_cstring("key");
        let value = str_to_cstring("value");
        let structure = unsafe {
            gst_structure_new(
                structure_name.as_ptr(),
                key.as_ptr(),
                G_TYPE_STRING,
                value.as_ptr(),
                null() as *const i8,
            )
        };
        let msg = unsafe { gst_message_new_application(null_mut(), structure) };
        let gstreamer_message = ImplGstreamerMessage::new(msg, structure);

        let value = gstreamer_message.read("key");

        assert_eq!(value, "value");
    }

    #[test]
    fn test_drop() {
        before_each();

        let _lock = LOCK.lock().unwrap();
        let structure_name = str_to_cstring("structure_name");
        let structure = unsafe { gst_structure_new_empty(structure_name.as_ptr()) };
        let msg = unsafe { gst_message_new_application(null_mut(), structure) };

        {
            let _gstreamer_message = ImplGstreamerMessage::new(msg, structure);
        }

        assert_eq!(unsafe { MESSAGE_UNREF_CALL_NB }, 1);
    }
}
