use std::{
    fmt::Debug,
    sync::{Arc, Mutex, MutexGuard},
};

use super::sys;

pub(crate) trait Bus: Debug + Send + Sync {
    fn set(&self, bus: sys::bus::Bus);
    fn get_lock(&self) -> MutexGuard<Option<sys::bus::Bus>>;
    fn take(&self) -> sys::bus::Bus;
}

pub(crate) fn new_arc() -> Arc<dyn Bus> {
    Arc::<Bus_>::default()
}

#[derive(Debug, Default)]
struct Bus_(Mutex<Option<sys::bus::Bus>>);

unsafe impl Send for Bus_ {}
unsafe impl Sync for Bus_ {}

impl Bus for Bus_ {
    fn set(&self, bus: sys::bus::Bus) {
        let mut bus_lock = self
            .0
            .try_lock()
            .expect("The Gstreamer bus is locked or poisoned.");

        if bus_lock.is_some() {
            panic!("The gst bus is already assigned.");
        }

        *bus_lock = Some(bus);
    }

    fn get_lock(&self) -> MutexGuard<Option<sys::bus::Bus>> {
        self.0.lock().unwrap()
    }

    fn take(&self) -> sys::bus::Bus {
        let mut bus_lock = self
            .0
            .try_lock()
            .expect("Cannot drop the Gstreamer bus because it is locked or poisoned");

        bus_lock
            .take()
            .expect("Cannot drop the Gstreamer bus because it is null.")
    }
}
