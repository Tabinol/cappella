pub(crate) mod gstreamer_bus;
pub(crate) mod gstreamer_data;
pub(crate) mod gstreamer_message;
pub(crate) mod gstreamer_pipe;
pub(crate) mod gstreamer_thread;

#[cfg(test)]
pub(crate) mod gstreamer_tests_common {
    use std::{
        ptr,
        sync::{Mutex, MutexGuard},
    };

    use gstreamer::ffi::{
        GstBus, GstElement, GstObject, GstState, GstStateChangeReturn, GST_STATE_CHANGE_SUCCESS,
    };

    static LOCK: Mutex<()> = Mutex::new(());

    pub(crate) static mut ELEMENT_SET_STATE_CHANGE: GstState = 0;
    pub(crate) static mut ELEMENT_SET_STATE_RESULT: GstStateChangeReturn = 0;
    pub(crate) static mut OBJECT_UNREF_CALL_NB: u32 = 0;

    pub(crate) fn get_gst_bus_ptr() -> *mut GstBus {
        static mut ITEM: i32 = 0;
        unsafe { ptr::addr_of_mut!(ITEM) as *mut GstBus }
    }

    pub(crate) fn get_gst_element_ptr() -> *mut GstElement {
        static mut ITEM: i32 = 0;
        unsafe { ptr::addr_of_mut!(ITEM) as *mut GstElement }
    }

    pub(crate) fn lock() -> MutexGuard<'static, ()> {
        LOCK.lock()
            .unwrap_or_else(|poison_error| poison_error.into_inner())
    }

    pub(crate) fn before_each() {
        unsafe {
            ELEMENT_SET_STATE_CHANGE = -1;
            ELEMENT_SET_STATE_RESULT = GST_STATE_CHANGE_SUCCESS;
            OBJECT_UNREF_CALL_NB = 0;
        }
    }

    #[no_mangle]
    pub(crate) extern "C" fn gst_element_set_state(
        _element: *mut GstElement,
        state: GstState,
    ) -> GstStateChangeReturn {
        unsafe { ELEMENT_SET_STATE_CHANGE = state };
        unsafe { ELEMENT_SET_STATE_RESULT }
    }

    #[no_mangle]
    extern "C" fn gst_object_unref(_object: *mut GstObject) {
        unsafe { OBJECT_UNREF_CALL_NB += 1 };
    }
}
