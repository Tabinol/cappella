use std::fmt::Debug;

use dyn_clone::DynClone;
use gstreamer_sys::{
    gst_message_parse_state_changed, gst_message_unref, gst_structure_get_name,
    gst_structure_get_string, GstMessage, GstState, GstStructure, GST_MESSAGE_APPLICATION,
    GST_MESSAGE_DURATION_CHANGED, GST_MESSAGE_EOS, GST_MESSAGE_ERROR, GST_MESSAGE_STATE_CHANGED,
    GST_STATE_NULL,
};

use crate::{
    pointer::{PointerConst, PointerMut},
    utils::{cstring_ptr_to_str, str_to_cstring},
};

#[derive(Clone, Debug)]
pub(crate) enum MsgType {
    Error,
    Eos,
    DurationChanged,
    StateChanged,
    Application,
    Unsupported,
}

pub(crate) trait LocalGstreamerMessage: Debug + DynClone + Send + Sync {
    fn msg_type(&self) -> MsgType;
    fn parse_state_changed(&self);
    fn name(&self) -> &str;
    fn read(&self, name: &str) -> &str;
}

#[derive(Clone, Debug)]
pub(crate) struct ImplLocalGstreamerMessage {
    gst_message: PointerMut<GstMessage>,
    gst_structure: PointerConst<GstStructure>,
}

dyn_clone::clone_trait_object!(LocalGstreamerMessage);

impl ImplLocalGstreamerMessage {
    pub(crate) fn new(
        gst_message: PointerMut<GstMessage>,
        gst_structure: PointerConst<GstStructure>,
    ) -> Box<dyn LocalGstreamerMessage> {
        Box::new(Self {
            gst_message,
            gst_structure,
        })
    }
}

impl LocalGstreamerMessage for ImplLocalGstreamerMessage {
    fn msg_type(&self) -> MsgType {
        match unsafe { self.gst_message.get().read() }.type_ {
            GST_MESSAGE_ERROR => MsgType::Error,
            GST_MESSAGE_EOS => MsgType::Eos,
            GST_MESSAGE_DURATION_CHANGED => MsgType::DurationChanged,
            GST_MESSAGE_STATE_CHANGED => MsgType::StateChanged,
            GST_MESSAGE_APPLICATION => MsgType::Application,
            _ => MsgType::Unsupported,
        }
    }

    fn parse_state_changed(&self) {
        let mut old_state: GstState = GST_STATE_NULL;
        let mut new_state: GstState = GST_STATE_NULL;
        let mut pending_state: GstState = GST_STATE_NULL;
        unsafe {
            gst_message_parse_state_changed(
                self.gst_message.get(),
                &mut old_state,
                &mut new_state,
                &mut pending_state,
            )
        };
    }

    fn name(&self) -> &str {
        let name_ptr = unsafe { gst_structure_get_name(self.gst_structure.get()) };
        unsafe { cstring_ptr_to_str(name_ptr) }
    }

    fn read(&self, name: &str) -> &str {
        unsafe {
            let name_cstring = str_to_cstring(name);
            let value_ptr =
                gst_structure_get_string(self.gst_structure.get(), name_cstring.as_ptr());
            cstring_ptr_to_str(value_ptr)
        }
    }
}

impl Drop for ImplLocalGstreamerMessage {
    fn drop(&mut self) {
        println!("Drop message!");
        unsafe { gst_message_unref(self.gst_message.get()) };
    }
}
