use std::{
    fmt::Debug,
    sync::{mpsc::Receiver, Arc, RwLock},
};

use crate::{
    frontend::frontend_pipe::FrontendPipe,
    gstreamer::{
        gstreamer::{Gstreamer, GST_CLOCK_TIME_NONE},
        gstreamer_message::{GstreamerMessage, MsgType},
        gstreamer_pipeline::{GstreamerPipeline, GST_STATE_PAUSED, GST_STATE_PLAYING},
    },
};

use super::{
    streamer,
    streamer_pipe::{Message, MESSAGE_FIELD_JSON, MESSAGE_NAME},
};

#[derive(Clone, Debug, Default)]
pub(crate) enum Status {
    #[default]
    None,
    Wait,
    Play(String),
    PlayNext(String),
    End,
}

pub(crate) trait StreamerLoop: Debug + Send + Sync {
    fn run(&self, receiver: Receiver<streamer::Message>);
    fn status(&self) -> Status;
}

#[derive(Clone, Debug)]
struct Data {
    uri: String,
    is_playing: bool,
    duration: i64,
}

#[derive(Debug)]
pub(crate) struct ImplStreamerLoop {
    frontend_pipe: Arc<dyn FrontendPipe>,
    gstreamer: Arc<dyn Gstreamer>,
    status: RwLock<Status>,
}

unsafe impl Send for ImplStreamerLoop {}
unsafe impl Sync for ImplStreamerLoop {}

impl ImplStreamerLoop {
    pub(crate) fn new(
        frontend_pipe: Arc<dyn FrontendPipe>,
        gstreamer: Arc<dyn Gstreamer>,
    ) -> ImplStreamerLoop {
        Self {
            frontend_pipe,
            gstreamer,
            status: RwLock::default(),
        }
    }

    fn gst_thread(&self, receiver: Receiver<streamer::Message>) {
        *self.status.write().unwrap() = Status::Wait;

        'end_gst_thread: loop {
            let current_status = (*self.status.read().unwrap()).clone();

            match current_status {
                Status::None => panic!("Incorrect status `None` for the streamer loop."),
                Status::Wait => {
                    let message = receiver.recv().unwrap();

                    match message {
                        streamer::Message::None => {
                            panic!("Incorrect message `None` for the streamer loop.")
                        }
                        streamer::Message::Play(uri) => {
                            *self.status.write().unwrap() = Status::Play(uri)
                        }
                        streamer::Message::End => *self.status.write().unwrap() = Status::End,
                    }
                }
                Status::Play(uri) => {
                    let mut data = Data {
                        uri: uri.to_owned(),
                        is_playing: true,
                        duration: GST_CLOCK_TIME_NONE,
                    };
                    self.gst(&mut data);
                }
                Status::PlayNext(uri) => {
                    *self.status.write().unwrap() = Status::Play(uri.to_owned())
                }
                Status::End => break 'end_gst_thread,
            }
        }
    }

    fn gst(&self, data: &mut Data) {
        self.gstreamer.init();

        let pipeline = self.gstreamer.launch(&data.uri);

        self.loop_gst(data, &*pipeline);
    }

    fn loop_gst(&self, data: &mut Data, pipeline: &dyn GstreamerPipeline) {
        'end_gst: loop {
            let msg_opt = self.gstreamer.bus_timed_pop_filtered();

            let mut status = self.status.write().unwrap();

            if let Some(msg) = msg_opt {
                let new_status_opt = self.handle_message(data, &*msg, pipeline);

                if let Some(new_status) = new_status_opt {
                    *status = new_status;
                }
            } else {
                if data.is_playing {
                    self.update_position(data, pipeline);
                }
            }

            if !matches!(*status, Status::Play(_)) {
                break 'end_gst;
            }
        }
    }

    fn handle_message(
        &self,
        data: &mut Data,
        msg: &dyn GstreamerMessage,
        pipeline: &dyn GstreamerPipeline,
    ) -> Option<Status> {
        match msg.msg_type() {
            MsgType::None => {
                panic!("Status wait is only for debugging.");
            }
            MsgType::Error => {
                eprintln!("Error received from element.");
                Some(Status::Wait)
            }
            MsgType::Eos => {
                // TODO remove?
                println!("End-Of-Stream reached.");
                Some(Status::Wait)
            }
            MsgType::DurationChanged => {
                data.duration = GST_CLOCK_TIME_NONE as i64;
                None
            }
            MsgType::StateChanged => {
                msg.parse_state_changed();
                None
            }
            MsgType::Application => self.handle_application_message(data, pipeline, msg),
            MsgType::Unsupported(gst_message_type) => {
                eprintln!("Unexpected message number received: {gst_message_type}");
                None
            }
        }
    }

    fn handle_application_message(
        &self,
        data: &mut Data,
        pipeline: &dyn GstreamerPipeline,
        msg: &dyn GstreamerMessage,
    ) -> Option<Status> {
        let name = msg.name();

        if name.ne(MESSAGE_NAME) {
            eprintln!("Streamer pipe message name error: {name}");
            return None;
        }

        let json = msg.read(MESSAGE_FIELD_JSON);
        let message = serde_json::from_str(json)
            .expect(format!("Unreadable streamer message: {json}").as_str());

        match message {
            Message::None => {
                eprintln!("Message with 'None' is an error.");
                None
            }
            Message::Pause => {
                if data.is_playing {
                    pipeline.set_state(GST_STATE_PAUSED);
                    data.is_playing = false;
                } else {
                    pipeline.set_state(GST_STATE_PLAYING);
                    data.is_playing = true;
                }
                None
            }
            Message::Stop => {
                // TODO remove?
                println!("Stop request.");
                Some(Status::Wait)
            }
            Message::Next(uri) => {
                // TODO remove?
                println!("Stop request (Async) and new uri '{uri}'.");
                Some(Status::PlayNext(uri))
            }
        }
    }

    fn update_position(&self, data: &mut Data, pipeline: &dyn GstreamerPipeline) {
        let current: i64 = if let Some(position) = pipeline.query_position() {
            position
        } else {
            eprintln!("Could not query current position.");
            -1
        };

        if data.duration == GST_CLOCK_TIME_NONE as i64 {
            if let Some(duration) = pipeline.query_duration() {
                data.duration = duration;
            } else {
                eprintln!("Could not query current duration.");
            }
        }

        // TODO Temp
        println!("Position {} / {}", current, data.duration);
    }
}

impl StreamerLoop for ImplStreamerLoop {
    fn run(&self, receiver: Receiver<streamer::Message>) {
        self.gst_thread(receiver);
    }

    fn status(&self) -> Status {
        (*self.status.read().unwrap()).clone()
    }
}
