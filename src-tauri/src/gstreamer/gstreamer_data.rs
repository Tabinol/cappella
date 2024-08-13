use std::{
    fmt::Debug,
    ptr::null_mut,
    sync::{Arc, Mutex},
};

use gstreamer_sys::GstElement;

use crate::frontend::frontend_pipe::FrontendPipe;

pub(crate) trait GstreamerData: Debug + Send + Sync {
    fn send_data(&self, frontend_pipe: Box<dyn FrontendPipe>, uri: String);
    fn consume(&self) -> Option<Transfer>;
}

pub(crate) fn new_arc() -> Arc<dyn GstreamerData> {
    Arc::<GstreamerData_>::default()
}

#[derive(Debug)]
pub(crate) struct Data {
    pub(in crate::gstreamer) frontend_pipe: Box<dyn FrontendPipe>,
    pub(in crate::gstreamer) pipeline: *mut GstElement,
    pub(in crate::gstreamer) is_playing: bool,
    pub(in crate::gstreamer) duration: i64,
}

#[derive(Debug)]
pub(crate) struct Transfer {
    pub(in crate::gstreamer) uri: String,
    pub(in crate::gstreamer) data: Data,
}

#[derive(Debug, Default)]
struct GstreamerData_(Mutex<Option<Transfer>>);

unsafe impl Send for GstreamerData_ {}
unsafe impl Sync for GstreamerData_ {}

impl GstreamerData for GstreamerData_ {
    fn send_data(&self, frontend_pipe: Box<dyn FrontendPipe>, uri: String) {
        let mut data_lock = self.0.lock().unwrap();

        if data_lock.is_some() {
            eprintln!("GStreamer data not consumed.");
        }

        let data = Data {
            frontend_pipe,
            pipeline: null_mut(),
            is_playing: bool::default(),
            duration: i64::default(),
        };

        let transfer = Transfer { uri, data };

        *data_lock = Some(transfer);
    }

    fn consume(&self) -> Option<Transfer> {
        let mut data_lock = self.0.lock().unwrap();

        data_lock.take()
    }
}
