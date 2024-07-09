use std::{
    fmt::Debug,
    ptr::null_mut,
    sync::{Arc, Mutex},
};

use dyn_clone::DynClone;
use gstreamer_sys::{
    gst_element_query_duration, gst_element_query_position, gst_element_set_state,
    gst_object_unref, GstBus, GstElement, GstObject, GST_FORMAT_TIME,
};

use crate::pointer::PointerMut;

pub(crate) type GstState = i32;
#[allow(unused)]
pub(crate) const GST_STATE_VOID_PENDING: GstState = gstreamer_sys::GST_STATE_VOID_PENDING;
pub(crate) const GST_STATE_NULL: GstState = gstreamer_sys::GST_STATE_NULL;
#[allow(unused)]
pub(crate) const GST_STATE_READY: GstState = gstreamer_sys::GST_STATE_READY;
pub(crate) const GST_STATE_PAUSED: GstState = gstreamer_sys::GST_STATE_PAUSED;
pub(crate) const GST_STATE_PLAYING: GstState = gstreamer_sys::GST_STATE_PLAYING;

pub(crate) trait LocalGstreamerPipeline: Debug + DynClone + Send + Sync {
    fn set_state(&self, gst_state: GstState);
    fn query_position(&self) -> Option<i64>;
    fn query_duration(&self) -> Option<i64>;
}

dyn_clone::clone_trait_object!(LocalGstreamerPipeline);

#[derive(Clone, Debug)]
pub(crate) struct ImplLocalGstreamerPipeline {
    gst_element: PointerMut<GstElement>,
    bus: Arc<Mutex<PointerMut<GstBus>>>,
}

impl ImplLocalGstreamerPipeline {
    pub(crate) fn new(
        gst_element: PointerMut<GstElement>,
        bus: Arc<Mutex<PointerMut<GstBus>>>,
    ) -> Box<dyn LocalGstreamerPipeline> {
        Box::new(Self { gst_element, bus })
    }
}

impl LocalGstreamerPipeline for ImplLocalGstreamerPipeline {
    fn set_state(&self, gst_state: GstState) {
        unsafe { gst_element_set_state(self.gst_element.get(), gst_state) };
    }

    fn query_position(&self) -> Option<i64> {
        let mut position: i64 = -1;
        let result = unsafe {
            gst_element_query_position(self.gst_element.get(), GST_FORMAT_TIME, &mut position)
        };

        if !result.is_positive() {
            return None;
        }

        Some(position)
    }

    fn query_duration(&self) -> Option<i64> {
        let mut duration: i64 = -1;
        let result = unsafe {
            gst_element_query_duration(self.gst_element.get(), GST_FORMAT_TIME, &mut duration)
        };

        if !result.is_positive() {
            return None;
        }

        Some(duration)
    }
}

impl Drop for ImplLocalGstreamerPipeline {
    fn drop(&mut self) {
        println!("Drop pipeline!");
        let mut bus = self.bus.lock().unwrap();

        unsafe {
            gst_object_unref(bus.get() as *mut GstObject);
            gst_element_set_state(self.gst_element.get(), GST_STATE_NULL);
            gst_object_unref(self.gst_element.get() as *mut GstObject);
        }

        bus.replace(null_mut());
    }
}
