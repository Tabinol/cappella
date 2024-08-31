use std::{
    any::Any,
    ffi::{c_char, CStr, CString},
    ptr::{null_mut, NonNull},
};

use glib_sys::{GType, GFALSE};
use gstreamer_sys::{
    gst_message_new_application, gst_structure_get_int64, gst_structure_get_name,
    gst_structure_get_string, gst_structure_get_uint64, GstStructure,
};

use super::message::Message;

pub(crate) type CField = (CString, GType, Box<dyn Any>);

pub(crate) type MemoryStore = (CString, Vec<CField>);

#[derive(Debug)]
pub(crate) struct Structure(NonNull<GstStructure>, Option<MemoryStore>);

impl Structure {
    pub(crate) fn new(ptr: NonNull<GstStructure>, memory_store: Option<MemoryStore>) -> Self {
        Self(ptr, memory_store)
    }

    pub(crate) fn get(&self) -> *mut GstStructure {
        self.0.as_ptr()
    }

    pub(crate) fn name(&self) -> &str {
        unsafe {
            let name_ptr = gst_structure_get_name(self.get());
            CStr::from_ptr(name_ptr).to_str().unwrap()
        }
    }

    pub(crate) fn message_new_application(&self) -> Result<Message, String> {
        let message_ptr = unsafe { gst_message_new_application(null_mut(), self.get()) };

        if let Some(message) = NonNull::new(message_ptr) {
            return Ok(Message::new(message));
        }

        Err(format!("Error creating the message"))
    }

    pub(crate) fn get_string(&self, field_name: &str) -> Result<String, String> {
        let field_name_cstring = self.field_name_to_cstring(field_name)?;

        let value_ptr =
            unsafe { gst_structure_get_string(self.get(), field_name_cstring.as_ptr()) };

        if value_ptr.is_null() {
            return Err(format!(
                "The value is `null` for the String field `{field_name}`."
            ));
        }

        Ok(unsafe { CString::from_raw(value_ptr as *mut c_char) }
            .to_str()
            .unwrap()
            .to_owned())
    }

    #[allow(dead_code)]
    pub(crate) fn get_i64(&self, field_name: &str) -> Result<i64, String> {
        let field_name_cstring = self.field_name_to_cstring(field_name)?;

        let mut value: i64 = 0;
        let result =
            unsafe { gst_structure_get_int64(self.get(), field_name_cstring.as_ptr(), &mut value) };

        if result == GFALSE {
            return Err(format!(
                "The value is `null` for the i64 field `{field_name}`."
            ));
        }

        Ok(value)
    }

    pub(crate) fn get_u64(&self, field_name: &str) -> Result<u64, String> {
        let field_name_cstring = self.field_name_to_cstring(field_name)?;

        let mut value: u64 = 0;
        let result = unsafe {
            gst_structure_get_uint64(self.get(), field_name_cstring.as_ptr(), &mut value)
        };

        if result == GFALSE {
            return Err(format!(
                "The value is `null` for the u64 field `{field_name}`."
            ));
        }

        Ok(value)
    }

    fn field_name_to_cstring(&self, field_name: &str) -> Result<CString, String> {
        let field_name_res = CString::new(field_name);

        if field_name_res.is_err() {
            return Err(format!("Unable to get the field name for GStreamer."));
        }

        Ok(field_name_res.unwrap())
    }
}
