use std::{any::Any, ffi::CString, fmt::Debug};

use glib_sys::GType;
use gobject_sys::{G_TYPE_INT64, G_TYPE_STRING, G_TYPE_UINT64};

pub(crate) trait Field: Debug {
    fn field_name(&self) -> &str;
    fn g_type(&self) -> GType;
    fn c_value(&self) -> Box<dyn Any>;
}

pub(crate) fn new_box_string(field_name: &str, value: &str) -> Box<dyn Field> {
    Box::new(FieldString {
        field_name: field_name.to_owned(),
        value: value.to_owned(),
    })
}

#[allow(dead_code)]
pub(crate) fn new_box_i64(field_name: &str, value: i64) -> Box<dyn Field> {
    Box::new(FieldI64 {
        field_name: field_name.to_owned(),
        value,
    })
}

pub(crate) fn new_box_u64(field_name: &str, value: u64) -> Box<dyn Field> {
    Box::new(FieldU64 {
        field_name: field_name.to_owned(),
        value,
    })
}

#[derive(Debug)]
pub(crate) struct FieldString {
    field_name: String,
    value: String,
}

impl Field for FieldString {
    fn field_name(&self) -> &str {
        self.field_name.as_str()
    }

    fn g_type(&self) -> GType {
        G_TYPE_STRING
    }

    fn c_value(&self) -> Box<dyn Any> {
        Box::new(CString::new(self.value.as_str()))
    }
}

#[derive(Debug)]
pub(crate) struct FieldI64 {
    field_name: String,
    value: i64,
}

impl Field for FieldI64 {
    fn field_name(&self) -> &str {
        self.field_name.as_str()
    }

    fn g_type(&self) -> GType {
        G_TYPE_INT64
    }

    fn c_value(&self) -> Box<dyn Any> {
        Box::new(self.value)
    }
}

#[derive(Debug)]
pub(crate) struct FieldU64 {
    field_name: String,
    value: u64,
}

impl Field for FieldU64 {
    fn field_name(&self) -> &str {
        self.field_name.as_str()
    }

    fn g_type(&self) -> GType {
        G_TYPE_UINT64
    }

    fn c_value(&self) -> Box<dyn Any> {
        Box::new(self.value)
    }
}
