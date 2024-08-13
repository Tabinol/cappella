use std::{
    fmt::Debug,
    ptr::null_mut,
    sync::{
        mpsc::{SendError, Sender},
        Arc, MutexGuard,
    },
};

use gstreamer_sys::{gst_bus_post, gst_message_new_application, gst_structure_new_empty, GstBus};

use crate::utils::cstring_converter::str_to_cstring;

use super::{gstreamer_bus::GstreamerBus, gstreamer_message::GstreamerMessage};

pub(crate) const MESSAGE_NAME: &str = "APP_MSG";

pub(crate) trait GstreamerPipe: Debug {
    fn send(&self, gstreamer_message: GstreamerMessage);
}

pub(crate) fn new_box(
    gstreamer_bus: Arc<dyn GstreamerBus>,
    sender: Sender<GstreamerMessage>,
) -> Box<dyn GstreamerPipe> {
    Box::new(GstreamerPipe_::new(gstreamer_bus, sender))
}

#[derive(Debug)]
struct GstreamerPipe_ {
    gstreamer_bus: Arc<dyn GstreamerBus>,
    sender: Sender<GstreamerMessage>,
}

impl GstreamerPipe_ {
    fn new(gstreamer_bus: Arc<dyn GstreamerBus>, sender: Sender<GstreamerMessage>) -> Self {
        Self {
            gstreamer_bus,
            sender,
        }
    }

    fn send_to_thread(
        &self,
        gstreamer_message: GstreamerMessage,
    ) -> Result<(), SendError<GstreamerMessage>> {
        self.sender.send(gstreamer_message)
    }

    fn send_to_gst(&self, bus_lock: MutexGuard<*mut GstBus>) {
        let structure;
        let name_cstring = str_to_cstring(MESSAGE_NAME);

        unsafe {
            structure = gst_structure_new_empty(name_cstring.as_ptr());
            let gst_msg = gst_message_new_application(null_mut(), structure);
            gst_bus_post(*bus_lock, gst_msg);
        }
    }
}

impl GstreamerPipe for GstreamerPipe_ {
    fn send(&self, gstreamer_message: GstreamerMessage) {
        if let Some(err) = self.send_to_thread(gstreamer_message).err() {
            eprintln!("Error while sending the message to GStreamer thread: {err}");
            return;
        }

        let bus_lock = self.gstreamer_bus.get_lock();

        if !(*bus_lock).is_null() {
            self.send_to_gst(bus_lock);
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use std::sync::{Arc, Mutex};

//     use crate::{
//         gstreamer::{
//             gstreamer::Gstreamer, gstreamer_message::GstreamerMessage,
//             gstreamer_pipeline::GstreamerPipeline,
//         },
//         player::streamer_pipe::{
//             self, Message, StreamerPipe, StreamerPipe_, MESSAGE_FIELD_JSON, MESSAGE_NAME,
//         },
//     };

//     #[derive(Debug, Default)]
//     struct MockGstreamer {
//         name: Mutex<String>,
//         key: Mutex<String>,
//         value: Mutex<String>,
//     }

//     impl Gstreamer for MockGstreamer {
//         fn init(&self) {}

//         fn launch(&self, _uri: &str) -> Box<dyn GstreamerPipeline> {
//             panic!("Not implemented!")
//         }

//         fn bus_timed_pop_filtered(&self) -> Option<Box<dyn GstreamerMessage>> {
//             panic!("Not implemented!")
//         }

//         fn send_to_gst(&self, name: &str, key: &str, value: &str) {
//             *self.name.lock().unwrap() = name.to_string();
//             *self.key.lock().unwrap() = key.to_string();
//             *self.value.lock().unwrap() = value.to_string();
//         }
//     }

//     #[test]
//     fn test_send_pause() {
//         let gstreamer = Arc::<MockGstreamer>::default();
//         let streamer_pipe = StreamerPipe_::new(gstreamer.clone());

//         streamer_pipe.send(streamer_pipe::Message::Pause);

//         assert_eq!(*gstreamer.name.lock().unwrap(), MESSAGE_NAME);
//         assert_eq!(*gstreamer.key.lock().unwrap(), MESSAGE_FIELD_JSON);
//         let message: Message = serde_json::from_str(&*gstreamer.value.lock().unwrap()).unwrap();
//         assert!(matches!(message, Message::Pause));
//     }

//     #[test]
//     fn test_send_next() {
//         let gstreamer = Arc::<MockGstreamer>::default();
//         let streamer_pipe = StreamerPipe_::new(gstreamer.clone());

//         streamer_pipe.send(streamer_pipe::Message::Next("new_uri".to_string()));

//         assert_eq!(*gstreamer.name.lock().unwrap(), MESSAGE_NAME);
//         assert_eq!(*gstreamer.key.lock().unwrap(), MESSAGE_FIELD_JSON);
//         let message: Message = serde_json::from_str(&*gstreamer.value.lock().unwrap()).unwrap();
//         assert!(matches!(message, Message::Next(_)));
//         assert_eq!(
//             if let Message::Next(uri) = message {
//                 uri
//             } else {
//                 String::new()
//             },
//             "new_uri"
//         );
//     }

//     #[test]
//     fn test_send_stop() {
//         let gstreamer = Arc::<MockGstreamer>::default();
//         let streamer_pipe = StreamerPipe_::new(gstreamer.clone());

//         streamer_pipe.send(streamer_pipe::Message::Stop);

//         assert_eq!(*gstreamer.name.lock().unwrap(), MESSAGE_NAME);
//         assert_eq!(*gstreamer.key.lock().unwrap(), MESSAGE_FIELD_JSON);
//         let message: Message = serde_json::from_str(&*gstreamer.value.lock().unwrap()).unwrap();
//         assert!(matches!(message, Message::Stop));
//     }
// }
