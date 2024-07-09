use std::fmt::Debug;

use dyn_clone::DynClone;

use crate::local_gstreamer::LocalGstreamer;

pub(crate) const MESSAGE_NAME: &str = "APP_MSG";
pub(crate) const MESSAGE_FIELD_JSON: &str = "JSON";

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub(crate) enum Message {
    None,
    Pause,
    Next(String),
    Stop,
    End,
}

pub(crate) trait StreamerPipe: Debug + DynClone + Send + Sync {
    fn send(&self, message: Message);
}

dyn_clone::clone_trait_object!(StreamerPipe);

#[derive(Clone, Debug)]
pub(crate) struct ImplStreamerPipe {
    local_gstreamer: Box<dyn LocalGstreamer>,
}

impl ImplStreamerPipe {
    pub(crate) fn new(local_gstreamer: Box<dyn LocalGstreamer>) -> Box<dyn StreamerPipe> {
        Box::new(Self { local_gstreamer })
    }
}

impl StreamerPipe for ImplStreamerPipe {
    fn send(&self, message: Message) {
        let json = serde_json::to_string(&message).unwrap();
        self.local_gstreamer
            .send_to_gst(MESSAGE_NAME, MESSAGE_FIELD_JSON, json.as_str());
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use crate::{
        local_gstreamer::LocalGstreamer,
        local_gstreamer_message::LocalGstreamerMessage,
        local_gstreamer_pipeline::LocalGstreamerPipeline,
        streamer_pipe::{self, ImplStreamerPipe, Message, MESSAGE_FIELD_JSON, MESSAGE_NAME},
    };

    #[derive(Clone, Debug, Default)]
    struct MockLocalGstreamer {
        name: Arc<Mutex<String>>,
        key: Arc<Mutex<String>>,
        value: Arc<Mutex<String>>,
    }

    impl LocalGstreamer for MockLocalGstreamer {
        fn init(&self) {}

        fn launch(&self, _uri: &str) -> Box<dyn LocalGstreamerPipeline> {
            panic!("Not implemented!")
        }

        fn bus_timed_pop_filtered(&self) -> Option<Box<dyn LocalGstreamerMessage>> {
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
        let local_gstreamer = Box::new(MockLocalGstreamer::default());
        let streamer_pipe = ImplStreamerPipe::new(local_gstreamer.clone());

        streamer_pipe.send(streamer_pipe::Message::Pause);

        assert_eq!(*local_gstreamer.name.lock().unwrap(), MESSAGE_NAME);
        assert_eq!(*local_gstreamer.key.lock().unwrap(), MESSAGE_FIELD_JSON);
        let message: Message =
            serde_json::from_str(&*local_gstreamer.value.lock().unwrap()).unwrap();
        assert!(matches!(message, Message::Pause));
    }

    #[test]
    fn test_send_next() {
        let local_gstreamer = Box::new(MockLocalGstreamer::default());
        let streamer_pipe = ImplStreamerPipe::new(local_gstreamer.clone());

        streamer_pipe.send(streamer_pipe::Message::Next("new_uri".to_string()));

        assert_eq!(*local_gstreamer.name.lock().unwrap(), MESSAGE_NAME);
        assert_eq!(*local_gstreamer.key.lock().unwrap(), MESSAGE_FIELD_JSON);
        let message: Message =
            serde_json::from_str(&*local_gstreamer.value.lock().unwrap()).unwrap();
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
        let local_gstreamer = Box::new(MockLocalGstreamer::default());
        let streamer_pipe = ImplStreamerPipe::new(local_gstreamer.clone());

        streamer_pipe.send(streamer_pipe::Message::Stop);

        assert_eq!(*local_gstreamer.name.lock().unwrap(), MESSAGE_NAME);
        assert_eq!(*local_gstreamer.key.lock().unwrap(), MESSAGE_FIELD_JSON);
        let message: Message =
            serde_json::from_str(&*local_gstreamer.value.lock().unwrap()).unwrap();
        assert!(matches!(message, Message::Stop));
    }

    #[test]
    fn test_send_end() {
        let local_gstreamer = Box::new(MockLocalGstreamer::default());
        let streamer_pipe = ImplStreamerPipe::new(local_gstreamer.clone());

        streamer_pipe.send(streamer_pipe::Message::End);

        assert_eq!(*local_gstreamer.name.lock().unwrap(), MESSAGE_NAME);
        assert_eq!(*local_gstreamer.key.lock().unwrap(), MESSAGE_FIELD_JSON);
        let message: Message =
            serde_json::from_str(&*local_gstreamer.value.lock().unwrap()).unwrap();
        assert!(matches!(message, Message::End));
    }
}
