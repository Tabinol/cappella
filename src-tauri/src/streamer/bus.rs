use std::{fmt::Debug, sync::Arc};

use parking_lot::{Mutex, MutexGuard};

use crate::local::{app_error::AppError, mutex_lock_timeout::MutexLockTimeout};

use super::sys;

pub trait Bus: Debug + Send + Sync {
    fn set(&self, bus: sys::bus::Bus) -> Result<(), AppError>;
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
    fn set(&self, bus: sys::bus::Bus) -> Result<(), AppError> {
        let mut bus_lock = self.0.try_lock_default_duration()?;

        if bus_lock.is_some() {
            return Err(AppError::new(
                "The GStreamer bus is already assigned but it shouldn't. Force reassign."
                    .to_owned(),
            ));
        }

        *bus_lock = Some(bus);

        Ok(())
    }

    fn get_lock(&self) -> Result<MutexGuard<Option<sys::bus::Bus>>, AppError> {
        self.0.try_lock_default_duration()
    }

    fn take(&self) -> Result<sys::bus::Bus, AppError> {
        let mut bus_lock = self.0.try_lock_default_duration()?;

        bus_lock.take().ok_or_else(|| {
            AppError::new("Cannot drop the Gstreamer bus because it is null.".to_owned())
        })
    }
}
