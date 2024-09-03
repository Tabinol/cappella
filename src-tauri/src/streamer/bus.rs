use std::{
    fmt::Debug,
    sync::{Arc, Mutex, MutexGuard},
};

use crate::local::{
    app_error::AppError,
    mutex_lock_timeout::{MutexLockTimeout, LOCK_STANDARD_TIMEOUT_DURATION},
};

use super::sys;

pub trait Bus: Debug + Send + Sync {
    fn set(&self, bus: sys::bus::Bus);
    fn get_lock(&self) -> Result<MutexGuard<Option<sys::bus::Bus>>, AppError>;
    fn take(&self) -> Result<sys::bus::Bus, AppError>;
}

pub fn new_arc() -> Arc<dyn Bus> {
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
            .try_lock_timeout(LOCK_STANDARD_TIMEOUT_DURATION)
            .or_else(|bus_try| {
                eprintln!("Try lock error timeout on the GStreamer bus: {bus_try}");
                eprintln!("Trying a clear poison...");
                self.0.clear_poison();
                self.0.try_lock()
            })
            .expect("The GStreamer bus is impossible to unlock.");

        if bus_lock.is_some() {
            eprintln!("The GStreamer bus is already assigned but it shouldn't. Force reassign.");
        }

        *bus_lock = Some(bus);
    }

    fn get_lock(&self) -> Result<MutexGuard<Option<sys::bus::Bus>>, AppError> {
        self.0.try_lock_timeout(LOCK_STANDARD_TIMEOUT_DURATION)
    }

    fn take(&self) -> Result<sys::bus::Bus, AppError> {
        let mut bus_lock = self.0.try_lock_timeout(LOCK_STANDARD_TIMEOUT_DURATION)?;

        bus_lock.take().ok_or_else(|| {
            AppError::new("Cannot drop the Gstreamer bus because it is null.".to_owned())
        })
    }
}
