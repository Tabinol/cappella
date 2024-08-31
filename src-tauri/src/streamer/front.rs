use std::{
    fmt::Debug,
    sync::{
        mpsc::{self},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use super::{
    bus::Bus,
    streamer_loop::{self, StreamerLoop},
};

const THREAD_NAME: &str = "streamer";
const STOP_TIMEOUT_DURATION: Duration = Duration::from_secs(5);

pub(crate) trait Front: Debug + Send + Sync {
    fn start(&self, app_handle_addr: usize, uri: &str);
    fn is_running(&self) -> bool;
    fn wait_until_end(&self);
}

pub(crate) fn new_box(bus: Arc<dyn Bus>) -> Box<dyn Front> {
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
    fn start(&self, app_handle_addr: usize, uri: &str) {
        let mut join_handle_lock = self.join_handle.lock().unwrap();

        if join_handle_lock.is_some() {
            eprintln!("GStreamer loop already started");
            return;
        }

        let bus = self.bus.clone();
        let uri_owned = uri.to_owned();
        let (sender, receiver) = mpsc::channel::<()>();
        *self.receiver.lock().unwrap() = Some(receiver);

        let join_handle = thread::Builder::new()
            .name(THREAD_NAME.to_string())
            .spawn(move || {
                streamer_loop::new_impl(bus, sender).start_loop(app_handle_addr, &uri_owned);
            })
            .expect("Unable to start the GStreamer loop.");

        *join_handle_lock = Some(join_handle);
    }

    fn is_running(&self) -> bool {
        let mut join_handle_lock = self.join_handle.lock().unwrap();

        if join_handle_lock.is_none() {
            return false;
        }

        if join_handle_lock.as_ref().unwrap().is_finished() {
            *join_handle_lock = None;
            return false;
        }

        true
    }

    fn wait_until_end(&self) {
        let mut join_handle_lock = self.join_handle.lock().unwrap();

        if join_handle_lock.is_some() {
            let receiver = self.receiver.lock().unwrap().take().unwrap();
            let join_handle = join_handle_lock.take().unwrap();
            let result = receiver.recv_timeout(STOP_TIMEOUT_DURATION).or_else(|_| {
                eprintln!(
                    "GStreamer wait stop timeout afert `{}` seconds.",
                    STOP_TIMEOUT_DURATION.as_secs()
                );
                Err(())
            });

            if result.is_ok() {
                join_handle_lock.take().unwrap();
                join_handle.join().unwrap();
            }
        }
    }
}
