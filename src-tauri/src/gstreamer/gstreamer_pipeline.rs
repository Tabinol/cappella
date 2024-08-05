use std::{
    fmt::Debug,
    sync::{Arc, Mutex},
};

use dyn_clone::DynClone;
use gstreamer_sys::{
    gst_element_query_duration, gst_element_query_position, gst_element_set_state,
    gst_object_unref, GstBus, GstElement, GstObject, GST_FORMAT_TIME,
};

pub(crate) type GstState = i32;
#[allow(unused)]
pub(crate) const GST_STATE_VOID_PENDING: GstState = gstreamer_sys::GST_STATE_VOID_PENDING;
pub(crate) const GST_STATE_NULL: GstState = gstreamer_sys::GST_STATE_NULL;
#[allow(unused)]
pub(crate) const GST_STATE_READY: GstState = gstreamer_sys::GST_STATE_READY;
pub(crate) const GST_STATE_PAUSED: GstState = gstreamer_sys::GST_STATE_PAUSED;
pub(crate) const GST_STATE_PLAYING: GstState = gstreamer_sys::GST_STATE_PLAYING;

pub(crate) trait GstreamerPipeline: Debug + DynClone {
    fn set_state(&self, gst_state: GstState);
    fn query_position(&self) -> Option<i64>;
    fn query_duration(&self) -> Option<i64>;
}

dyn_clone::clone_trait_object!(GstreamerPipeline);

pub(crate) fn new_boxed(
    gst_element: *mut GstElement,
    bus: Arc<Mutex<*mut GstBus>>,
) -> Box<dyn GstreamerPipeline> {
    Box::new(GstreamerPipeline_::new(gst_element, bus))
}

#[derive(Clone, Debug)]
struct GstreamerPipeline_ {
    gst_element: *mut GstElement,
    bus: Arc<Mutex<*mut GstBus>>,
}

impl GstreamerPipeline_ {
    fn new(gst_element: *mut GstElement, bus: Arc<Mutex<*mut GstBus>>) -> Self {
        Self { gst_element, bus }
    }
}

impl GstreamerPipeline for GstreamerPipeline_ {
    fn set_state(&self, gst_state: GstState) {
        unsafe { gst_element_set_state(self.gst_element, gst_state) };
    }

    fn query_position(&self) -> Option<i64> {
        let mut position: i64 = -1;
        let result =
            unsafe { gst_element_query_position(self.gst_element, GST_FORMAT_TIME, &mut position) };

        if !result.is_positive() {
            return None;
        }

        Some(position)
    }

    fn query_duration(&self) -> Option<i64> {
        let mut duration: i64 = -1;
        let result =
            unsafe { gst_element_query_duration(self.gst_element, GST_FORMAT_TIME, &mut duration) };

        if !result.is_positive() {
            return None;
        }

        Some(duration)
    }
}

impl Drop for GstreamerPipeline_ {
    fn drop(&mut self) {
        let bus = self.bus.lock().unwrap();

        unsafe {
            if !bus.is_null() {
                gst_object_unref(*bus as *mut GstObject);
            }

            gst_element_set_state(self.gst_element, GST_STATE_NULL);
            gst_object_unref(self.gst_element as *mut GstObject);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use gstreamer::glib::ffi::{gboolean, GTRUE};
    use gstreamer_sys::{GstElement, GstFormat};

    use crate::gstreamer::{
        gstreamer_pipeline::{GST_STATE_NULL, GST_STATE_PAUSED},
        tests_common::{
            self, get_gst_bus_ptr, get_gst_element_ptr, ELEMENT_SET_STATE_CHANGE,
            ELEMENT_SET_STATE_RESULT, OBJECT_UNREF_CALL_NB,
        },
    };

    #[no_mangle]
    extern "C" fn gst_element_query_position(
        _element: *mut GstElement,
        _format: GstFormat,
        cur: *mut i64,
    ) -> gboolean {
        unsafe { *cur = 10 };
        GTRUE
    }

    #[no_mangle]
    extern "C" fn gst_element_query_duration(
        _element: *mut GstElement,
        _format: GstFormat,
        duration: *mut i64,
    ) -> gboolean {
        unsafe { *duration = 11 };
        GTRUE
    }

    fn before_each() {
        tests_common::before_each();
    }

    #[test]
    fn test_set_state() {
        let _lock = tests_common::lock();
        before_each();

        let gst_element = get_gst_element_ptr();
        let bus = Arc::new(Mutex::new(get_gst_bus_ptr()));
        let gstreamer_pipeline = super::new_boxed(gst_element, bus);

        gstreamer_pipeline.set_state(GST_STATE_PAUSED);

        assert_eq!(unsafe { ELEMENT_SET_STATE_CHANGE }, GST_STATE_PAUSED)
    }

    #[test]
    fn test_query_position() {
        let _lock = tests_common::lock();
        before_each();

        let gst_element = get_gst_element_ptr();
        let bus = Arc::new(Mutex::new(get_gst_bus_ptr()));
        let gstreamer_pipeline = super::new_boxed(gst_element, bus);

        let position = gstreamer_pipeline.query_position();

        assert!(position.is_some());
        assert_eq!(position.unwrap(), 10);
    }

    #[test]
    fn test_query_duration() {
        let _lock = tests_common::lock();
        before_each();

        let gst_element = get_gst_element_ptr();
        let bus = Arc::new(Mutex::new(get_gst_bus_ptr()));
        let gstreamer_pipeline = super::new_boxed(gst_element, bus);

        let duration = gstreamer_pipeline.query_duration();

        assert!(duration.is_some());
        assert_eq!(duration.unwrap(), 11);
    }

    #[test]
    fn test_drop() {
        let _lock = tests_common::lock();
        before_each();

        {
            let gst_element = get_gst_element_ptr();
            let bus = Arc::new(Mutex::new(get_gst_bus_ptr()));
            let _gstreamer_pipeline = super::new_boxed(gst_element, bus);
        }

        assert_eq!(unsafe { OBJECT_UNREF_CALL_NB }, 2);
        assert_eq!(unsafe { ELEMENT_SET_STATE_RESULT }, GST_STATE_NULL);
    }
}
