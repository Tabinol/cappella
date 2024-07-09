use std::{
    fmt::Debug,
    sync::{mpsc::Receiver, Arc, Mutex},
};

use dyn_clone::DynClone;

use crate::{
    frontend_pipe::FrontendPipe,
    local_gstreamer::{LocalGstreamer, GST_CLOCK_TIME_NONE},
    local_gstreamer_message::{LocalGstreamerMessage, MsgType},
    local_gstreamer_pipeline::{LocalGstreamerPipeline, GST_STATE_PAUSED, GST_STATE_PLAYING},
    streamer::Status,
    streamer_pipe::{Message, MESSAGE_FIELD_JSON, MESSAGE_NAME},
};

pub(crate) trait StreamerLoop: Debug + DynClone + Send + Sync {
    fn run(&self, receiver: Receiver<Status>);
}

dyn_clone::clone_trait_object!(StreamerLoop);

#[derive(Clone, Debug)]
struct Data {
    uri: String,
    is_playing: bool,
    duration: i64,
}

#[derive(Clone, Debug)]
pub(crate) struct ImplStreamerLoop {
    frontend_pipe: Box<dyn FrontendPipe>,
    local_gstreamer: Box<dyn LocalGstreamer>,
    status: Arc<Mutex<Status>>,
    streamer_thread_lock: Arc<Mutex<()>>,
}

impl ImplStreamerLoop {
    pub(crate) fn new(
        frontend_pipe: Box<dyn FrontendPipe>,
        local_gstreamer: Box<dyn LocalGstreamer>,
        status: Arc<Mutex<Status>>,
        streamer_thread_lock: Arc<Mutex<()>>,
    ) -> Box<dyn StreamerLoop> {
        Box::new(Self {
            frontend_pipe,
            local_gstreamer,
            status,
            streamer_thread_lock,
        })
    }

    fn gst_thread(&self, receiver: Receiver<Status>) {
        let _streamer_thread_lock = self.streamer_thread_lock.lock().unwrap();
        *self.status.lock().unwrap() = Status::Wait;

        'end_gst_thread: loop {
            let status_clone = Arc::clone(&self.status);
            let mut current_status = status_clone.lock().unwrap().clone();

            if matches!(current_status, Status::Wait) {
                current_status = receiver.recv().unwrap();
                *status_clone.lock().unwrap() = current_status.clone();
            }

            if let Status::Play(uri) = current_status {
                let mut data = Data {
                    uri: uri.to_owned(),
                    is_playing: true,
                    duration: GST_CLOCK_TIME_NONE,
                };
                self.gst(&mut data);
            }

            let mut status_lock = status_clone.lock().unwrap();

            if let Status::PlayNext(uri) = &*status_lock {
                *status_lock = Status::Play(uri.to_owned());
            }

            if matches!(&*status_lock, Status::End) {
                break 'end_gst_thread;
            }
        }
    }

    fn gst(&self, data: &mut Data) {
        self.local_gstreamer.init();

        let pipeline = self.local_gstreamer.launch(&data.uri);

        self.loop_gst(data, &pipeline);
    }

    fn loop_gst(&self, data: &mut Data, pipeline: &Box<dyn LocalGstreamerPipeline>) {
        'end_gst: loop {
            let msg_opt = self.local_gstreamer.bus_timed_pop_filtered();

            let status_clone = Arc::clone(&self.status);
            let mut status_lock = status_clone.lock().unwrap();

            if let Some(msg) = msg_opt {
                let new_status_opt = self.handle_message(data, &msg, pipeline);

                if let Some(new_status) = new_status_opt {
                    *status_lock = new_status;
                }
            } else {
                if data.is_playing {
                    self.update_position(data, pipeline);
                }
            }

            if !matches!(&*status_lock, Status::Play(_)) {
                break 'end_gst;
            }
        }
    }

    fn handle_message(
        &self,
        data: &mut Data,
        msg: &Box<dyn LocalGstreamerMessage>,
        pipeline: &Box<dyn LocalGstreamerPipeline>,
    ) -> Option<Status> {
        match msg.msg_type() {
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
            _ => {
                eprintln!("Unexpected message received");
                None
            }
        }
    }

    fn handle_application_message(
        &self,
        data: &mut Data,
        pipeline: &Box<dyn LocalGstreamerPipeline>,
        msg: &Box<dyn LocalGstreamerMessage>,
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
            Message::End => {
                // TODO remove?
                println!("End request.");
                Some(Status::End)
            }
        }
    }

    fn update_position(&self, data: &mut Data, pipeline: &Box<dyn LocalGstreamerPipeline>) {
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
    fn run(&self, receiver: Receiver<Status>) {
        self.gst_thread(receiver);
    }
}
