use std::fmt::Debug;

use tauri::{AppHandle, Error, EventTarget};

use crate::frontend::frontend_pipe;

pub(crate) trait TauriAppHandle: Debug {
    fn emit_to(
        &self,
        window_label: &str,
        event: &str,
        message: frontend_pipe::Message,
    ) -> Result<(), Error>;
}

impl TauriAppHandle for AppHandle {
    fn emit_to(
        &self,
        window_label: &str,
        event: &str,
        message: frontend_pipe::Message,
    ) -> Result<(), Error> {
        tauri::Emitter::emit_to(self, EventTarget::window(window_label), event, message)
    }
}
