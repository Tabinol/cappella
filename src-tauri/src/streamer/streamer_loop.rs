use std::{
    fmt::Debug,
    sync::{mpsc, Arc},
    time::Duration,
};

use gstreamer_sys::{
    GstState, GST_CLOCK_TIME_NONE, GST_FORMAT_TIME, GST_MESSAGE_APPLICATION,
    GST_MESSAGE_DURATION_CHANGED, GST_MESSAGE_EOS, GST_MESSAGE_ERROR, GST_MESSAGE_STATE_CHANGED,
    GST_STATE_PAUSED, GST_STATE_PLAYING,
};

use crate::{
    frontend::{self},
    local::app_error::AppError,
};

use super::{
    bus::Bus,
    message::{AppHandleAddr, Message, Uri},
    pipe::MESSAGE_NAME,
    sys::{self, element::Element},
};

const UPDATE_POSITION_DURATION: Duration = Duration::from_millis(100);

pub trait StreamerLoop: Debug {
    fn start_loop(&self, app_handle_addr: usize, uri: &str);
}

pub fn new_impl(bus: Arc<dyn Bus>, sender: mpsc::Sender<()>) -> impl StreamerLoop {
    StreamerLoop_ { bus, sender }
}

#[derive(Debug)]
struct StreamerLoop_ {
    bus: Arc<dyn Bus>,
    sender: mpsc::Sender<()>,
}

#[derive(Debug)]
struct Data {
    frontend_pipe: Box<dyn frontend::pipe::Pipe>,
    element: Element,
    is_playing: bool,
    duration: i64,
}

impl StreamerLoop for StreamerLoop_ {
    fn start_loop(&self, app_handle_addr: usize, uri: &str) {
        let mut play = Some((app_handle_addr, uri.to_owned()));

        while let Some((app_handle_addr, uri)) = play {
            let result = self.gst_loop(app_handle_addr, &uri);

            play = match result {
                Ok(play_next) => play_next,
                Err(err) => {
                    eprintln!("Error from the GStreamer loop: {err}");
                    None
                }
            }
        }
    }
}

impl StreamerLoop_ {
    fn gst_loop(
        &self,
        app_handle_addr: usize,
        uri: &str,
    ) -> Result<Option<(AppHandleAddr, Uri)>, AppError> {
        let frontend_pipe = frontend::pipe::new_box(app_handle_addr);
        let element = Element::new(uri).unwrap_or_else(|err| panic!("{err}"));
        self.bus.set(element.get_bus()?)?;

        let mut data = Data {
            frontend_pipe,
            element,
            is_playing: true,
            duration: GST_CLOCK_TIME_NONE as i64,
        };

        let mut message = Message::None;

        while !matches!(message, Message::Play(_, _) | Message::Stop) {
            let bus_lock = self.bus.get_lock()?;

            if let Some(bus) = bus_lock.as_ref() {
                let msg_opt = bus.timed_pop_filtered(
                    UPDATE_POSITION_DURATION,
                    GST_MESSAGE_STATE_CHANGED
                        | GST_MESSAGE_ERROR
                        | GST_MESSAGE_EOS
                        | GST_MESSAGE_DURATION_CHANGED
                        | GST_MESSAGE_APPLICATION,
                )?;

                if let Some(msg) = msg_opt {
                    message = self.handle_message(&mut data, &msg)?;
                } else {
                    if data.is_playing {
                        self.update_position(&mut data);
                    }
                }
            } else {
                panic!("The gst bus is null.");
            }
        }

        let _bus = self.bus.take();
        self.sender.send(()).unwrap();

        if let Message::Play(app_handle_addr, uri) = message {
            return Ok(Some((app_handle_addr, uri)));
        }

        Ok(None)
    }

    fn handle_message(
        &self,
        data: &mut Data,
        msg: &sys::message::Message,
    ) -> Result<Message, AppError> {
        match msg.type_() {
            GST_MESSAGE_ERROR => {
                return Err(AppError::new("Error received from element.".to_owned()));
            }
            GST_MESSAGE_EOS => {
                // TODO remove?
                println!("End-Of-Stream reached.");
                Ok(Message::Stop)
            }
            GST_MESSAGE_DURATION_CHANGED => {
                data.duration = GST_CLOCK_TIME_NONE as i64;
                Ok(Message::None)
            }
            GST_MESSAGE_STATE_CHANGED => {
                msg.state_changed();
                Ok(Message::None)
            }
            GST_MESSAGE_APPLICATION => self.handle_application_message(data, msg),
            gst_message_type => {
                eprintln!("Unexpected message number received: {gst_message_type}");
                Ok(Message::None)
            }
        }
    }

    fn handle_application_message(
        &self,
        data: &mut Data,
        msg: &sys::message::Message,
    ) -> Result<Message, AppError> {
        let structure = msg.structure()?;
        let name = structure.name();

        if name.ne(MESSAGE_NAME) {
            return Err(AppError::new(format!(
                "Streamer pipe message name error: {name}"
            )));
        }

        let message = Message::from_structure(structure)?;

        match message {
            Message::None => Err(AppError::new(
                "Message with 'None' is an error due to a possible receive timeout.".to_owned(),
            )),
            Message::Pause => {
                let element = &data.element;
                if data.is_playing {
                    self.set_state(element, GST_STATE_PAUSED)?;
                    data.is_playing = false;
                } else {
                    self.set_state(element, GST_STATE_PLAYING)?;
                    data.is_playing = true;
                }
                Ok(Message::None)
            }
            default => Ok(default),
        }
    }

    fn update_position(&self, data: &mut Data) {
        let current = data.element.query_position(GST_FORMAT_TIME).unwrap_or({
            eprintln!("Could not query current position.");
            -1
        });

        data.duration = data.element.query_duration(GST_FORMAT_TIME).unwrap_or({
            eprintln!("Could not query current duration.");
            data.duration
        });

        // TODO Temp
        println!("Position {} / {}", current, data.duration);
    }

    pub fn set_state(&self, element: &Element, state: GstState) -> Result<(), AppError> {
        element.set_state(state)
    }
}
