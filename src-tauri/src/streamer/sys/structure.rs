use std::{
    any::Any,
    collections::VecDeque,
    ffi::{c_char, CString},
    ptr::{null, null_mut},
};

use glib_sys::{GType, GFALSE};
use gstreamer_sys::{
    gst_message_new_application, gst_structure_get_int64, gst_structure_get_name,
    gst_structure_get_string, gst_structure_get_uint64, gst_structure_new, GstStructure,
};

use crate::local::app_error::AppError;

use super::{message::Message, structure_field};

pub type CField = (CString, GType, Box<dyn Any>);

pub type MemoryStore = (CString, Vec<CField>);

#[derive(Debug)]
pub struct Structure {
    ptr: *mut GstStructure,
    name: String,
    _memory_store: Option<MemoryStore>,
}

impl Structure {
    pub fn new(name: &str, fields: Vec<Box<dyn structure_field::Field>>) -> Result<Self, AppError> {
        let c_name = CString::new(name).unwrap();
        let mut c_fields = Vec::<CField>::new();
        let mut c_values = VecDeque::<*const c_char>::new();

        for field in fields {
            let field_name = CString::new(field.field_name()).unwrap();
            let g_type = field.g_type();
            let value = field.c_value();
            c_values.push_back(field_name.as_ptr());
            c_values.push_back(g_type as *const c_char);
            c_values.push_back(std::ptr::addr_of!(*value) as *const c_char);
            c_fields.push((field_name, g_type, value));
        }

        c_values.push_back(null());
        let first_field = c_values.pop_front().unwrap();
        let ptr = unsafe { gst_structure_new(c_name.as_ptr(), first_field, c_values) };

        if ptr.is_null() {
            return Err(AppError::new(
                "GStreamer returned a null structure for the message.".to_owned(),
            ));
        }

        Ok(Self {
            ptr,
            name: name.to_owned(),
            _memory_store: Some((c_name, c_fields)),
        })
    }

    pub fn new_from_message(ptr: *mut GstStructure) -> Self {
        if ptr.is_null() {
            panic!("The message structure is null.");
        }

        let name_ptr = unsafe { gst_structure_get_name(ptr) };
        let name = unsafe { CString::from_raw(name_ptr as *mut c_char) }
            .to_str()
            .unwrap()
            .to_owned();

        Self {
            ptr,
            name,
            _memory_store: None,
        }
    }

    pub fn get(&self) -> *mut GstStructure {
        self.ptr
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn message_new_application(&self) -> Result<Message, AppError> {
        let message_ptr = unsafe { gst_message_new_application(null_mut(), self.get()) };

        Message::new(message_ptr)
    }

    pub fn get_string(&self, field_name: &str) -> Result<String, AppError> {
        let field_name_cstring = self.field_name_to_cstring(field_name)?;

        let value_ptr =
            unsafe { gst_structure_get_string(self.get(), field_name_cstring.as_ptr()) };

        if value_ptr.is_null() {
            return Err(AppError::new(format!(
                "The value is `null` for the String field `{field_name}`."
            )));
        }

        Ok(unsafe { CString::from_raw(value_ptr as *mut c_char) }
            .to_str()
            .unwrap()
            .to_owned())
    }

    #[allow(dead_code)]
    pub fn get_i64(&self, field_name: &str) -> Result<i64, AppError> {
        let field_name_cstring = self.field_name_to_cstring(field_name)?;

        let mut value: i64 = 0;
        let result =
            unsafe { gst_structure_get_int64(self.get(), field_name_cstring.as_ptr(), &mut value) };

        if result == GFALSE {
            return Err(AppError::new(format!(
                "The value is `null` for the i64 field `{field_name}`."
            )));
        }

        Ok(value)
    }

    pub fn get_u64(&self, field_name: &str) -> Result<u64, AppError> {
        let field_name_cstring = self.field_name_to_cstring(field_name)?;

        let mut value: u64 = 0;
        let result = unsafe {
            gst_structure_get_uint64(self.get(), field_name_cstring.as_ptr(), &mut value)
        };

        if result == GFALSE {
            return Err(AppError::new(format!(
                "The value is `null` for the u64 field `{field_name}`."
            )));
        }

        Ok(value)
    }

    fn field_name_to_cstring(&self, field_name: &str) -> Result<CString, AppError> {
        Ok(CString::new(field_name)?)
    }
}
