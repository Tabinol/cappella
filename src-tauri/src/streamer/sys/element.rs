use std::{
    ffi::{c_char, CString},
    fmt::Debug,
    ptr::null_mut,
};

use glib_sys::{gboolean, GFALSE};
use gstreamer_sys::{
    gst_element_get_bus, gst_element_query_duration, gst_element_set_state, gst_init,
    gst_object_unref, gst_parse_launch, GstElement, GstFormat, GstObject, GstState,
    GstStateChangeReturn, GST_STATE_CHANGE_SUCCESS, GST_STATE_NULL,
};

use super::bus::Bus;

#[derive(Debug)]
pub struct Element(*mut GstElement);

impl Element {
    pub fn new(uri: &str) -> Result<Self, String> {
        let args = std::env::args()
            .map(|arg| CString::new(arg).unwrap())
            .collect::<Vec<CString>>();

        let mut c_args = args
            .iter()
            .map(|arg| arg.clone().into_raw())
            .collect::<Vec<*mut c_char>>();

        unsafe { gst_init(&mut (c_args.len() as i32), &mut c_args.as_mut_ptr()) };

        let pipeline_description = CString::new(format!("playbin uri=\"{uri}\""))
            .or_else(|_| Err(format!("Error on pipeline description conversion.")))?;

        let element_ptr = unsafe { gst_parse_launch(pipeline_description.as_ptr(), null_mut()) };

        if element_ptr.is_null() {
            return Err(format!("The pipeline is null."));
        }

        Ok(Self(element_ptr))
    }

    pub fn get(&self) -> *mut GstElement {
        self.0
    }

    pub fn set_state(&self, state: GstState) -> Result<(), GstStateChangeReturn> {
        let state_change_return = unsafe { gst_element_set_state(self.get(), state) };

        if state_change_return != GST_STATE_CHANGE_SUCCESS {
            return Err(state_change_return);
        }

        Ok(())
    }

    pub fn get_bus(&self) -> Bus {
        let bus = unsafe { gst_element_get_bus(self.get()) };
        if bus.is_null() {
            panic!("The GStreamer bus is null.");
        }

        Bus::new(bus)
    }

    pub fn query_duration(&self, format: GstFormat) -> Result<i64, String> {
        self.query(|duration| unsafe { gst_element_query_duration(self.get(), format, duration) })
    }

    pub fn query_position(&self, format: GstFormat) -> Result<i64, String> {
        self.query(|position| unsafe { gst_element_query_duration(self.get(), format, position) })
    }

    fn query<F>(&self, f: F) -> Result<i64, String>
    where
        F: FnOnce(*mut i64) -> gboolean,
    {
        let mut value: i64 = -1;
        let result = f(&mut value);

        if result == GFALSE || value == -1 {
            return Err("No result returned form the duration or position query.".to_owned());
        }

        Ok(value)
    }
}

impl Drop for Element {
    fn drop(&mut self) {
        self.set_state(GST_STATE_NULL).unwrap_or_else(|status| {
            eprintln!("GStreamer set state returns status `{status}`");
        });

        unsafe {
            gst_object_unref(self.get() as *mut GstObject);
        }
    }
}

#[cfg(test)]
mod test {
    use crate::streamer::sys::{
        common_tests::{RcRefCellTestStructure, TestStructure},
        element::Element,
    };

    #[test]
    fn test_new() {
        let test_structure = TestStructure::new_arc_mutex();
        let uri = format!("{}", test_structure.test_nb());

        let element = Element::new(&uri).unwrap();

        assert!(!element.0.is_null());
    }
}
