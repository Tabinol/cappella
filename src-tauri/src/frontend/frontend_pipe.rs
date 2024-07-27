use std::{fmt::Debug, sync::Arc};

use crate::tauri::tauri_app_handle::TauriAppHandle;

const PLAYER_EVENT_NAME: &str = "PLAYER_EVENT";

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub(crate) enum Message {
    #[default]
    None,
}

pub(crate) trait FrontendPipe: Debug {
    fn send(&self, message: Message);
}

#[derive(Debug)]
pub(crate) struct ImplFrontendPipe {
    tauri_app_handle: Arc<dyn TauriAppHandle>,
}

impl ImplFrontendPipe {
    pub(crate) fn new(tauri_app_handle: Arc<dyn TauriAppHandle>) -> ImplFrontendPipe {
        Self { tauri_app_handle }
    }
}

impl FrontendPipe for ImplFrontendPipe {
    fn send(&self, message: Message) {
        //(*self.app_handle).emit_to(MAIN_WINDOW_LABEL, event, payload);
    }
}
