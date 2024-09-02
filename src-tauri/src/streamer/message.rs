use super::sys::{structure::Structure, structure_field};

pub type Uri = String;
pub type AppHandleAddr = usize;

const TITLE_FIELD: &str = "TITLE";
const PTR_FIELD: &str = "PTR";
const URI_FIELD: &str = "URI";

const MESSAGE_TITLE_VALUE_NONE: &str = "None";
const MESSAGE_TITLE_VALUE_PLAY: &str = "Play";
const MESSAGE_TITLE_VALUE_PAUSE: &str = "Pause";
const MESSAGE_TITLE_VALUE_STOP: &str = "Stop";

#[derive(Debug, Default)]
pub enum Message {
    #[default]
    None,
    Play(AppHandleAddr, Uri),
    Pause,
    Stop,
}

impl Message {
    pub fn from_structure(structure: Structure) -> Result<Self, String> {
        let name = structure.get_string(TITLE_FIELD)?;

        match name.as_str() {
            MESSAGE_TITLE_VALUE_NONE => Ok(Message::None),
            MESSAGE_TITLE_VALUE_PLAY => {
                let box_frontend_pipe_ptr = structure.get_u64(PTR_FIELD)? as usize;
                let uri = structure.get_string(URI_FIELD)?;
                Ok(Message::Play(box_frontend_pipe_ptr, uri))
            }
            MESSAGE_TITLE_VALUE_PAUSE => Ok(Message::Pause),
            MESSAGE_TITLE_VALUE_STOP => Ok(Message::Stop),
            other => Err(format!("the message name `{other}` is not supported.")),
        }
    }
    pub fn to_structure(self, name: &str) -> Result<Structure, String> {
        match self {
            Message::None => Structure::new(
                name,
                vec![(structure_field::new_box_string(TITLE_FIELD, MESSAGE_TITLE_VALUE_NONE))],
            ),
            Self::Play(box_frontend_pipe_ptr, uri) => Structure::new(
                name,
                vec![
                    (structure_field::new_box_string(TITLE_FIELD, MESSAGE_TITLE_VALUE_PLAY)),
                    (structure_field::new_box_u64(PTR_FIELD, box_frontend_pipe_ptr as u64)),
                    (structure_field::new_box_string(URI_FIELD, &uri)),
                ],
            ),
            Message::Pause => Structure::new(
                name,
                vec![(structure_field::new_box_string(TITLE_FIELD, MESSAGE_TITLE_VALUE_PAUSE))],
            ),
            Message::Stop => Structure::new(
                name,
                vec![(structure_field::new_box_string(TITLE_FIELD, MESSAGE_TITLE_VALUE_STOP))],
            ),
        }
    }
}
