use std::{
    fmt::Debug,
    ptr::null_mut,
    sync::{Arc, Mutex, MutexGuard},
};

use gstreamer_sys::GstBus;

pub(crate) trait GstreamerBus: Debug + Send + Sync {
    fn set(&self, bus: *mut GstBus);
    fn get_lock(&self) -> MutexGuard<*mut GstBus>;
    fn take(&self) -> *mut GstBus;
}

pub(crate) fn new_arc() -> Arc<dyn GstreamerBus> {
    Arc::<GstreamerBus_>::default()
}

#[derive(Debug)]
struct GstreamerBus_(Mutex<*mut GstBus>);

unsafe impl Send for GstreamerBus_ {}
unsafe impl Sync for GstreamerBus_ {}

impl Default for GstreamerBus_ {
    fn default() -> Self {
        Self(Mutex::new(null_mut()))
    }
}

impl GstreamerBus for GstreamerBus_ {
    fn set(&self, bus: *mut GstBus) {
        if bus.is_null() {
            panic!("GStreamer bus is null.");
        }

        let mut bus_lock = self
            .0
            .try_lock()
            .expect("The Gstreamer bus is locked or poisoned.");

        if !(*bus_lock).is_null() {
            panic!("The gst bus is already assigned.");
        }

        (*bus_lock) = bus;
    }

    fn get_lock(&self) -> MutexGuard<*mut GstBus> {
        let bus_lock = self.0.lock().unwrap();
        if !(*bus_lock).is_null() {
            return bus_lock;
        }

        panic!("The GStreamer bus is null.");
    }

    fn take(&self) -> *mut GstBus {
        let mut bus_lock = self
            .0
            .try_lock()
            .expect("Cannot drop the Gstreamer bus because it is locked or poisoned");

        if (*bus_lock).is_null() {
            panic!("Cannot drop the Gstreamer bus because it is locked, poisoned or null.");
        }

        let result = *bus_lock;
        *bus_lock = null_mut();

        result
    }
}
