use std::{
    fmt::Debug,
    sync::{
        mpsc::{self},
        Arc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use parking_lot::Mutex;

use crate::local::{app_error::AppError, mutex_lock_timeout::MutexLockTimeout};

use super::{
    bus::Bus,
    streamer_loop::{self, StreamerLoop},
};

const THREAD_NAME: &str = "streamer";
const STOP_TIMEOUT_DURATION: Duration = Duration::from_secs(5);

pub trait Front: Debug + Send + Sync {
    fn start(&self, app_handle_addr: usize, uri: &str) -> Result<(), AppError>;
    fn is_running(&self) -> Result<bool, AppError>;
    fn wait_until_end(&self) -> Result<(), AppError>;
}

pub fn new_box(bus: Arc<dyn Bus>) -> Box<dyn Front> {
    Box::new(Front_ {
        bus,
        receiver: Mutex::default(),
        join_handle: Mutex::default(),
    })
}

#[derive(Debug)]
struct Front_ {
    bus: Arc<dyn Bus>,
    receiver: Mutex<Option<mpsc::Receiver<()>>>,
    join_handle: Mutex<Option<JoinHandle<()>>>,
}

impl Front for Front_ {
    fn start(&self, app_handle_addr: usize, uri: &str) -> Result<(), AppError> {
        let mut join_handle_lock = self.join_handle.try_lock_default_duration()?;

        if join_handle_lock.is_some() {
            return Err(AppError::new("GStreamer loop already started".to_owned()));
        }

        let bus = self.bus.clone();
        let uri_owned = uri.to_owned();
        let (sender, receiver) = mpsc::channel::<()>();
        *self.receiver.try_lock_default_duration()? = Some(receiver);

        let join_handle =
            thread::Builder::new()
                .name(THREAD_NAME.to_string())
                .spawn(move || {
                    streamer_loop::new_impl(bus, sender).start_loop(app_handle_addr, &uri_owned);
                })?;

        *join_handle_lock = Some(join_handle);

        Ok(())
    }

    fn is_running(&self) -> Result<bool, AppError> {
        let mut join_handle_lock = self.join_handle.try_lock_default_duration()?;

        if let Some(join_handle) = &*join_handle_lock {
            if join_handle.is_finished() {
                *join_handle_lock = None;
                return Ok(false);
            }
            return Ok(true);
        }

        Ok(false)
    }

    fn wait_until_end(&self) -> Result<(), AppError> {
        let mut join_handle_lock = self.join_handle.try_lock_default_duration()?;

        if let Some(join_handle) = join_handle_lock.take() {
            let receiver = self
                .receiver
                .try_lock_default_duration()?
                .take()
                .ok_or_else(|| {
                    AppError::new("The receiver doesn't exist for the parent thread.".to_owned())
                })?;
            receiver.recv_timeout(STOP_TIMEOUT_DURATION)?;
            join_handle.join().or_else(|err| {
                Err(AppError::new(format!(
                    "Error on GStreamer thread join handle: {err:?}."
                )))
            })?;
        }

        Ok(())
    }
}
