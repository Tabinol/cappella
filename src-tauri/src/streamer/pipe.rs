use std::{fmt::Debug, sync::Arc};

use crate::local::app_error::AppError;

use super::{bus::Bus, message::Message};

pub const MESSAGE_NAME: &str = "APP_MSG";

pub trait Pipe: Debug {
    fn send(&self, message: Message) -> Result<(), AppError>;
}

pub fn new_box(bus: Arc<dyn Bus>) -> Box<dyn Pipe> {
    Box::new(Pipe_ { bus })
}

#[derive(Debug)]
struct Pipe_ {
    bus: Arc<dyn Bus>,
}

impl Pipe for Pipe_ {
    fn send(&self, message: Message) -> Result<(), AppError> {
        let bus_lock = self.bus.get_lock()?;

        if let Some(bus) = bus_lock.as_ref() {
            let structure = message.to_structure(MESSAGE_NAME)?;
            let message = structure.message_new_application()?;

            return bus.post(&message);
        }

        Err(AppError::new(format!(
            "The bus is null or the thread is not started. Message: {message:?}"
        )))
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
