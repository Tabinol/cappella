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

use crate::frontend::{self};

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
            play = self.gst_loop(app_handle_addr, &uri);
        }
    }
}

impl StreamerLoop_ {
    fn gst_loop(&self, app_handle_addr: usize, uri: &str) -> Option<(AppHandleAddr, Uri)> {
        let frontend_pipe = frontend::pipe::new_box(app_handle_addr);
        let element = Element::new(uri).unwrap_or_else(|err| panic!("{err}"));
        self.bus.set(element.get_bus());

        let mut data = Data {
            frontend_pipe,
            element,
            is_playing: true,
            duration: GST_CLOCK_TIME_NONE as i64,
        };

        let mut message = Message::None;

        while !matches!(message, Message::Play(_, _) | Message::Stop) {
            let bus_lock = self.bus.get_lock();

            if let Some(bus) = bus_lock.as_ref() {
                let msg_opt = bus.timed_pop_filtered(
                    UPDATE_POSITION_DURATION,
                    GST_MESSAGE_STATE_CHANGED
                        | GST_MESSAGE_ERROR
                        | GST_MESSAGE_EOS
                        | GST_MESSAGE_DURATION_CHANGED
                        | GST_MESSAGE_APPLICATION,
                );

                if let Some(msg) = msg_opt {
                    message = self.handle_message(&mut data, &msg);
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
            return Some((app_handle_addr, uri));
        }

        None
    }

    fn handle_message(&self, data: &mut Data, msg: &sys::message::Message) -> Message {
        match msg.type_() {
            GST_MESSAGE_ERROR => {
                eprintln!("Error received from element.");
                Message::Stop
            }
            GST_MESSAGE_EOS => {
                // TODO remove?
                println!("End-Of-Stream reached.");
                Message::Stop
            }
            GST_MESSAGE_DURATION_CHANGED => {
                data.duration = GST_CLOCK_TIME_NONE as i64;
                Message::None
            }
            GST_MESSAGE_STATE_CHANGED => {
                msg.state_changed();
                Message::None
            }
            GST_MESSAGE_APPLICATION => {
                self.handle_application_message(data, msg)
                    .unwrap_or_else(|err| {
                        eprintln!("Error message on streamer message receiver: {err}");
                        Message::None
                    })
            }
            gst_message_type => {
                eprintln!("Unexpected message number received: {gst_message_type}");
                Message::None
            }
        }
    }

    fn handle_application_message(
        &self,
        data: &mut Data,
        msg: &sys::message::Message,
    ) -> Result<Message, String> {
        let structure = msg.structure();
        let name = structure.name();

        if name.ne(MESSAGE_NAME) {
            return Err(format!("Streamer pipe message name error: {name}"));
        }

        let message = Message::from_structure(structure)?;

        match message {
            Message::None => {
                Err("Message with 'None' is an error due to a possible receive timeout.".to_owned())
            }
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

    pub fn set_state(&self, element: &Element, state: GstState) -> Result<(), String> {
        if let Err(err_code) = element.set_state(state) {
            return Err(format!("Error code on GStreamer set state: {err_code}"));
        }

        Ok(())
    }
}
