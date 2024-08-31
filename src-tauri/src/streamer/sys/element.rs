use std::{fmt::Debug, ptr::NonNull};

use glib_sys::{gboolean, GFALSE};
use gstreamer_sys::{
    gst_element_get_bus, gst_element_query_duration, gst_element_set_state, gst_object_unref,
    GstElement, GstFormat, GstObject, GstState, GstStateChangeReturn, GST_STATE_CHANGE_SUCCESS,
    GST_STATE_NULL,
};

use super::bus::Bus;

#[derive(Debug)]
pub(crate) struct Element(NonNull<GstElement>);

impl Element {
    pub(crate) fn new(element: NonNull<GstElement>) -> Self {
        Self(element)
    }

    pub(crate) fn get(&self) -> *mut GstElement {
        self.0.as_ptr()
    }

    pub(crate) fn set_state(&self, state: GstState) -> Result<(), GstStateChangeReturn> {
        let state_change_return = unsafe { gst_element_set_state(self.get(), state) };

        if state_change_return != GST_STATE_CHANGE_SUCCESS {
            return Err(state_change_return);
        }

        Ok(())
    }

    pub(crate) fn get_bus(&self) -> Bus {
        let bus = unsafe { gst_element_get_bus(self.get()) };
        let bus_non_null = NonNull::new(bus).unwrap();

        Bus::new(bus_non_null)
    }

    pub(crate) fn query_duration(&self, format: GstFormat) -> Result<i64, String> {
        self.query(|duration| unsafe { gst_element_query_duration(self.get(), format, duration) })
    }

    pub(crate) fn query_position(&self, format: GstFormat) -> Result<i64, String> {
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
