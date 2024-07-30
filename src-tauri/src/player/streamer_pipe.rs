use std::{fmt::Debug, sync::Arc};

use crate::gstreamer::gstreamer::Gstreamer;

pub(crate) const MESSAGE_NAME: &str = "APP_MSG";
pub(crate) const MESSAGE_FIELD_JSON: &str = "JSON";

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub(crate) enum Message {
    #[default]
    None,
    Pause,
    Next(String),
    Stop,
}

pub(crate) trait StreamerPipe: Debug + Send + Sync {
    fn send(&self, message: Message);
}
#[derive(Debug)]
pub(crate) struct ImplStreamerPipe {
    gstreamer: Arc<dyn Gstreamer>,
}

impl ImplStreamerPipe {
    pub(crate) fn new(gstreamer: Arc<dyn Gstreamer>) -> ImplStreamerPipe {
        Self { gstreamer }
    }
}

unsafe impl Send for ImplStreamerPipe {}
unsafe impl Sync for ImplStreamerPipe {}

impl StreamerPipe for ImplStreamerPipe {
    fn send(&self, message: Message) {
        let json = serde_json::to_string(&message).unwrap();
        self.gstreamer
            .send_to_gst(MESSAGE_NAME, MESSAGE_FIELD_JSON, json.as_str());
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use crate::{
        gstreamer::{
            gstreamer::Gstreamer, gstreamer_message::GstreamerMessage,
            gstreamer_pipeline::GstreamerPipeline,
        },
        player::streamer_pipe::{
            self, ImplStreamerPipe, Message, StreamerPipe, MESSAGE_FIELD_JSON, MESSAGE_NAME,
        },
    };

    #[derive(Debug, Default)]
    struct MockGstreamer {
        name: Arc<Mutex<String>>,
        key: Arc<Mutex<String>>,
        value: Arc<Mutex<String>>,
    }

    impl Gstreamer for MockGstreamer {
        fn init(&self) {}

        fn launch(&self, _uri: &str) -> Box<dyn GstreamerPipeline> {
            panic!("Not implemented!")
        }

        fn bus_timed_pop_filtered(&self) -> Option<Box<dyn GstreamerMessage>> {
            panic!("Not implemented!")
        }

        fn send_to_gst(&self, name: &str, key: &str, value: &str) {
            *self.name.lock().unwrap() = name.to_string();
            *self.key.lock().unwrap() = key.to_string();
            *self.value.lock().unwrap() = value.to_string();
        }
    }

    #[test]
    fn test_send_pause() {
        let gstreamer = Arc::new(MockGstreamer::default());
        let streamer_pipe = ImplStreamerPipe::new(gstreamer.clone());

        streamer_pipe.send(streamer_pipe::Message::Pause);

        assert_eq!(*gstreamer.name.lock().unwrap(), MESSAGE_NAME);
        assert_eq!(*gstreamer.key.lock().unwrap(), MESSAGE_FIELD_JSON);
        let message: Message = serde_json::from_str(&*gstreamer.value.lock().unwrap()).unwrap();
        assert!(matches!(message, Message::Pause));
    }

    #[test]
    fn test_send_next() {
        let gstreamer = Arc::new(MockGstreamer::default());
        let streamer_pipe = ImplStreamerPipe::new(gstreamer.clone());

        streamer_pipe.send(streamer_pipe::Message::Next("new_uri".to_string()));

        assert_eq!(*gstreamer.name.lock().unwrap(), MESSAGE_NAME);
        assert_eq!(*gstreamer.key.lock().unwrap(), MESSAGE_FIELD_JSON);
        let message: Message = serde_json::from_str(&*gstreamer.value.lock().unwrap()).unwrap();
        assert!(matches!(message, Message::Next(_)));
        assert_eq!(
            if let Message::Next(uri) = message {
                uri
            } else {
                String::new()
            },
            "new_uri"
        );
    }

    #[test]
    fn test_send_stop() {
        let gstreamer = Arc::new(MockGstreamer::default());
        let streamer_pipe = ImplStreamerPipe::new(gstreamer.clone());

        streamer_pipe.send(streamer_pipe::Message::Stop);

        assert_eq!(*gstreamer.name.lock().unwrap(), MESSAGE_NAME);
        assert_eq!(*gstreamer.key.lock().unwrap(), MESSAGE_FIELD_JSON);
        let message: Message = serde_json::from_str(&*gstreamer.value.lock().unwrap()).unwrap();
        assert!(matches!(message, Message::Stop));
    }
}
